# Rust SDK Roadmap

3.1.0 已完成 Windows x64 GA：195 导出的 `kui-sys`、核心安全 RAII 门面、
Full Profile 能力、8 个 quickstart + 211 个 canonical examples、219 个标准预编译
Windows x64 产物、自托管构建与发布包 consumer/全量启动验证。

后续工作不影响 Windows GA：

- Linux/macOS 平台实现与独立 CI（当前明确跳过）。
- 把当前只能通过 `kui::sys` 使用的 API 逐步提升为安全门面：先补回调（事件、定时器、尺寸变化、异步 HTTP），再按领域补到 195 个导出全覆盖（`T-DEBT-SDK-RUST-SAFE-API-COVERAGE`）。
- 随新增 C ABI 导出同步生成器、ABI layout 与回归门。
