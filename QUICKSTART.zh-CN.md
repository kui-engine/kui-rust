# KUI Rust 快速上手

[English](QUICKSTART.md) · **简体中文**

在一个空的 Cargo 项目里，用一个 HTML 文件做出原生窗口：一张带小表单和计数器的卡片，每点一次按钮，计数加一。

<p align="center"><img src="docs/media/quickstart.gif" width="360" alt="快速开始窗口的录屏：输入姓名、选套餐和日期、切换选项、拖动滑块，再点三次按钮"></p>
<p align="center"><sub>真实窗口的录屏：改表单，再点三次按钮。</sub></p>

页面就是普通的 HTML 和 CSS。除了计数器，还有一小块表单（文本框、下拉选择、日期选择、复选框、单选和滑块）。这些控件由 KUI 自己绘制和处理，不需要写代码：上面的录屏里依次输入了姓名、选了套餐和日期、切换了选项、拖了滑块，又点了按钮。

完整 API、构建脚本和全部示例见 [`README.md`](README.md) 与 [`GUIDE.md`](GUIDE.md)。

## 1. 需要什么

本页在 Windows 10 x64 上用以下版本验证过：

- Rust 1.98.0 stable，目标 `x86_64-pc-windows-msvc`（cargo 1.98.0）
- Visual Studio 2022 C++ 生成工具（Rust 的 MSVC 目标要用它的链接器）
- Git 2.45.1

## 2. 拿到 SDK

仓库里带运行库和导入库 `kui_c.lib`。在一个空目录里执行：

```powershell
mkdir kui-quickstart; cd kui-quickstart
git clone --depth 1 https://github.com/kui-engine/kui-rust.git
$KUI = "$PWD\kui-rust"
mkdir hello-kui; cd hello-kui
```

固定版本的写法：`git clone --depth 1 --branch v3.1.0 https://github.com/kui-engine/kui-rust.git`。

> 请保留 `--depth 1`。每个版本都把运行库提交进历史，完整历史很大。

## 3. 建项目

`hello-kui` 里放 `Cargo.toml`、`src/main.rs` 和 `app.html`。

`Cargo.toml`：

```toml
[package]
name = "hello-kui"
version = "0.1.0"
edition = "2021"

[dependencies]
kui = { path = "../kui-rust" }
```

path 依赖让编译直接用项目旁边的克隆，本页就是用这种写法实测的。另一种写法是 git 依赖：`kui = { git = "https://github.com/kui-engine/kui-rust", tag = "v3.1.0" }`（crates.io 上的 `kui` 这个名字属于别的项目，所以没有发布到 crates.io）。

> git 依赖会把整个仓库（含运行库）检出到 Cargo 的主目录下（本版约 0.9 GB），`kui-sys` 会在那里自己找到 `kui_c.lib`。DLL 仍从一个你知道位置的目录复制，例如 `$KUI`。

`src/main.rs`：

```rust
use std::error::Error;

use kui::{Platform, Window, WindowProps};

fn main() -> Result<(), Box<dyn Error>> {
    let platform = Platform::init()?; // this thread is now the UI thread
    let window = Window::new(&WindowProps {
        title: "KUI Quick Start".into(),
        width: 560,
        height: 640,
        no_resize: true,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?; // a counted reference to the window's document

    let page = std::env::current_exe()?.with_file_name("app.html");
    document.load_html_file(page.to_str().ok_or("path is not UTF-8")?)?;

    let button = document.get_element_by_id("inc")?.ok_or("app.html has no #inc")?;
    let doc = document.clone(); // the listener must be 'static, so it keeps its own reference
    let mut clicks = 0;
    let listener = button.add_event_listener("click", move |_event| {
        clicks += 1;
        if let Ok(Some(count)) = doc.get_element_by_id("count") {
            let _ = count.set_text_content(&clicks.to_string()); // the DOM update
        }
    })?;

    window.show()?;
    platform.run()?; // returns when the window is closed

    button.remove_event_listener("click", listener)?; // frees the closure and its `doc`
    Ok(()) // then button, document, window and platform drop in reverse order
}
```

`app.html` 是界面。程序只依赖其中两个 id：`#count`（数字）和 `#inc`（按钮）。

<details>
<summary>app.html（界面，复制即可）</summary>

