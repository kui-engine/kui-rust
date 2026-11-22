#!/usr/bin/env python3
"""
KUI DevTools MCP Server
=======================
Exposes KUI application debugging via the Chrome DevTools Protocol (CDP)
as Model Context Protocol (MCP) tools, enabling Windsurf's AI agent to
directly inspect and interact with running KUI applications.

Transport: stdio (Windsurf default)
Protocol:  CDP over WebSocket, token-auth handled automatically

Architecture: short-lived CDP connections with MCP-process-local state
  kui_connect() saves ws_url in this stdio MCP server process.
  Every other tool opens a fresh WebSocket, sends ONE (or more) CDP command(s),
  waits for the response(s), then closes.  Each agent owns a separate stdio MCP
  process, so multiple agents can select different KUI targets without sharing
  credentials or overwriting each other's current connection.

Usage (Windsurf mcp_config.json / user settings):
  {
    "mcpServers": {
      "kui-devtools": {
        "command": "python",
        "args": ["<SDK_PATH>/tools/devtools_mcp/server.py"]
      }
    }
  }

Requirements:
  pip install mcp websockets httpx
  (or: pip install -r tools/kui_devtools_mcp/requirements.txt)
"""

from __future__ import annotations

import asyncio
import base64
import json
import os
import re
import sys
import threading
from typing import Any

import httpx
import websockets
import websockets.exceptions
from mcp.server.fastmcp import FastMCP

# MCP 工具自身的版本独立于 KUI SDK 和 Python `mcp` 依赖版本。VERSION 会随
# 发版镜像进 sdk/cpp/tools/devtools_mcp，缺失时宁可启动失败，也不静默把
# FastMCP 框架版本冒充为本工具版本。
with open(os.path.join(os.path.dirname(__file__), "VERSION"), encoding="utf-8") as _version_file:
    __version__ = _version_file.read().strip()

# ── MCP server instance ───────────────────────────────────────────────────────
# DevTools performance responses are far larger than the websockets default
# 1 MiB receive limit: `Performance.exportRecording` alone is capped at 4 MiB
# and a 512-frame `Performance.getRecording` page routinely exceeds 1 MiB.
# Without raising this the connection is closed with "message too big" and the
# tool call fails instead of returning evidence.
_WS_MAX_MESSAGE_BYTES = 16 * 1024 * 1024

mcp = FastMCP(
    "kui-devtools",
    instructions=(
        "Debug and automate KUI applications via the Chrome DevTools Protocol. "
        "Call kui_connect() first to attach to a running KUI app."
    ),
)
# 当前 FastMCP 构造器没有公开 version 参数；其 low-level Server 支持版本字段。
# 显式写入后，Codex/Claude 等客户端在 initialize 响应中看到的是 KUI MCP
# 版本，而不是本机安装的 Python mcp 包版本。
mcp._mcp_server.version = __version__

# ── MCP-process-local connection state ────────────────────────────────────────
# A stdio MCP server process belongs to one agent/client.  Keep the selected
# target (including its auth token) in that process instead of a machine-wide
# TEMP file: separate agents are isolated automatically, disconnecting one
# cannot clear another, and credentials no longer need to be persisted to disk.
# Module globals survive the short-lived asyncio.run() contexts used below.
_STATE_LOCK = threading.RLock()
_STATE: dict[str, Any] = {}


# Default DevTools base port; KUI windows allocate 9229, 9230, ... upward.
_DEFAULT_PORT = 9229
# How many consecutive ports kui_targets()/kui_connect() scan when probing for
# live KUI applications (covers ~16 windows/instances on one machine).
_SCAN_SPAN = 16


def _state_save(ws_url: str, port: int, target_id: str = "", title: str = "") -> None:
    with _STATE_LOCK:
        _STATE.clear()
        _STATE.update({"ws_url": ws_url, "port": port,
                       "target_id": target_id, "title": title})


def _state_load() -> tuple[str, int]:
    """Return (ws_url, port) or ('', 0) if not connected. (back-compat shape)"""
    with _STATE_LOCK:
        if not _STATE:
            return "", 0
        return _STATE.get("ws_url", ""), _STATE.get("port", _DEFAULT_PORT)


def _state_load_full() -> dict:
    """Return the full saved connection dict, or {} if not connected."""
    with _STATE_LOCK:
        return dict(_STATE)


def _state_clear() -> None:
    with _STATE_LOCK:
        _STATE.clear()


def _performance_session_id(session_id: str = "") -> str:
    """Resolve an explicit or process-local fine-grained recording id."""
    if session_id:
        return str(session_id)
    with _STATE_LOCK:
        saved = str(_STATE.get("performance_session_id", ""))
    if not saved:
        raise RuntimeError(
            "No performance recording session. Call kui_perf_record_start() "
            "or pass session_id explicitly."
        )
    return saved


def _remember_performance_session(session_id: str) -> None:
    with _STATE_LOCK:
        _STATE["performance_session_id"] = str(session_id)


def _stop_performance_session_best_effort() -> str:
    """Stop this process's leased recording before dropping connection state."""
    with _STATE_LOCK:
        session_id = str(_STATE.get("performance_session_id", ""))
        connected = bool(_STATE.get("ws_url"))
    if not session_id or not connected:
        return ""
    try:
        _one("Performance.stopRecording", {"sessionId": session_id})
        return f"Stopped leased performance session {session_id}."
    except Exception as exc:  # disconnect must still forget credentials/state
        return f"Could not stop leased performance session {session_id}: {exc}"


# ── Target discovery (KUI-039: unified "one address, all targets") ───────────

def _ws_port(ws_url: str) -> int:
    """Extract the TCP port from a ws://host:PORT/path URL (0 if absent)."""
    m = re.search(r"ws://[^/:]+:(\d+)", ws_url or "")
    return int(m.group(1)) if m else 0


async def _discover_async(base_port: int = _DEFAULT_PORT,
                          span: int = _SCAN_SPAN) -> list[dict]:
    """Discover all live KUI targets on localhost.

    KUI's /json endpoint aggregates every live target across processes
    (KUI-039), so a single GET usually returns the full set. We still scan a
    small port range concurrently and de-duplicate, so discovery also works
    against older single-target engines and when the base port is not bound.
    """
    ports = list(range(base_port, base_port + max(1, span)))

    async with httpx.AsyncClient() as client:
        async def probe(p: int):
            try:
                r = await client.get(f"http://localhost:{p}/json", timeout=1.0)
                r.raise_for_status()
                data = r.json()
                return data if isinstance(data, list) else []
            except Exception:
                return []

        lists = await asyncio.gather(*[probe(p) for p in ports])

    seen: set[str] = set()
    out: list[dict] = []
    for lst in lists:
        for t in lst:
            ws = t.get("webSocketDebuggerUrl", "")
            if not ws or ws in seen:
                continue
            seen.add(ws)
            out.append({
                "id": t.get("id", ""),
                "title": t.get("title", "KUI Document"),
                "url": t.get("url", ""),
                "ws_url": ws,
                "port": _ws_port(ws),
            })
    out.sort(key=lambda d: d["port"])
    return out


# ── Core CDP executor ─────────────────────────────────────────────────────────

async def _exec_async(
    commands: list[tuple[str, dict]],
    timeout: float = 10.0,
) -> list[dict]:
    """
    Open a fresh WebSocket, send all commands (in order), collect all
    matching responses by id, close, return results list.

    Push-only events (no id) are silently ignored.
    """
    ws_url, _ = _state_load()
    if not ws_url:
        raise RuntimeError("Not connected to KUI. Call kui_connect() first.")

    n = len(commands)
    pending: set[int] = set(range(1, n + 1))
    results: list[Any] = [None] * n

    async with websockets.connect(
        ws_url,
        open_timeout=5.0,
        ping_interval=None,  # no background keepalive task
        close_timeout=2.0,
        max_size=_WS_MAX_MESSAGE_BYTES,
    ) as ws:
        for idx, (method, params) in enumerate(commands, start=1):
            await ws.send(json.dumps({"id": idx, "method": method, "params": params}))

        deadline = asyncio.get_event_loop().time() + timeout
        async for raw in ws:
            try:
                d: dict = json.loads(raw)
            except json.JSONDecodeError:
                continue

            cmd_id = d.get("id")
            if cmd_id in pending:
                pending.discard(cmd_id)
                if "error" in d:
                    e = d["error"]
                    msg = e.get("message", str(e)) if isinstance(e, dict) else str(e)
                    raise RuntimeError(f"CDP error from '{commands[cmd_id - 1][0]}': {msg}")
                results[cmd_id - 1] = d.get("result", {})
                if not pending:
                    break

            if asyncio.get_event_loop().time() > deadline:
                raise TimeoutError(
                    f"Timeout ({timeout}s) waiting for CDP responses. "
                    "Is the KUI main thread calling devtools.poll()?"
                )

    return results


