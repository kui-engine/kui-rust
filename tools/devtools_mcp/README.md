# KUI DevTools MCP Server

> **Applies to:** KUI SDK v3.3.0+  
> **MCP version:** 1.4.0
> **Python:** 3.10+  
> **Transport:** stdio (Codex / Claude Code / Windsurf / Cursor / Claude Desktop)

让 Codex、Claude Code、Windsurf 等 AI 编码助手通过 **Model Context Protocol (MCP)** 直接连接运行中的 KUI 应用，实现 agent 级别的 DOM 检查、布局调试、输入注入、Lua 脚本断点调试和截图录屏。

---

## 目录

1. [概述](#1-概述)
2. [安装](#2-安装)
3. [配置 AI 客户端](#3-配置-ai-客户端)
4. [KUI 应用侧配置](#4-kui-应用侧配置)
5. [快速上手](#5-快速上手)
6. [工具参考](#6-工具参考)
7. [使用场景示例](#7-使用场景示例)
8. [故障排查](#8-故障排查)

---

## 1. 概述

### 1.1 这是什么

`server.py` 是一个 **MCP server**，内部通过 Chrome DevTools Protocol (CDP) 的 WebSocket 接口与 KUI DevTools 通信。启动后，AI agent 无需任何额外代码即可：

- 截图查看当前 UI 状态
- 查询 / 修改 DOM 结构与 CSS 样式
- 注入鼠标 / 键盘事件
- 读取元素的精确布局尺寸
- 调试 Lua 脚本（设置断点 / 步进 / 查看变量）
- 捕获连续帧录屏

### 1.2 架构

```
AI Agent (Codex / Claude Code / Windsurf / Cursor)
        │  MCP tool calls (stdio)
        ▼
server.py  ←── 每次工具调用建立短连接 ──►  KUI 应用进程
  (asyncio + websockets)                  DevTools :9229
  自动从 /json 获取 auth token
```

`kui_connect()` 把带 token 的 WebSocket URL 仅保存在当前 stdio MCP server 进程内存中；后续每个工具调用读取本进程状态、建立一次 CDP WebSocket、收取响应后关闭。该状态可跨 FastMCP 的短生命 event loop 保留，但不持久化到磁盘。

### 1.3 多 agent / 多应用隔离

标准 stdio MCP 部署中，每个 agent/client 拥有自己的 `server.py` 进程。因此多个 agent 可分别调用 `kui_connect(target=...)` 选中不同 KUI 应用；当前 target、auth token 和 `kui_disconnect()` 只影响所属 MCP 进程，不会覆盖或断开其他 agent。MCP 进程重启后内存状态会清空，需重新调用 `kui_connect()`。

### 1.4 与直接使用 Chrome DevTools 的区别

| | Chrome DevTools (浏览器) | 本 MCP Server |
|--|--------------------------|--------------|
| 操作者 | 人工 | AI Agent |
| 接口 | 浏览器 UI | MCP 结构化工具 |
| Lua 调试 | ❌ | ✅ 断点/步进/变量 |
| 截图保存 | 手动 | 自动保存文件 |
| 自动化测试 | 需要 Playwright | 内置支持 |

---

## 2. 安装

```powershell
# 推荐：安装到当前 Python 环境
pip install -r sdk/cpp/tools/devtools_mcp/requirements.txt

# 或者单独安装
pip install mcp websockets httpx
```

**依赖版本：**

| 包 | 最低版本 | 说明 |
|---|---------|------|
| `mcp` | 1.0.0 | MCP Python SDK（FastMCP） |
| `websockets` | 12.0 | CDP WebSocket 客户端 |
| `httpx` | 0.27.0 | 从 `/json` 获取 auth token |

---

## 3. 配置 AI 客户端

### Codex CLI / 桌面端 / IDE 扩展

Codex 三个本地客户端共享 `~/.codex/config.toml`。仓库维护者应指向工程权威源；SDK 使用者则把路径换成 SDK 内的 `tools/devtools_mcp/server.py`：

```powershell
# 仓库开发：始终使用最新工程版
codex mcp add kui-devtools -- python <REPO_PATH>/tools/kui_devtools_mcp/server.py

# 确认注册结果
codex mcp get kui-devtools
codex mcp list
```

注册后重启 Codex 桌面端/IDE 扩展或新开任务，使新的 `kui_*` 工具进入工具清单。若机器上有多个 Python，建议把命令中的 `python` 换成已安装本目录 requirements 的解释器绝对路径。仓库根 `.mcp.json` 是 Claude Code 等客户端的项目配置，不会被 Codex 当作 `config.toml` 读取。

### Claude Code (CLI)

Claude Code 通过 `claude mcp add` 命令注册 MCP server，**无需手写 JSON**。在 SDK 目录下执行：

```bash
# 默认 local 作用域：仅当前用户 + 当前项目可见
claude mcp add kui-devtools python <SDK_PATH>/tools/devtools_mcp/server.py
```

`<SDK_PATH>` 替换为实际路径（解压后的 SDK 包根目录，如 `/path/to/kui-c-sdk`）。注册后**重载会话**（重新打开项目或新开对话），即可在对话中直接调用 `kui_connect`、`kui_screenshot` 等工具。

#### 作用域（决定"谁能用"）

| 作用域 | 命令参数 | 存储位置 | 可见范围 |
|--------|---------|---------|---------|
| **local**（默认） | （无） | `~/.claude.json` 项目段 | 仅你 + 当前项目 |
| **project** | `--scope project` | 仓库根 `.mcp.json`（**入库共享**） | clone 此仓库的任何人 |
| **user** | `--scope user` | `~/.claude.json` 全局段 | 你的所有项目 |

团队共享推荐 `--scope project`，它在仓库根生成 `.mcp.json` 并纳入版本控制：

```bash
claude mcp add --scope project kui-devtools python <SDK_PATH>/tools/devtools_mcp/server.py
```

> **首次加载提示：** Claude Code 出于安全考虑，对项目级 `.mcp.json` 中新出现的 MCP server 会在会话启动时**请求确认**，批准后工具才生效。用 `/mcp` 命令可随时查看已加载的 server 与工具列表。

> **可移植性：** `.mcp.json` 若写绝对路径（如 `/path/to/...`）只对本机有效。要跨机器 / 团队共享，建议把 `args` 改为相对仓库根的路径（如 `tools/kui_devtools_mcp/server.py`）——Claude Code 以工作区根解析相对路径，别人 clone 到任意盘符均可用。

#### 常用管理命令

```bash
claude mcp list                  # 列出已注册的 MCP server 及作用域
claude mcp get kui-devtools      # 查看单个 server 详情
claude mcp remove kui-devtools   # 移除（按作用域）
```

### Windsurf

打开 **Settings → MCP Servers → Edit Config**，添加：

```json
{
  "mcpServers": {
    "kui-devtools": {
      "command": "python",
      "args": ["<SDK_PATH>/tools/devtools_mcp/server.py"]
    }
  }
}
```

将 `<SDK_PATH>` 替换为 SDK 的实际路径，例如 `/path/to/kui-c-sdk`。

保存后 Windsurf **自动重启** MCP server，工具立即可用。

### Cursor

在 `.cursor/mcp.json` 中添加：

```json
{
  "mcpServers": {
    "kui-devtools": {
      "command": "python",
      "args": ["<SDK_PATH>/tools/devtools_mcp/server.py"]
    }
  }
}
```

### Claude Desktop

在 `claude_desktop_config.json` 中添加（路径见 Claude 文档）：

```json
{
  "mcpServers": {
    "kui-devtools": {
      "command": "python",
      "args": ["<SDK_PATH>/tools/devtools_mcp/server.py"]
    }
  }
}
```

---

## 4. KUI 应用侧配置

在你的 KUI 应用代码中启用 DevTools 服务器：

```cpp
#include <kui/devtools/devtools_server.h>

// 创建并附加 DevTools server
auto devtools = std::make_unique<kui::DevToolsServer>();
devtools->attach(window->document());
devtools->start(9229);   // 监听端口，默认 9229

// 在主事件循环中调用 poll()（必须）
while (app.isRunning()) {
    window->pollEvents();
    devtools->poll();    // 在主线程处理 DevTools 命令
}
```

> **重要：** `devtools->poll()` 必须在主线程的事件循环中每帧调用，否则 agent 的命令不会被处理。

使用 `Window::startDevTools()` 或 `--devtools` 的内建窗口集成时，`Window::processFrame()` 会自动在帧节奏早退前执行 `poll()`，应用无需再手工重复轮询。传输线程只负责唤醒主线程；只读 DOM/CSS 查询不会因此把 `Document` 标成待重绘，真正改变页面状态的命令仍由各自 handler 显式请求重绘。

### 可选：Lua 脚本调试

若需要 Debugger domain（断点 / 步进 / 变量查看）：

```cpp
devtools->attachScriptEngine(window->scriptEngine());
```

### 可选：禁用 Token 认证（仅开发环境）

```cpp
devtools->setTokenAuth(false);  // 取消 WebSocket token 校验
devtools->start(9229);
```

---

## 5. 快速上手

配置完成后，在 AI 对话中直接说：

```
连接到 KUI 应用（端口 9229），截图看一下当前状态
```

Agent 会依次调用：

```
kui_connect(port=9229)
kui_screenshot(output_path="debug.png")
```

然后读取截图并分析。

### 5.1 多个 KUI 应用与多个 agent

每个 KUI 窗口都应启动自己的 DevTools server。推荐让 KUI 自动分配未占用端口，并把不含认证 token 的实际端口和 target id 写入该应用自己的启动日志：

```cpp
#include <kui/kui.h>
#include <kui/devtools/devtools_server.h>
#include <cstdio>

if (auto* devtools = window->startDevTools(0)) {
    std::fprintf(stderr, "KUI DevTools target=kui-%u port=%u\n",
                 devtools->port(), devtools->port());
}
```

不要把 `window->devToolsUrl()` 写入日志；URL 包含该实例的认证 token。`startDevTools(0)` 会自动选端口；即使显式请求的端口被占用，最终也必须以 `devtools->port()` 返回的实际端口为准。

每个 agent 使用独立的 stdio MCP `server.py` 进程，并按以下顺序连接：

```text
1. 从自己启动/管理的 KUI 应用日志读取 target id，例如 kui-9231。
2. kui_targets(port=9229)                         # 发现全部活跃 target
3. kui_connect(port=9229, target="kui-9231")     # 用精确 id 选择自己的应用
4. kui_status()                                   # 核对 current target/port
5. kui_page_info()                                # 再核对标题、视口等页面身份
6. kui_screenshot("before.png")                  # 写操作前可选的视觉确认
7. 确认无误后再调用 kui_click/kui_set_style 等写工具
```

选择规则：

- **首选精确 target id `kui-<实际端口>`**。它能区分同名示例，也是多 agent 自动化应持久记录的身份。
- `target` 也接受 title 子串，但标题可能为空、重复，或在 DevTools 启动后发生变化；只应在结果唯一时作为人工便利入口。
- `kui_connect(port=...)` 的 `port` 是发现入口，`target` 才决定最终连接哪个应用。KUI-039 引擎连接任一活跃端口的 `/json` 通常都能聚合发现本机全部 target。
- 对旧引擎或使用相距较远的自定义端口，改用目标附近的端口调用 `kui_targets(port=<起始端口>)`；MCP 会从该端口开始扫描一段端口范围作为兜底。
- 多应用时不要省略 `target`，除非明确接受连接 `port` 对应应用或发现列表第一项。

版本 1.2.1 起，当前 target 和 token 只保存在所属 MCP 进程内存中。Agent A 的 `kui_connect()` / `kui_disconnect()` 不会改变 Agent B 的连接；MCP 进程重启后则需要重新执行以上发现与连接流程。

### 典型调试会话

```
你：帮我检查 #hero-banner 的布局，宽高和内边距是多少？

Agent：
  → kui_connect()
  → kui_box_model("#hero-banner")       // 精确像素坐标
  → kui_get_style("#hero-banner")       // 计算样式（含 padding/margin）

你：把它的背景色改成蓝色看看效果

Agent：
  → kui_set_style("#hero-banner", "background-color: #0066cc")
  → kui_screenshot("after.png")
```

---

## 6. 工具参考

### 6.1 连接管理

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_targets` | `port=9229` | **列出本机全部活跃 KUI 应用/窗口**(KUI-039 统一发现) |
| `kui_connect` | `port=9229, target=""` | 连接 KUI DevTools，**首先调用**；`target` 按 id(如 `kui-9230`)/title 子串选多应用之一 |
| `kui_disconnect` | — | 先 best-effort 停止本 MCP 进程的 leased 性能录制，再清除连接状态 |
| `kui_status` | — | 当前连接 + 可发现的全部 target |

### 6.2 截图与页面

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_screenshot` | `output_path="kui_screenshot.png"` | 截图保存 PNG |
| `kui_reload` | — | 重载文档 |
| `kui_page_info` | — | 标题/URL/视口尺寸/滚动位置 |
| `kui_layout_metrics` | — | 视口与内容布局尺寸 |
| `kui_box_model` | `selector` | 元素盒模型四层四边形（content/padding/border/margin）+ 宽高，CSS 像素 |
| `kui_scroll_to` | `y` | 滚动到指定 Y 坐标 |
| `kui_scroll_info` | — | 当前滚动位置 + 总高度 |
| `kui_network_resources` | — | 已加载资源列表 |

### 6.3 输入注入

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_click` | `x, y, button="left", click_count=1, modifiers=0` | 鼠标点击 |
| `kui_mouse_move` | `x, y` | 移动鼠标（不点击） |
| `kui_scroll` | `x, y, delta_x=0, delta_y=120` | 滚轮事件 |
| `kui_type` | `text` | 逐字符输入文本 |
| `kui_key` | `key, event_type="keyDown", modifiers=0` | 键盘事件 |

**`modifiers` 位掩码：** 1=Alt, 2=Ctrl, 4=Meta, 8=Shift

**常用 `key` 值：** `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`, `"Delete"`, `"ArrowUp/Down/Left/Right"`, `"Home"`, `"End"`, `"F1"–"F12"`

### 6.4 DOM 查询

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_dom` | `selector=""`, `depth=3` | DOM 树或元素属性 |
| `kui_outer_html` | `selector` | 元素 outerHTML |
| **`kui_box_model`** | `selector` | **布局尺寸：content/padding/border/margin 坐标** |
| `kui_element_at` | `x, y` | 坐标→元素（截图后反查） |
| `kui_event_listeners` | `selector` | 元素的事件监听器列表 |

**`kui_box_model` 返回示例：**
```json
{
  "content": [10, 20, 330, 20, 330, 68, 10, 68],
  "padding": [8, 16, 332, 16, 332, 72, 8, 72],
  "border":  [7, 15, 333, 15, 333, 73, 7, 73],
  "margin":  [7, 3,  333, 3,  333, 85, 7, 85],
  "width":   320.0,
  "height":  48.0
}
```
- 每组 8 个数 = 4 个顶点 (x,y) 顺时针排列
- `content[0,1]` = 内容区左上角视口坐标 (x, y)
- `width` / `height` = 内容区尺寸（CSS 像素）
- `padding - content` 差值 = 内边距
- `margin - border` 差值 = 外边距

### 6.5 DOM 修改

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_set_attr` | `selector, name, value` | 设置 HTML 属性 |
| `kui_remove_attr` | `selector, name` | 删除 HTML 属性 |
| `kui_set_text` | `selector, text` | 设置文字内容 |
| `kui_set_html` | `selector, html` | 替换 innerHTML |
| `kui_dispatch_event` | `selector, event_type, bubbles, cancelable, detail` | 触发合成 DOM 事件 |

### 6.6 CSS

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_get_style` | `selector` | 所有计算样式（含 width/height/margin/padding） |
| `kui_set_style` | `selector, css_text` | 注入 CSS **规则**（`CSS.addRule`，新增独立规则，不覆盖已有规则） |
| `kui_set_style_property` | `selector, prop, value, important?` | 设置单个**内联** CSS 属性（`CSS.setProperty`，即 `element.style`，等价于 DevTools Styles 面板的 "element.style {}" 区块）；任意元素均可，不要求带 `id` |
| `kui_matched_rules` | `selector` | 匹配该元素的所有 CSS 规则 |
| `kui_send` | `method, params_json="{}"` | 直发任意 CDP 命令并返回原始 JSON；上面这些专用工具没覆盖到的域用它 |
| `kui_remove_style` | `selector, prop` | 删除内联 CSS 属性（`CSS.removeProperty`）；任意元素均可，不要求带 `id` |
| `kui_force_pseudo` | `selector, pseudo` | 强制伪状态（`hover`/`focus`/`active`/`visited`） |

### 6.7 调试信息

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_console` | — | 所有 console 消息 |
| `kui_eval` | `expression` | 执行 Lua 表达式（全局上下文） |
| `kui_accessibility` | — | 完整 AX 可访问性树 |
| `kui_performance` | — | 性能指标快照（帧率/布局次数等） |
| `kui_timeline` | — | 布局/绘制/合成事件时间线 |

### 6.8 性能分析

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_profile_start` | — | 开始 Lua 脚本 profiler（不是渲染 CPU profiler） |
| `kui_profile_stop` | — | 停止并返回 `profileKind="lua"` 的脚本分析结果 |
| `kui_render_stats` | `window=120` | 读取帧工作量、present、damage 和 animation source 统计 |
| `kui_repaint_rects` | `since_frame=0` | 读取指定帧后的重绘矩形记录 |
| `kui_render_option` | `name="", enabled=true, value=-1` | 查询或切换渲染优化运行时开关 |
| `kui_perf_capabilities` | — | 查询 schema、模式、上限及 CPU/内存/GPU availability；不启动录制 |
| `kui_perf_record_start` | `mode="attribution", max_duration_ms=120000, lease_ttl_ms=120000, ..., gpu_sampling=false, categories=null, max_nodes=0, buffer_size=0` | 显式开始有界细粒度录制；MCP 强制使用 leased 生命周期；Windows PDH GPU 采样需显式开启且可能扰动尾延迟。录制到 `max_duration_ms` 自动停止（`stopReason: "timeout"`），超过 `lease_ttl_ms` 没有任何该 session 的调用也会停止（`"lease-expired"`）；agent 两次工具调用常隔几十秒，因此两者默认 120 s |
| `kui_perf_record_stop` | `session_id=""` | 显式停止录制并保留证据；空 id 使用本 MCP 进程最近 session |
| `kui_perf_record_get` | `session_id="", cursor="0", max_frames=120, resource_cursor="0", max_resource_samples=120, detail="compact", layout_frames_only=false` | 分别按 frame/resource cursor 显式分页，不在后台轮询。默认 `compact`：每帧只保留耗时、非零工作量、layout 路径与分段耗时、整支搬回统计和少量渲染字段，资源样本去掉内存分类说明，数值不改；`detail="full"` 返回原始协议数据；`layout_frames_only=true` 只列有布局工作的帧 |
| `kui_perf_frame` | `frame_id, session_id="", detail="full"` | 查询单帧 phase/work/damage/layer/DOM/DisplayList 细节；`detail="compact"` 与 `kui_perf_record_get` 同样裁剪 |
| `kui_perf_hot_nodes` | `session_id="", sort_by="estimatedCost", limit=50, detail="compact", min_layout_runs=0` | 按 dirty/style/layout/paint/displayList 等维度排序热点节点；逐节点 raster 不可用时不会冒充。默认 `compact` 只返回节点身份、非零计数与 `layoutTopologyTransfer`；`min_layout_runs>0` 时按 layout 排序自动翻页，只保留重跑次数达到阈值的节点，并给出按整支搬回结局聚合的 `layoutRunsByTransferOutcome` |
| `kui_perf_node_history` | `performance_node_id, session_id="", cursor="0", max_events=200` | 分页查询节点有序 dirty 事件、关联帧/update pass 和累计工作量 |
| `kui_perf_resolve_node` | `performance_node_id, session_id=""` | 把性能节点映射为安全截断的 tag/id/class 元数据与存活状态；不返回 selector（固定为 null） |
| `kui_perf_buffer` | `session_id=""` | 查询缓冲占用、丢失计数和建议降级模式 |
| `kui_perf_insights` | `session_id=""` | 获取 dirty/style/layout/paint/damage/resource/buffer 七类证据诊断卡 |
| `kui_perf_trace` | `session_id="", cursor="0", max_events=200` | 分页读取 DeepTrace 的 paint-owner range、backend replay、present 与 surface 生命周期事件 |
| `kui_perf_reveal_node` | `performance_node_id, session_id="", highlight=true` | 把仍存活的性能节点在 DOM 面板中定位并高亮；tombstone 节点明确拒绝 |
| `kui_perf_export_recording` | `session_id="", confirm_export=false, max_bytes=4194304` | 导出已冻结录制的脱敏 JSON；需显式 `confirm_export=True`，该标记由调用方声明，引擎不能验证是否真有人工确认 |
| `kui_perf_capture_interaction` | `action="click", selector="", x=-1, y=-1, key="", text="", modifiers=0, mode="attribution", warmup_action=false, warmup_ms=200, settle_ms=500, max_frames=64, node_limit=30` | **一次调用完成「开始录制 → 点击/按键/输入 → 等待 → 停止」**，全程同一条 WebSocket，不受工具调用间隔影响；返回有工作的帧（compact）、合计，以及 attribution 模式下按整支搬回结局聚合的重跑节点与前 `node_limit` 个布局节点。`selector` 取边框盒中心；`warmup_action=true` 先预演一次同样的输入再录，避开首次冷缓存；session id 会被记住，可继续用 `kui_perf_frame` 等下钻 |

测一次交互（点击、按键、输入）时优先用一步到位的工具：

```text
kui_perf_capture_interaction(selector="#advanceButton", warmup_action=True)
kui_perf_frame(frame_id="...", detail="compact")   # 需要时下钻单帧
```

需要自己控制录制区间时的显式流程：

```text
kui_perf_capabilities()
kui_perf_record_start(mode="attribution")
# 在 KUI 应用中复现交互；MCP WebSocket 关闭不会结束 leased session
kui_perf_record_stop()
kui_perf_record_get(max_frames=512, layout_frames_only=True)
kui_perf_hot_nodes(min_layout_runs=1)
kui_perf_node_history(performance_node_id="...")
kui_perf_insights()
kui_perf_buffer()
```

需要进程级 GPU engine 与 dedicated/shared/committed memory 时使用
`kui_perf_record_start(mode="summary", gpu_sampling=true)`。默认 `false` 不创建 GPU
采样线程或 PDH query，资源结果为 `not_requested`；应把 GPU 模式与默认轻量模式分开做
observer A/B，不能混用 baseline。

`available` 之外的资源状态（例如 `unsupported`、`collector_failed`）必须按原值解释，
不能把 `null` 当作 0% CPU、0 字节内存或 0% GPU。长录制分别按
`hasMore/cursor` 和 `resourceHasMore/resourceCursor` 继续读取；
若出现丢帧或节点事件丢失，应保留该证据并按 `suggestedMode` 降低采集颗粒度后复测。

### 6.9 Screencast（连续帧捕获）

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_screencast_start` | `every_nth_frame=2` | 开始连续帧捕获 |
| `kui_screencast_stop` | — | 停止捕获 |
| `kui_screencast_save` | `output_dir="."`, `max_frames=100` | 保存缓冲帧为 frame_NNNN.png |

### 6.10 Storage

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_storage_get_all` | `local=True` | 读取全部 localStorage/sessionStorage |
| `kui_storage_set` | `key, value, local=True` | 写入 |
| `kui_storage_remove` | `key, local=True` | 删除一个键 |
| `kui_storage_clear` | — | 清空全部 storage |

### 6.11 推送事件

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_events` | `count=20` | 读取最近收到的推送事件 |
| `kui_wait_event` | `event_name, timeout=5.0` | 阻塞等待特定推送事件 |

**常用事件名：**
- `DOM.childNodeInserted` / `DOM.childNodeRemoved`
- `DOM.attributeModified` / `DOM.characterDataModified`
- `Console.messageAdded`
- `Page.screencastFrame`
- `Debugger.paused`

### 6.12 Lua 脚本调试

> 需要调用 `devtools->attachScriptEngine(engine)` 后方可使用。

| 工具 | 参数 | 说明 |
|------|------|------|
| `kui_debugger_enable` | — | 启用调试器（必须首先调用） |
| `kui_debugger_disable` | — | 禁用调试器 |
| `kui_debugger_state` | — | 当前状态：running / paused / disabled |
| `kui_script_sources` | — | 所有已加载 Lua 脚本列表 |
| `kui_breakpoint_set` | `source, line, condition="", log_message=""` | 设置断点 |
| `kui_breakpoint_remove` | `breakpoint_id` | 删除断点 |
| `kui_breakpoints` | — | 所有活跃断点 |
| `kui_exception_breaks` | `mode="none"` | 异常断点模式（`none`/`uncaught`/`all`） |
| `kui_pause` | — | 在下一行暂停执行 |
| `kui_resume` | — | 从断点继续执行 |
| `kui_step` | `action="over"` | 步进（`over`/`into`/`out`） |
| `kui_call_stack` | — | 当前调用栈（暂停时有效） |
| `kui_scope` | `frame_index=0` | 指定帧的局部变量和 upvalue |
| `kui_debugger_eval` | `expression, frame_index=0` | 在暂停帧中求值 Lua 表达式 |

### 6.13 原始 CDP 通道

```python
kui_send(method, params_json="{}")
```

发送任意 CDP 命令，适用于上面没有封装的命令。例如：

```
# 读取 stylesheet 内容
kui_send("CSS.getStylesheetText", '{"stylesheetId": 0}')

# 强制更新布局
kui_send("Page.getLayoutMetrics", "{}")
```

---

## 7. 使用场景示例

### 场景 A：布局问题排查

```
你：#sidebar 在 flex 容器里宽度不对，帮我查一下

Agent：
  1. kui_connect()
  2. kui_screenshot("before.png")             → 截图看现状
  3. kui_box_model("#sidebar")                → 实际像素宽度
  4. kui_get_style("#sidebar")               → width / flex-basis / min-width
  5. kui_get_style("#sidebar").parent?       → 父容器 flex 设置
  6. kui_matched_rules("#sidebar")           → 哪条规则生效
  7. kui_set_style("#sidebar", "flex: 0 0 240px")  → 临时修复验证
  8. kui_screenshot("after.png")             → 截图对比
```

### 场景 B：交互自动化测试

```
你：模拟用户登录流程，填写用户名密码后点提交

Agent：
  1. kui_connect()
  2. kui_click(300, 200)                     → 点击 username input
  3. kui_type("testuser@example.com")
  4. kui_key("Tab")                          → 跳转到密码框
  5. kui_type("password123")
  6. kui_wait_event("DOM.attributeModified") → 等待输入完成
  7. kui_click(300, 280)                     → 点击提交按钮
  8. kui_wait_event("DOM.childNodeInserted", timeout=3.0)
  9. kui_outer_html("#result-message")       → 验证结果
```

### 场景 C：Lua 脚本调试

```
你：onButtonClick 函数里的 counter 变量值不对，帮我调试

Agent：
  1. kui_connect()
  2. kui_debugger_enable()
  3. kui_script_sources()                    → 找到脚本文件名
  4. kui_breakpoint_set("ui.lua", 42)        → 在第 42 行设断点
  5. kui_click(200, 100)                     → 触发按钮点击
  6. kui_wait_event("Debugger.paused", timeout=5.0)
  7. kui_call_stack()                        → 查看调用栈
  8. kui_scope(0)                            → 查看 counter 局部变量
  9. kui_debugger_eval("counter * 2")        → 计算表达式验证
 10. kui_step("over")                        → 步进下一行
 11. kui_scope(0)                            → 再次查看变量变化
 12. kui_resume()                            → 继续执行
```

### 场景 D：性能分析

```
你：这个列表滚动时帧率降了，帮我分析一下瓶颈

Agent：
  1. kui_connect()
  2. kui_perf_capabilities()                 → 确认资源提供器与模式
  3. kui_perf_record_start(mode="attribution")
  4. kui_scroll(400, 300, 0, -500)           → 模拟快速滚动
  5. kui_scroll(400, 300, 0, -500)
  6. kui_scroll(400, 300, 0, -500)
  7. kui_perf_record_stop()                  → 显式停止 leased session
  8. kui_perf_record_get(max_frames=512)     → 帧、工作量、资源与脏区
 9. kui_perf_hot_nodes(sort_by="paint")     → 定位高频节点
 10. kui_perf_insights()                     → 获取证据、原因、规避与 KUI 调查建议
 11. kui_perf_buffer()                       → 检查证据是否丢失
```

### 场景 E：坐标→元素反查

```
你：截图中坐标 (350, 180) 处的元素是什么，样式如何？

Agent：
  1. kui_connect()
  2. kui_element_at(350, 180)               → 返回元素 tag/id/class/nodeIndex
  3. kui_get_style(".found-class")          → 查看其计算样式
  4. kui_box_model(".found-class")          → 查看其精确尺寸和位置
```

---

## 8. 故障排查

### `RuntimeError: Cannot reach KUI DevTools at localhost:9229`

- KUI 应用未运行，或未调用 `devtools->start(9229)`
- 端口号与配置的不一致

### `TimeoutError: No response for '...' within 10s`

- KUI 主线程未在事件循环中调用 `devtools->poll()`
- 应用在加载大型文档（增大 timeout，在 `kui_send` 中指定）

### `CDP error: method not found`

- 命令名称拼写错误
- 该功能未在此 KUI 版本中实现（最低要求 v3.3.0）

### Lua 调试工具无效

- 未调用 `devtools->attachScriptEngine(engine)`
- 未先调用 `kui_debugger_enable()`

### Claude Code 看不到 `kui_*` 工具

- 注册后需**重载会话**（重新打开项目或新开对话）才会加载
- 项目级 `.mcp.json` 首次加载时需在安全提示中**批准**该 server
- 用 `/mcp` 确认 `kui-devtools` 已连接；用 `claude mcp list` 核对作用域与路径
- 确认 `python` 在 PATH 中且已 `pip install mcp websockets httpx`

### Windsurf 找不到工具

- 检查 `server.py` 路径是否正确（使用正斜杠 `/`）
- 确认 `pip install mcp websockets httpx` 已执行
- 在 Windsurf 中重载 MCP 配置

---

## 文件结构

```
sdk/cpp/tools/devtools_mcp/
├── server.py         ← MCP server 主体（80 个工具，stdio transport）
├── VERSION           ← MCP 工具自身版本（独立于 KUI SDK 版本）
├── requirements.txt  ← Python 依赖声明
├── README.md         ← 本文件
└── CHANGELOG.md      ← 版本历史
```

---

## 参见

- [`sdk/cpp/docs/devtools.md`](../../docs/devtools.md) — KUI DevTools C++ API 完整参考
- `sdk/cpp/examples/native/devtools/` — C++ 示例（01–10）
- [Model Context Protocol](https://modelcontextprotocol.io) — MCP 规范