```html
﻿<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>KUI Quick Start</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  html, body { height: 100%; }
  body {
    display: flex; align-items: center; justify-content: center;
    font-family: "Segoe UI", system-ui, -apple-system, "Helvetica Neue", Arial, sans-serif;
    font-size: 14px; color: #e2e8f0;
    background: linear-gradient(135deg, #0f172a 0%, #1e1b4b 55%, #312e81 100%);
  }
  .card {
    display: flex; flex-direction: column;
    width: 480px; padding: 28px 30px 20px;
    border-radius: 24px;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid rgba(255, 255, 255, 0.14);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45);
  }
  .head { display: flex; flex-direction: column; align-items: center; text-align: center; }
  .badge {
    display: inline-block; padding: 4px 14px; border-radius: 999px;
    font-size: 12px; font-weight: 600; letter-spacing: 2px;
    color: #c7d2fe; background: rgba(99, 102, 241, 0.28);
  }
  h1 { margin-top: 12px; font-size: 26px; font-weight: 700; color: #ffffff; }
  .sub { margin-top: 6px; font-size: 14px; color: #94a3b8; }

  .field { display: flex; flex-direction: column; margin-top: 16px; }
  .field label { margin-bottom: 6px; font-size: 12px; font-weight: 600; letter-spacing: 0.5px; color: #a5b4fc; }
  .pair { display: flex; gap: 14px; }
  .pair .field { flex: 1; }
  input[type="text"], input[type="date"], select {
    width: 100%; padding: 9px 12px; font-size: 14px; color: #f1f5f9;
    background: rgba(15, 23, 42, 0.55);
    border: 1px solid rgba(165, 180, 252, 0.28); border-radius: 10px;
  }
  input[type="text"]:focus, input[type="date"]:focus, select:focus { border-color: #818cf8; outline: none; }

  .opts { display: flex; align-items: center; justify-content: space-between; margin-top: 16px; }
  .opts label { display: flex; align-items: center; font-size: 14px; color: #cbd5e1; }
  .opts input { margin-right: 8px; }
  .radios { display: flex; gap: 16px; }
  input[type="range"] { width: 100%; }

  .action { display: flex; align-items: center; justify-content: space-between; margin-top: 22px; padding-top: 18px;
            border-top: 1px solid rgba(255, 255, 255, 0.1); }
  .counter { display: flex; align-items: baseline; }
  #count { font-size: 44px; font-weight: 800; line-height: 1; color: #a5b4fc; }
  #label { margin-left: 8px; font-size: 14px; color: #94a3b8; }
  #inc {
    padding: 12px 30px; border: 0; border-radius: 14px;
    font-size: 16px; font-weight: 600; color: #ffffff;
    background: linear-gradient(135deg, #6366f1, #8b5cf6);
    box-shadow: 0 10px 24px rgba(99, 102, 241, 0.45);
    transition: transform 0.15s, box-shadow 0.15s;
  }
  #inc:hover { transform: translateY(-2px); box-shadow: 0 14px 30px rgba(99, 102, 241, 0.6); }
  #inc:active { transform: translateY(1px); }
  .foot { margin-top: 18px; text-align: center; font-size: 12px; color: #64748b; }
</style>
</head>
<body>
  <div class="card">
    <div class="head">
      <span class="badge">KUI</span>
      <h1>Hello from KUI</h1>
      <p class="sub">A native window drawn from HTML and CSS.</p>
    </div>

    <div class="field">
      <label>NAME</label>
      <input id="name" type="text" value="Ada Lovelace">
    </div>
    <div class="pair">
      <div class="field">
        <label>PLAN</label>
        <select id="plan">
          <option>Free</option>
          <option selected>Pro</option>
          <option>Team</option>
        </select>
      </div>
      <div class="field">
        <label>START DATE</label>
        <input id="start" type="date" value="2026-12-01">
      </div>
    </div>
    <div class="opts">
      <label><input id="news" type="checkbox" checked>Email me updates</label>
      <div class="radios">
        <label><input type="radio" name="billing" checked>Monthly</label>
        <label><input type="radio" name="billing">Yearly</label>
      </div>
    </div>
    <div class="field">
      <label>VOLUME</label>
      <input id="volume" type="range" min="0" max="100" value="50">
    </div>

    <div class="action">
      <div class="counter"><div id="count">0</div><div id="label">clicks</div></div>
      <button id="inc">Click me</button>
    </div>
    <div class="foot">One engine · Windows, Linux, macOS</div>
  </div>
</body>
</html>
```

</details>

## 4. 运行库放哪