def _run_async(coro, timeout: float = 15.0):
    """Run a coroutine in a dedicated thread to avoid nested-event-loop errors.

    FastMCP itself runs inside an asyncio event loop, so asyncio.run() cannot
    be called directly from tool functions.  Spawning a daemon thread gives us
    a clean, loop-free context for each CDP round-trip.
    """
    result: list = [None]
    exc:    list = [None]

    def _worker():
        try:
            result[0] = asyncio.run(coro)
        except Exception as e:
            exc[0] = e

    t = threading.Thread(target=_worker, daemon=True)
    t.start()
    t.join(timeout=timeout)
    if t.is_alive():
        raise TimeoutError(f"CDP async operation timed out after {timeout}s")
    if exc[0] is not None:
        raise exc[0]
    return result[0]


def _one(method: str, params: dict | None = None, timeout: float = 10.0) -> dict:
    """Send a single CDP command; return its result dict."""
    return _run_async(_exec_async([(method, params or {})], timeout))[0]


def _many(
    commands: list[tuple[str, dict]], timeout: float = 10.0
) -> list[dict]:
    """Send multiple CDP commands over one connection; return list of results."""
    return _run_async(_exec_async(commands, timeout))


# ── Helpers ───────────────────────────────────────────────────────────────────

def _j(data: Any, indent: int = 2) -> str:
    return json.dumps(data, ensure_ascii=False, indent=indent)


# ── Helpers: compact performance evidence for agents ─────────────────────────
# 性能录制的原始 JSON 面向机器与导出：每帧带全部后端/GPU 字段，每个资源样本带整份
# 内存分类说明，热点节点带全部计数器。一次点击的 attribution 录制轻易超过 MCP 的
# 输出上限。compact 视图只做裁剪（去零值、去说明性常量、保留原字段名与数值），
# 不改变任何数值的含义；需要逐字段核对时仍可用 detail="full" 读取原始数据。

_FRAME_RENDER_KEEP = (
    "mode", "presented", "damageRectCount", "damagePercentViewport",
    "layersTotal", "layersRastered", "backendReplayCpuMs", "presentCpuMs",
    "backendParagraphBuildCpuMs", "layoutTextMeasureCpuMs", "layoutLineBreakCpuMs",
)
_NODE_IDENTITY_KEYS = ("performanceNodeId", "nodeName", "id", "className", "live",
                       "layoutTopologyTransfer")


def _round(value: Any) -> Any:
    return round(value, 3) if isinstance(value, float) else value


def _nonzero(mapping: Any, *, min_abs: float = 0.0) -> dict:
    if not isinstance(mapping, dict):
        return {}
    out = {}
    for key, value in mapping.items():
        if isinstance(value, bool) or value is None:
            continue
        if isinstance(value, (int, float)) and abs(value) > min_abs:
            out[key] = _round(value)
    return out


def _compact_frame(frame: dict) -> dict:
    """裁剪一帧：保留耗时、非零工作量、layout 路径与分段、整支搬回统计和少量渲染字段。"""
    out: dict[str, Any] = {"frameId": frame.get("frameId"),
                           "totalMs": _round(frame.get("totalMs"))}
    if frame.get("updatePassCount", 0) != 1:
        out["updatePassCount"] = frame.get("updatePassCount")
    if frame.get("forcedGeometryFlush"):
        out["forcedGeometryFlush"] = True
    out["phases"] = _nonzero(frame.get("phases"))
    out["work"] = _nonzero(frame.get("work"))
    layout = frame.get("layout") or {}
    compact_layout: dict[str, Any] = {"boxesLive": layout.get("boxesLive")}
    if layout.get("passes"):
        compact_layout["paths"] = _nonzero(layout.get("paths"))
        if layout.get("fullRebuildReasons"):
            compact_layout["fullRebuildReasons"] = layout.get("fullRebuildReasons")
        compact_layout["engineMs"] = _round(layout.get("engineMs"))
        compact_layout["subPhases"] = _nonzero(layout.get("subPhases"), min_abs=0.0005)
    transfer = layout.get("topologyTransfer") or {}
    if transfer.get("rebuilds"):
        compact_transfer = _nonzero({k: v for k, v in transfer.items() if k != "outcomes"})
        compact_transfer["outcomes"] = transfer.get("outcomes") or {}
        compact_layout["topologyTransfer"] = compact_transfer
    out["layout"] = compact_layout
    render = frame.get("render") or {}
    compact_render = {}
    for key in _FRAME_RENDER_KEEP:
        value = render.get(key)
        if value in (None, 0, 0.0, False):
            continue
        compact_render[key] = _round(value)
    if compact_render:
        out["render"] = compact_render
    return out


def _frame_has_work(frame: dict) -> bool:
    work = frame.get("work") or {}
    return any(work.get(key) for key in (
        "stylesResolved", "layoutDispatchVisits", "paintBoxPhaseVisits",
        "displayListOpsEmitted")) or bool((frame.get("layout") or {}).get("passes"))


def _frame_has_layout_work(frame: dict) -> bool:
    layout = frame.get("layout") or {}
    paths = layout.get("paths") or {}
    return bool((frame.get("work") or {}).get("layoutDispatchVisits")) or any(
        paths.get(key) for key in ("selective", "fullRebuild"))


def _compact_resource(sample: dict) -> dict:
    cpu = sample.get("cpu") or {}
    memory = sample.get("memory") or {}
    gpu = sample.get("gpu") or {}
    out: dict[str, Any] = {"sequence": sample.get("sequence"),
                           "elapsedMs": _round(sample.get("elapsedMs"))}
    if cpu.get("availability") == "available":
        out["cpuPercentOneCore"] = _round(cpu.get("percentOneCore"))
    if memory.get("availability") == "available":
        out["workingSetBytes"] = memory.get("workingSetBytes")
        out["privateBytes"] = memory.get("privateBytes")
        out["kuiAccountedBytes"] = memory.get("kuiAccountedBytes")
    if gpu.get("engineAvailability") == "available":
        out["gpuMaxEnginePercent"] = _round(gpu.get("maxEnginePercent"))
    return out


def _compact_recording(recording: dict, layout_frames_only: bool = False) -> dict:
    frames = recording.get("frames") or []
    kept = [f for f in frames if (_frame_has_layout_work(f) if layout_frames_only
                                  else True)]
    summary = recording.get("summary") or {}
    buffer = summary.get("buffer") or {}
    out = {
        "detail": "compact",
        "sessionId": recording.get("sessionId"),
        "active": recording.get("active"),
        "mode": (recording.get("effectiveConfig") or {}).get("mode"),
        "frames": [_compact_frame(f) for f in kept],
        "framesOmitted": len(frames) - len(kept),
        "cursor": recording.get("cursor"),
        "hasMore": recording.get("hasMore"),
        "summary": {
            "frameCount": summary.get("frameCount"),
            "work": _nonzero(summary.get("work")),
            "loss": _nonzero({k: v for k, v in buffer.items() if k.startswith("dropped")}),
        },
        "resources": [_compact_resource(s) for s in recording.get("resources") or []],
        "resourceCursor": recording.get("resourceCursor"),
        "resourceHasMore": recording.get("resourceHasMore"),
        "activityEventCount": len(recording.get("activityEvents") or []),
    }
    return out


def _compact_node(node: dict) -> dict:
    out = {key: node.get(key) for key in _NODE_IDENTITY_KEYS
           if node.get(key) not in (None, "")}
    for key, value in node.items():
        if key in _NODE_IDENTITY_KEYS or key.endswith("FrameId") or key == "nodeKey":
            continue
        if isinstance(value, bool) or value in (None, 0, 0.0):
            continue
        if isinstance(value, (int, float)):
            out[key] = _round(value)
    return out


def _layout_runs_by_transfer_outcome(nodes: list[dict]) -> dict:
    groups: dict[str, dict[str, int]] = {}
    for node in nodes:
        runs = node.get("layoutRuns") or 0
        if not runs:
            continue
        # none：节点没有整树重建结局（新插入的节点没有旧盒，或本次录制里没有发生整树重建）
        key = node.get("layoutTopologyTransfer") or "none"
        group = groups.setdefault(key, {"nodes": 0, "layoutRuns": 0})
        group["nodes"] += 1
        group["layoutRuns"] += runs
    return dict(sorted(groups.items(), key=lambda item: -item[1]["layoutRuns"]))


# ── Tools: Connection management ─────────────────────────────────────────────

