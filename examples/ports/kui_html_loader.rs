// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_html_loader.rs; edit that file, not this copy.
//! Host logic of examples/html_loader/main.cpp (kui_html_loader, the baseline variant): scans the
//! sample folder, loads the first sample with the navigation bar injected after <body>, titles the
//! window after it, and wires the bar: the sample <select> switches pages, "Open File..." loads any
//! HTML file, "Inspect" toggles its own highlight. Each load gets a fresh script engine and the
//! virtual_components sample gets its VirtualList / ListView / TreeView and scroll-unit toggle.
//! The four kui_html_loader* ports are the same file except `VARIANT`.
//! `prepare` sets the variant's renderer presets (KUI_ENABLE_RETAINED_SURFACE /
//! KUI_ENABLE_LAYER_PROMOTION, only when unset: the user's environment wins) before any KUI call,
//! handles --help / --list and parses the command line as the original (sample name substring,
//! --file, --dev, --devtools [port], --devtools-port, --size, --frameless, --shadow, --radius,
//! --sizeable, --show-after-load, --dpr).
//!
//! Differences from the original, all forced by the C ABI or the runner:
//! - Window shape: --radius, --sizeable and --show-after-load go into WindowProps (`frame_radius`,
//!   `sizeable`, `hidden`), so they take effect at creation like the original's WindowConfig;
//!   --show-after-load then renders the first frame (`process_frame`) and shows the window once the
//!   page is loaded.
//! - --dev and the script engine's console: `console_set_*`; --no-token: `set_devtools_token_auth`;
//!   --dpr: `set_render_device_pixel_ratio`. --devtools uses the window's
//!   DevTools server (`start_devtools`) and is skipped when it already runs.
//! - The runner loads the packaged "No samples found" page first; the port replaces it.
//! - Base path: `Document::set_base_path`.
//! - Sample switching listens to the DOM "change" event and reads selectedIndex through Lua; the
//!   page is replaced from a zero-delay timer, because a listener must not destroy the tree that
//!   is dispatching it (the original's onChange runs after dispatch).
//! - "Inspect" only toggles the button style: there is no ABI for EventDispatcher's inspector
//!   mode or inspect callback (DevTools on demand, launchBrowser, pushInspectElement).
//! - Open dialog filters carry no labels. The stack trace writer and the crash reporter are installed in
//!   `prepare`, as the original does before Platform::init.
//! - Samples are also searched in <exe>/assets/native_samples, where the Rust package puts them.

use crate::canonical_runner::Context;
use kui::{
    Document, FileDialogOptions, ListView, ListViewConfig, ListViewItem, Platform, ScriptEngine,
    ScriptValue, ScrollUnit, SelectionMode, TreeView, TreeViewConfig, VirtualList,
    VirtualListConfig, Window, WindowProps,
};
use std::cell::{Cell, RefCell};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const VARIANT: u32 = 0; // 1 = retained surface, 2 = layer promotion, 3 = both

const INSPECT_ON: &str = "padding:4px 12px;font-size:12px;cursor:pointer;background:#0c4a6e;color:#38bdf8;border:1px solid #38bdf8;border-radius:4px;flex-shrink:0;";
const INSPECT_OFF: &str = "padding:4px 12px;font-size:12px;cursor:pointer;background:#0f172a;color:#94a3b8;border:1px solid #475569;border-radius:4px;flex-shrink:0;";

struct Sample {
    name: String,
    filename: String,
    path: PathBuf,
}

/// Command line (main() of the original), filled by `prepare`.
#[derive(Default)]
struct Options {
    samples: Vec<Sample>,
    current: i32,
    file_to_load: Option<String>,
    devtools: bool,
    devtools_port: u16,
    sizeable: bool,
    show_after_load: bool,
    radius: Option<f32>,
    dev: bool,
    no_token: bool,
    backend_dpr: f32,
}

thread_local! {
    static OPTIONS: RefCell<Options> = RefCell::new(Options {
        devtools_port: 9229,
        ..Options::default()
    });
}

