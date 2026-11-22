> 原样取自 Rust SDK 包（`kui-rust-sdk-3.1.0-<平台>.zip`）的 README：文中包里的 `native/` 是本仓库的 `runtime/<平台>/`，`bin/` 是 `build.ps1` / `build.sh` 的输出目录；在本仓库里不设变量、不给参数即可。见 [README.md](README.md)。
> The Rust SDK package README as is: its `native/` is this repository's `runtime/<platform>/` and `bin/` is the output folder of `build.ps1` / `build.sh`; in this repository no variables or parameters are needed. See [README.md](README.md).

# KUI Rust SDK 3.1.0

状态：Windows x64 Full Profile GA；Linux x64 / macOS arm64 同口径 Release 包（3.61.8 起）。

SDK 分为两层：

- `kui-sys`：从权威 `kui_c.h` 机械生成并检验的全部 C ABI 声明，完整覆盖
  product-major-3 导出，不要求 consumer 安装 bindgen 或 libclang。
- `kui`：日常使用的安全门面，提供 `Result`、RAII、UTF-8 转换、UI 线程
  `!Send + !Sync` 约束，覆盖冻结清单内的全部导出：Window（状态、外观、事件、尺寸回调）、
  Document / Element（结构与查询、树遍历、classList、innerHTML、样式、几何、滚动、焦点、
  自定义元素）、`Event`（只在回调内有效）、定时器与 idle、显示器、剪贴板、文件对话框、
  Lua、HTTP（工作线程 / 主线程两种回调）、media、Lottie、PDF、extension/3D 与 DevTools 注入。
  回调闭包不要求 `Send`（`post_to_main_thread` 与 `HTTPClient::fetch_async` 除外），保存在
  UI 线程的登记表里，移除监听 / 清除定时器 / 销毁窗口时释放。`kui::sys` 仍然公开，作为
  直接调用 C ABI 的出口。

3D 仍由独立 `kui_ext_3d.dll` 提供，媒体仍由 `kui_media_*` 后端提供。Rust SDK
负责类型安全地接入既有 Full Profile，不复制这些实现。

## 构建

要求：Windows x64、Rust stable MSVC toolchain，以及导入库 `lib/kui_c.lib` 与运行库 `bin/`。不设环境变量时
`kui-sys` 与 `build.ps1` 依次找 SDK 包里的 `native/`、对外仓库（`git clone kui-rust`）里的 `runtime/windows-x64/`、KUI 仓库内开发的
`../c/build/{Release,Debug}`，所以解压包或 clone 仓库后 `cargo build` 与 `build.ps1` 都不用额外参数；都找不到时报错并列出试过的目录。

```powershell
pwsh -File build.ps1 -Configuration Release -Output bin
```

脚本会依次执行格式门、全部 unit/integration/example tests，并精确构建 8 个
quickstart 和 225 个 canonical Release example（211 个与 C++ 原版一一对应 + 14 个 API 巡览；数量读 `examples/canonical_manifest.json`），把页面、素材与 Full Profile
runtime 放到输出目录；最终 `bin/` 固定包含 233 个 EXE。直接使用 Cargo 时：

默认输出就是仓库内 `sdk/rust/bin/`。该目录包含 233 个 EXE、assets 和 Full Profile
runtime，是可直接运行的本地 SDK 产物；它属于可重建二进制，因此由 Git 忽略。

`build.ps1 -Configuration Debug` 默认输出到 `bin\debug`：包里只有 Release 导入库与核心（`native/lib`、
`native/bin`，2026-11-20 起不再带 `debug/` 子目录），debug profile 同样链接、运行它们——Rust 在 MSVC 目标上本来就总用
Release CRT，C ABI 也不传 CRT 对象；仓库内开发取 `../c/build/Debug`（导入库与运行时取自同一处）。扩展与媒体
运行时用 Release 版；`-FullProfileRuntimeDir` 可另指扩展
运行时目录。

```powershell
$env:KUI_C_SDK = (Resolve-Path .\native)
$env:PATH = "$env:KUI_C_SDK\bin;$env:PATH"
cargo test --all-targets -- --test-threads=1
cargo build --release --examples
```

`KUI_C_SDK`（运行库根目录，导入库在其 `lib/` 下）与 `KUI_C_LIB_DIR`（导入库所在目录）可显式指定，优先于自动探测。Release
与 debug profile 都链接包 / 仓库里的 `lib/kui_c.lib`；仓库内开发按 profile 取 `sdk/c/build/<配置>`。Windows 上直接 `cargo run` / `cargo test`
时运行库要能被找到：把 `native\bin`（仓库里是 `runtime\windows-x64\bin`）加进 `PATH`，或用 `build.ps1`（它把运行库放到程序旁边）。