@mcp.tool()
def kui_connect(port: int = 9229, target: str = "") -> str:
    """Connect to a running KUI application's DevTools server.

    KUI's /json endpoint aggregates ALL live KUI targets on the machine
    (KUI-039 unified discovery), so this discovers every running app/window —
    not just the one on `port`. Call kui_targets() first to list them.

    Args:
        port:   DevTools base port to query/scan (default 9229). Discovery also
                scans the next several ports, so this rarely needs changing.
        target: Optional selector when multiple apps are running — match by
                target id (e.g. "kui-9230") or a case-insensitive substring of
                the title. Empty = pick the app on `port`, else the first one.
    """
    async def _do():
        targets = await _discover_async(base_port=port)
        if not targets:
            raise RuntimeError(
                f"No KUI DevTools found on localhost:{port}.."
                f"{port + _SCAN_SPAN - 1}. Make sure a KUI app is running and "
                "calls startDevTools()."
            )

        chosen = None
        if target:
            tl = target.lower()
            for t in targets:
                if t["id"] == target or tl in t["title"].lower():
                    chosen = t
                    break
            if chosen is None:
                listing = "\n".join(
                    f"  - {t['id']}  port={t['port']}  title={t['title']!r}"
                    for t in targets
                )
                raise RuntimeError(
                    f"No target matched {target!r}. Available:\n{listing}"
                )
        else:
            # Prefer the app exactly on `port`; otherwise the first discovered.
            for t in targets:
                if t["port"] == port:
                    chosen = t
                    break
            if chosen is None:
                chosen = targets[0]

        ws_url = chosen["ws_url"]
        # Verify the WebSocket is reachable before saving.
        async with websockets.connect(
            ws_url, open_timeout=5.0, ping_interval=None, close_timeout=2.0,
            max_size=_WS_MAX_MESSAGE_BYTES,
        ):
            pass

        _state_save(ws_url, chosen["port"], chosen["id"], chosen["title"])

        others = [t for t in targets if t["ws_url"] != ws_url]
        extra = ""
        if others:
            extra = ("\n\nOther live KUI targets "
                     "(switch with kui_connect(target=...)):\n" + "\n".join(
                         f"  - {t['id']}  port={t['port']}  title={t['title']!r}"
                         for t in others))
        return (
            f"Connected to KUI DevTools\n"
            f"  Target: {chosen['id']}\n"
            f"  Port  : {chosen['port']}\n"
            f"  Title : {chosen['title']}\n"
            f"  WS    : {ws_url}" + extra
        )

    return _run_async(_do())


@mcp.tool()
def kui_disconnect() -> str:
    """Stop any leased performance recording and disconnect from DevTools."""
    cleanup = _stop_performance_session_best_effort()
    _state_clear()
    suffix = f"\n{cleanup}" if cleanup else ""
    return "Disconnected from KUI DevTools" + suffix


@mcp.tool()
def kui_status() -> str:
    """Return current connection status plus all discoverable KUI targets."""
    full = _state_load_full()
    ws_url = full.get("ws_url", "")
    lines: list[str] = []
    if not ws_url:
        lines.append("Status: NOT CONNECTED (call kui_connect() first)")
    else:
        lines.append("Status: CONNECTED")
        lines.append(f"  Target: {full.get('target_id', '?')}")
        lines.append(f"  Port  : {full.get('port', '?')}")
        lines.append(f"  Title : {full.get('title', '?')}")
        lines.append(f"  WS    : {ws_url}")
        try:
            info = _one("Page.getInfo")
            lines.append(f"  Page  : {_j(info)}")
        except Exception as e:
            lines.append(f"  (CDP call failed: {e})")
    try:
        targets = _run_async(_discover_async())
        if targets:
            lines.append("\nDiscoverable KUI targets:")
            for t in targets:
                mark = "*" if t["ws_url"] == ws_url else " "
                lines.append(
                    f" {mark} {t['id']}  port={t['port']}  title={t['title']!r}")
        else:
            lines.append("\nNo live KUI targets discovered.")
    except Exception as e:
        lines.append(f"\n(target discovery failed: {e})")
    return "\n".join(lines)


@mcp.tool()
def kui_targets(port: int = 9229) -> str:
    """List all live KUI applications/windows discoverable on localhost.

    Thanks to KUI-039 unified discovery, KUI's /json endpoint aggregates every
    live target across processes; this scans from `port` upward and returns the
    de-duplicated set. Use the printed id/title with kui_connect(target=...).

    Args:
        port: Base port to query/scan (default 9229).
    """
    targets = _run_async(_discover_async(base_port=port))
    if not targets:
        return (f"No live KUI targets on localhost:{port}.."
                f"{port + _SCAN_SPAN - 1}.\n"
                "Start a KUI app with startDevTools() (or --devtools).")
    cur = _state_load_full().get("ws_url", "")
    lines = [f"Found {len(targets)} live KUI target(s):"]
    for t in targets:
        mark = "  (connected)" if t["ws_url"] == cur else ""
        lines.append(f"  - {t['id']}  port={t['port']}  title={t['title']!r}{mark}")
        lines.append(f"      {t['ws_url']}")
    return "\n".join(lines)


# ── Tools: Raw CDP ────────────────────────────────────────────────────────────

@mcp.tool()
def kui_send(method: str, params_json: str = "{}") -> str:
    """Send any CDP command to KUI and return the raw JSON response.

    Use this for commands not covered by the specialized kui_* tools.

    Args:
        method:      CDP method, e.g. "DOM.getDocument", "CSS.getMatchedRules"
        params_json: JSON object string of parameters, e.g. '{"depth": 3}'
    """
    try:
        params = json.loads(params_json)
    except json.JSONDecodeError as e:
        return f"ERROR: params_json is not valid JSON — {e}"
    return _j(_one(method, params))


# ── Tools: Page / Document ────────────────────────────────────────────────────

@mcp.tool()
def kui_screenshot(output_path: str = "kui_screenshot.png") -> str:
    """Capture a PNG screenshot of the KUI window and save it to disk.

    Args:
        output_path: Destination file path for the PNG.
                     Absolute path recommended; relative = working directory.
    """
    result = _one("Page.captureScreenshot")
    b64 = result.get("data", "")
    if not b64:
        return f"ERROR: No screenshot data in response: {result}"

    png_bytes = base64.b64decode(b64)
    abs_path = os.path.abspath(output_path)
    parent = os.path.dirname(abs_path)
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(abs_path, "wb") as f:
        f.write(png_bytes)

    return (
        f"Screenshot saved: {abs_path}\n"
        f"Size: {result.get('width')}×{result.get('height')} px, "
        f"{len(png_bytes):,} bytes"
    )


@mcp.tool()
def kui_reload() -> str:
    """Reload the HTML document currently loaded in the KUI window."""
    _one("Page.reload")
    return "Page reload triggered"


@mcp.tool()
def kui_layout_metrics() -> str:
    """Get viewport and content layout dimensions of the KUI window."""
    return _j(_one("Page.getLayoutMetrics"))


@mcp.tool()
def kui_performance() -> str:
    """Get performance metrics: frame time, layout count, paint count, etc."""
    return _j(_one("Performance.snapshot"))


@mcp.tool()
def kui_render_stats(window: int = 120) -> str:
    """Get render-mode stats (FR-DEVTOOLS-RENDEROBS).

    Returns a summary (framesPainted/framesReused/skipRate, work-fps vs
    present-fps, layer R/C/S counts, partialDamageEnabled, plus animationSources
    — KUI-070 idle full-frame diagnosis: the per-source breakdown of what keeps
    the engine producing a frame every vblank: animationTimerActive (master
    switch; false = on-demand rendering engaged, frame-skip NOT regressed),
    activeTransitions, activeAnimations, lottie, view3d, scriptRaf) plus the
    most recent `window` per-frame records (mode = full|partial|layer|skipped).
    Enabling this also turns on the per-frame repaint recording ring.

    Args:
        window: number of recent frames to include (max 240).
    """
    return _j(_one("Performance.renderStats", {"window": window}))


@mcp.tool()
def kui_repaint_rects(since_frame: int = 0) -> str:
    """Get the rectangles that repainted, incrementally since a frame index.

    Returns {frames:[{i,mode,overflow,rects:[{x,y,w,h}]}], lastFrame}.
    Poll with since_frame = lastFrame to stream only new repaints (the data
    behind the DevTools paint-flashing overlay).

    Args:
        since_frame: only return frames with index > this (0 = all recent).
    """
    return _j(_one("Performance.repaintRects", {"sinceFrame": since_frame}))


@mcp.tool()
def kui_render_option(name: str = "", enabled: bool = True, value: int = -1) -> str:
    """Get or set runtime render-optimization toggles (FR-DEVTOOLS-RENDEROBS-004).

    With no `name`, returns the current options (retainedDLSkip,
    retainedSurface, layerPromotion, partialDamage, selectiveLayout,
    ifcAlternateSlot, layerLimit) with their value/availability.
    Turning `ifcAlternateSlot` off releases every IFC result-cache second slot
    (one laid-out SkParagraph each) immediately.  With a `name`, flips that
    optimization live (bool options use `enabled`; layerLimit uses `value`).

    Args:
        name:    option to set; empty = list all.
        enabled: target on/off for boolean options.
        value:   integer value for layerLimit (>=0); ignored for bools.
    """
    if not name:
        return _j(_one("Rendering.getOptions"))
    params = {"name": name, "enabled": enabled}
    if value >= 0:
        params["value"] = value
    return _j(_one("Rendering.setOption", params))


# ── Tools: Fine-grained render performance recording (R-29 Phase 7) ─────────

@mcp.tool()
def kui_perf_capabilities() -> str:
    """Report exact performance schema, modes, limits, and CPU/memory/GPU availability.

    Availability is explicit: unsupported or failed providers are never returned
    as a numeric zero. This call does not start recording.
    """
    return _j(_one("Performance.getCapabilities"))


