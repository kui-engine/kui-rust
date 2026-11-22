# KUI Rust SDK

[English](README.md) · **简体中文**

更多截图与动图：[kui](https://github.com/kui-engine/kui)

KUI 是**跨平台的原生 GUI 引擎**：用 HTML / CSS 写界面，可以从 C、C++、Go、Rust 接入，有 Windows x64、Linux x64、macOS arm64 三个平台的包。本仓库是它的 Rust 绑定：`kui`（安全门面）与 `kui-sys`（原始 FFI），版本 **3.1.0**，通过冻结的 C ABI 工作。本仓库自带三个平台的运行库与
Windows 导入库（`runtime/`），克隆后即可编译运行。API、构建脚本与示例的详细说明见 [`GUIDE.md`](GUIDE.md)（取自 SDK 包的 README）。

从空项目到一个窗口（含一次 DOM 更新、一个事件监听）：[快速开始](QUICKSTART.zh-CN.md)。

### 获取

```bash
git clone --depth 1 https://github.com/kui-engine/kui-rust.git
```

仓库自带三个平台的运行库（`runtime/`，本版共 475.7 MiB），克隆完就能编译、运行。**请用 `--depth 1`**：每个版本都把运行库提交进历史，完整历史会随版本增加而变大（每个版本约多一份 runtime/ 的大小）。要换版本时用 `git clone --depth 1 --branch v<版本> …`，或在已有克隆里 `git fetch --depth 1 origin tag v<版本>`。

在你的工程里用 git 依赖：`kui = { git = "https://github.com/kui-engine/kui-rust", tag = "v3.1.0" }`（crates.io 上的 `kui` 是别人的包，暂不发布）。`kui-sys` 的
`build.rs` 不设任何变量时依次找 SDK 包的 `native/`、本仓库的 `runtime/<平台>/`（Cargo 的 git 依赖检出的就是整个仓库，所以同样找得到）；
`KUI_C_SDK` / `KUI_C_LIB_DIR` 仍然优先。运行时：Windows 把 `runtime/windows-x64/bin/` 里的 DLL 放到程序旁边（或加入 PATH）；
Linux / macOS 上本 crate 自己的示例与测试带指向 `runtime/<平台>/lib` 的 rpath，直接运行，你自己的程序把该目录的库放到程序旁边。

### 编译并运行一个示例

要求：Rust stable（Windows 用 MSVC 目标）。最低版本以 `Cargo.toml` 为准、未单独验证，本版实际验证过的版本见下表。

**Linux 另需**：运行库依赖系统的 libX11、libXext、libXrandr、libGL 与 libstdc++（Debian / Ubuntu 上常见包名 `libx11-6 libxext6 libxrandr2 libgl1`；服务器版默认没有 X11 与 GL 运行库）。程序要连图形会话，没有桌面时在命令前加 `xvfb-run -a`（Go / Rust 的 `./build.sh` 会跑测试，同样需要显示服务器）。

在仓库根目录，Windows（PowerShell；导出自检从干净的 `git clone` 原样执行这组命令，程序启动并正常运行）：

```powershell
cargo build --example hello_world
Copy-Item runtime\windows-x64\bin\* target\debug\examples\
.\target\debug\examples\hello_world.exe
```

Linux / macOS（bash；macOS 命令相同。导出后在真实 `git clone` 里验证过：Linux（Debian 12，Xvfb）原样执行这组命令，构建并启动通过；macOS arm64 构建与依赖闭合通过，**没有启动带窗口的程序**（构建机是共享设备））：

```bash
cargo build --example hello_world
./target/debug/examples/hello_world
```

本版实际验证过的工具链：

| 平台 | 系统 | CMake 与 C / C++ 编译器 | Go | Rust |
|---|---|---|---|---|
| Windows x64 | Windows 10 | CMake 3.31.6、MSVC 19.43（Visual Studio 2022）；Go 的 cgo 用 LLVM clang 22.1.6 | 1.26.7 | 1.98.0 |
| Linux x64 | Debian 12（Xvfb） | CMake 4.4.0、gcc 12.2.0 | 1.26.7 | 1.98.0 |
| macOS arm64 | macOS 15.7.2 | CMake 4.0.2、Apple clang 17.0.0 | 1.26.7 | 1.98.0 |

一次编译全部示例并跑测试：Windows `./build.ps1`，Linux / macOS `./build.sh`，不用任何参数（默认依次找包里的 `native/`、本仓库的
`runtime/<平台>/`，`-NativeSdk` / `-n`、`-f` 仍可改指别处），结果放在 `bin/`。

**运行库怎么找到**：Linux / macOS 上 `cargo build` 产出在 `target/` 里的程序带两条 RPATH：`$ORIGIN`（macOS `@executable_path`）与本仓库
`runtime/<平台>/lib` 的**绝对**路径，所以在 clone 里直接能跑，clone 挪走后就找不到库；`build.sh` 输出到 `bin/` 的程序旁边已放好库，走
`$ORIGIN`。要分发给别人，把程序和 `runtime/<平台>/lib`（Windows：`runtime/windows-x64/bin` 的 DLL）里的库放进同一目录；你自己 crate 的程序
不会自动带这些 RPATH，Linux / macOS 上给它加相对 RPATH（`-Wl,-rpath,$ORIGIN`，macOS `@executable_path`）或运行时设库搜索路径。

### runtime/ 里有什么

| 目录 | 文件 | 大小 |
|---|---:|---:|
| `runtime/windows-x64/` | 18 | 166.7 MiB |
| `runtime/linux-x64/` | 17 | 227.7 MiB |
| `runtime/macos-arm64/` | 17 | 81.3 MiB |

内容取自 Rust 包的 `native/bin/` 与 `native/lib/`（Windows 导入库 `kui_c.lib`）或 `native/lib/`（Linux / macOS）。它们与 Release 上同平台 SDK 包里的那份逐字节相同、相对位置不变（Linux / macOS 的 RPATH 依赖这个相对位置）。

### Release 上有什么

每个版本的 [kui Release](https://github.com/kui-engine/kui/releases/tag/v3.61.8) 上有三类东西：

1. **SDK 包** `kui-<语言>-sdk-<版本>-<平台>.zip`：就是对应源码仓库在一个平台上的打包（`runtime/<平台>/` 在包里是 `bin/`、`lib/`，Go / Rust 是 `native/`），不带预编译示例程序；不想 `git clone` 时下载它。
2. **预编译示例包** `kui-<语言>-examples-<版本>-<平台>.zip`：示例程序加运行库，解压后进 `bin/` 直接运行，不需要编译器。
3. **校验和** `SHA256SUMS.txt`（只列上面两类包）与 FFmpeg 对应源码。

| 包 | 内容 | Windows x64 | Linux x64 | macOS arm64 |
|---|---|---|---|---|
| `kui-rust-sdk` | SDK（= 源码仓库的单平台打包） | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-windows-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-linux-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-macos-arm64.zip) |
| `kui-rust-examples` | 预编译示例（解压即运行） | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-windows-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-linux-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-macos-arm64.zip) |