- **链接**：`kui-sys` 的构建脚本会自己找到 `$KUI/runtime/windows-x64/lib/kui_c.lib`，不用设变量。`KUI_C_SDK`（运行库根目录）或 `KUI_C_LIB_DIR`（`kui_c.lib` 所在目录）可以改指别处。
- **运行**：Windows 在可执行文件旁边（或 `PATH` 里）找 `kui_c.dll` 和 `kui.dll`。构建步骤把 `$KUI/runtime/windows-x64/bin` 连同 `app.html` 复制到 `target\debug`。本程序只用到这两个 DLL，其余文件是 3D、编辑器、PDF 扩展、媒体插件和 FFmpeg。

## 5. 最小窗口

`Platform::init` 在当前线程上启动 KUI。`Window::new` 创建窗口，`width`、`height` 是客户区大小，这里是 560×640。`load_html_file` 加载 `app.html`，`platform.run()` 跑消息循环，直到窗口被关掉。

## 6. 一个 DOM 更新

`doc.get_element_by_id("count")` 返回 `Result<Option<Element>>`（没有这个 id 时是 `None`），`set_text_content` 替换它的文字。本例在点击回调里调用它，数字跟着 `clicks` 走。

## 7. 一个事件监听

`button.add_event_listener("click", 闭包)` 在事件派发时于 UI 线程上调用闭包。闭包必须是 `'static`，但不要求 `Send`，所以它持有文档的一个克隆，而不是借用 `count`。

## 8. 资源释放

- `Platform`、`Window`、`Document`、`Element` 都不是 `Send`：在调用 `Platform::init` 的线程上创建、使用和 drop。其他线程用 `kui::post_to_main_thread` 把工作投递过来。
- `Window` 拥有原生窗口，drop 即销毁窗口。`window.document()` 和 `document.clone()` 各返回一份计数引用，drop 时释放。
- `Element<'doc>` 是计数引用，生命周期绑在它的 `Document` 上，不可能比文档活得久。drop 元素不会移除它的监听。
- 监听闭包保存在 UI 线程的登记表里，直到监听被移除（否则留到线程退出）。`run()` 之后调用 `remove_event_listener`，趁平台还在时释放闭包和它持有的文档克隆。
- 传给监听回调的 `Event` 只在回调内有效，由生命周期保证。
- 返回时局部变量倒序 drop：`button`、`document`、`window`，最后是 `platform`，它负责关闭 KUI。

## 9. 构建与运行

Windows PowerShell，在 `hello-kui` 里：

```powershell
cargo build
Copy-Item $KUI\runtime\windows-x64\bin\*, app.html target\debug\
.\target\debug\hello-kui.exe
```

窗口显示 0，每点一次加一。关掉窗口程序就结束。

> 本页只在 Windows x64 上验证过。Linux 与 macOS 的运行库在 `$KUI/runtime/linux-x64/lib` 或 `$KUI/runtime/macos-arm64/lib`，`kui-sys` 会自己链接它。你自己的程序不会自动得到指向那个目录的运行时搜索路径：把运行库放到程序旁边并给程序一个相对 RPATH（`$ORIGIN`，macOS 是 `@executable_path`），或在运行时设置库搜索路径。macOS 上窗口必须在主线程（`main` 所在的线程）创建。这两个平台验证过的命令见 [`README.md`](README.md) 的「编译并运行一个示例」。

## 10. 常见问题

- **`Error: "app.html has no #inc"`**：`app.html` 不在可执行文件旁边。本版 `load_html_file` 不会报告文件不存在（文档只是保持为空），而且相对路径按当前目录解析，不按可执行文件所在目录；所以 `main.rs` 用 `current_exe()` 拼出完整路径。
- **退出码 `0xC0000135`，或弹窗提示找不到 `kui_c.dll`**：可执行文件旁边没有 DLL。`cargo clean` 会连同 DLL 一起删掉 `target\debug`，要重新复制。
- **`cargo build`（debug profile）需要 Debug 版运行库吗？** 不需要。Rust 的 MSVC 目标总是用 Release CRT，C ABI 也不传 CRT 对象，debug profile 链接同一个 `kui_c.lib`。
- **`kui-sys` 的构建脚本 panic，提示「找不到 kui_c.lib」**：它在所用的仓库里没找到运行库。检查 `Cargo.toml` 里的 `path`，或把 `KUI_C_SDK` 设成 `lib/` 下有 `kui_c.lib` 的目录。