@mcp.tool()
def kui_perf_record_start(
    mode: str = "attribution",
    max_frames: int = 300,
    max_duration_ms: int = 120000,
    resource_interval_ms: int = 1000,
    gpu_sampling: bool = False,
    lease_ttl_ms: int = 120000,
    node_details: bool = True,
    categories: list[str] | None = None,
    max_nodes: int = 0,
    buffer_size: int = 0,
) -> str:
    """Explicitly start a bounded fine-grained render recording.

    The MCP transport opens a new WebSocket for each tool call, so this always
    requests `lifecycle=leased`; closing that short-lived socket must not stop
    the recording. Always finish with kui_perf_record_stop(), then retrieve the
    retained evidence with kui_perf_record_get().

    To measure one interaction (click / key / typing), prefer
    kui_perf_capture_interaction(): it records, performs the input and stops in a
    single call, so agent latency between tool calls cannot end the recording.

    The recording stops by itself when `max_duration_ms` elapses
    (stopReason "timeout") or when no tool call touches the session for
    `lease_ttl_ms` (stopReason "lease-expired"). Agent tool calls are often
    tens of seconds apart, so both defaults are 120 s here.

    Args:
        mode: summary | timeline | attribution | deep-trace.
        max_frames: Retained-frame cap (1..4096).
        max_duration_ms: Automatic duration cap in milliseconds (stopReason "timeout").
        resource_interval_ms: CPU/memory sample interval (minimum 250 ms).
        gpu_sampling: Opt in to Windows PDH GPU sampling; may affect tail latency.
        lease_ttl_ms: Stop after this long without a session call (stopReason "lease-expired").
        node_details: Promote timeline to attribution for per-node evidence.
        categories: Optional timing/work/render/resources/updatePasses/nodes/trace subset.
        max_nodes: Attribution node cap; 0 uses the engine default.
        buffer_size: Requested bounded buffer budget in bytes; 0 uses defaults.
    """
    params = {
        "mode": mode,
        "lifecycle": "leased",
        "maxFrames": max_frames,
        "maxDurationMs": max_duration_ms,
        "resourceIntervalMs": resource_interval_ms,
        "gpuSampling": gpu_sampling,
        "leaseTtlMs": lease_ttl_ms,
        "nodeDetails": node_details,
    }
    if categories is not None:
        params["categories"] = list(categories)
    if max_nodes:
        params["maxNodes"] = max_nodes
    if buffer_size:
        params["bufferSize"] = buffer_size
    result = _one("Performance.startRecording", params)
    session_id = result.get("sessionId") if isinstance(result, dict) else None
    if session_id:
        _remember_performance_session(str(session_id))
    return _j(result)


@mcp.tool()
def kui_perf_record_stop(session_id: str = "") -> str:
    """Explicitly stop a fine-grained recording without discarding its evidence.

    Args:
        session_id: Recording id; empty uses the last session started by this
                    MCP process.
    """
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.stopRecording", {"sessionId": sid}))


@mcp.tool()
def kui_perf_record_get(
    session_id: str = "", cursor: str = "0", max_frames: int = 120,
    resource_cursor: str = "0", max_resource_samples: int = 120,
    detail: str = "compact", layout_frames_only: bool = False,
) -> str:
    """Read a retained recording page, including summary, work, and resources.

    This is an explicit read; the MCP server never polls in the background.
    Follow `hasMore=true` using the returned `cursor` to page through long runs.

    `detail="compact"` (default) keeps each frame's timing, non-zero work
    counters, layout path / sub-phase timing, topology-transfer statistics and a
    few render fields, and drops per-sample memory-category descriptions; values
    are not altered. Use `detail="full"` for the raw protocol payload.

    Args:
        session_id: Recording id; empty uses this MCP process's last session.
        cursor: Return frame ids greater than this cursor (default 0).
        max_frames: Maximum returned frames for this page (1..512).
        resource_cursor: Return resource sequences greater than this cursor.
        max_resource_samples: Maximum resource samples for this page (1..512).
        detail: compact | full.
        layout_frames_only: compact only — omit frames that did no layout work
                            (the omitted count is reported as framesOmitted).
    """
    if detail not in ("compact", "full"):
        raise RuntimeError("detail must be 'compact' or 'full'")
    sid = _performance_session_id(session_id)
    recording = _one("Performance.getRecording", {
        "sessionId": sid, "cursor": str(cursor), "maxFrames": max_frames,
        "resourceCursor": str(resource_cursor),
        "maxResourceSamples": max_resource_samples,
    })
    if detail == "full" or not isinstance(recording, dict):
        return _j(recording)
    return _j(_compact_recording(recording, layout_frames_only))


@mcp.tool()
def kui_perf_frame(frame_id: str, session_id: str = "", detail: str = "full") -> str:
    """Read exact phase, work, damage, layer, DOM, and display-list data for one frame.

    Args:
        frame_id: Frame id from a recording page.
        session_id: Recording id; empty uses the last session.
        detail: full (default, raw protocol frame) | compact (same trimming as
                kui_perf_record_get).
    """
    if detail not in ("compact", "full"):
        raise RuntimeError("detail must be 'compact' or 'full'")
    sid = _performance_session_id(session_id)
    frame = _one("Performance.getFrameDetails", {
        "sessionId": sid, "frameId": str(frame_id),
    })
    if detail == "full" or not isinstance(frame, dict) or "frameId" not in frame:
        return _j(frame)
    return _j(_compact_frame(frame))


@mcp.tool()
def kui_perf_hot_nodes(
    session_id: str = "", sort_by: str = "estimatedCost", limit: int = 50,
    detail: str = "compact", min_layout_runs: int = 0,
) -> str:
    """Rank high-frequency nodes by invalidation, style, layout, paint, or DL work.

    Attribution mode is required for node-level results.

    `detail="compact"` (default) returns each node's identity, non-zero counters
    and its `layoutTopologyTransfer` outcome only; `detail="full"` returns the
    raw protocol page. `min_layout_runs > 0` switches to layout ranking, pages
    through the recording until nodes fall below the threshold, and adds
    `layoutRunsByTransferOutcome` (nodes and layout runs grouped by the node's
    last topology-transfer outcome).

    Args:
        session_id: Recording id; empty uses the last session.
        sort_by: estimatedCost | dirty | style | layout | paint | displayList |
                 raster (DeepTrace submitted drawable ops; not GPU completion).
        limit: Maximum nodes returned (1..200; with min_layout_runs, the cap on
               listed nodes — grouping still covers every qualifying node).
        detail: compact | full.
        min_layout_runs: Only nodes whose layout algorithm ran at least this often.
    """
    if detail not in ("compact", "full"):
        raise RuntimeError("detail must be 'compact' or 'full'")
    sid = _performance_session_id(session_id)
    if min_layout_runs <= 0:
        page = _one("Performance.getHotNodes", {
            "sessionId": sid, "sortBy": sort_by, "limit": limit,
        })
        if detail == "full" or not isinstance(page, dict) or "nodes" not in page:
            return _j(page)
        return _j({
            "detail": "compact",
            "sortBy": page.get("sortBy"),
            "totalNodes": page.get("totalNodes"),
            "incomplete": page.get("incomplete"),
            "nextCursor": page.get("nextCursor"),
            "hasMore": page.get("hasMore"),
            "nodes": [_compact_node(n) for n in page.get("nodes") or []],
        })
    return _j(_collect_layout_nodes(
        lambda params: _one("Performance.getHotNodes", params),
        sid, min_layout_runs, limit, detail))


def _collect_layout_nodes(fetch, sid: str, min_layout_runs: int, limit: int,
                          detail: str = "compact") -> dict:
    """按 layout 排序分页读取，直到重跑次数低于阈值；返回按搬回结局的分组与前 limit 个节点。"""
    qualifying: list[dict] = []
    cursor = "0"
    incomplete = False
    total = 0
    while True:
        page = fetch({"sessionId": sid, "sortBy": "layout", "limit": 200, "cursor": cursor})
        if not isinstance(page, dict) or "nodes" not in page:
            return page if isinstance(page, dict) else {"error": str(page)}
        total = page.get("totalNodes") or total
        incomplete = incomplete or bool(page.get("incomplete"))
        nodes = page.get("nodes") or []
        below = False
        for node in nodes:
            if (node.get("layoutRuns") or 0) < min_layout_runs:
                below = True
                break
            qualifying.append(node)
        if below or not page.get("hasMore") or not nodes:
            break
        cursor = str(page.get("nextCursor"))
    listed = qualifying[:max(1, min(limit, 200))]
    return {
        "detail": detail,
        "sortBy": "layout",
        "minLayoutRuns": min_layout_runs,
        "recordedNodes": total,
        "incomplete": incomplete,
        "qualifyingNodes": len(qualifying),
        "qualifyingLayoutRuns": sum(n.get("layoutRuns") or 0 for n in qualifying),
        "layoutRunsByTransferOutcome": _layout_runs_by_transfer_outcome(qualifying),
        "listedNodes": len(listed),
        "nodes": listed if detail == "full" else [_compact_node(n) for n in listed],
    }


@mcp.tool()
def kui_perf_node_history(
    performance_node_id: str, session_id: str = "", cursor: str = "0",
    max_events: int = 200,
) -> str:
    """Read ordered dirty events, related update passes, and node totals."""
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.getNodeHistory", {
        "sessionId": sid, "performanceNodeId": str(performance_node_id),
        "cursor": str(cursor), "maxEvents": max_events,
    }))


