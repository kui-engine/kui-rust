# KUI 文档（随 C / C++ Wrapper / Go / Rust 包）

这些文档与宿主语言无关，和原生 C++ SDK 包 `docs/` 里的同名文件逐字相同。各语言的 API 用法见包根 `README.md`；
原生 C++ SDK 专属的入门、教程与组件 / 扩展编写指南只在原生 C++ SDK 包里。调试运行中的 KUI 程序用包里的
`tools/devtools_mcp`（DevTools MCP 服务，用法见其 README）。

- [`features.md`](features.md)：KUI 能力边界
- [`positioning_and_benchmark.md`](positioning_and_benchmark.md)：KUI 官方定位与对标口径声明
- [`css_feature_matrix.md`](css_feature_matrix.md)：KUI CSS 特性支持矩阵入口
- [`css-fonts.md`](css-fonts.md)：CSS 字体支持（@font-face / document.fonts）
- [`wpt_conformance_snapshot.md`](wpt_conformance_snapshot.md)：KUI WPT 一致性快照
- [`known_issues.md`](known_issues.md)：KUI 已知问题与书面例外汇总
- [`performance.md`](performance.md)：性能相关的特性与开关
- [`devtools.md`](devtools.md)：KUI DevTools
- [`devtools_security.md`](devtools_security.md)：KUI DevTools — 安全说明（本机调试服务）
- [`devtools_perf_fields.md`](devtools_perf_fields.md)：DevTools 性能面字段契约
- [`lua.md`](lua.md)：KUI Lua Scripting - In-Depth Guide
- [`extension_repaint_contract.md`](extension_repaint_contract.md)：扩展元素重绘约定（Extension Repaint Contract，扩展 API v1.11）
- [`kui-editor.md`](kui-editor.md)：`<kui-editor>` - Code Editor Component
- [`adr/ADR-001-out-of-scope-features.md`](adr/ADR-001-out-of-scope-features.md)：ADR-001: OUT_OF_SCOPE 特性取舍（GUI 子集边界）

> 这些文档是 3.61.8 发布时的文档，只做了两类修正（SDK 包与源码仓库里的一样）：①指向这里没有的文件（原生 C++ SDK 才有的教程 / FAQ / API 导航、核心源码路径、内部工作日志）的链接，去掉了链接、只留文字；②多列 / CSS Shapes / subgrid 的说明改成与源码一致（它们不是编译期禁用）。
> These documents are the 3.61.8 documents with two kinds of corrections (the same in the SDK packages and the source repositories): links to files that are not here (tutorial / FAQ / API index of the native C++ SDK, core source paths, internal work logs) are plain text, and the notes on multi-column / CSS Shapes / subgrid now match the source (they are not disabled at compile time).
