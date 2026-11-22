# KUI DevTools — 安全说明（本机调试服务）

> 关联：IMPL-CR-007（`docs/KUI审查报告交叉评估与修复计划_20260528.md` §6）· P-AUD-6

## 定位

KUI DevTools 服务端（`src/devtools/devtools_server.cpp`）是一个**仅供本机开发调试**的 HTTP + WebSocket 服务，实现 Chrome DevTools Protocol（CDP）子集，供浏览器前端 / Playwright / puppeteer 连接以审查 DOM、布局、渲染信息。

**严禁在生产部署中暴露 DevTools 服务。** 它默认仅 `bind` 到 `localhost`，但其设计目标是开发期可用性，而非对抗性安全边界。

## 认证模型

- 每次 `start()` 生成一个一次性 32 字符 hex **token**。
- 只有携带正确 `?token=<token>` 的 WebSocket 升级请求才被接受（`tokenAuthEnabled`）。
- 直接来自浏览器地址栏的导航请求无法携带自定义 token，会被拒绝。

## token 暴露面与缓解

| 暴露点 | 风险 | 缓解（已实施） |
|---|---|---|
| 调试日志拼接 `?token=<hex>`（多人共享工作站 / 日志截图分享） | 旁观者复用 token 发起未授权连接 | **日志脱敏** `redactToken()`（`devtools_log_redact.h`）：`token=<前4位>****`，保留追踪辨识度但不可复用 |
| `GET /json` 返回的 `webSocketDebuggerUrl` 含完整 token | 未鉴权 GET 即可取得 token | **`KUI_DEVTOOLS_PUBLIC_JSON` cmake option**（见下） |
| 访问主页 `/` 的 302 redirect 自动跳转到 `/?token=` | 同上 | 同一 option 控制 |
| KUI-039 跨进程发现表 `%TEMP%\kui-devtools\<port>.json` 写入 `"token"`（同机同用户可读文件） | 同机其它进程读取 registry 复用 token | **REVIEW-20260624-OBS-001**：registry token 写入受同一 `KUI_DEVTOOLS_PUBLIC_JSON` gate——hardened（OFF）时注册的 `TargetInfo.token` 留空，token 不落地 registry |

KUI-081 之后，DevTools MCP 选中 target 时不再把带 token 的 WebSocket URL 写入全机共享的 `%TEMP%\kui_mcp_state.json`，而是仅保存在当前 stdio MCP server 进程内存。这一改动同时消除了多 agent 连接覆盖和 MCP 自身的 token 持久化；KUI-039 引擎跨进程发现表的安全边界不变。

## `KUI_DEVTOOLS_PUBLIC_JSON` 编译开关

| 值 | 行为 | 适用 |
|---|---|---|
| **ON**（默认） | `/json` 的 `webSocketDebuggerUrl` 携带 `?token=`；访问主页未带 token 时 302 重定向到带 token 的 URL | 本机开发：CDP 自动发现可用，puppeteer/playwright 免手动配置 token |
| **OFF**（production-hardened） | `/json` 省略 token；主页不再 302 自动带 token；**跨进程 registry JSON 也不再写入 token**（REVIEW-20260624-OBS-001）。调用方须自行带 token 连接 WebSocket | 需要把 DevTools 端口暴露到更广网络面、或不希望 token 出现在任何未鉴权 HTTP 响应或本地 registry 文件的构建 |

配置示例：

```bash
cmake -S . -B build -DKUI_DEVTOOLS_PUBLIC_JSON=OFF
```

> token 的**真实验证逻辑不受此开关影响**：WebSocket 升级始终校验 token。OFF 仅停止在未鉴权的 `/json` / 302 响应中**主动播发** token。

## 建议

- 开发：保持默认 ON，享受 CDP 自动发现便利。
- 任何非纯本机场景：`-DKUI_DEVTOOLS_PUBLIC_JSON=OFF`，并确保 DevTools 端口不对外暴露。
- 不要把含 `?token=` 的日志/截图分享到不可信渠道（即便已脱敏，也应避免）。
