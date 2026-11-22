# KUI Rust quickstart

**English** · [简体中文](QUICKSTART.zh-CN.md)

Build a native window from an HTML file in an empty Cargo project: a card with a small form and a counter that goes up each time you click the button.

<p align="center"><img src="docs/media/quickstart.gif" width="360" alt="The quickstart window, recorded: typing a name, picking a plan and a date, switching options, dragging the slider and clicking the button three times"></p>
<p align="center"><sub>A real window, recorded: edit the form, then click the button three times.</sub></p>

The page is ordinary HTML and CSS. Next to the counter it has a small form (text input, select, date picker, checkbox, radio buttons and a slider). KUI draws and handles those controls itself, so they work without any code: the recording above types a name, picks a plan and a date, switches the options, drags the slider and clicks the button.

The full API, the build scripts and all examples are in [`README.md`](README.md) and [`GUIDE.md`](GUIDE.md).

## 1. What you need

This page was verified on Windows 10 x64 with these versions:

- Rust 1.98.0 stable, target `x86_64-pc-windows-msvc` (cargo 1.98.0)
- Visual Studio 2022 C++ build tools (the MSVC linker that Rust's MSVC target uses)
- Git 2.45.1

## 2. Get the SDK

The repository carries the runtime libraries and the import library `kui_c.lib`. In an empty folder:

```powershell
mkdir kui-quickstart; cd kui-quickstart
git clone --depth 1 https://github.com/kui-engine/kui-rust.git
$KUI = "$PWD\kui-rust"
mkdir hello-kui; cd hello-kui
```

To pin a version: `git clone --depth 1 --branch v3.1.0 https://github.com/kui-engine/kui-rust.git`.

> Keep `--depth 1`. Every version commits its runtime libraries, so the full history is large.

## 3. Create the project

`hello-kui` holds `Cargo.toml`, `src/main.rs` and `app.html`.

`Cargo.toml`:

```toml
[package]
name = "hello-kui"
version = "0.1.0"
edition = "2021"

[dependencies]
kui = { path = "../kui-rust" }
```

The path dependency builds against the clone next to the project; this page was tested with it. The alternative is a git dependency, `kui = { git = "https://github.com/kui-engine/kui-rust", tag = "v3.1.0" }` (the `kui` name on crates.io belongs to another project, so the crate is not published there).

> A git dependency checks out the whole repository, runtime included, under Cargo's home (about 0.9 GB for this version), and `kui-sys` finds `kui_c.lib` there by itself. You still copy the DLLs from a folder you know, such as `$KUI`.

`src/main.rs`:

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

`app.html` is the interface. The program only relies on two ids in it: `#count` (the number) and `#inc` (the button).

<details>
<summary>app.html (the interface, copy as is)</summary>

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

## 4. Where the runtime goes

- **Linking**: the build script of `kui-sys` finds `$KUI/runtime/windows-x64/lib/kui_c.lib` by itself; no variables needed. `KUI_C_SDK` (runtime root) or `KUI_C_LIB_DIR` (folder of `kui_c.lib`) override the search.
- **Running**: Windows looks for `kui_c.dll` and `kui.dll` next to the executable (or on `PATH`). The build step copies `$KUI/runtime/windows-x64/bin`, together with `app.html`, into `target\debug`. This program needs only those two DLLs; the other files are the 3D, editor and PDF extensions, the media plugins and FFmpeg.

## 5. Minimal window

`Platform::init` starts KUI on the current thread. `Window::new` creates the window; `width` and `height` are the client area, 560×640 here. `load_html_file` loads `app.html`, and `platform.run()` runs the message loop until the window is closed.

## 6. A DOM update

`doc.get_element_by_id("count")` returns `Result<Option<Element>>` (`None` when no element has that id); `set_text_content` replaces its text. Here it runs inside the click handler, so the number follows `clicks`.

## 7. An event listener

`button.add_event_listener("click", closure)` calls the closure on the UI thread while the event is dispatched. The closure must be `'static` but need not be `Send`, so it owns a clone of the document instead of borrowing `count`.

## 8. Releasing resources

- `Platform`, `Window`, `Document` and `Element` are not `Send`; create, use and drop them on the thread that called `Platform::init`. Other threads hand work over with `kui::post_to_main_thread`.
- `Window` owns the native window; dropping it destroys the window. `window.document()` and `document.clone()` each return a counted reference that is released on drop.
- `Element<'doc>` is a counted reference tied to its `Document` by lifetime, so it cannot outlive it. Dropping an element does not remove its listeners.
- A listener's closure lives in a UI-thread table until the listener is removed (otherwise until the thread exits). `remove_event_listener` after `run()` frees the closure and the document clone it owns while the platform is still up.
- The `Event` passed to a listener is valid only inside the callback; its lifetime enforces that.
- On return, locals drop in reverse order: `button`, `document`, `window`, then `platform`, which shuts KUI down.

## 9. Build and run

Windows PowerShell, inside `hello-kui`:

```powershell
cargo build
Copy-Item $KUI\runtime\windows-x64\bin\*, app.html target\debug\
.\target\debug\hello-kui.exe
```

The window shows 0; every click adds one. Closing the window ends the program.

> This page was only verified on Windows x64. On Linux and macOS the runtime is `$KUI/runtime/linux-x64/lib` or `$KUI/runtime/macos-arm64/lib`, and `kui-sys` links against it by itself. Your own program does not get a run-time search path to that folder: put the libraries next to it and give it a relative RPATH (`$ORIGIN`, or `@executable_path` on macOS), or set the library search path when you run it. On macOS the window must be created on the main thread, which is where `main` runs. Commands verified on those platforms are in "Building and running an example" in [`README.md`](README.md).

## 10. Common problems

- **`Error: "app.html has no #inc"`**: `app.html` is not next to the executable. `load_html_file` does not report a missing file in this version (the document just stays empty), and a relative path is resolved against the current directory, not the executable's folder; that is why `main.rs` builds the full path from `current_exe()`.
- **Exit code `0xC0000135`, or a dialog saying `kui_c.dll` was not found**: the DLLs are not next to the executable. `cargo clean` deletes `target\debug` with them; copy them again.
- **Do I need a Debug runtime for `cargo build`?** No. Rust's MSVC target always uses the release CRT, the C ABI passes no CRT objects, and the debug profile links the same `kui_c.lib`.
- **`kui-sys` build script panics with "找不到 kui_c.lib"**: it found no runtime in the repository it was built from. Check the `path` in `Cargo.toml`, or set `KUI_C_SDK` to a folder whose `lib/` holds `kui_c.lib`.
