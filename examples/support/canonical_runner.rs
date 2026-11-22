//! 全部生成式 Rust canonical examples（产品清单 + API 巡览）共用的 Full Profile 宿主。

use kui::{
    Document, ExtensionValue, HTTPClient, MenuItem, PDFOptions, Platform, PopupMode, ScriptEngine,
    StyleProfile, ToastLevel, ToastOptions, TrayEvent, Window, WindowProps,
};
use std::cell::Cell;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

const FLAG_EXTENSIONS: u32 = 1 << 0;
const FLAG_DEVTOOLS: u32 = 1 << 1;
const FLAG_FRAMELESS: u32 = 1 << 2;
const FLAG_TRANSPARENT: u32 = 1 << 3;
const FLAG_ALWAYS_ON_TOP: u32 = 1 << 4;
// 1 << 5 以前是「圆角」；没有原版设置窗口圆角（见生成器）。
const FLAG_MULTIWINDOW: u32 = 1 << 6;
const FLAG_LOTTIE: u32 = 1 << 7;
const FLAG_PDF: u32 = 1 << 8;
const FLAG_TRAY: u32 = 1 << 9;
const FLAG_SHADOW: u32 = 1 << 10;
const FLAG_SIZEABLE: u32 = 1 << 11;
const FLAG_CLOSE_TOGETHER: u32 = 1 << 12;

pub struct Spec {
    pub target: &'static str,
    pub title: &'static str,
    pub width: i32,
    pub height: i32,
    pub kind: &'static str,
    pub variant: i32,
    pub flags: u32,
    pub asset: &'static str,
    /// 示例自己的宿主逻辑（取元素、登记监听、定时器……）；`None` 表示这个示例只是页面。
    /// 页面和各 kind 的初始化做完、事件循环开始之前，在 UI 线程上调用一次；返回错误则示例退出。
    pub ready: Option<ReadyFn>,
    /// 可选：读完命令行之后、建平台与窗口之前调用一次。拿到命令行参数（不含程序名）和可修改的
    /// 窗口创建属性；可以在这里设 KUI 启动时读的环境变量、解析自己的命令行。
    /// 返回 `Some(退出码)` 则示例立即以该退出码结束（例如打印了 --help / --list）。
    pub prepare: Option<PrepareFn>,
    /// 可选：窗口、文档、脚本引擎（已绑定）都有了、页面还没加载时调用一次，可在这里定义自定义元素、
    /// 设样式配置、注册组件。此时 `Context::second` 为 `None`。返回错误则示例退出。
    pub before_load: Option<ReadyFn>,
}

/// 交给示例宿主逻辑的句柄，生命周期由运行器管理。
/// 各示例只读自己用得到的字段，其余字段对它们是死代码，所以整体允许。
#[allow(dead_code)]
pub struct Context<'a> {
    pub window: &'a Rc<Window>,
    pub document: &'a Document,
    /// 平台对象：定时器（`set_interval` / `set_timeout`）、剪贴板、对话框等。
    pub platform: &'a Rc<Platform>,
    /// 已绑定到 `document`：页面脚本、读取表单控件的当前值。
    pub script: &'a Rc<ScriptEngine>,
    /// 只有多窗口示例才有（`Rc`：示例要在 `'static` 闭包里关它时可以 `Rc::downgrade`）。
    pub second: Option<(&'a Rc<Window>, &'a Document)>,
}