### 平台

| 平台 | 状态 | 打包时实际做过的验收 |
|---|---|---|
| Windows x64 | Full Profile GA | 4 个 SDK 包解压后从包内源码重建，并通过包外使用方矩阵；4 个预编译示例包的全部程序逐个实际启动（C 222、C++ Wrapper 190、Go 219、Rust 219）。导出后另在四个仓库的真实 `git clone` 里按 README 命令构建并启动（含零参数 `build.ps1`） |
| Linux x64 | 同一冻结 C ABI 的 Release 包 | SDK 包从包内源码重建全部通过；示例包在 Xvfb 下启动：C 221/222、C++ Wrapper 190/190、Go 218/219、Rust 218/219；C、Go、Rust 各有 1 个托盘示例未启动（Xvfb 没有托盘宿主，`kui_window_tray_create` 返回不支持），按白名单放行。导出后另在四个仓库的真实 `git clone` 里按 README 命令构建并在 Xvfb 下启动（含 Go / Rust 零参数 `build.sh` 各 219 个程序） |
| macOS arm64 | 同一冻结 C ABI 的 Release 包 | SDK 包在包内源码重建全部通过（C 11 + 211、C++ Wrapper 189 + demo + 3 组测试、Go / Rust 8 + 211），重建后不启动；示例包只核对布局、清单、运行库逐字节一致、依赖与 RPATH，另启动 6 个无窗口 C 示例与 C++ Wrapper demo。**带窗口程序的全量启动没有在 macOS 上做**（构建机是共享设备）。导出后另在四个仓库的真实 `git clone` 里零参数构建（含 Go / Rust `build.sh` 各 219 个程序）并核对依赖与 rpath 闭合，只运行无窗口的程序与测试，**没有启动带窗口的程序** |