## Examples

SDK 固定交付 8 个 quickstart，以及 225 个 canonical 源码与页面：211 个与正式 C++ SDK example
产品一一对应，另有 14 个 API 巡览示例。canonical 入口为 `examples/canonical_*.rs`，由
`scripts/generate_go_rust_examples.py` 以 C canonical manifest 为权威生成。

**预编译程序在单独的示例包里**（2026-11-20 起 SDK 包不带 `bin/`）：`kui-rust-examples-<版本>-<平台>.zip`（同一个 GitHub
Release）的 `bin/` 就是 `build.ps1` / `build.sh` 的输出——233 个程序、与本包 `native/` 逐字节相同的运行库和 assets（另带 `script_html/`），解压即可
运行。可从源码运行 quickstart、自己编出同样的 `bin/`，或运行示例包里的标准产物：

```powershell
cargo run --release --example hello_world
.\bin\kui_rust_hello_world.exe
.\bin\kui_rust_canonical_native_3d_engine_01_hello_triangle.exe
```

`Platform`、`Window`、`Document`、`Element`、`ScriptEngine` 和 `HTTPClient` 均按
所有权释放原生 handle。UI 对象不能跨线程发送；任何 FFI 回调都不得让 panic
越过 C ABI 边界。

### Linux x64 / macOS arm64

要求：Rust stable，以及运行库 `libkui_c.so|.dylib`、`libkui.so|.dylib` 与扩展 / 媒体插件：不给参数时依次找 SDK 包里的 `native/lib`、
对外仓库里的 `runtime/<linux-x64|macos-arm64>/lib`、KUI 仓库内开发的 `../c/build` 与 `../cpp/lib`（`-n` / `-f` 可显式指定）。在对外仓库里，
`kui` crate 的示例与测试另带一条指向 `runtime/<平台>/lib` 的运行时搜索路径，`cargo run --example` / `cargo test` 不用设 `LD_LIBRARY_PATH`。

```bash
./build.sh -o bin        # Linux 没有桌面时：xvfb-run -a ./build.sh -o bin
```

与 `build.ps1` 同一组步骤与数量门，`bin/` 里是 233 个无扩展名的程序，旁边放着运行库、插件与 assets。只有 Release。
`kui` crate 的示例与测试带 `$ORIGIN`（macOS `@executable_path`）运行时搜索路径，程序与 `libkui_c` 放在同一目录即可
运行；直接 `cargo test` 时用 `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` 指向 `native/lib`（`build.sh` 已这样做）。
`kui-sys` 按目标平台找 `kui_c.lib` / `libkui_c.so` / `libkui_c.dylib`，`KUI_C_LIB_DIR` / `KUI_C_SDK` 可覆盖。

macOS 只允许在主线程（`main` 所在的线程）初始化平台、创建窗口。libtest 把每个 `#[test]` 放在子线程里跑，所以带窗口的
测试要在 `Cargo.toml` 里设 `harness = false`、由自己的 `main` 在主线程执行——本 crate 的 `tests/` 就是这样做的（运行器在
`tests/common/mod.rs`，三个平台同一套）。

## 发布边界

- SDK/crate 版本线为 3.1.0；底层 KUI 产品版本独立记录在发布包清单中。
- C ABI 是兼容性基线，`bindings.rs` 禁止手改。
- GA 声明适用于 Windows x64、Linux x64 与 macOS arm64 的 Full Profile；Linux / macOS 包只有 Release。
  `HTTPClient` 在 Linux 上需要发行版的 libcurl（`libcurl.so.4` 或 `libcurl-gnutls.so.4`，运行期加载；没有时请求如实报错），
  macOS `.app` 包里请求明文 `http://` 域名需要宿主在 `Info.plist` 配置 App Transport Security 例外。
- SDK 包不带预编译程序与 `bin/`（`SDK_MANIFEST.json` 的 `prebuilt_bin` 为 `false`，仍列出构建脚本会产出的 233 个程序名）；
  它们在 `kui-rust-examples-*` 示例包里，示例包另带同格式的 `SDK_MANIFEST.json`（`prebuilt_bin` 为 `true`）。
- 包里还带 `tools/devtools_mcp/`（调试运行中的 KUI 程序，用法见其 README）和 `docs/`（14 份与语言无关的文档，索引在 `docs/README.md`）；C++ 专属的入门、教程与组件 / 扩展编写指南只在 C++ 包里。
- 使用 `kui::sys` 属于显式 `unsafe` 开发，调用者必须遵守 handle 和 UI 线程契约。