pub type ReadyFn = fn(&Context<'_>) -> Result<(), Box<dyn Error>>;
pub type PrepareFn = fn(&[String], &mut WindowProps) -> Option<i32>;

pub fn run(spec: Spec) -> Result<(), Box<dyn Error>> {
    let executable = env::current_exe()?;
    let executable_dir = executable.parent().unwrap_or_else(|| Path::new("."));
    let arguments: Vec<String> = env::args().skip(1).collect();
    let (requested, width, height, port) = host_arguments(&spec, &arguments);
    let mut props = WindowProps {
        title: spec.title.into(),
        width,
        height,
        no_decorations: spec.flags & FLAG_FRAMELESS != 0,
        frameless: spec.flags & FLAG_FRAMELESS != 0,
        transparent: spec.flags & FLAG_TRANSPARENT != 0,
        always_on_top: spec.flags & FLAG_ALWAYS_ON_TOP != 0,
        frameless_shadow: spec.flags & FLAG_SHADOW != 0,
        ..WindowProps::default()
    };
    if let Some(prepare) = spec.prepare {
        if let Some(code) = prepare(&arguments, &mut props) {
            let _ = std::io::Write::flush(&mut std::io::stdout());
            std::process::exit(code);
        }
    }
    let asset = find_asset(executable_dir, &requested)?;

    let platform = Rc::new(Platform::init()?);
    let window = Rc::new(Window::new(&props)?);
    let mut document = window.document()?;
    if spec.flags & FLAG_SIZEABLE != 0 {
        window.set_frameless_sizeable(true)?;
    }
    if spec.kind == "html_loader" {
        // 与 examples/html_loader/main.cpp 相同：浏览器默认值、内置组件、弹出层用独立的原生窗口。
        document.set_style_profile(StyleProfile::BrowserCompat)?;
        document.register_builtin_components()?;
        document.popups().set_mode(PopupMode::NativeWindow)?;
    }
    let script = Rc::new(ScriptEngine::new()?);
    script.bind_document(&document)?;
    window.set_script_engine(Some(&*script))?;
    if spec.flags & FLAG_EXTENSIONS != 0 {
        let count = document.init_extensions(&executable_dir.to_string_lossy())?;
        eprintln!("[{}] extensions={count}", spec.target);
    }
    if let Some(before_load) = spec.before_load {
        before_load(&Context {
            window: &window,
            document: &document,
            platform: &platform,
            script: &script,
            second: None,
        })?;
    }
    // 与 C / Go 相同按文件加载：内核从文件所在目录设 base path，相对的 <link> / <script src> / 图片才能解析。
    document.load_html_file(&asset.to_string_lossy())?;
    script.execute_scripts()?;

    // 环境变量 KUI_EXAMPLE_DEVTOOLS_PORT 是与 C / Go 运行器共用的测试开关：设了就启动 DevTools
    // （自己启动 DevTools 的示例也改用这个端口），scripts/verify_example_ports.py 靠它真实点击并读回 DOM。
    let env_port = env::var("KUI_EXAMPLE_DEVTOOLS_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok());
    if spec.flags & FLAG_DEVTOOLS != 0 || env_port.is_some() {
        let port = env_port.unwrap_or(port);
        let started = window.start_devtools(port)?;
        eprintln!("[{}] devtools={started} port={port}", spec.target);
    }
    setup_kind(&spec, &document, executable_dir)?;
    if spec.flags & FLAG_TRAY != 0 {
        setup_tray(&window, &platform, &document)?;
    }
    if spec.flags & FLAG_LOTTIE != 0 {
        setup_lottie(&document, executable_dir)?;
    }
    if spec.flags & FLAG_PDF != 0 {
        let written = document.print_to_pdf(
            "kui-rust-example.pdf",
            &PDFOptions {
                title: spec.title.into(),
                author: "KUI Rust SDK canonical example".into(),
                page_width_pt: 612.0,
                page_height_pt: 792.0,
                generate_outline: true,
                ..PDFOptions::default()
            },
        )?;
        eprintln!("[{}] PDF written={written}", spec.target);
    }

    let second = if spec.flags & FLAG_MULTIWINDOW != 0 {
        let second = Rc::new(Window::new(&WindowProps {
            title: "KUI Rust — second window".into(),
            width: spec.width,
            height: spec.height,
            ..WindowProps::default()
        })?);
        if spec.flags & FLAG_CLOSE_TOGETHER != 0 {
            close_together(&[&window, &second])?;
        }
        let mut second_document = second.document()?;
        second_document.load_html("<html><body style='padding:32px;background:#172554;color:white'><h2>Second window through Rust SDK</h2></body></html>")?;
        second.show()?;
        Some((second, second_document))
    } else {
        None
    };
    if let Some(ready) = spec.ready {
        ready(&Context {
            window: &window,
            document: &document,
            platform: &platform,
            script: &script,
            second: second.as_ref().map(|(window, document)| (window, document)),
        })?;
    }
    window.show()?;
    let _ = platform.run()?;
    drop(second);
    Ok(())
}

/// 页面路径、窗口宽高与 DevTools 端口；与 C 的 parse_file_host、Go 的 arguments() 一致。
/// html_loader 的命令行归示例自己的 prepare 解析，运行器总是先加载打包的页面；file_host（advex_host）认 `--file / --width / --height / --port`，
/// 第一个不以 `-` 开头的参数在没有 `--file` 时当页面，`--file` 总是优先。
fn host_arguments(spec: &Spec, arguments: &[String]) -> (String, i32, i32, u16) {
    let (mut width, mut height, mut port) = (spec.width, spec.height, 9300u16);
    if spec.kind != "file_host" {
        return (spec.asset.into(), width, height, port);
    }
    let (mut file, mut positional): (Option<String>, Option<String>) = (None, None);
    let mut index = 0;
    while index < arguments.len() {
        let value = arguments.get(index + 1);
        match (arguments[index].as_str(), value) {
            ("--file", Some(value)) => file = Some(value.clone()),
            ("--width", Some(value)) => width = value.parse().unwrap_or(width),
            ("--height", Some(value)) => height = value.parse().unwrap_or(height),
            ("--port", Some(value)) => port = value.parse().unwrap_or(port),
            (other, _) => {
                if positional.is_none() && !other.starts_with('-') {
                    positional = Some(other.to_string());
                }
                index += 1;
                continue;
            }
        }
        index += 2;
    }
    let path = file.or(positional).unwrap_or_else(|| {
        eprintln!(
            "[{}] no --file supplied; opening the packaged Rust host page. \
             Use --file <html> for ADVEX automation.",
            spec.target
        );
        spec.asset.into()
    });
    (path, width, height, port)
}

fn find_asset(executable_dir: &Path, relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    let requested = PathBuf::from(relative);
    let candidates = [
        requested.clone(),
        executable_dir.join("assets").join(relative),
        executable_dir.join(relative),
        executable_dir.join("..").join("assets").join(relative),
        PathBuf::from("examples").join("assets").join(relative),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        // 不用 canonicalize：它在 Windows 上给出 \\?\ 前缀的路径，内核按它拼出的相对资源路径
        // （css/x.css）带正斜杠，\\?\ 路径不做斜杠规整，相对 <link> / 图片就找不到了。
        .map(std::path::absolute)
        .transpose()?
        .ok_or_else(|| format!("asset not found: {relative}").into())
}

fn setup_kind(
    spec: &Spec,
    document: &Document,
    executable_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    match spec.kind {
        "three_d" => setup_3d(spec, document)?,
        "media" => setup_media(spec, document, executable_dir)?,
        "network" => {
            let client = HTTPClient::new()?;
            client.set_user_agent("kui-rust-canonical/3.1.0")?;
            if let Some(status) = document.get_element_by_id("status")? {
                status.set_text_content(
                    "Rust HTTP client ready; press Send in hosts with event bindings.",
                )?;
            }
        }
        "hello_extension" => setup_hello_extension(document)?,
        _ => {}
    }
    Ok(())
}

fn setup_3d(spec: &Spec, document: &Document) -> Result<(), Box<dyn Error>> {
    const METHODS: [&str; 26] = [
        "createTriangle",
        "createBox",
        "createSphere",
        "createBox",
        "createSphere",
        "createBox",
        "createBox",
        "createBox",
        "createSphere",
        "createSphere",
        "createBox",
        "createSphere",
        "createBox",
        "createBox",
        "createSphere",
        "createBox",
        "createBox",
        "createBox",
        "createSphere",
        "createSphere",
        "createPlane",
        "createBox",
        "createBox",
        "createSphere",
        "createBox",
        "createSphere",
    ];
    let Some(index) = spec.variant.checked_sub(1).map(|value| value as usize) else {
        return Ok(());
    };
    if index >= METHODS.len() {
        return Ok(());
    }
    let Some(view) = document.get_element_by_id("view")? else {
        return Ok(());
    };
    let values = view.invoke_extension(
        METHODS[index],
        &[
            ExtensionValue::Number(0.18 + (index % 3) as f64 * 0.20),
            ExtensionValue::Number(0.45 + (index % 2) as f64 * 0.25),
            ExtensionValue::Number(0.92 - (index % 4) as f64 * 0.13),
        ],
        1,
    )?;
    if let Some(entity) = values.first() {
        let _ = view.invoke_extension("addToScene", std::slice::from_ref(entity), 0)?;
    }
    Ok(())
}

fn setup_media(
    spec: &Spec,
    document: &Document,
    executable_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let Some(media) = document.get_element_by_id("media")? else {
        return Ok(());
    };
    let relative = if spec.variant == 1 {
        "native_media/sample.mp3"
    } else {
        "native_media/sample.mp4"
    };
    if let Ok(path) = find_asset(executable_dir, relative) {
        media.set_src(&path.to_string_lossy())?;
    }
    if let Some(status) = document.get_element_by_id("status")? {
        status.set_text_content("Media element configured through the Rust SDK.")?;
    }
    Ok(())
}

fn setup_hello_extension(document: &Document) -> Result<(), Box<dyn Error>> {
    let Some(element) = document.get_element_by_id("hello")? else {
        return Ok(());
    };
    let values = element.invoke_extension("greet", &[ExtensionValue::String("Rust SDK".into())], 1);
    if let (Ok(values), Some(status)) = (values, document.get_element_by_id("status")?) {
        if let Some(ExtensionValue::String(text)) = values.first() {
            status.set_text_content(text)?;
        }
    }
    Ok(())
}

fn setup_lottie(document: &Document, executable_dir: &Path) -> Result<(), Box<dyn Error>> {
    let Ok(path) = find_asset(executable_dir, "native_animation/pulse.json") else {
        return Ok(());
    };
    let Some(element) = document.query_selector("lottie-player")? else {
        return Ok(());
    };
    element.set_lottie_src(&path.to_string_lossy())?;
    element.play_lottie()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 个别 canonical 示例自带的应用行为，照 C++ 原版移植（与 C runtime、Go runner 一一对应）。
// ---------------------------------------------------------------------------

const TRAY_MENU_TOGGLE: u32 = 1;
const TRAY_MENU_INFO: u32 = 2;
const TRAY_MENU_WARN: u32 = 3;
const TRAY_MENU_ERROR: u32 = 4;
const TRAY_MENU_ABOUT: u32 = 5;
const TRAY_MENU_QUIT: u32 = 6;
const TRAY_TOOLTIPS: [&str; 3] = [
    "KUI Tray Icon Example — Idle",
    "KUI Tray Icon Example — Syncing…",
    "KUI Tray Icon Example — All done ✓",
];
const TRAY_LABELS: [&str; 3] = ["Idle", "Busy", "OK"];

/// kui_tray_icon_example：examples/tray_icon_example 的托盘行为。
struct TrayApp {
    window: Weak<Window>,
    platform: Weak<Platform>,
    document: Document,
    clicks: Cell<u32>,
    doubles: Cell<u32>,
    rights: Cell<u32>,
    balloon_clicks: Cell<u32>,
    variant: Cell<usize>,
    tray_visible: Cell<bool>,
    close_hinted: Cell<bool>,
}

impl TrayApp {
    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(element)) = self.document.get_element_by_id(id) {
            let _ = element.set_text_content(text);
        }
    }
    /// 最新一条放在最前，最多 10 条（与原版相同）。
    fn log(&self, message: &str) {
        let Ok(Some(list)) = self.document.get_element_by_id("logList") else {
            return;
        };
        let Ok(item) = self.document.create_element("li") else {
            return;
        };
        // 时间戳是 UTC：标准库没有本地时区（原版用本地时间）。
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let seconds = elapsed.as_secs();
        let stamp = format!(
            "{:02}:{:02}:{:02}.{:03}",
            (seconds / 3600) % 24,
            (seconds / 60) % 60,
            seconds % 60,
            elapsed.subsec_millis()
        );
        let _ = item.set_text_content(&format!("{stamp}  {message}"));
        let first = list.first_child().ok().flatten();
        let _ = list.insert_before(&item, first.as_ref());
        while list.child_count().unwrap_or(0) > 10 {
            match list.last_child() {
                Ok(Some(last)) => {
                    let _ = list.remove_child(&last);
                }
                _ => break,
            }
        }
    }
    fn with_window(&self, call: impl FnOnce(&Window)) {
        if let Some(window) = self.window.upgrade() {
            call(&window);
        }
    }
    fn visible(&self) -> bool {
        self.window
            .upgrade()
            .is_some_and(|w| w.is_visible().unwrap_or(false))
    }
    fn toast(&self, level: ToastLevel, message: &str) {
        if self.visible() {
            let _ = self.document.show_toast(&ToastOptions {
                message: message.into(),
                level,
                position: "top-right".into(),
                duration_ms: 2500,
                ..ToastOptions::default()
            });
        }
    }
    fn show_window(&self) {
        self.with_window(|w| {
            let _ = w.show();
            let _ = w.focus();
        });
    }
    fn balloon(&self, icon: i32, title: &str, body: &str) {
        self.log(&format!("[menu] Balloon: {title}"));
        self.with_window(|w| {
            let _ = w.show_tray_balloon(title, body, icon, 4000);
        });
    }
    fn bump(&self, counter: &Cell<u32>, id: &str) {
        counter.set(counter.get() + 1);
        self.set_text(id, &counter.get().to_string());
    }
    fn on_tray(&self, event: TrayEvent) {
        match event {
            TrayEvent::Click => {
                self.bump(&self.clicks, "cntClick");
                self.log("[tray] left-click");
                self.toast(ToastLevel::Info, "Tray: left-click");
                if self.visible() {
                    self.with_window(|w| {
                        let _ = w.hide();
                    });
                } else {
                    self.show_window();
                }
            }
            TrayEvent::DoubleClick => {
                self.bump(&self.doubles, "cntDouble");
                self.log("[tray] double-click (show + focus)");
                self.show_window();
                self.toast(ToastLevel::Success, "Window restored");
            }
            TrayEvent::RightClick => {
                self.bump(&self.rights, "cntRight");
                self.log("[tray] right-click (context menu)");
            }
            TrayEvent::BalloonClick => {
                self.bump(&self.balloon_clicks, "cntBalloon");
                self.log("[tray] balloon-click (restore window)");
                self.show_window();
                self.toast(ToastLevel::Info, "Balloon clicked");
            }
        }
    }
    fn on_menu(&self, id: u32) {
        match id {
            TRAY_MENU_TOGGLE => {
                if self.visible() {
                    self.log("[menu] Hide window");
                    self.with_window(|w| {
                        let _ = w.hide();
                    });
                } else {
                    self.log("[menu] Show window");
                    self.show_window();
                }
            }
            TRAY_MENU_INFO => self.balloon(1, "KUI", "Hello from system tray!"),
            TRAY_MENU_WARN => self.balloon(2, "Heads up", "Something may need attention."),
            TRAY_MENU_ERROR => self.balloon(3, "Error", "An example error has occurred."),
            TRAY_MENU_ABOUT => {
                self.log("[menu] About");
                self.with_window(|w| {
                    let _ = w.show_tray_balloon(
                        "About KUI Tray Example",
                        "KUI Framework v1.x · ENH-P1-003 System Tray Icon · © 2026",
                        1,
                        5000,
                    );
                });
            }
            TRAY_MENU_QUIT => {
                self.log("[menu] Quit requested");
                self.close_hinted.set(true); // 之后的关闭真正退出
                self.with_window(|w| {
                    let _ = w.destroy_tray();
                    let _ = w.close();
                });
                if let Some(platform) = self.platform.upgrade() {
                    let _ = platform.quit(0);
                }
            }
            _ => {}
        }
    }
    fn update_badge(&self) {
        if let Ok(Some(badge)) = self.document.get_element_by_id("statusBadge") {
            let _ = badge.set_class_name(if self.tray_visible.get() {
                "status-dot"
            } else {
                "status-dot off"
            });
        }
        self.set_text(
            "statusText",
            if self.tray_visible.get() {
                "active"
            } else {
                "hidden"
            },
        );
    }
    fn on_button(&self, id: &str) {
        match id {
            "btnBalloonInfo" => self.balloon(
                1,
                "Info from app",
                "This balloon was sent by the in-app button.",
            ),
            "btnBalloonWarn" => self.balloon(2, "Warning from app", "Remember to save your work."),
            "btnBalloonError" => {
                self.balloon(3, "Error from app", "Simulated error for demonstration.")
            }
            "btnToggleTooltip" => {
                self.variant.set((self.variant.get() + 1) % 3);
                self.with_window(|w| {
                    let _ = w.set_tray_tooltip(TRAY_TOOLTIPS[self.variant.get()]);
                });
                self.set_text("tooltipLabel", TRAY_LABELS[self.variant.get()]);
                self.log("[ui] tooltip changed");
            }
            "btnToggleVisible" => {
                self.tray_visible.set(!self.tray_visible.get());
                self.with_window(|w| {
                    let _ = w.set_tray_visible(self.tray_visible.get());
                });
                self.update_badge();
                let shown = self.tray_visible.get();
                self.set_text(
                    "btnToggleVisible",
                    if shown { "Hide Tray" } else { "Show Tray" },
                );
                self.log(if shown {
                    "[ui] tray shown"
                } else {
                    "[ui] tray hidden"
                });
            }
            _ => {}
        }
    }
    /// 第一次关闭缩到托盘并弹气泡，第二次真正关闭。
    fn on_close(&self, event: &kui::Event<'_>) {
        if !self.close_hinted.get() {
            self.close_hinted.set(true);
            let _ = event.prevent_default();
            self.with_window(|w| {
                let _ = w.hide();
                let _ = w.show_tray_balloon(
                    "Still running",
                    "KUI Tray Example was minimized to the system tray. Double-click the icon to restore, or right-click for Quit.",
                    1,
                    5000,
                );
            });
            self.log("[close] minimized to tray (first close)");
            return;
        }
        self.with_window(|w| {
            let _ = w.destroy_tray();
        });
    }
}

fn setup_tray(
    window: &Rc<Window>,
    platform: &Rc<Platform>,
    document: &Document,
) -> Result<(), Box<dyn Error>> {
    let app = Rc::new(TrayApp {
        window: Rc::downgrade(window),
        platform: Rc::downgrade(platform),
        document: document.clone(),
        clicks: Cell::new(0),
        doubles: Cell::new(0),
        rights: Cell::new(0),
        balloon_clicks: Cell::new(0),
        variant: Cell::new(0),
        tray_visible: Cell::new(true),
        close_hinted: Cell::new(false),
    });
    window
        .create_tray("", TRAY_TOOLTIPS[0], true)
        .map_err(|e| format!("failed to register tray icon - system tray may be disabled: {e}"))?;
    let events = Rc::clone(&app);
    window.on_tray_event(move |event| events.on_tray(event))?;
    let item = |text: &str, id: u32| MenuItem {
        text: text.into(),
        id,
        ..MenuItem::default()
    };
    let separator = || MenuItem {
        separator: true,
        ..MenuItem::default()
    };
    let items = [
        item("Show / Hide Window", TRAY_MENU_TOGGLE),
        separator(),
        item("Send Info Balloon", TRAY_MENU_INFO),
        item("Send Warning Balloon", TRAY_MENU_WARN),
        item("Send Error Balloon", TRAY_MENU_ERROR),
        separator(),
        item("About…", TRAY_MENU_ABOUT),
        separator(),
        item("Quit", TRAY_MENU_QUIT),
    ];
    let menu = Rc::clone(&app);
    window.set_tray_context_menu(&items, Some(move |id| menu.on_menu(id)))?;
    app.log("[init] tray icon installed");
    for id in [
        "btnBalloonInfo",
        "btnBalloonWarn",
        "btnBalloonError",
        "btnToggleTooltip",
        "btnToggleVisible",
    ] {
        if let Some(button) = document.get_element_by_id(id)? {
            let buttons = Rc::clone(&app);
            button.add_event_listener("click", move |_| buttons.on_button(id))?;
        }
    }
    let closing = Rc::clone(&app);
    window.add_event_listener("close", false, move |event| closing.on_close(event))?;
    app.log("[init] ready");
    Ok(())
}

/// kui_multi_window_animation：关掉任何一个窗口就关掉全部。
fn close_together(windows: &[&Rc<Window>]) -> Result<(), Box<dyn Error>> {
    let closing = Rc::new(Cell::new(false));
    let all: Vec<Weak<Window>> = windows.iter().map(|w| Rc::downgrade(w)).collect();
    for (index, window) in windows.iter().enumerate() {
        let others = all.clone();
        let flag = Rc::clone(&closing);
        window.add_event_listener("close", false, move |_| {
            if flag.replace(true) {
                return;
            }
            for (other, weak) in others.iter().enumerate() {
                if other != index {
                    if let Some(w) = weak.upgrade() {
                        let _ = w.close();
                    }
                }
            }
        })?;
    }
    Ok(())
}