@mcp.tool()
def kui_perf_resolve_node(performance_node_id: str, session_id: str = "") -> str:
    """Resolve a recorded node to a stable session/document-scoped identity.

    The returned backendNodeId is stable only within the recording's document
    epoch. Use kui_perf_reveal_node() for a fresh Elements-panel locator;
    never persist the ephemeral DFS selector.
    """
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.resolveNode", {
        "sessionId": sid, "performanceNodeId": str(performance_node_id),
    }))


@mcp.tool()
def kui_perf_reveal_node(
    performance_node_id: str, document_epoch: str,
    session_id: str = "", highlight: bool = False,
) -> str:
    """Reveal a live recorded node in Elements using a fresh DOM locator.

    Args:
        performance_node_id: Session-scoped node id returned by hot-nodes.
        document_epoch: Exact epoch returned by start/resolveNode.
        session_id: Recording id; empty uses the last session.
        highlight: Also paint the DevTools highlight overlay.

    Frozen, detached, or epoch-mismatched nodes return an explicit unavailable
    result instead of an unsafe stale selector.
    """
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.revealNode", {
        "sessionId": sid,
        "documentEpoch": str(document_epoch),
        "performanceNodeId": str(performance_node_id),
        "highlight": highlight,
    }))


@mcp.tool()
def kui_perf_export_recording(
    session_id: str = "", confirm_export: bool = False,
    max_bytes: int = 4 * 1024 * 1024,
) -> str:
    """Export a stopped recording as one bounded, privacy-redacted JSON payload.

    Args:
        session_id: Recording id; empty uses the last session.
        confirm_export: Must be true to assert an explicit user-approved export.
        max_bytes: UTF-8 payload limit, 1024..4194304 bytes.

    The engine excludes DOM text, input values, URLs, tokens, selectors,
    element ids/classes, and stable GPU adapter/engine identifiers. Active
    recordings must be stopped first so the returned payload is atomic.
    """
    if not confirm_export:
        raise RuntimeError(
            "Export requires explicit confirmation: set confirm_export=True."
        )
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.exportRecording", {
        "sessionId": sid,
        "userInitiated": True,
        "maxBytes": max_bytes,
    }))


@mcp.tool()
def kui_perf_buffer(session_id: str = "") -> str:
    """Read buffer use, dropped evidence counts, and the suggested lower-cost mode."""
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.bufferUsage", {"sessionId": sid}))


@mcp.tool()
def kui_perf_insights(session_id: str = "") -> str:
    """Return evidence-backed dirty/style/layout/paint/damage/resource/buffer cards."""
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.getInsights", {"sessionId": sid}))


@mcp.tool()
def kui_perf_trace(
    session_id: str = "", cursor: str = "0", max_events: int = 200,
) -> str:
    """Read one bounded DeepTrace event page.

    Events contain paint-owner DisplayList ranges, concrete backend replay
    receipts, operation-type distributions, surface metadata, and present-call
    receipts. GPU/display completion remains explicitly unsupported unless a
    backend provider reports it.
    """
    sid = _performance_session_id(session_id)
    return _j(_one("Performance.getTrace", {
        "sessionId": sid, "cursor": str(cursor), "maxEvents": max_events,
    }))


async def _capture_interaction_async(
    mode: str, action: str, x: float, y: float, selector: str, key: str,
    text: str, modifiers: int, warmup_action: bool, warmup_ms: int,
    settle_ms: int, max_frames: int,
) -> dict:
    """在同一条 WebSocket 上完成：（可选预演）→ 开始录制 → 输入 → 等待 → 停止。"""
    ws_url, _ = _state_load()
    if not ws_url:
        raise RuntimeError("Not connected to KUI. Call kui_connect() first.")
    async with websockets.connect(
        ws_url, open_timeout=5.0, ping_interval=None, close_timeout=2.0,
        max_size=_WS_MAX_MESSAGE_BYTES,
    ) as ws:
        next_id = 0

        async def call(method: str, params: dict | None = None) -> dict:
            nonlocal next_id
            next_id += 1
            request_id = next_id
            await ws.send(json.dumps({"id": request_id, "method": method,
                                      "params": params or {}}))
            deadline = asyncio.get_event_loop().time() + 10.0
            while True:
                remaining = deadline - asyncio.get_event_loop().time()
                if remaining <= 0:
                    raise TimeoutError(f"Timeout waiting for '{method}'. "
                                       "Is the KUI main thread calling devtools.poll()?")
                raw = await asyncio.wait_for(ws.recv(), timeout=remaining)
                try:
                    message = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                if message.get("id") != request_id:
                    continue  # 推送事件
                if "error" in message:
                    error = message["error"]
                    detail = error.get("message", str(error)) if isinstance(error, dict) else str(error)
                    raise RuntimeError(f"CDP error from '{method}': {detail}")
                return message.get("result", {})

        target: dict[str, Any] = {}
        if action == "click":
            if selector:
                box = await call("DOM.getBoxModel", {"selector": selector})
                model = box.get("model", box) if isinstance(box, dict) else {}
                quad = model.get("border") or model.get("content")
                if not quad or len(quad) < 8:
                    raise RuntimeError(f"No box model for selector {selector!r}: {box}")
                x = (quad[0] + quad[2]) / 2.0
                y = (quad[1] + quad[5]) / 2.0
            if x < 0 or y < 0:
                raise RuntimeError("click needs selector or non-negative x/y")
            target = {"x": round(x, 2), "y": round(y, 2)}

        async def perform() -> None:
            if action == "click":
                base = {"x": x, "y": y, "button": "left", "clickCount": 1,
                        "modifiers": modifiers}
                await call("Input.dispatchMouseEvent",
                           {**base, "type": "mouseMoved", "button": "none"})
                await call("Input.dispatchMouseEvent", {**base, "type": "mousePressed"})
                await call("Input.dispatchMouseEvent", {**base, "type": "mouseReleased"})
            elif action == "key":
                await call("Input.dispatchKeyEvent",
                           {"type": "keyDown", "key": key, "modifiers": modifiers})
                await call("Input.dispatchKeyEvent",
                           {"type": "keyUp", "key": key, "modifiers": modifiers})
            elif action == "type":
                for ch in text:
                    await call("Input.dispatchKeyEvent",
                               {"type": "char", "key": ch, "text": ch})

        if warmup_action and action != "none":
            await perform()
            await asyncio.sleep(max(0, settle_ms) / 1000.0)
        started = await call("Performance.startRecording", {
            "mode": mode, "lifecycle": "leased", "maxFrames": max_frames,
            "maxDurationMs": max(10000, warmup_ms + settle_ms + 10000),
            "resourceIntervalMs": 1000, "leaseTtlMs": 60000,
            "nodeDetails": mode in ("attribution", "deep-trace"),
        })
        session_id = str(started.get("sessionId", ""))
        if not session_id:
            raise RuntimeError(f"startRecording returned no sessionId: {started}")
        try:
            await asyncio.sleep(max(0, warmup_ms) / 1000.0)
            await perform()
            await asyncio.sleep(max(0, settle_ms) / 1000.0)
        finally:
            stopped = await call("Performance.stopRecording", {"sessionId": session_id})
        return {"sessionId": session_id, "stopped": stopped, "target": target}


