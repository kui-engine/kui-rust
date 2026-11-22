# KUI Rust SDK

**English** · [简体中文](README.zh-CN.md)

More screenshots and animations: [kui](https://github.com/kui-engine/kui)

KUI is a **cross-platform native GUI engine**: the interface is written in HTML / CSS, you drive it from C, C++, Go or Rust, and there are packages for Windows x64, Linux x64 and macOS arm64. This repository is its Rust binding: `kui` (the safe facade) and `kui-sys` (raw FFI), version **3.1.0**, working through the frozen C ABI. This
repository carries the runtimes for three platforms and the Windows import library (`runtime/`), so you can build and run right
after cloning. The API, the build scripts and the examples are described in [`GUIDE.md`](GUIDE.md) (the SDK package README, in
Chinese).

From an empty project to a window, with one DOM update and one event listener: [Quick start](QUICKSTART.md).

### Getting it

```bash
git clone --depth 1 https://github.com/kui-engine/kui-rust.git
```

The repository carries the runtimes for all three platforms (`runtime/`, 475.7 MiB in this version), so you can build and run right after cloning. **Please use `--depth 1`**: every version commits its runtimes, so the full history grows with each release (roughly one `runtime/` per version). To get another version use `git clone --depth 1 --branch v<version> …`, or `git fetch --depth 1 origin tag v<version>` in an existing clone.

Use it as a git dependency: `kui = { git = "https://github.com/kui-engine/kui-rust", tag = "v3.1.0" }` (the `kui` name on crates.io belongs to someone
else; not published there yet). With no variables set, the `build.rs` of `kui-sys` looks for the SDK package's `native/` and then
this repository's `runtime/<platform>/` (a Cargo git dependency checks out the whole repository, so it is found there too);
`KUI_C_SDK` / `KUI_C_LIB_DIR` still take priority. At run time on Windows put the DLLs from `runtime/windows-x64/bin/` next to the
program (or on PATH); on Linux / macOS this crate's own examples and tests carry an rpath to `runtime/<platform>/lib` and run as
they are, and your own programs need the libraries from that folder next to them.

### Building and running an example

Requirements: Rust stable (the MSVC target on Windows). The minimum version is governed by `Cargo.toml` and was not verified separately; the versions actually verified for this release are in the table below.

**On Linux you also need** the system libX11, libXext, libXrandr, libGL and libstdc++ (common Debian / Ubuntu package names: `libx11-6 libxext6 libxrandr2 libgl1`; server installs lack the X11 and GL runtime by default). The programs connect to a graphical session; without a desktop prefix the commands with `xvfb-run -a` (the Go / Rust `./build.sh` runs tests and needs a display server too).

From the repository root on Windows (PowerShell; the export self-check runs exactly these commands in a clean `git clone` and the program starts and keeps running):

```powershell
cargo build --example hello_world
Copy-Item runtime\windows-x64\bin\* target\debug\examples\
.\target\debug\examples\hello_world.exe
```

Linux / macOS (bash; the same commands on macOS. Verified after export in real `git clone`s: on Linux (Debian 12, Xvfb) these exact commands build and the programs start; on macOS arm64 the build and the dependency closure pass, but **no windowed program was launched** (the build Mac is a shared machine)):

```bash
cargo build --example hello_world
./target/debug/examples/hello_world
```

Toolchains actually verified for this release:

| Platform | OS | CMake and C / C++ compiler | Go | Rust |
|---|---|---|---|---|
| Windows x64 | Windows 10 | CMake 3.31.6, MSVC 19.43 (Visual Studio 2022); LLVM clang 22.1.6 for Go's cgo | 1.26.7 | 1.98.0 |
| Linux x64 | Debian 12 (Xvfb) | CMake 4.4.0, gcc 12.2.0 | 1.26.7 | 1.98.0 |
| macOS arm64 | macOS 15.7.2 | CMake 4.0.2, Apple clang 17.0.0 | 1.26.7 | 1.98.0 |

To build every example and run the tests: `./build.ps1` on Windows, `./build.sh` on Linux / macOS, with no parameters (they look for
the package's `native/` and then this repository's `runtime/<platform>/`; `-NativeSdk` / `-n`, `-f` still override); the results go
to `bin/`.

**How the runtime is found**: on Linux / macOS the programs that `cargo build` puts in `target/` carry two RPATHs: `$ORIGIN`
(`@executable_path` on macOS) and the **absolute** path of this repository's `runtime/<platform>/lib`, so they run inside the clone
but not after the clone is moved; the programs `build.sh` puts in `bin/` have the libraries next to them and use `$ORIGIN`. To ship
to someone else, put the program and the libraries from `runtime/<platform>/lib` (on Windows the DLLs from `runtime/windows-x64/bin`)
in the same folder; programs of your own crate do not get these RPATHs automatically, so on Linux / macOS give them a relative RPATH
(`-Wl,-rpath,$ORIGIN`, `@executable_path` on macOS) or set the library search path at run time.

### What is in runtime/

| Folder | Files | Size |
|---|---:|---:|
| `runtime/windows-x64/` | 18 | 166.7 MiB |
| `runtime/linux-x64/` | 17 | 227.7 MiB |
| `runtime/macos-arm64/` | 17 | 81.3 MiB |

Each folder holds the Rust package's `native/bin/` and `native/lib/` (Windows import library `kui_c.lib`) or `native/lib/` (Linux / macOS), byte-identical to the same-platform SDK package on the Release and in the same relative layout (the Linux / macOS RPATHs rely on it).

### What is on the Release

Each version's [kui Release](https://github.com/kui-engine/kui/releases/tag/v3.61.8) has three kinds of attachments:

1. **SDK packages** `kui-<language>-sdk-<version>-<platform>.zip`: the corresponding source repository packed for one platform (`runtime/<platform>/` is `bin/` and `lib/` in the package, `native/` for Go / Rust), without prebuilt example programs; download one if you prefer not to `git clone`.
2. **Prebuilt examples packages** `kui-<language>-examples-<version>-<platform>.zip`: example programs plus the runtime; extract and run them from `bin/`, no compiler needed.
3. **Checksums** `SHA256SUMS.txt` (covering the two kinds above) and the FFmpeg corresponding source.

| Package | Contents | Windows x64 | Linux x64 | macOS arm64 |
|---|---|---|---|---|
| `kui-rust-sdk` | SDK (= the source repository for one platform) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-windows-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-linux-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-sdk-3.1.0-macos-arm64.zip) |
| `kui-rust-examples` | Prebuilt examples (run right after extraction) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-windows-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-linux-x64.zip) | [zip](https://github.com/kui-engine/kui/releases/download/v3.61.8/kui-rust-examples-3.1.0-macos-arm64.zip) |

### Platforms

| Platform | Status | Acceptance actually performed when packaged |
|---|---|---|
| Windows x64 | Full Profile GA | The 4 SDK packages are rebuilt from their own sources after extraction and pass the out-of-package consumer matrix; every program in the 4 prebuilt examples packages is actually launched (C 222, C++ Wrapper 190, Go 219, Rust 219). After export, the four repositories were built from real `git clone`s with the README commands and the programs launched (including the parameterless `build.ps1`) |
| Linux x64 | Release package of the same frozen C ABI | The SDK packages rebuild from their sources; the example programs are launched under Xvfb: C 221/222, C++ Wrapper 190/190, Go 218/219, Rust 218/219 — C, Go and Rust each leave out 1 tray example (Xvfb has no tray host, `kui_window_tray_create` reports not supported), allow-listed. After export, the four repositories were built from real `git clone`s with the README commands and launched under Xvfb (including the parameterless Go / Rust `build.sh`, 219 programs each) |
| macOS arm64 | Release package of the same frozen C ABI | The SDK packages rebuild from their sources (C 11 + 211, C++ Wrapper 189 + demo + 3 test suites, Go / Rust 8 + 211) without launching the rebuilt programs; the examples packages are checked for layout, manifest, byte-identical runtimes, dependencies and RPATHs, plus launching 6 windowless C examples and the C++ Wrapper demo. **Windowed programs were not launched on macOS** (the build Mac is a shared machine). After export, the four repositories were built from real `git clone`s with no parameters (including the Go / Rust `build.sh`, 219 programs each) and the dependency and rpath closure checked; only windowless programs and tests were run, **no windowed program was launched** |

All three platforms ship Release runtimes only, without debug symbols; your Debug builds link the same runtime (the C ABI passes no CRT objects across the boundary). Platform capabilities are not fully symmetric; see `docs/known_issues.md` for known differences.

### If a downloaded program does not start

The prebuilt programs and runtimes are not signed yet:

- **Windows**: for a zip downloaded by a browser, when SmartScreen says "Windows protected your PC", click "More info" → "Run anyway"; or run `Get-ChildItem -Recurse <extracted folder> | Unblock-File` in PowerShell once. Files from `git clone` carry no mark-of-the-web, so SmartScreen does not block them.
- **macOS**: Gatekeeper blocks a zip downloaded by a browser; run `xattr -dr com.apple.quarantine <extracted folder>`.
- **Linux**: if your unzip tool drops permissions, `chmod +x` the programs under `bin/` (`git clone` keeps the runtime's executable bits).

### License and third-party components

MIT, see `LICENSE` (copyright Alloy).

KUI itself is MIT. The video plugin `kui_media_video` loads FFmpeg (LGPL-3.0-or-later, an LGPL build without `--enable-gpl` / `--enable-nonfree`) as separate shared libraries; if you do not need video, leave out `kui_media_video` and the FFmpeg libraries (the audio plugin does not use FFmpeg). In the repository's `runtime/<platform>/` and in every package, `LICENSE.FFmpeg.txt` (LGPLv3) and `LICENSE.FFmpeg.GPLv3.txt` sit next to the FFmpeg libraries; all other third-party components are under permissive licenses, see `THIRD_PARTY_NOTICES.md`.

**The corresponding source of the FFmpeg libraries distributed with the binaries** is attached to the [kui Release](https://github.com/kui-engine/kui/releases/tag/v3.61.8); checksums are in `FFMPEG_SOURCE_SHA256SUMS.txt`:

- Windows x64 / Linux x64 (BtbN build `autobuild-2026-10-31-12-59`, FFmpeg `n9.0.2-39-g7adfbe40dc`): `ffmpeg-7adfbe40dcde598f93a6963d2f7b22cd75202ccc.tar.gz` (source of FFmpeg commit `7adfbe40dcde598f93a6963d2f7b22cd75202ccc`) and `FFmpeg-Builds-autobuild-2026-10-31-12-59.tar.gz` (the build scripts);
- macOS arm64 (built from the official release tarball): `ffmpeg-9.0.2.tar.xz` and its signature `ffmpeg-9.0.2.tar.xz.asc`.

Upstream: <https://github.com/FFmpeg/FFmpeg/tree/7adfbe40dcde598f93a6963d2f7b22cd75202ccc>, <https://github.com/BtbN/FFmpeg-Builds/tree/autobuild-2026-10-31-12-59>, <https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz>.
