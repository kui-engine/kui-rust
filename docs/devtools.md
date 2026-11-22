# KUI DevTools

KUI 内置 DevTools HTTP/WebSocket 服务和前端，不需要 SDK 额外携带一套外部前端资源。协议、方法和默认值以 `kui/devtools/devtools_server.h`、`src/devtools/devtools_server.cpp` 与 `src/devtools/devtools_protocol.h` 为准。

## 启动

应用为目标窗口启动 DevTools 后，可访问服务返回的 `/json` 目标发现接口和内置前端。服务默认只绑定 `127.0.0.1`，并使用每实例认证 token 隔离连接。

需要跨机器调试时使用 SSH 隧道，不要把监听地址改成公网接口：

```bash
ssh -L 9229:localhost:9229 user@host
```

## 协议

- WebSocket 消息采用带 `id`、`method`、`params` 的 JSON 命令。
- 新客户端默认使用 v3.2 外层协议：错误位于顶层 `error`，符合 JSON-RPC 2.0 风格。
- 连接 URL 使用 `?proto=v3.1` 可选择兼容外层：错误保留在 `result` 内。
- 不要再把 v3.1 兼容格式描述为当前默认。

收发方向的大小限制不同：当前服务端 WebSocket 解析器的单帧入站 payload 上限为 **2 MiB（2,097,152 字节）**，依据 `src/devtools/devtools_transport.h::kMaxPayloadLen`；MCP 客户端的 **16 MiB** 是 `tools/kui_devtools_mcp/server.py::_WS_MAX_MESSAGE_BYTES` 对服务端响应的接收上限。16 MiB 不表示服务端可接收同等大小命令，也不是 Lua 引擎的内存上限。发送大脚本/数据时应按实际服务端限制设计协议，不要从 MCP 的接收能力推导请求容量。

示例：

```json
{"id":1,"method":"Runtime.evaluate","params":{"expression":"return 1 + 1"}}
```

具体 domain 和方法持续演进，不在本文维护易漂移的固定数量。可从内置前端、服务路由表和 `tools/kui_devtools_mcp/README.md` 查询当前入口。

## 主要能力

- DOM/CSS 检查与样式修改；
- Runtime/Console 的 Lua 执行与结构化返回；
- Network、Performance、页面截图与 screencast；
- 多窗口目标发现；
- 渲染优化开关和诊断状态；
- Lua 调试器。原生 C++ 调试仍应使用平台调试器。

## 安全

- DevTools 能读取应用 DOM、日志、网络数据和脚本状态，应视为高权限调试接口。
- token 不能写入日志、文档或提交；连接结束后应失效。
- DOM 中的密码、令牌和个人信息会对已授权 DevTools 客户端可见，应用应按自身安全策略脱敏。
- 不要在生产环境无条件启动或对外暴露 DevTools。

## MCP 工具

SDK 随包提供 `tools/devtools_mcp/` 自动化控制入口。1.2.1 起，每个 stdio MCP 进程在自身内存中保存当前 target 和认证 token；多个 agent 各自启动 MCP 进程后，连接、切换和断开互不影响。MCP 进程重启会清空状态，需要重新连接。

应用侧推荐自动分配端口，并只记录不含 token 的实际 target id：

```cpp
#include <kui/kui.h>
#include <kui/devtools/devtools_server.h>
#include <cstdio>

if (auto* devtools = window->startDevTools(0)) {
    std::fprintf(stderr, "KUI DevTools target=kui-%u port=%u\n",
                 devtools->port(), devtools->port());
}
```

不要把 `window->devToolsUrl()` 写入日志，因为它包含认证 token。若显式端口被占用，KUI 会尝试后续端口，因此始终以 `devtools->port()` 为准。

多个 KUI 示例同时运行时，每个 agent 使用下面的固定流程：

1. 从自己启动/管理的应用日志取得精确 target id，例如 `kui-9231`。
2. 调用 `kui_targets(port=9229)` 发现全部活跃 KUI target。
3. 调用 `kui_connect(port=9229, target="kui-9231")`。这里 `port` 是发现入口，`target` 才决定最终应用。
4. 调用 `kui_status()` 核对当前 target/端口，再用 `kui_page_info()` 核对标题和视口；执行输入或样式修改前，必要时再截图确认。
5. MCP 进程若重启，从第 2 步重新连接。

精确 `kui-<实际端口>` 是自动化的权威选择键。标题子串只适合唯一标题：标题可能为空、重复，或在启动 DevTools 后变化。同名示例并存时不要省略 `target`，也不要仅靠标题猜测。旧引擎或相距较远的自定义端口可把 `kui_targets(port=...)` 的起始端口移到目标附近。

完整安装、客户端配置、逐工具说明和多目标示例见随包 `tools/devtools_mcp/README.md`。
工具数量以该 README 的工具表为准，它由 `scripts/check_mcp_tool_docs.py` 对着
`server.py` 的 `@mcp.tool` 逐个校验，本文件不再另抄一份数字。

## 排障

- 找不到目标：检查应用是否为正确窗口启动服务，并查看 `/json`。
- WebSocket 被拒绝：检查目标 URL 和认证 token 是否来自同一实例。
- Lua evaluate 失败：确认脚本引擎已附加；不要把 C++ 表达式发送给 Lua Runtime。
- 截图或状态陈旧：确认主线程仍在轮询并驱动帧，不要只看协议请求成功。