@mcp.tool()
def kui_perf_capture_interaction(
    action: str = "click",
    selector: str = "",
    x: float = -1.0,
    y: float = -1.0,
    key: str = "",
    text: str = "",
    modifiers: int = 0,
    mode: str = "attribution",
    warmup_action: bool = False,
    warmup_ms: int = 200,
    settle_ms: int = 500,
    max_frames: int = 64,
    node_limit: int = 30,
) -> str:
    """Record one interaction in a single call and return a compact per-frame summary.

    Sequence (one WebSocket): optional warm-up action → start recording →
    wait `warmup_ms` → perform the input → wait `settle_ms` → stop. Then reads
    the frozen recording and returns only frames that did style/layout/paint
    work, trimmed like kui_perf_record_get(detail="compact"), plus totals. In
    attribution mode it also returns layout-run nodes grouped by their
    topology-transfer outcome and the top `node_limit` layout nodes.

    The session id is remembered, so kui_perf_frame / kui_perf_hot_nodes /
    kui_perf_node_history can drill down afterwards.

    Args:
        action: click | key | type | none (none = just record `warmup_ms+settle_ms`).
        selector: click target; its border-box center is used (overrides x/y).
        x, y: click position in viewport CSS pixels when no selector is given.
        key: DOM key name for action="key" (keyDown + keyUp).
        text: characters for action="type".
        modifiers: 1=Alt, 2=Ctrl, 4=Meta, 8=Shift.
        mode: timeline (timing only, lowest overhead) | attribution | deep-trace.
        warmup_action: perform the same input once before recording, so the
                       recorded pass is not a first-time (cold cache) run.
        warmup_ms: delay between start and input.
        settle_ms: delay after input before stopping (and after the warm-up).
        max_frames: retained-frame cap for this recording.
        node_limit: top layout nodes listed (attribution modes).
    """
    if action not in ("click", "key", "type", "none"):
        raise RuntimeError("action must be click | key | type | none")
    if mode not in ("timeline", "attribution", "deep-trace"):
        raise RuntimeError("mode must be timeline | attribution | deep-trace")
    if action == "key" and not key:
        raise RuntimeError("action='key' needs key")
    if action == "type" and not text:
        raise RuntimeError("action='type' needs text")
    budget = 20.0 + (warmup_ms + 2 * settle_ms) / 1000.0 + len(text) * 0.05
    captured = _run_async(_capture_interaction_async(
        mode, action, x, y, selector, key, text, modifiers, warmup_action,
        warmup_ms, settle_ms, max_frames), timeout=budget)
    sid = captured["sessionId"]
    _remember_performance_session(sid)
    recording = _one("Performance.getRecording", {
        "sessionId": sid, "cursor": "0", "maxFrames": 512,
        "resourceCursor": "0", "maxResourceSamples": 1,
    })
    frames = (recording.get("frames") or []) if isinstance(recording, dict) else []
    worked = [f for f in frames if _frame_has_work(f)]
    totals: dict[str, Any] = {"frames": len(frames), "framesWithWork": len(worked)}
    for key_name in ("totalMs",):
        totals[key_name] = round(sum(f.get(key_name) or 0 for f in worked), 3)
    for phase in ("styleMs", "layoutMs", "paintMs", "compositeMs"):
        totals[phase] = round(sum((f.get("phases") or {}).get(phase) or 0 for f in worked), 3)
    for counter in ("stylesResolved", "layoutDispatchVisits", "layoutAlgorithmRuns",
                    "paragraphCacheMisses", "paragraphCacheWidthVariantMisses",
                    "backendParagraphBuilds", "backendParagraphShapeCacheHits",
                    "backendParagraphMeasureCalls", "backendParagraphShapeCacheLayoutFirstHits",
                    "inlineParagraphShapeRepeats", "inlineParagraphShapeLruHits",
                    "ifcResultCacheHits", "ifcResultCacheMissThrash",
                    "displayListOpsEmitted"):
        totals[counter] = sum((f.get("work") or {}).get(counter) or 0 for f in worked)
    totals["fullRebuilds"] = sum(
        ((f.get("layout") or {}).get("paths") or {}).get("fullRebuild") or 0 for f in worked)
    stopped = captured.get("stopped") or {}
    result: dict[str, Any] = {
        "sessionId": sid,
        "mode": mode,
        "action": action,
        "target": captured.get("target") or None,
        "stopReason": stopped.get("stopReason"),
        "loss": _nonzero(stopped.get("loss")),
        "totals": totals,
        "frames": [_compact_frame(f) for f in worked],
    }
    if mode in ("attribution", "deep-trace"):
        result["layoutNodes"] = _collect_layout_nodes(
            lambda params: _one("Performance.getHotNodes", params),
            sid, 1, node_limit)
    return _j(result)


# ── Tools: Input injection ────────────────────────────────────────────────────

@mcp.tool()
def kui_click(
    x: float,
    y: float,
    button: str = "left",
    click_count: int = 1,
    modifiers: int = 0,
) -> str:
    """Click at viewport coordinates (CSS pixels) in the KUI window.

    Args:
        x:           X coordinate (viewport-relative, CSS pixels)
        y:           Y coordinate (viewport-relative, CSS pixels)
        button:      "left" | "middle" | "right" (default "left")
        click_count: 1 = single click, 2 = double-click
        modifiers:   Bitmask — 1=Alt, 2=Ctrl, 4=Meta, 8=Shift
    """
    base: dict[str, Any] = {
        "x": x, "y": y,
        "button": button,
        "clickCount": click_count,
        "modifiers": modifiers,
    }
    _many([
        ("Input.dispatchMouseEvent", {**base, "type": "mousePressed"}),
        ("Input.dispatchMouseEvent", {**base, "type": "mouseReleased"}),
    ])
    return f"Clicked ({x}, {y}) [{button}, count={click_count}, mod={modifiers}]"


@mcp.tool()
def kui_mouse_move(x: float, y: float) -> str:
    """Move the mouse cursor to viewport coordinates without clicking.

    Args:
        x: X coordinate (CSS pixels)
        y: Y coordinate (CSS pixels)
    """
    _one("Input.dispatchMouseEvent", {"type": "mouseMoved", "x": x, "y": y, "button": "none"})
    return f"Mouse moved to ({x}, {y})"


@mcp.tool()
def kui_scroll(
    x: float,
    y: float,
    delta_x: float = 0.0,
    delta_y: float = 120.0,
) -> str:
    """Scroll at the given coordinates.

    Args:
        x:       X coordinate of scroll origin (CSS pixels)
        y:       Y coordinate of scroll origin (CSS pixels)
        delta_x: Horizontal scroll delta in pixels (positive = right)
        delta_y: Vertical scroll delta in pixels (positive = down, default 120)
    """
    _one("Input.dispatchMouseEvent", {
        "type": "mouseWheel",
        "x": x, "y": y,
        "deltaX": delta_x, "deltaY": delta_y,
        "button": "none",
    })
    return f"Scrolled at ({x}, {y}), delta=({delta_x}, {delta_y})"


@mcp.tool()
def kui_type(text: str) -> str:
    """Type a string into the currently focused KUI element (char-by-char).

    Args:
        text: Plain text string to type. No escape sequences.
    """
    cmds = [
        ("Input.dispatchKeyEvent", {"type": "char", "key": ch, "text": ch})
        for ch in text
    ]
    if cmds:
        _many(cmds)
    preview = repr(text[:60]) + ("..." if len(text) > 60 else "")
    return f"Typed {len(text)} character(s): {preview}"


@mcp.tool()
def kui_key(
    key: str,
    event_type: str = "keyDown",
    modifiers: int = 0,
) -> str:
    """Send a keyboard event to the focused element.

    Common key names: "Enter", "Escape", "Tab", "Backspace", "Delete",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End",
    "PageUp", "PageDown", "F1"–"F12", "a"–"z", "0"–"9".

    Args:
        key:        DOM key name string
        event_type: "keyDown" | "keyUp" | "char"
        modifiers:  Bitmask — 1=Alt, 2=Ctrl, 4=Meta, 8=Shift
    """
    params: dict[str, Any] = {"type": event_type, "key": key, "modifiers": modifiers}
    if event_type == "char":
        params["text"] = key
    _one("Input.dispatchKeyEvent", params)
    return f"Key: {event_type} '{key}' (modifiers={modifiers})"


# ── Tools: DOM ────────────────────────────────────────────────────────────────

@mcp.tool()
def kui_dom(selector: str = "", depth: int = 3) -> str:
    """Get the DOM tree or a specific element's properties.

    Args:
        selector: CSS selector to locate a specific element.
                  Leave empty to get the full document tree.
        depth:    Tree depth when fetching the full document (1–10, default 3)
    """
    if selector:
        return _j(_one("DOM.querySelector", {"selector": selector}))
    return _j(_one("DOM.getDocument", {"depth": max(1, min(depth, 10))}))


@mcp.tool()
def kui_outer_html(selector: str) -> str:
    """Get the outer HTML markup of the element matching selector.

    Args:
        selector: CSS selector, e.g. "#main", ".card:first-child"
    """
    result = _one("DOM.getOuterHTML", {"selector": selector})
    # KUI returns the markup string directly in `result`, whereas standard CDP
    # wraps it as {"outerHTML": ...}. Accept both shapes.
    if isinstance(result, str):
        return result
    if isinstance(result, dict):
        return result.get("outerHTML", _j(result))
    return _j(result)


@mcp.tool()
def kui_box_model(selector: str) -> str:
    """Get layout box model quads (content / padding / border / margin) for an element.

    Returns four 8-float coordinate arrays (clockwise from top-left) plus
    element width and height in CSS pixels.

    Args:
        selector: CSS selector identifying the element
    """
    return _j(_one("DOM.getBoxModel", {"selector": selector}))


@mcp.tool()
def kui_set_attr(selector: str, name: str, value: str) -> str:
    """Set an HTML attribute on the element matching selector.

    Args:
        selector: CSS selector
        name:     Attribute name (e.g. "class", "data-active", "disabled")
        value:    Attribute value string
    """
    return _j(_one("DOM.setAttribute", {"selector": selector, "name": name, "value": value}))


# ── Tools: CSS ────────────────────────────────────────────────────────────────

@mcp.tool()
def kui_get_style(selector: str) -> str:
    """Get all computed CSS properties for the element matching selector.

    Args:
        selector: CSS selector, e.g. "#header", "button.primary"
    """
    return _j(_one("CSS.getFullComputedStyle", {"selector": selector}))


@mcp.tool()
def kui_set_style(selector: str, css_text: str) -> str:
    """Inject a CSS rule targeting selector into the document.

    This uses CSS.addRule which appends a new rule to the document's
    dynamic stylesheet — it does not overwrite existing rules.

    Args:
        selector: CSS selector for the new rule (e.g. "#box", ".item:hover")
        css_text: CSS declarations without braces (e.g. "color: red; display: none")

    Example:
        kui_set_style("#hero", "background: linear-gradient(#f00,#00f); opacity: 0.8")
    """
    return _j(_one("CSS.addRule", {"selector": selector, "cssText": css_text}))