struct Loader {
    document: Document,
    window: Rc<Window>,
    platform: Rc<Platform>,
    runner_script: Rc<ScriptEngine>,
    engine: RefCell<Option<Rc<ScriptEngine>>>, // recreated on every load, like setupScriptEngine
    samples: Vec<Sample>,
    current: Cell<i32>,
    inspect: Cell<bool>,
    vlist: RefCell<Option<VirtualList>>,
    list: RefCell<Option<ListView>>,
    tree: RefCell<Option<TreeView>>,
}

/// `_putenv_s` of the CRT kui.dll uses (ucrtbase.dll): it keeps its own copy of the environment,
/// so `env::set_var` (SetEnvironmentVariableW) alone never reaches KUI's getenv on Windows.
#[cfg(windows)]
fn crt_setenv(key: &str, value: &str) {
    use std::ffi::{c_char, c_int, c_void, CString};
    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    type PutenvS = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
    let (Ok(key), Ok(value)) = (CString::new(key), CString::new(value)) else {
        return;
    };
    // SAFETY: ucrtbase.dll exports _putenv_s with exactly this signature; both strings are
    // NUL-terminated and outlive the call.
    unsafe {
        let crt = LoadLibraryA(c"ucrtbase.dll".as_ptr());
        if crt.is_null() {
            return;
        }
        let proc = GetProcAddress(crt, c"_putenv_s".as_ptr());
        if !proc.is_null() {
            let putenv_s: PutenvS = std::mem::transmute(proc);
            putenv_s(key.as_ptr(), value.as_ptr());
        }
    }
}

/// applyR17EnvPreset: the user's value wins, so only unset variables are written.
fn apply_variant_preset() {
    let set_if_unset = |key: &str| {
        if env::var(key).map_or(true, |value| value.is_empty()) {
            env::set_var(key, "1"); // libc setenv on POSIX, where KUI's getenv sees it
            #[cfg(windows)]
            crt_setenv(key, "1");
        }
    };
    if VARIANT & 1 != 0 {
        set_if_unset("KUI_ENABLE_RETAINED_SURFACE");
    }
    if VARIANT & 2 != 0 {
        set_if_unset("KUI_ENABLE_LAYER_PROMOTION");
    }
}

const USAGE: &str = "Usage: kui_html_loader [options] [sample]
  --file <path.html>   load an arbitrary HTML file
  --list               print available samples and exit
  --dev                enable dev console (errors/warnings to stderr)
  --devtools [port]    start DevTools server (default 9229)
  --devtools-port N    explicit DevTools server port
  --no-token           disable DevTools WebSocket token auth (testing)
Window shape (defaults: 1024x768 decorated window):
  --size WxH           initial client size, e.g. 1280x720
  --frameless          frameless window (no title bar / borders)
  --shadow             window shadow (frameless only)
  --radius <px>        rounded frameless window, radius in DIP
  --sizeable           edge-resizable frameless window
  --show-after-load    create hidden, load the page, render one frame, then show()
                       (the order real apps such as mini-erp-burn use)
  --dpr <x>            simulated backend device pixel ratio (measurement only)";

