# KUI DevTools MCP Server — Changelog

All notable changes to this tool follow [Semantic Versioning](https://semver.org).

---

## [Unreleased]

### Added

- 录制合计（`kui_perf_record_get` / `kui_perf_capture_interaction` 的 totals）新增 `backendParagraphShapeCacheLayoutFirstHits`：后端整形缓存命中里，由 LayoutContext 段落缓存未命中驱动、第一次查表就命中的部分（T-PERF-SHAPE-CACHE-HIT-ATTRIBUTION，2026-10-31）。与 `backendParagraphShapeCacheHits`（整帧所有查表命中）本就不相等。

## [1.4.0] — 2026-09-16

### Added

- `kui_perf_capture_interaction`：一次调用在同一条 WebSocket 上完成「（可选预演）→ 开始录制 → 点击 / 按键 / 输入 → 等待 → 停止」，返回有工作的帧、合计，以及按整支搬回结局聚合的布局重跑节点。此前 agent 需要分几次工具调用，调用间隔一长录制就会被时长上限停掉。
- `kui_perf_record_get(detail="compact", layout_frames_only=...)`、`kui_perf_frame(detail=...)`、`kui_perf_hot_nodes(detail="compact", min_layout_runs=...)`：裁剪输出（去零值、去说明性常量，数值不改）；`min_layout_runs` 自动翻页并按 `layoutTopologyTransfer` 聚合。
- compact 帧包含引擎新增的 `layout.paths / engineMs / subPhases`（layout 分段耗时）与 `work.paragraphCacheWidthVariantMisses`。

### Changed

- `kui_perf_record_get` 与 `kui_perf_hot_nodes` 默认改为 `detail="compact"`；需要原始协议数据时传 `detail="full"`。一次点击的 attribution 录制原先 `hot_nodes(limit=200)` 约 220 KB、`record_get` 每页带整份内存分类说明，经常超出 MCP 输出上限。
- `kui_perf_record_start` 的 `max_duration_ms` 与 `lease_ttl_ms` 默认由 30 s 改为 120 s，并在说明里写明 `stopReason` 的 `timeout`（时长上限）与 `lease-expired`（租约过期）区别。
- 工具总数 79 → 80；README 版本头由过期的 1.3.0 更正为 1.4.0。

## [1.3.1] — 2026-09-10

### Fixed

- WebSocket 接收上限由 `websockets` 默认的 1 MiB 提升到 16 MiB。此前
  `kui_perf_export_recording`（上限 4 MiB）与 README 推荐的
  `kui_perf_record_get(max_frames=512)` 只要响应超过 1 MiB 就会被以
  “message too big” 关闭连接，工具调用直接失败而不是返回证据。

### Changed

- 工具表补齐 `kui_perf_trace`、`kui_perf_reveal_node`、
  `kui_perf_export_recording`，工具总数由文档中的 76 更正为实际的 79。
- `kui_perf_resolve_node` 的描述改为与实现一致：返回安全截断的
  tag/id/class 元数据与存活状态，不返回 selector。
- 明确 `confirm_export` 是调用方声明，引擎无法验证是否真有人工确认。

## [1.3.0] — 2026-09-09

### Added

- 新 `Performance.getCapabilities/startRecording/stopRecording/getRecording/getFrameDetails/getHotNodes/getNodeHistory/resolveNode/bufferUsage` 暴露为 9 个细粒度渲染性能工具，覆盖帧工作量、脏区、CPU/内存/GPU availability、热点节点、失效原因与缓冲丢失。
- MCP 的短连接录制固定使用 `lifecycle=leased`，session id 仅保存在当前 MCP 进程；连接关闭不会误停录制，启停和读取均需显式工具调用。

### Changed

- 工具版本升级为 1.3.0，工具总数由 66 增至 75；旧 `kui_performance`、`kui_timeline` 与 Lua CPU profiling 保持兼容。

## [1.2.1] — 2026-08-11

### Changed

- README 新增多应用/多 agent 的标准目标识别流程：应用记录不含 token 的精确 `kui-<实际端口>`，agent 先发现、按 id 连接并复核页面身份后再执行写操作；明确标题子串仅适合唯一标题。

### Fixed

- 修复多 agent 同时调试多个 KUI 应用时的连接状态串扰：`kui_connect()` 不再读写全机共享的 `%TEMP%/kui_mcp_state.json`，而是将当前 target 和 auth token 保存在各自的 stdio MCP server 进程内。一个 agent 切换或断开 target 不再影响其他 agent，token 也不再由 MCP 持久化到磁盘。

## [1.2.0] — 2026-08-08

### Added

- 新增 `kui_render_stats`、`kui_repaint_rects`、`kui_render_option` 三个渲染观测/运行时开关工具。
- 新增 `kui_set_style_property`，可按 selector 修改任意元素的单个内联 CSS 属性并设置 `!important`。

### Changed

- 新增独立 `VERSION` 文件，MCP initialize 响应现在上报 KUI DevTools MCP 自身版本，不再把 Python FastMCP 依赖版本误报为工具版本。
- 文档补充 Codex CLI/桌面端安装方式，连接模型更正为逐调用短连接，工具总数更新为 66。
- SDK 发版仍以 `tools/kui_devtools_mcp/` 为唯一权威源，并在完整发版自检中逐文件核对压缩包副本。

## [1.1.1] — 2026-06-30

### Changed

- **`kui_render_stats` docstring 补 `animationSources`(KUI-070)** —— 引擎 3.44.0 在 `Performance.renderStats` 新增 `animationSources` 动画源分量(`animationTimerActive`/`activeTransitions`/`activeAnimations`/`lottie`/`view3d`/`scriptRaf`),用于诊断 idle 持续全窗渲染的真实源。MCP 工具透传 CDP JSON、无需改逻辑,仅更新工具描述以便发现该字段。

## [1.1.0] — 2026-06-14

### Added

- **KUI-039 统一发现：`kui_targets()`** —— 列出本机全部活跃 KUI 应用/窗口(借助引擎跨进程聚合的 `/json`，并对 9229.. 端口段并发扫描去重兜底兼容旧引擎)。
- **`kui_connect(port, target=...)`** —— 多应用场景按 target id(如 `kui-9230`)或 title 子串选择并切换「当前连接」；`target` 留空时优先连 `port` 上的应用、否则第一个。返回结果附列其它可切换 target。
- **`kui_status`** —— 现额外列出全部可发现 target，并标注当前连接。

### Notes

- 随 KUI SDK 3.16.0 发布。需引擎 ≥ 3.16.0 才有跨进程聚合 `/json`；对旧引擎自动降级为端口扫描发现。

---

## [1.0.1] — 2026-05-30

### Fixed

- **`kui_outer_html`**：KUI 的 `DOM.getOuterHTML` 把标记字符串直接放在 CDP
  `result` 字段（标准 CDP 包装为 `{"outerHTML": ...}`）。原实现假定 `result`
  为 dict 并调用 `.get()`，对 KUI 后端抛出 `'str' object has no attribute 'get'`。
  现兼容 `str` / `dict` 两种返回形态。

### Verified

- 经 `kui_html_loader --devtools` + MCP **stdio 客户端**端到端验证：61 个工具全部
  正确暴露；`kui_connect` / `kui_page_info` / `kui_box_model` / `kui_get_style` /
  `kui_set_style`（写操作实时重绘）/ `kui_outer_html` / `kui_screenshot` /
  `kui_console` 等核心工具数据准确。

### Docs

- README 新增 **Claude Code (CLI)** 安装指引：`claude mcp add` 命令、
  local / project / user 三种作用域对比、`.mcp.json` 入库共享与安全批准、
  相对路径可移植性提醒、常用管理命令；故障排查补充「Claude Code 看不到
  `kui_*` 工具」一节。

---

## [1.0.0] — 2026-05-28

### Added

- Initial release bundled with KUI SDK v3.3.0
- **61 MCP tools** covering all KUI DevTools CDP command domains:
  - **Connection** (3): `kui_connect`, `kui_disconnect`, `kui_status`
  - **Page / Screenshot** (7): `kui_screenshot`, `kui_reload`, `kui_page_info`,
    `kui_layout_metrics`, `kui_scroll_to`, `kui_scroll_info`, `kui_network_resources`
  - **Input injection** (5): `kui_click`, `kui_mouse_move`, `kui_scroll`,
    `kui_type`, `kui_key`
  - **DOM query** (5): `kui_dom`, `kui_outer_html`, `kui_box_model`,
    `kui_element_at`, `kui_event_listeners`
  - **DOM mutation** (5): `kui_set_attr`, `kui_remove_attr`, `kui_set_text`,
    `kui_set_html`, `kui_dispatch_event`
  - **CSS** (5): `kui_get_style`, `kui_set_style`, `kui_matched_rules`,
    `kui_remove_style`, `kui_force_pseudo`
  - **Diagnostics** (5): `kui_console`, `kui_eval`, `kui_accessibility`,
    `kui_performance`, `kui_timeline`
  - **Profiling** (2): `kui_profile_start`, `kui_profile_stop`
  - **Screencast** (3): `kui_screencast_start`, `kui_screencast_stop`,
    `kui_screencast_save`
  - **Storage** (4): `kui_storage_get_all`, `kui_storage_set`,
    `kui_storage_remove`, `kui_storage_clear`
  - **Push events** (3): `kui_events`, `kui_wait_event`, `kui_send`
  - **Lua Debugger** (13): `kui_debugger_enable`, `kui_debugger_disable`,
    `kui_debugger_state`, `kui_script_sources`, `kui_breakpoint_set`,
    `kui_breakpoint_remove`, `kui_breakpoints`, `kui_exception_breaks`,
    `kui_pause`, `kui_resume`, `kui_step`, `kui_call_stack`, `kui_scope`,
    `kui_debugger_eval`
- Automatic auth-token extraction from `GET /json` CDP discovery endpoint
- Persistent WebSocket connection with asyncio `Future`-based request/response matching
- Push-event ring buffer (capacity 200) with one-shot `wait_event()` support
- Graceful reconnect error messages with actionable hints
- `kui_send` raw CDP gateway for commands not covered by dedicated tools