@mcp.tool()
def kui_set_style_property(selector: str, prop: str, value: str, important: bool = False) -> str:
    """Set a single inline CSS property (element.style) on the element matching selector.

    Unlike kui_set_style (which injects a new stylesheet rule), this edits the
    element's own inline style — the highest-priority, single-element override,
    equivalent to what the DevTools Styles panel's "element.style {}" section edits.

    Args:
        selector:  CSS selector, e.g. "#header", "button.primary"
        prop:      CSS property name (e.g. "color", "display")
        value:     CSS value (e.g. "red", "flex")
        important: set the `!important` priority flag
    """
    return _j(_one("CSS.setProperty", {"selector": selector, "property": prop, "value": value, "important": important}))


# ── Tools: Console ────────────────────────────────────────────────────────────

@mcp.tool()
def kui_console() -> str:
    """Get all console messages (KUI_LOG_INFO/WARN/ERROR) from the KUI application."""
    return _j(_one("Console.getMessages"))


@mcp.tool()
def kui_eval(expression: str) -> str:
    """Evaluate a Lua expression in the KUI scripting engine and return the result.

    Requires the KUI app to have called window->attachScriptEngine().

    Args:
        expression: Lua expression string, e.g. 'document:getElementById("btn"):getAttribute("class")'
    """
    return _j(_one("Runtime.evaluate", {"expression": expression}))


# ── Tools: Storage ────────────────────────────────────────────────────────────

@mcp.tool()
def kui_storage_get_all(local: bool = True) -> str:
    """Get all key-value pairs from KUI's in-memory localStorage or sessionStorage.

    Args:
        local: True = localStorage (default), False = sessionStorage
    """
    return _j(_one("Storage.getDOMStorageItems", {"isLocalStorage": local}))


@mcp.tool()
def kui_storage_set(key: str, value: str, local: bool = True) -> str:
    """Set a key-value pair in KUI's in-memory localStorage or sessionStorage.

    Args:
        key:   Storage key string
        value: Value string to store
        local: True = localStorage (default), False = sessionStorage
    """
    return _j(_one("Storage.setDOMStorageItem", {
        "isLocalStorage": local, "key": key, "value": value,
    }))


# ── Tools: Push events ────────────────────────────────────────────────────────

@mcp.tool()
def kui_events(count: int = 20) -> str:
    """Collect push events from KUI for up to 0.5 s and return the last N.

    Note: In stateless mode this opens a fresh connection and collects
    whatever push events the server sends within the collection window.

    Args:
        count: Maximum number of events to return (default 20)
    """
    async def _collect() -> list[dict]:
        ws_url, _ = _state_load()
        if not ws_url:
            raise RuntimeError("Not connected to KUI. Call kui_connect() first.")
        events: list[dict] = []
        async with websockets.connect(
            ws_url, open_timeout=5.0, ping_interval=None, close_timeout=2.0,
            max_size=_WS_MAX_MESSAGE_BYTES,
        ) as ws:
            try:
                async with asyncio.timeout(0.5):
                    async for raw in ws:
                        d = json.loads(raw)
                        if "method" in d and d.get("id") is None:
                            events.append(d)
            except (asyncio.TimeoutError, TimeoutError):
                pass
        return events[-count:]

    events = asyncio.run(_collect())
    if not events:
        return "No push events received in collection window."
    return _j(events)


@mcp.tool()
def kui_wait_event(event_name: str, timeout: float = 5.0) -> str:
    """Block until KUI pushes a specific event, then return it.

    Useful for waiting for DOM mutations after a click before asserting state.

    Common event names:
      "DOM.childNodeInserted", "DOM.childNodeRemoved",
      "DOM.attributeModified", "DOM.characterDataModified",
      "Console.messageAdded", "Page.screencastFrame"

    Args:
        event_name: CDP event method name to wait for
        timeout:    Maximum seconds to wait (default 5.0)
    """
    async def _wait() -> dict:
        ws_url, _ = _state_load()
        if not ws_url:
            raise RuntimeError("Not connected to KUI. Call kui_connect() first.")
        async with websockets.connect(
            ws_url, open_timeout=5.0, ping_interval=None, close_timeout=2.0,
            max_size=_WS_MAX_MESSAGE_BYTES,
        ) as ws:
            deadline = asyncio.get_event_loop().time() + timeout
            async for raw in ws:
                d = json.loads(raw)
                if d.get("method") == event_name:
                    return d
                if asyncio.get_event_loop().time() > deadline:
                    raise TimeoutError(
                        f"Event '{event_name}' not received within {timeout}s"
                    )
        raise RuntimeError("WebSocket closed while waiting for event")

    return _j(asyncio.run(_wait()))


# ── Tools: Accessibility ──────────────────────────────────────────────────────

@mcp.tool()
def kui_accessibility() -> str:
    """Get the full Accessibility tree (AX tree) of the KUI document.

    Returns a tree of accessible nodes with roles, names, and properties.
    Useful for verifying ARIA attributes and screen-reader compatibility.
    """
    return _j(_one("Accessibility.getFullAXTree"))


# ── Tools: DOM extras ────────────────────────────────────────────────────────

@mcp.tool()
def kui_element_at(x: float, y: float) -> str:
    """Find the topmost element at the given viewport coordinates.

    Useful after a screenshot: identify which element is at a visual position
    before clicking or inspecting it.

    Args:
        x: X coordinate (CSS pixels, viewport-relative)
        y: Y coordinate (CSS pixels, viewport-relative)
    """
    return _j(_one("DOM.elementAtPoint", {"x": x, "y": y}))


@mcp.tool()
def kui_set_text(selector: str, text: str) -> str:
    """Set the text content of the element matching selector.

    Args:
        selector: CSS selector identifying the element
        text:     Plain text to set as the element's text content
    """
    return _j(_one("DOM.setTextContent", {"selector": selector, "text": text}))


@mcp.tool()
def kui_set_html(selector: str, html: str) -> str:
    """Replace the inner HTML of the element matching selector.

    Args:
        selector: CSS selector identifying the element
        html:     HTML markup to set as innerHTML
    """
    return _j(_one("DOM.setOuterHTML", {"selector": selector, "html": html}))


@mcp.tool()
def kui_remove_attr(selector: str, name: str) -> str:
    """Remove an HTML attribute from the element matching selector.

    Args:
        selector: CSS selector identifying the element
        name:     Attribute name to remove (e.g. "disabled", "class")
    """
    return _j(_one("DOM.removeAttribute", {"selector": selector, "name": name}))


@mcp.tool()
def kui_event_listeners(selector: str) -> str:
    """Get all event listeners attached to the element matching selector.

    Args:
        selector: CSS selector identifying the element
    """
    return _j(_one("DOM.getEventListeners", {"selector": selector}))


@mcp.tool()
def kui_dispatch_event(
    selector: str,
    event_type: str,
    bubbles: bool = True,
    cancelable: bool = True,
    detail: str = "",
) -> str:
    """Fire a synthetic DOM event on the element matching selector.

    Args:
        selector:   CSS selector identifying the target element
        event_type: Event type string, e.g. "click", "input", "my-custom-event"
        bubbles:    Whether the event bubbles up the DOM (default True)
        cancelable: Whether the event can be cancelled (default True)
        detail:     Optional JSON string stored in event.detail (never eval'd)
    """
    params: dict[str, Any] = {
        "selector": selector, "type": event_type,
        "bubbles": bubbles, "cancelable": cancelable,
    }
    if detail:
        params["detail"] = detail
    return _j(_one("DOM.dispatchEvent", params))


# ── Tools: CSS extras ─────────────────────────────────────────────────────────

@mcp.tool()
def kui_matched_rules(selector: str) -> str:
    """Get the CSS rules that match the element identified by selector.

    Returns all author and user-agent rules that apply, with their source
    (inline / stylesheet index / inherited).

    Args:
        selector: CSS selector identifying the element
    """
    return _j(_one("CSS.getMatchedRules", {"selector": selector}))


@mcp.tool()
def kui_remove_style(selector: str, prop: str) -> str:
    """Remove an inline CSS property (element.style) from the element matching selector.

    Works on any element, whether or not it has an HTML `id` attribute.

    Args:
        selector: CSS selector, e.g. "#header", "button.primary"
        prop:     CSS property name to remove (e.g. "color", "display")
    """
    return _j(_one("CSS.removeProperty", {"selector": selector, "property": prop}))


@mcp.tool()
def kui_force_pseudo(selector: str, pseudo: str) -> str:
    """Force a CSS pseudo-state on the element matching selector.

    Useful for inspecting :hover, :focus, :active, :visited styles without
    actually moving the mouse.

    Args:
        selector: CSS selector identifying the element
        pseudo:   Pseudo-state name: "hover" | "focus" | "active" | "visited"
    """
    return _j(_one("CSS.forcePseudoState", {"selector": selector, "pseudos": pseudo}))


# ── Tools: Page extras ────────────────────────────────────────────────────────

@mcp.tool()
def kui_page_info() -> str:
    """Get KUI page metadata: title, URL, scroll position, viewport size."""
    return _j(_one("Page.getInfo"))


@mcp.tool()
def kui_scroll_to(y: float) -> str:
    """Scroll the document to the given vertical position.

    Args:
        y: Target scroll Y position in CSS pixels
    """
    return _j(_one("Page.scrollTo", {"y": y}))