pub fn prepare(args: &[String], props: &mut WindowProps) -> Option<i32> {
    apply_variant_preset();
    let samples = find_samples();
    match args.first().map(String::as_str) {
        Some("--help") | Some("-h") => {
            println!("{USAGE}");
            return Some(0);
        }
        Some("--list") => {
            println!("Available samples ({}):", samples.len());
            for (i, sample) in samples.iter().enumerate() {
                println!("  [{i}] {:<30} {}", sample.name, sample.filename);
            }
            return Some(0);
        }
        _ => {}
    }
    OPTIONS.with(|cell| {
        let mut o = cell.borrow_mut();
        o.samples = samples;
        let mut sample_to_find = None;
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            let next = args.get(i + 1);
            match (arg, next) {
                ("--size", Some(value)) => {
                    i += 1;
                    let size = value
                        .split_once('x')
                        .and_then(|(w, h)| Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?)));
                    match size {
                        Some((w, h))
                            if (160..=16384).contains(&w) && (120..=16384).contains(&h) =>
                        {
                            props.width = w;
                            props.height = h;
                        }
                        _ => eprintln!(
                            "[HTML Loader] Ignoring invalid --size '{value}' (expected WxH)"
                        ),
                    }
                }
                ("--frameless", _) => {
                    props.frameless = true;
                    props.no_decorations = true;
                }
                ("--shadow", _) => props.frameless_shadow = true,
                ("--sizeable", _) => {
                    o.sizeable = true;
                    props.sizeable = true;
                }
                ("--show-after-load", _) => {
                    o.show_after_load = true;
                    props.hidden = true;
                }
                ("--radius", Some(value)) => {
                    i += 1;
                    if let Ok(r) = value.parse::<f64>() {
                        if (0.0..=1024.0).contains(&r) {
                            o.radius = Some(r as f32);
                            props.frame_radius = Some(r as f32);
                        }
                    }
                }
                ("--dpr", Some(value)) => {
                    i += 1;
                    o.backend_dpr = value.parse().unwrap_or(0.0);
                }
                ("--dev", _) => o.dev = true,
                ("--devtools", _) => {
                    o.devtools = true;
                    if let Some(port) = next.filter(|v| !v.starts_with('-')) {
                        if let Ok(port) = port.parse::<u16>() {
                            if port > 0 {
                                o.devtools_port = port;
                                i += 1;
                            }
                        }
                    }
                }
                ("--devtools-port", Some(value)) => {
                    i += 1;
                    if let Ok(port) = value.parse::<u16>() {
                        if port > 0 {
                            o.devtools_port = port;
                            o.devtools = true;
                        }
                    }
                }
                ("--no-token", _) => o.no_token = true,
                ("--file", Some(value)) => {
                    i += 1;
                    o.file_to_load = Some(value.clone());
                }
                (other, _) if !other.starts_with('-') => sample_to_find = Some(other.to_string()),
                _ => {}
            }
            i += 1;
        }
        if let Some(name) = sample_to_find {
            // Case-insensitive substring match on the file name; the first match wins.
            let needle = name.to_lowercase();
            match o
                .samples
                .iter()
                .position(|s| s.filename.to_lowercase().contains(&needle))
            {
                Some(index) => o.current = index as i32,
                None => eprintln!(
                    "[HTML Loader] No sample matching '{name}'. Use --list to see available."
                ),
            }
        }
    });
    if OPTIONS.with(|cell| cell.borrow().dev) {
        let _ = kui::console_set_enabled(true);
        let _ = kui::console_set_stderr_output(true);
        let _ = kui::console_set_min_level(kui::LogLevel::Debug);
        eprintln!("[DEV] Developer console enabled");
    }
    install_crash_diagnostics();
    None
}

/// The stack trace writer and the crash reporter, as the original installs them before Platform::init.
fn install_crash_diagnostics() {
    let stack = kui::StackTraceConfig {
        output_mode: 1 | 2 | 4,
        app_name: "kui_html_loader".into(),
        write_mini_dump: true,
        max_frames: 96,
        ..Default::default()
    };
    if !matches!(kui::stack_trace_install(&stack), Ok(true)) {
        eprintln!("[StackTrace] Warning: in-process crash handler install failed.");
    }
    let version = kui::version();
    let crash = kui::CrashReporterConfig {
        app_name: "kui_html_loader".into(),
        app_version: format!("{}.{}.{}", version.major, version.minor, version.patch),
        ..Default::default()
    };
    if !matches!(kui::crash_reporter_install(&crash), Ok(true)) {
        eprintln!("[CrashReporter] Warning: crash reporting unavailable.");
    }
}

/// The window shape (radius, sizeable, hidden) was set at creation through `WindowProps`.
pub fn before_load(_ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let options = OPTIONS.with(|cell| std::mem::take(&mut *cell.borrow_mut()));
    if options.backend_dpr > 0.0 {
        let _ = ctx
            .document
            .set_render_device_pixel_ratio(options.backend_dpr);
    }
    ctx.platform.set_f12_devtools_enabled(true)?;
    let loader = Rc::new(Loader {
        document: ctx.document.clone(),
        window: ctx.window.clone(),
        platform: ctx.platform.clone(),
        runner_script: ctx.script.clone(),
        engine: RefCell::new(None),
        samples: options.samples,
        current: Cell::new(options.current),
        inspect: Cell::new(false),
        vlist: RefCell::new(None),
        list: RefCell::new(None),
        tree: RefCell::new(None),
    });
    let on_close = loader.clone();
    ctx.window
        .add_event_listener("close", true, move |_| on_close.close())?;
    if options.devtools {
        let running = ctx
            .window
            .devtools_state()
            .map_or(false, |(running, _, _)| running);
        let started = !running
            && ctx
                .window
                .start_devtools(options.devtools_port)
                .unwrap_or(false);
        if running || started {
            if options.no_token {
                let _ = ctx.window.set_devtools_token_auth(false);
            }
            if let Ok(Some(url)) = ctx.window.devtools_url() {
                eprintln!("[DevTools] Open {url} in your browser to inspect");
            }
        }
    }
    // No samples and no --file: the runner's page is the original's "No samples found" page.
    if let Some(file) = &options.file_to_load {
        load_external_file(&loader, Path::new(file));
    } else if !loader.samples.is_empty() {
        load_sample(&loader, loader.current.get());
    }
    if options.show_after_load {
        ctx.window.process_frame()?;
        ctx.window.show()?;
    }
    Ok(())
}