三个平台都只带 Release 运行库、不带调试符号；你的 Debug 构建同样链接它（C ABI 不跨边界传 CRT 对象）。平台能力并非完全对称，已知差异见 `docs/known_issues.md`。

### 下载后无法运行

预编译程序与运行库暂未签名：

- **Windows**：从浏览器下载的 zip 解压后，SmartScreen 提示「Windows 已保护你的电脑」时点「更多信息」→「仍要运行」；也可以在 PowerShell 里对解压目录执行 `Get-ChildItem -Recurse <解压目录> | Unblock-File` 一次性解除。`git clone` 得到的文件不带「来自网络」标记，不会被 SmartScreen 拦。
- **macOS**：从浏览器下载的 zip 解压后会被 Gatekeeper 拦下，执行 `xattr -dr com.apple.quarantine <解压目录>`。
- **Linux**：用不保留权限的工具解压时，对 `bin/` 下的程序执行 `chmod +x`（`git clone` 会保留运行库的可执行位）。

### 许可证与第三方组件

MIT，见 `LICENSE`（版权方 Alloy）。

KUI 本体是 MIT。视频插件 `kui_media_video` 以独立动态库方式加载 FFmpeg（LGPL-3.0-or-later，LGPL 构建，没有 `--enable-gpl` / `--enable-nonfree`）；不需要视频时不部署 `kui_media_video` 与 FFmpeg 库即可，音频插件不用 FFmpeg。仓库的 `runtime/<平台>/` 与每个包里，FFmpeg 库旁都有 `LICENSE.FFmpeg.txt`（LGPLv3）与 `LICENSE.FFmpeg.GPLv3.txt`，其余第三方组件均为宽松许可证，详见 `THIRD_PARTY_NOTICES.md`。

**随二进制分发的 FFmpeg 库的对应源码**作为 [kui 的 Release](https://github.com/kui-engine/kui/releases/tag/v3.61.8) 附件提供，校验值见 `FFMPEG_SOURCE_SHA256SUMS.txt`：

- Windows x64 / Linux x64（BtbN 构建 `autobuild-2026-10-31-12-59`，FFmpeg `n9.0.2-39-g7adfbe40dc`）：`ffmpeg-7adfbe40dcde598f93a6963d2f7b22cd75202ccc.tar.gz`（FFmpeg 提交 `7adfbe40dcde598f93a6963d2f7b22cd75202ccc` 的源码）与 `FFmpeg-Builds-autobuild-2026-10-31-12-59.tar.gz`（该构建的脚本）；
- macOS arm64（官方发布包源码构建）：`ffmpeg-9.0.2.tar.xz` 与签名 `ffmpeg-9.0.2.tar.xz.asc`。

上游地址：<https://github.com/FFmpeg/FFmpeg/tree/7adfbe40dcde598f93a6963d2f7b22cd75202ccc>、<https://github.com/BtbN/FFmpeg-Builds/tree/autobuild-2026-10-31-12-59>、<https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz>。