@mcp.tool()
def kui_scroll_info() -> str:
    """Get the current document scroll position and total scrollable height."""
    return _j(_one("Page.getScrollInfo"))


@mcp.tool()
def kui_network_resources() -> str:
    """Get all HTTP resources loaded by the KUI document (images, scripts, etc.)."""
    return _j(_one("Network.getResources"))


# ── Tools: Screencast ─────────────────────────────────────────────────────────

@mcp.tool()
def kui_screencast_start(every_nth_frame: int = 2) -> str:
    """Start continuous frame capture (screencast).

    KUI will push Page.screencastFrame events for every N-th rendered frame.
    Collect them with kui_events() or kui_wait_event('Page.screencastFrame').

    Args:
        every_nth_frame: Capture interval — 1 = every frame, 2 = every other, etc.
                         Higher values reduce overhead. Default 2.
    """
    result = _one("Page.startScreencast", {
        "format": "png", "everyNthFrame": max(1, every_nth_frame),
    })
    return f"Screencast started (every {every_nth_frame} frame(s))\n" + _j(result)


@mcp.tool()
def kui_screencast_stop() -> str:
    """Stop the running screencast and flush any remaining frames."""
    return "Screencast stopped\n" + _j(_one("Page.stopScreencast"))


@mcp.tool()
def kui_screencast_save(output_dir: str = ".", max_frames: int = 100) -> str:
    """Collect screencast frames for 3 s and save each as a numbered PNG file.

    Args:
        output_dir: Directory to write frame_0000.png, frame_0001.png, etc.
        max_frames: Cap on frames to save (avoids huge dumps, default 100)
    """
    async def _collect_frames() -> list[dict]:
        ws_url, _ = _state_load()
        if not ws_url:
            raise RuntimeError("Not connected. Call kui_connect() first.")
        frames: list[dict] = []
        async with websockets.connect(
            ws_url, open_timeout=5.0, ping_interval=None, close_timeout=2.0,
            max_size=_WS_MAX_MESSAGE_BYTES,
        ) as ws:
            try:
                async with asyncio.timeout(3.0):
                    async for raw in ws:
                        d = json.loads(raw)
                        if d.get("method") == "Page.screencastFrame":
                            frames.append(d.get("params", {}))
                            if len(frames) >= max_frames:
                                break
            except (asyncio.TimeoutError, TimeoutError):
                pass
        return frames

    os.makedirs(output_dir, exist_ok=True)
    frames = asyncio.run(_collect_frames())
    if not frames:
        return "No screencast frames received. Did you call kui_screencast_start first?"

    saved = 0
    for frame in frames:
        b64 = frame.get("data", "")
        if not b64:
            continue
        frame_num = frame.get("metadata", {}).get("frameNumber", saved)
        with open(os.path.join(output_dir, f"frame_{frame_num:04d}.png"), "wb") as f:
            f.write(base64.b64decode(b64))
        saved += 1

    return (
        f"Saved {saved} frame(s) to {os.path.abspath(output_dir)}/\n"
        f"Total frames received: {len(frames)}"
    )


# ── Tools: Performance profiling ─────────────────────────────────────────────

@mcp.tool()
def kui_profile_start() -> str:
    """Start the Lua script profiler, not a renderer/process CPU profiler."""
    return "Lua script profiling started\n" + _j(_one("Performance.startProfiling"))


@mcp.tool()
def kui_profile_stop() -> str:
    """Stop Lua script profiling and return data marked profileKind=lua."""
    results = _many([
        ("Performance.stopProfiling", {}),
        ("Performance.getProfilingResult", {}),
    ])
    return _j(results[1])


@mcp.tool()
def kui_timeline() -> str:
    """Get the performance event timeline (layout, paint, composite events)."""
    return _j(_one("Performance.getTimeline"))


# ── Tools: Debugger ───────────────────────────────────────────────────────────

@mcp.tool()
def kui_debugger_enable() -> str:
    """Enable the Lua script debugger.

    Must be called before setting breakpoints.
    Requires the KUI app to have called window->attachScriptEngine().
    """
    return "Debugger enabled\n" + _j(_one("Debugger.enable"))


@mcp.tool()
def kui_debugger_disable() -> str:
    """Disable the Lua script debugger and remove all breakpoints."""
    return "Debugger disabled\n" + _j(_one("Debugger.disable"))


@mcp.tool()
def kui_breakpoint_set(
    source: str,
    line: int,
    condition: str = "",
    log_message: str = "",
) -> str:
    """Set a breakpoint in a Lua script.

    Args:
        source:      Script source name or filename (e.g. "main.lua", "ui/handlers.lua")
        line:        1-based line number to break on
        condition:   Optional Lua expression; break only when it evaluates to true
        log_message: Optional log message (logpoint — logs but does not pause)
    """
    return _j(_one("Debugger.setBreakpoint", {
        "source": source, "line": line,
        "condition": condition, "logMessage": log_message,
    }))


@mcp.tool()
def kui_breakpoint_remove(breakpoint_id: int) -> str:
    """Remove a breakpoint by its numeric ID (returned by kui_breakpoint_set).

    Args:
        breakpoint_id: Numeric ID returned from kui_breakpoint_set
    """
    return _j(_one("Debugger.removeBreakpoint", {"breakpointId": breakpoint_id}))


@mcp.tool()
def kui_breakpoints() -> str:
    """List all active breakpoints with their source, line, and condition."""
    return _j(_one("Debugger.getBreakpoints"))


@mcp.tool()
def kui_debugger_state() -> str:
    """Get the current debugger state: running | paused | disabled.

    When paused, also returns the current source file and line number.
    """
    return _j(_one("Debugger.getState"))


@mcp.tool()
def kui_pause() -> str:
    """Pause Lua script execution at the next executed line."""
    return "Pause requested\n" + _j(_one("Debugger.pause"))


@mcp.tool()
def kui_resume() -> str:
    """Resume Lua script execution after a breakpoint or pause."""
    return "Resumed\n" + _j(_one("Debugger.resume"))


@mcp.tool()
def kui_step(action: str = "over") -> str:
    """Step through Lua script execution.

    Args:
        action: "over"  — step over the current line (do not enter function calls)
                "into"  — step into function calls
                "out"   — step out of the current function
    """
    valid = {"over", "into", "out"}
    if action not in valid:
        return f"ERROR: action must be one of {valid}, got '{action}'"
    method_map = {
        "over": "Debugger.stepOver",
        "into": "Debugger.stepInto",
        "out":  "Debugger.stepOut",
    }
    return f"Step {action}\n" + _j(_one(method_map[action]))


@mcp.tool()
def kui_call_stack() -> str:
    """Get the current Lua call stack (only valid when execution is paused).

    Returns a list of stack frames with source file, line, and function name.
    """
    return _j(_one("Debugger.getCallStack"))


@mcp.tool()
def kui_scope(frame_index: int = 0) -> str:
    """Get local variables and upvalues for a call stack frame.

    Args:
        frame_index: 0-based index into the call stack (0 = innermost frame)
    """
    return _j(_one("Debugger.getScope", {"frameIndex": frame_index}))


@mcp.tool()
def kui_debugger_eval(expression: str, frame_index: int = 0) -> str:
    """Evaluate a Lua expression in the context of a paused stack frame.

    Different from kui_eval: this evaluates in the paused frame's local scope,
    allowing access to local variables and upvalues.

    Args:
        expression:  Lua expression to evaluate
        frame_index: Stack frame index (0 = innermost, default 0)
    """
    return _j(_one("Debugger.evaluate", {
        "expression": expression, "frameIndex": frame_index,
    }))


@mcp.tool()
def kui_script_sources() -> str:
    """Get all Lua script source files known to the debugger.

    Returns source names, line counts, and whether each script is currently loaded.
    """
    return _j(_one("Debugger.getScriptSources"))


@mcp.tool()
def kui_exception_breaks(mode: str = "none") -> str:
    """Configure exception breakpoints for Lua runtime errors.

    Args:
        mode: "none"     — never break on exceptions (default)
              "uncaught" — break only on unhandled errors
              "all"      — break on every error (including pcall'd ones)
    """
    valid = {"none", "uncaught", "all"}
    if mode not in valid:
        return f"ERROR: mode must be one of {valid}, got '{mode}'"
    return f"Exception breakpoints: {mode}\n" + _j(
        _one("Debugger.setExceptionBreakpoints", {"state": mode})
    )


# ── Tools: Storage extras ─────────────────────────────────────────────────────

@mcp.tool()
def kui_storage_remove(key: str, local: bool = True) -> str:
    """Remove a key from KUI's in-memory localStorage or sessionStorage.

    Args:
        key:   Storage key to remove
        local: True = localStorage (default), False = sessionStorage
    """
    return _j(_one("Storage.removeDOMStorageItem", {"isLocalStorage": local, "key": key}))


@mcp.tool()
def kui_storage_clear() -> str:
    """Clear all KUI in-memory storage (both localStorage and sessionStorage)."""
    return "Storage cleared\n" + _j(_one("Storage.clearDataForOrigin"))


# ── Entry point ───────────────────────────────────────────────────────────────

if __name__ == "__main__":
    mcp.run()