/// <exe>/samples, ./samples, examples/html_loader/samples, then the package layout.
fn find_samples() -> Vec<Sample> {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let dirs = [
        exe_dir.join("samples"),
        PathBuf::from("samples"),
        PathBuf::from("examples/html_loader/samples"),
        exe_dir.join("assets").join("native_samples"),
    ];
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.file_type().map_or(false, |kind| kind.is_file()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                Path::new(name)
                    .extension()
                    .map_or(false, |ext| ext == "html")
            })
            .collect();
        names.sort();
        if !names.is_empty() {
            return names
                .into_iter()
                .map(|filename| Sample {
                    name: pretty_name(&filename),
                    path: dir.join(&filename),
                    filename,
                })
                .collect();
        }
    }
    Vec::new()
}

/// "01_box_model.html" -> "Box Model"
fn pretty_name(filename: &str) -> String {
    let mut s = filename.rsplit_once('.').map_or(filename, |(stem, _)| stem);
    let b = s.as_bytes();
    if b.len() > 3 && b[2] == b'_' && b[0].is_ascii_digit() && b[1].is_ascii_digit() {
        s = &s[3..];
    }
    let mut out = String::new();
    let mut cap_next = true;
    for c in s.chars() {
        if c == '_' {
            out.push(' ');
            cap_next = true;
        } else {
            out.push(if cap_next { c.to_ascii_uppercase() } else { c });
            cap_next = false;
        }
    }
    out
}

fn nav_bar(loader: &Loader) -> String {
    let current = loader.current.get();
    let mut nav = String::from(
        r#"
<div id="__nav" style="
    display: flex; align-items: center; gap: 12px;
    padding: 8px 16px; margin-bottom: 16px;
    background: #1e293b; border: 1px solid #334155;
    border-radius: 8px; font-family: 'Segoe UI', sans-serif;">
  <span style="font-size: 13px; font-weight: 700; color: #38bdf8;">KUI</span>
  <span style="font-size: 12px; color: #64748b;">HTML Loader</span>
  <select id="__sample_select" style="
      flex: 1; padding: 4px 8px; font-size: 13px;
      background: #0f172a; color: #e2e8f0;
      border: 1px solid #475569; border-radius: 4px;">
"#,
    );
    for (i, sample) in loader.samples.iter().enumerate() {
        let selected = if i as i32 == current { " selected" } else { "" };
        nav += &format!(
            "    <option value=\"{i}\"{selected}>{}</option>\n",
            sample.filename
        );
    }
    nav += r#"  </select>
  <button id="__btn_open" style="padding:4px 12px;font-size:12px;cursor:pointer;background:#0f172a;color:#38bdf8;border:1px solid #38bdf8;border-radius:4px;flex-shrink:0;">Open File...</button>
  <button id="__btn_inspect" style="padding:4px 12px;font-size:12px;cursor:pointer;background:#0f172a;color:#94a3b8;border:1px solid #475569;border-radius:4px;flex-shrink:0;">Inspect</button>
  <span id="__filename" style="font-size:11px;color:#475569;max-width:200px;overflow:hidden;">"#;
    if current >= 0 && (current as usize) < loader.samples.len() {
        nav += &loader.samples[current as usize].filename;
    }
    nav += "</span>\n</div>\n";
    nav
}

/// Inserts the bar after the <body ...> tag, or wraps the content when there is none.
fn inject_nav(loader: &Loader, content: &str) -> String {
    let nav = nav_bar(loader);
    let Some(body) = content.to_ascii_lowercase().find("<body") else {
        return format!("<html><body>\n{nav}{content}\n</body></html>");
    };
    match content[body..].find('>') {
        Some(end) => {
            let at = body + end + 1;
            format!("{}\n{nav}{}", &content[..at], &content[at..])
        }
        None => content.to_string(),
    }
}

impl Loader {
    fn current_engine(&self) -> Rc<ScriptEngine> {
        self.engine
            .borrow()
            .clone()
            .unwrap_or_else(|| self.runner_script.clone())
    }

    fn release_components(&self) {
        *self.vlist.borrow_mut() = None;
        *self.list.borrow_mut() = None;
        *self.tree.borrow_mut() = None;
    }

    /// setupScriptEngine: drop the previous engine, bind a new one and run the page's scripts.
    /// Loads only happen from timers or ready, never inside a running script.
    fn setup_script_engine(&self) {
        match self.engine.borrow_mut().take() {
            Some(old) => {
                let _ = old.unbind_document();
            }
            None => {
                let _ = self.runner_script.unbind_document();
            }
        }
        // print() / console.log() output reaches DevTools and stderr (useful during development).
        let _ = kui::console_set_enabled(true);
        let _ = kui::console_set_stderr_output(true);
        let _ = kui::console_set_min_level(kui::LogLevel::Debug);
        let Ok(engine) = ScriptEngine::new() else {
            return;
        };
        let _ = engine.set_error_handler(|message, source, line| {
            eprintln!("[Lua] Error in {source}:{line} — {message}")
        });
        let _ = engine.bind_document(&self.document);
        let _ = engine.execute_scripts();
        let _ = self.window.set_script_engine(Some(&engine)); // re-attaches DevTools too
        *self.engine.borrow_mut() = Some(Rc::new(engine));
    }

    fn close(&self) {
        self.release_components();
        if let Some(engine) = self.engine.borrow_mut().take() {
            let _ = self.window.set_script_engine(None);
            let _ = engine.unbind_document();
        }
    }
}

/// Shared part of loadSample / loadExternalFile; `html` already carries the bar.
fn load_page(loader: &Rc<Loader>, html: &str, source: &Path, title_name: &str) {
    let dir = source
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .into_owned();
    let _ = loader.document.set_base_path(&format!("{dir}/"));
    loader.release_components();
    let mut document = loader.document.clone();
    let _ = document.load_html(html);
    let _ = loader
        .window
        .set_title(&format!("KUI HTML Loader — {title_name}"));
    wire_nav(loader);
    loader.setup_script_engine();
}

fn load_sample(loader: &Rc<Loader>, index: i32) {
    let Some(sample) = usize::try_from(index)
        .ok()
        .and_then(|i| loader.samples.get(i))
    else {
        return;
    };
    loader.current.set(index);
    let content = match fs::read_to_string(&sample.path) {
        Ok(text) if !text.is_empty() => text,
        _ => {
            eprintln!("[HTML Loader] Failed to read: {}", sample.path.display());
            return;
        }
    };
    load_page(
        loader,
        &inject_nav(loader, &content),
        &sample.path,
        &sample.name,
    );
    eprintln!(
        "[HTML Loader] Loaded: {} ({})",
        sample.name, sample.filename
    );
    if sample.filename.contains("virtual_components") {
        setup_virtual_components(loader);
    }
}

fn load_external_file(loader: &Rc<Loader>, path: &Path) {
    let content = match fs::read_to_string(path) {
        Ok(text) if !text.is_empty() => text,
        _ => {
            eprintln!("[HTML Loader] Failed to read: {}", path.display());
            return;
        }
    };
    loader.current.set(-1); // the bar then shows no sample selected
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    load_page(loader, &inject_nav(loader, &content), path, &name);
    eprintln!("[HTML Loader] Loaded external file: {}", path.display());
    if let Ok(Some(span)) = loader.document.get_element_by_id("__filename") {
        let _ = span.set_text_content(&name);
    }
}

fn listen(
    loader: &Rc<Loader>,
    id: &str,
    event: &str,
    handler: impl FnMut(&kui::Event<'_>) + 'static,
) {
    if let Ok(Some(element)) = loader.document.get_element_by_id(id) {
        let _ = element.add_event_listener(event, handler);
    }
}

fn wire_nav(loader: &Rc<Loader>) {
    let l = loader.clone();
    listen(loader, "__sample_select", "change", move |_| {
        let code = "return document:getElementById('__sample_select').selectedIndex";
        let index = match l.current_engine().eval_value(code, "example") {
            Ok(out) if out.success => match out.value {
                ScriptValue::Int(value) => value as i32,
                ScriptValue::Float(value) => value as i32,
                _ => -1,
            },
            _ => -1,
        };
        if index >= 0 && (index as usize) < l.samples.len() && index != l.current.get() {
            let target = l.clone();
            let _ = l
                .platform
                .set_timeout(0, move || load_sample(&target, index));
        }
    });
    let l = loader.clone();
    listen(loader, "__btn_open", "click", move |_| {
        let options = FileDialogOptions {
            title: "Open HTML File".into(),
            filters: vec!["*.html;*.htm".into(), "*.*".into()],
            ..FileDialogOptions::default()
        };
        if let Ok(Some(path)) = l.platform.open_file_dialog(&options) {
            if !path.is_empty() {
                let target = l.clone();
                let _ = l
                    .platform
                    .set_timeout(0, move || load_external_file(&target, Path::new(&path)));
            }
        }
    });
    let l = loader.clone();
    listen(loader, "__btn_inspect", "click", move |_| {
        l.inspect.set(!l.inspect.get());
        if let Ok(Some(button)) = l.document.get_element_by_id("__btn_inspect") {
            let _ = button.set_attribute(
                "style",
                if l.inspect.get() {
                    INSPECT_ON
                } else {
                    INSPECT_OFF
                },
            );
        }
    });
}

fn style(element: &kui::Element<'_>, properties: &[(&str, &str)]) {
    for (name, value) in properties {
        let _ = element.set_style_property(name, value, false);
    }
}

fn toggle_styles(document: &Document, line: bool) {
    for (index, id) in ["vc-scroll-px", "vc-scroll-line"].iter().enumerate() {
        if let Ok(Some(button)) = document.get_element_by_id(id) {
            let active = (index == 1) == line;
            style(
                &button,
                &[
                    ("background", if active { "#0ea5e9" } else { "#1e293b" }),
                    ("color", if active { "#ffffff" } else { "#94a3b8" }),
                    ("border-color", if active { "#0ea5e9" } else { "#334155" }),
                ],
            );
        }
    }
}

/// setupVirtualComponents of the original.
fn setup_virtual_components(loader: &Rc<Loader>) {
    const FOLDER: &str = "📁";
    const FILE: &str = "📄";
    let document = &loader.document;
    if let Ok(Some(container)) = document.get_element_by_id("vlist-container") {
        let config = VirtualListConfig {
            total_items: 100_000,
            item_height: 32.0,
            overscan: 5,
            render_item: Some(Box::new(|doc, index| {
                let row = doc.create_element("div").ok()?;
                let odd = index % 2 == 1;
                style(
                    &row,
                    &[
                        ("display", "flex"),
                        ("align-items", "center"),
                        ("padding", "0 12px"),
                        ("font-size", "13px"),
                        ("color", if odd { "#94a3b8" } else { "#cbd5e1" }),
                        (
                            "background",
                            if odd {
                                "transparent"
                            } else {
                                "rgba(255,255,255,0.02)"
                            },
                        ),
                        ("font-family", "'Segoe UI', sans-serif"),
                    ],
                );
                if let Ok(number) = doc.create_element("span") {
                    style(
                        &number,
                        &[
                            ("color", "#475569"),
                            ("width", "60px"),
                            ("font-size", "11px"),
                        ],
                    );
                    let _ = number.set_text_content(&index.to_string());
                    let _ = row.append_child(&number);
                }
                if let Ok(text) = doc.create_element("span") {
                    let milestone = if index % 1000 == 0 {
                        "  ** milestone **"
                    } else {
                        ""
                    };
                    let _ = text.set_text_content(&format!("Item #{index}{milestone}"));
                    let _ = row.append_child(&text);
                }
                Some(row)
            })),
            ..VirtualListConfig::default()
        };
        *loader.vlist.borrow_mut() = VirtualList::new(&container, config).ok();
    }
    if let Ok(Some(container)) = document.get_element_by_id("listview-container") {
        let config = ListViewConfig {
            item_count: 50_000,
            item_height: 36.0,
            overscan: 5,
            selection_mode: SelectionMode::Single,
            item_provider: Some(Box::new(|index| {
                const ICONS: [&str; 6] = ["📄", "📁", "🔧", "📦", "🌐", "⚡"];
                ListViewItem {
                    icon: ICONS[(index % 6) as usize].into(),
                    text: format!("List item {index}"),
                    subtitle: format!("Description for item {index}"),
                    disabled: index % 50 == 49,
                }
            })),
            on_select: Some(Box::new(|index| eprintln!("[ListView] Selected: {index}"))),
            ..ListViewConfig::default()
        };
        *loader.list.borrow_mut() = ListView::new(&container, config).ok();
    }
    if let Ok(Some(container)) = document.get_element_by_id("treeview-container") {
        if let Ok(tree) = TreeView::new() {
            let child = |parent: u32, text: &str, icon: &str| {
                tree.add_child(parent, text, icon).unwrap_or(0)
            };
            let src = tree.add_node("src", FOLDER).unwrap_or(0);
            let comp = child(src, "components", FOLDER);
            for name in [
                "Button.tsx",
                "Input.tsx",
                "Modal.tsx",
                "Table.tsx",
                "Dropdown.tsx",
            ] {
                child(comp, name, FILE);
            }
            let hooks = child(src, "hooks", FOLDER);
            for name in ["useAuth.ts", "useQuery.ts", "useTheme.ts"] {
                child(hooks, name, FILE);
            }
            let pages = child(src, "pages", FOLDER);
            let home = child(pages, "Home", FOLDER);
            child(home, "index.tsx", FILE);
            child(home, "styles.css", FILE);
            let dash = child(pages, "Dashboard", FOLDER);
            for name in ["index.tsx", "Chart.tsx", "Stats.tsx"] {
                child(dash, name, FILE);
            }
            let settings = child(pages, "Settings", FOLDER);
            child(settings, "Profile.tsx", FILE);
            child(settings, "Security.tsx", FILE);
            let utils = child(src, "utils", FOLDER);
            for name in ["api.ts", "format.ts", "validate.ts"] {
                child(utils, name, FILE);
            }
            child(src, "App.tsx", FILE);
            child(src, "index.tsx", FILE);
            let public = tree.add_node("public", FOLDER).unwrap_or(0);
            child(public, "index.html", FILE);
            child(public, "favicon.ico", FILE);
            for name in ["package.json", "tsconfig.json", "README.md"] {
                let _ = tree.add_node(name, FILE);
            }
            let _ = tree.expand(src);
            let _ = tree.setup(
                &container,
                TreeViewConfig {
                    item_height: 28.0,
                    indent_width: 18.0,
                    overscan: 5,
                    on_select: Some(Box::new(|index| {
                        eprintln!("[TreeView] Selected node: {index}")
                    })),
                    on_toggle: Some(Box::new(|index, expanded| {
                        eprintln!(
                            "[TreeView] Node {index} {}",
                            if expanded { "expanded" } else { "collapsed" }
                        )
                    })),
                    ..TreeViewConfig::default()
                },
            );
            *loader.tree.borrow_mut() = Some(tree);
        }
    }
    for (id, unit) in [
        ("vc-scroll-px", ScrollUnit::Pixel),
        ("vc-scroll-line", ScrollUnit::Line),
    ] {
        let l = loader.clone();
        listen(loader, id, "click", move |_| {
            if let Some(v) = l.vlist.borrow().as_ref() {
                let _ = v.set_scroll_unit(unit);
            }
            if let Some(v) = l.list.borrow().as_ref() {
                let _ = v.set_scroll_unit(unit);
            }
            if let Some(v) = l.tree.borrow().as_ref() {
                let _ = v.set_scroll_unit(unit);
            }
            toggle_styles(&l.document, unit == ScrollUnit::Line);
            eprintln!(
                "[VirtualComponents] scrollUnit -> {}",
                if unit == ScrollUnit::Line {
                    "line"
                } else {
                    "pixel"
                }
            );
        });
    }
    toggle_styles(document, false); // default visual state: Pixel mode
}
