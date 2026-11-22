//! KUI Windows x64 Full Profile 的安全 Rust 门面。
//!
//! 冻结清单内的全部原始 C ABI 声明位于 [`sys`]。安全类型默认 `!Send + !Sync`，以锁定
//! KUI 的 UI 线程契约；拥有型 handle 在 `Drop` 中调用对应 release。

#![warn(missing_docs)]

pub use kui_sys as sys;

mod callbacks;
mod common;
mod components;
mod desktop;
mod dom;
mod events;
mod framework;
mod host;
mod paint;
mod script;
mod system;
mod widgets;

pub use common::{
    Cursor, FrameRadii, FrameState, MessageBoxButtons, MessageBoxResult, MessageBoxType,
};
pub use components::{
    ListView, ListViewConfig, ListViewItem, RenderItemFn, ScrollUnit, SelectionMode, StyleProfile,
    ThemeManager, TreeNode, TreeNodeUpdate, TreeView, TreeViewConfig, VirtualList,
    VirtualListConfig, VirtualListState,
};
pub use desktop::{Menu, MenuItem, ToastLevel, ToastOptions, TrayEvent};
pub use dom::{
    CustomElementDefinition, CustomElementHooks, ExtensionPerfStats, LottieLoadState, LottieState,
    PositioningOptions, ScrollMetrics,
};
pub use events::{
    DropEffect, EffectAllowed, Event, EventFlags, EventTypeTag, ListenerId, Modifiers, MouseButton,
    TimerId,
};
pub use framework::{
    IntersectionEntry, IntersectionOptions, MutationOptions, MutationRecord, MutationType,
    Observable, Observer, ResizeEntry, RouteGuardKind, RouteInfo, Router,
};
pub use host::{
    console_set_enabled, console_set_min_level, console_set_stderr_output, crash_reporter_install,
    crash_reporter_uninstall, disable_ime, stack_trace_install, stack_trace_uninstall,
    CrashReporterConfig, LogLevel, StackTraceConfig,
};
pub use paint::{
    input_modifiers, CornerRadii, CustomImeEvent, CustomKeyEvent, CustomMouseEvent,
    CustomMouseKind, CustomWheelEvent, DecorationStyle, ImeEventKind, PaintContext, TextMetrics,
    TextStyle,
};
pub use script::{ScriptFunction, ScriptOutput, ScriptResources, ScriptValue};
pub use system::{
    BoxModel, FileDialogOptions, InjectedKey, InjectedMouse, MonitorInfo, MouseInjection,
};
pub use widgets::{
    Audio, AudioGroup, AudioPlayOptions, InputPicker, InputPickerState, PickerMode, PopupMode,
    PopupPosition, PopupType, Popups, SoundInfo, TooltipTheme,
};

use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::rc::Rc;

/// SDK 结果类型。
pub type Result<T> = std::result::Result<T, Error>;

/// C ABI 错误，保留原始状态码与调用上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    status: sys::KuiStatus,
    context: &'static str,
    message: String,
}

impl Error {
    /// 原始 `KuiStatus`。
    pub fn status(&self) -> sys::KuiStatus {
        self.status
    }
    /// 失败调用的稳定上下文。
    pub fn context(&self) -> &'static str {
        self.context
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.context, self.message, self.status)
    }
}
impl std::error::Error for Error {}

fn error(status: sys::KuiStatus, context: &'static str) -> Error {
    let message = unsafe {
        let raw = sys::kui_status_message(status);
        if raw.is_null() {
            "unknown error".into()
        } else {
            CStr::from_ptr(raw).to_string_lossy().into_owned()
        }
    };
    Error {
        status,
        context,
        message,
    }
}
fn check(status: sys::KuiStatus, context: &'static str) -> Result<()> {
    if status == sys::KUI_OK {
        Ok(())
    } else {
        Err(error(status, context))
    }
}
fn check_buffer(status: sys::KuiStatus, context: &'static str) -> Result<()> {
    if status == sys::KUI_OK || status == sys::KUI_ERR_BUFFER_TOO_SMALL {
        Ok(())
    } else {
        Err(error(status, context))
    }
}
/// `(指针, 字节数)`。C ABI 约定长度为 0 时按 NUL 结尾读取，而空 `&str` 的指针是悬空的（如 0x1），
/// 直接传过去会被 `strlen` 读崩——空字符串改传一个真正的空 C 字符串。
fn bytes(value: &str) -> (*const c_char, usize) {
    if value.is_empty() {
        return (b"\0".as_ptr().cast(), 0);
    }
    (value.as_ptr().cast(), value.len())
}
fn bool_int(value: bool) -> i32 {
    i32::from(value)
}
fn read_string<F>(context: &'static str, mut call: F) -> Result<String>
where
    F: FnMut(*mut c_char, usize, *mut usize) -> sys::KuiStatus,
{
    let mut size = 0usize;
    check_buffer(call(ptr::null_mut(), 0, &mut size), context)?;
    let mut buffer = vec![0u8; size + 1];
    check(
        call(buffer.as_mut_ptr().cast(), buffer.len(), &mut size),
        context,
    )?;
    buffer.truncate(size);
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

/// 已加载 KUI 运行时版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    /// 主版本。
    pub major: i32,
    /// 次版本。
    pub minor: i32,
    /// 修订版本。
    pub patch: i32,
}
/// 查询运行时版本。
pub fn version() -> Version {
    let raw = unsafe { sys::kui_get_version() };
    Version {
        major: raw.major,
        minor: raw.minor,
        patch: raw.patch,
    }
}

/// 二维点。
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Point {
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
}
/// 二维尺寸。
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Size {
    /// 宽度。
    pub width: f32,
    /// 高度。
    pub height: f32,
}
/// 矩形。
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
    /// 宽度。
    pub width: f32,
    /// 高度。
    pub height: f32,
}

/// 初始化后的平台守卫。创建、使用和销毁必须位于同一 UI 线程。
pub struct Platform {
    _not_send: PhantomData<Rc<()>>,
}
impl Platform {
    /// 初始化 KUI 平台。
    pub fn init() -> Result<Self> {
        check(unsafe { sys::kui_platform_init() }, "Platform::init")?;
        Ok(Self {
            _not_send: PhantomData,
        })
    }
    /// 运行阻塞事件循环。
    pub fn run(&self) -> Result<i32> {
        let mut code = 0;
        check(unsafe { sys::kui_platform_run(&mut code) }, "Platform::run")?;
        Ok(code)
    }
    /// 请求退出事件循环。
    pub fn quit(&self, code: i32) -> Result<()> {
        check(unsafe { sys::kui_platform_quit(code) }, "Platform::quit")
    }
    /// 处理当前待决事件。
    pub fn poll_events(&self) -> Result<()> {
        check(
            unsafe { sys::kui_platform_poll_events() },
            "Platform::poll_events",
        )
    }
    /// 等待事件或超时。
    pub fn wait_events(&self, seconds: f64) -> Result<()> {
        check(
            unsafe { sys::kui_platform_wait_events(seconds) },
            "Platform::wait_events",
        )
    }
    /// 设置剪贴板文本。
    pub fn set_clipboard_text(&self, value: &str) -> Result<()> {
        let (raw, len) = bytes(value);
        check(
            unsafe { sys::kui_platform_set_clipboard_text(raw, len) },
            "Platform::set_clipboard_text",
        )
    }
    /// 获取剪贴板文本。
    pub fn clipboard_text(&self) -> Result<String> {
        read_string("Platform::clipboard_text", |buf, cap, out| unsafe {
            sys::kui_platform_get_clipboard_text(buf, cap, out)
        })
    }
    /// 操作系统名称。
    pub fn os_name(&self) -> Result<String> {
        read_string("Platform::os_name", |buf, cap, out| unsafe {
            sys::kui_platform_get_os_name(buf, cap, out)
        })
    }
    /// 操作系统版本。
    pub fn os_version(&self) -> Result<String> {
        read_string("Platform::os_version", |buf, cap, out| unsafe {
            sys::kui_platform_get_os_version(buf, cap, out)
        })
    }
}
impl Drop for Platform {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_platform_shutdown() };
        // 关闭后不再触发的定时器 / idle 闭包随之释放。
        callbacks::unregister_owner(callbacks::TIMER_OWNER);
    }
}

type MainThreadTask = Box<dyn FnOnce() + Send + 'static>;

/// 把 `task` 交给 UI 线程（调用 [`Platform::init`] 的线程）执行；可以从任何线程调用，用于把后台工作的
/// 结果带回界面，代替轮询。
///
/// `task` 在事件循环（[`Platform::run`] / [`Platform::wait_events`] / [`Platform::poll_events`]）里异步执行，
/// 同一线程投递的任务按投递顺序执行。平台未初始化或已开始关闭时返回错误（`KUI_ERR_NOT_SUPPORTED`），
/// `task` 不会执行；关闭时还在排队的任务被丢弃（在 UI 线程上 drop，不执行）。`task` 里的 panic 会被拦下，
/// 不会穿过 C ABI。
pub fn post_to_main_thread<F>(task: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    unsafe extern "C" fn run(user_data: *mut c_void) {
        let task = Box::from_raw(user_data as *mut MainThreadTask);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || task()));
    }
    unsafe extern "C" fn discard(user_data: *mut c_void) {
        let task = Box::from_raw(user_data as *mut MainThreadTask);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || drop(task)));
    }
    let raw = Box::into_raw(Box::new(Box::new(task) as MainThreadTask));
    let status =
        unsafe { sys::kui_platform_post_to_main_thread(Some(run), Some(discard), raw.cast()) };
    if status != sys::KUI_OK {
        // 被拒绝时两个回调都不会调用，所有权仍在这里。
        drop(unsafe { Box::from_raw(raw) });
    }
    check(status, "post_to_main_thread")
}

/// 窗口创建参数；零值语义与 C ABI 一致。
#[derive(Debug, Clone)]
pub struct WindowProps {
    /// 标题。
    pub title: String,
    /// 客户区宽度。
    pub width: i32,
    /// 客户区高度。
    pub height: i32,
    /// 禁止调整大小。
    pub no_resize: bool,
    /// 禁止系统装饰。
    pub no_decorations: bool,
    /// 透明窗口。
    pub transparent: bool,
    /// 总在最前。
    pub always_on_top: bool,
    /// 居中。
    pub centered: bool,
    /// 无框模式。
    pub frameless: bool,
    /// 无框窗口带系统阴影。
    pub frameless_shadow: bool,
    /// 创建时最大化。
    pub maximized: bool,
    /// 最小宽度；0 不限。
    pub min_width: i32,
    /// 最小高度；0 不限。
    pub min_height: i32,
    /// 最大宽度；0 不限。
    pub max_width: i32,
    /// 最大高度；0 不限。
    pub max_height: i32,
    /// 初始不透明度；0 使用运行时默认 1。
    pub opacity: f32,
    /// 无边框窗口可拖边缘调整大小（创建时生效）。
    pub sizeable: bool,
    /// 创建后不显示，之后调 `show`（没有「先显示再隐藏」的闪一下）。
    pub hidden: bool,
    /// 顶层窗口圆角（CSS px / DIP）；`None` 用平台默认。
    pub frame_radius: Option<f32>,
}
impl Default for WindowProps {
    fn default() -> Self {
        Self {
            title: "KUI App".into(),
            width: 800,
            height: 600,
            no_resize: false,
            no_decorations: false,
            transparent: false,
            always_on_top: false,
            centered: true,
            frameless: false,
            frameless_shadow: false,
            maximized: false,
            min_width: 0,
            min_height: 0,
            max_width: 0,
            max_height: 0,
            opacity: 0.0,
            sizeable: false,
            hidden: false,
            frame_radius: None,
        }
    }
}

/// 唯一拥有的原生窗口。
pub struct Window {
    raw: NonNull<sys::KuiWindow>,
    _not_send: PhantomData<Rc<()>>,
}
impl Window {
    /// 创建窗口。
    pub fn new(props: &WindowProps) -> Result<Self> {
        let (title, title_len) = bytes(&props.title);
        let raw_props = sys::KuiWindowProps {
            title,
            title_len,
            width: props.width,
            height: props.height,
            min_width: props.min_width,
            min_height: props.min_height,
            max_width: props.max_width,
            max_height: props.max_height,
            no_resize: bool_int(props.no_resize),
            no_decorations: bool_int(props.no_decorations),
            transparent: bool_int(props.transparent),
            always_on_top: bool_int(props.always_on_top),
            maximized: bool_int(props.maximized),
            centered: bool_int(props.centered),
            frameless: bool_int(props.frameless),
            frameless_shadow: bool_int(props.frameless_shadow),
            opacity: props.opacity,
        };
        let extended = sys::KuiWindowPropsEx {
            struct_size: std::mem::size_of::<sys::KuiWindowPropsEx>(),
            base: raw_props,
            sizeable: bool_int(props.sizeable),
            hidden: bool_int(props.hidden),
            has_frame_radius: bool_int(props.frame_radius.is_some()),
            frame_radius: props.frame_radius.unwrap_or(0.0),
        };
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_window_create_ex(&extended, &mut raw) },
            "Window::new",
        )?;
        Ok(Self {
            raw: NonNull::new(raw).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "Window::new"))?,
            _not_send: PhantomData,
        })
    }
    /// 获取窗口拥有文档的计数引用。
    pub fn document(&self) -> Result<Document> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_window_get_document(self.raw.as_ptr(), &mut raw) },
            "Window::document",
        )?;
        Document::from_raw(raw)
    }
    /// 显示窗口。
    pub fn show(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_show(self.raw.as_ptr()) },
            "Window::show",
        )
    }
    /// 隐藏窗口。
    pub fn hide(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_hide(self.raw.as_ptr()) },
            "Window::hide",
        )
    }
    /// 设置标题。
    pub fn set_title(&self, title: &str) -> Result<()> {
        let (raw, len) = bytes(title);
        check(
            unsafe { sys::kui_window_set_title(self.raw.as_ptr(), raw, len) },
            "Window::set_title",
        )
    }
    /// 获取标题。
    pub fn title(&self) -> Result<String> {
        read_string("Window::title", |buf, cap, out| unsafe {
            sys::kui_window_get_title(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 调整大小。
    pub fn resize(&self, width: i32, height: i32) -> Result<()> {
        check(
            unsafe { sys::kui_window_resize(self.raw.as_ptr(), width, height) },
            "Window::resize",
        )
    }
    /// 设置位置。
    pub fn set_position(&self, x: i32, y: i32) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_position(self.raw.as_ptr(), x, y) },
            "Window::set_position",
        )
    }
    /// 请求关闭。
    pub fn close(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_close(self.raw.as_ptr()) },
            "Window::close",
        )
    }
    /// 停止窗口的 DevTools 服务；`start_devtools` 可再次启动。
    pub fn stop_devtools(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_stop_devtools(self.raw.as_ptr()) },
            "Window::stop_devtools",
        )
    }
    /// `(是否在运行, 端口, 连接数)`；停止时端口与连接数为 0。访问 token 在 `devtools_url` 里。
    pub fn devtools_state(&self) -> Result<(bool, u16, u32)> {
        let (mut running, mut port, mut clients) = (0, 0u16, 0u32);
        check(
            unsafe {
                sys::kui_window_get_devtools_state(
                    self.raw.as_ptr(),
                    &mut running,
                    &mut port,
                    &mut clients,
                )
            },
            "Window::devtools_state",
        )?;
        Ok((running != 0, port, clients))
    }
    /// 启动 DevTools。
    pub fn start_devtools(&self, port: u16) -> Result<bool> {
        let mut started = 0;
        check(
            unsafe { sys::kui_window_start_devtools(self.raw.as_ptr(), port, &mut started) },
            "Window::start_devtools",
        )?;
        Ok(started != 0)
    }
    /// 绑定 Lua 脚本引擎。
    pub fn set_script_engine(&self, engine: Option<&ScriptEngine>) -> Result<()> {
        let raw = engine.map_or(ptr::null_mut(), |value| value.raw.as_ptr());
        check(
            unsafe { sys::kui_window_set_script_engine(self.raw.as_ptr(), raw) },
            "Window::set_script_engine",
        )
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        let owner = self.owner_key();
        let _ = unsafe { sys::kui_window_release(self.raw.as_ptr()) };
        // 窗口销毁后它的尺寸回调与窗口事件监听不会再触发。
        callbacks::unregister_owner(owner);
    }
}

/// 引用计数文档。
pub struct Document {
    raw: NonNull<sys::KuiDocument>,
    _not_send: PhantomData<Rc<()>>,
}
impl Document {
    fn from_raw(raw: *mut sys::KuiDocument) -> Result<Self> {
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "Document::from_raw"))?,
            _not_send: PhantomData,
        })
    }
    /// 创建独立文档。
    pub fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_document_create(&mut raw) },
            "Document::new",
        )?;
        Self::from_raw(raw)
    }
    /// 加载 HTML。
    pub fn load_html(&mut self, html: &str) -> Result<()> {
        let (raw, len) = bytes(html);
        check(
            unsafe { sys::kui_document_load_html(self.raw.as_ptr(), raw, len) },
            "Document::load_html",
        )
    }
    /// 添加样式表。
    pub fn add_stylesheet(&mut self, css: &str) -> Result<()> {
        let (raw, len) = bytes(css);
        check(
            unsafe { sys::kui_document_add_stylesheet(self.raw.as_ptr(), raw, len) },
            "Document::add_stylesheet",
        )
    }
    fn element(
        &self,
        status: sys::KuiStatus,
        raw: *mut sys::KuiElement,
        context: &'static str,
    ) -> Result<Option<Element<'_>>> {
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, context)?;
        Ok(Some(Element::from_raw(raw, self)?))
    }
    /// 按 id 查找元素。
    pub fn get_element_by_id(&self, id: &str) -> Result<Option<Element<'_>>> {
        let (value, len) = bytes(id);
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::kui_document_get_element_by_id(self.raw.as_ptr(), value, len, &mut raw) };
        self.element(status, raw, "Document::get_element_by_id")
    }
    /// CSS selector 查询。
    pub fn query_selector(&self, selector: &str) -> Result<Option<Element<'_>>> {
        let (value, len) = bytes(selector);
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::kui_document_query_selector(self.raw.as_ptr(), value, len, &mut raw) };
        self.element(status, raw, "Document::query_selector")
    }
    /// 创建元素。
    pub fn create_element(&self, tag: &str) -> Result<Element<'_>> {
        let (value, len) = bytes(tag);
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_document_create_element(self.raw.as_ptr(), value, len, &mut raw) },
            "Document::create_element",
        )?;
        Element::from_raw(raw, self)
    }
    /// 加载单个扩展 DLL；加载失败时返回 `false`。需要先 `init_extensions`（空目录也行）。
    pub fn load_extension(&self, path: &str) -> Result<bool> {
        let (raw, len) = bytes(path);
        let mut loaded = 0;
        check(
            unsafe { sys::kui_document_load_extension(self.raw.as_ptr(), raw, len, &mut loaded) },
            "Document::load_extension",
        )?;
        Ok(loaded != 0)
    }
    /// 按自定义元素标签卸载扩展；没加载时返回 `KUI_ERR_NOT_FOUND`。
    pub fn unload_extension(&self, tag_name: &str) -> Result<()> {
        let (raw, len) = bytes(tag_name);
        check(
            unsafe { sys::kui_document_unload_extension(self.raw.as_ptr(), raw, len) },
            "Document::unload_extension",
        )
    }
    /// 卸载该 DLL 的扩展并重新加载。
    pub fn reload_extension(&self, path: &str) -> Result<bool> {
        let (raw, len) = bytes(path);
        let mut reloaded = 0;
        check(
            unsafe {
                sys::kui_document_reload_extension(self.raw.as_ptr(), raw, len, &mut reloaded)
            },
            "Document::reload_extension",
        )?;
        Ok(reloaded != 0)
    }
    /// 卸载全部扩展。
    pub fn unload_all_extensions(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_unload_all_extensions(self.raw.as_ptr()) },
            "Document::unload_all_extensions",
        )
    }
    /// 监视目录（`None` 停止监视）；DLL 变化时在下一帧自动重载。
    pub fn watch_extensions(&self, directory: Option<&str>) -> Result<()> {
        let (raw, len) = directory.map_or((ptr::null(), 0), bytes);
        check(
            unsafe { sys::kui_document_watch_extensions(self.raw.as_ptr(), raw, len) },
            "Document::watch_extensions",
        )
    }
    /// `(已加载的扩展数, 是否在监视目录)`。
    pub fn extension_state(&self) -> Result<(usize, bool)> {
        let (mut count, mut watching) = (0usize, 0);
        check(
            unsafe {
                sys::kui_document_get_extension_state(self.raw.as_ptr(), &mut count, &mut watching)
            },
            "Document::extension_state",
        )?;
        Ok((count, watching != 0))
    }
    /// 初始化动态扩展目录。
    pub fn init_extensions(&self, directory: &str) -> Result<usize> {
        let (raw, len) = bytes(directory);
        let mut count = 0;
        check(
            unsafe { sys::kui_document_init_extensions(self.raw.as_ptr(), raw, len, &mut count) },
            "Document::init_extensions",
        )?;
        Ok(count)
    }
    /// 执行原始 DevTools JSON 命令。
    pub fn dispatch_devtools_command(&self, command: &str) -> Result<String> {
        let command = CString::new(command).map_err(|_| {
            error(
                sys::KUI_ERR_INVALID_ARG,
                "Document::dispatch_devtools_command",
            )
        })?;
        read_string(
            "Document::dispatch_devtools_command",
            |buf, cap, out| unsafe {
                sys::kui_devtools_dispatch_command(
                    self.raw.as_ptr(),
                    command.as_ptr(),
                    buf,
                    cap,
                    out,
                )
            },
        )
    }
    /// 输出 PDF；返回是否写入文件。
    pub fn print_to_pdf(&self, path: &str, options: &PDFOptions) -> Result<bool> {
        options.with_raw(|raw_options| {
            let (raw_path, len) = bytes(path);
            let mut written = 0;
            check(
                unsafe {
                    sys::kui_document_print_to_pdf(
                        self.raw.as_ptr(),
                        raw_path,
                        len,
                        raw_options,
                        &mut written,
                    )
                },
                "Document::print_to_pdf",
            )?;
            Ok(written != 0)
        })
    }
}
impl Drop for Document {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_document_release(self.raw.as_ptr()) };
        // 文档析构时路由等回调经 release_fn 移进了待释放列表；这里已回到 KUI 之外。
        callbacks::drain_released();
    }
}

/// 文档生命周期约束下的拥有型元素引用。
pub struct Element<'doc> {
    raw: NonNull<sys::KuiElement>,
    document: &'doc Document,
    _not_send: PhantomData<Rc<()>>,
}
impl<'doc> Element<'doc> {
    fn from_raw(raw: *mut sys::KuiElement, document: &'doc Document) -> Result<Self> {
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "Element::from_raw"))?,
            document,
            _not_send: PhantomData,
        })
    }
    /// 标签名。
    pub fn tag_name(&self) -> Result<String> {
        read_string("Element::tag_name", |buf, cap, out| unsafe {
            sys::kui_element_get_tag_name(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 设置属性。
    pub fn set_attribute(&self, name: &str, value: &str) -> Result<()> {
        let (name, name_len) = bytes(name);
        let (value, value_len) = bytes(value);
        check(
            unsafe {
                sys::kui_element_set_attribute(self.raw.as_ptr(), name, name_len, value, value_len)
            },
            "Element::set_attribute",
        )
    }
    /// 获取属性。
    pub fn attribute(&self, name: &str) -> Result<String> {
        let (name, len) = bytes(name);
        read_string("Element::attribute", |buf, cap, out| unsafe {
            sys::kui_element_get_attribute(self.raw.as_ptr(), name, len, buf, cap, out)
        })
    }
    /// 设置文本内容。
    pub fn set_text_content(&self, text: &str) -> Result<()> {
        let (raw, len) = bytes(text);
        check(
            unsafe { sys::kui_element_set_text_content(self.raw.as_ptr(), raw, len) },
            "Element::set_text_content",
        )
    }
    /// 文本内容。
    pub fn text_content(&self) -> Result<String> {
        read_string("Element::text_content", |buf, cap, out| unsafe {
            sys::kui_element_get_text_content(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 设置内联样式属性。
    pub fn set_style_property(&self, name: &str, value: &str, important: bool) -> Result<()> {
        let (name, name_len) = bytes(name);
        let (value, value_len) = bytes(value);
        check(
            unsafe {
                sys::kui_element_set_style_property(
                    self.raw.as_ptr(),
                    name,
                    name_len,
                    value,
                    value_len,
                    bool_int(important),
                )
            },
            "Element::set_style_property",
        )
    }
    /// 获取内联样式属性。
    pub fn style_property(&self, name: &str) -> Result<String> {
        let (name, name_len) = bytes(name);
        read_string("Element::style_property", |buf, cap, out| unsafe {
            sys::kui_element_get_style_property(self.raw.as_ptr(), name, name_len, buf, cap, out)
        })
    }
    /// 边界矩形。
    pub fn bounding_rect(&self) -> Result<Rect> {
        let mut raw = sys::KuiRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        check(
            unsafe { sys::kui_element_get_bounding_rect(self.raw.as_ptr(), &mut raw) },
            "Element::bounding_rect",
        )?;
        Ok(Rect {
            x: raw.x,
            y: raw.y,
            width: raw.width,
            height: raw.height,
        })
    }
    /// 添加子元素。
    pub fn append_child(&self, child: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_element_append_child(self.raw.as_ptr(), child.raw.as_ptr()) },
            "Element::append_child",
        )
    }
    /// 播放媒体元素。
    pub fn play(&self) -> Result<()> {
        check(
            unsafe { sys::kui_media_play(self.raw.as_ptr()) },
            "Element::play",
        )
    }
    /// 暂停媒体元素。
    pub fn pause(&self) -> Result<()> {
        check(
            unsafe { sys::kui_media_pause(self.raw.as_ptr()) },
            "Element::pause",
        )
    }
    /// 设置媒体源。
    pub fn set_src(&self, source: &str) -> Result<()> {
        let (raw, len) = bytes(source);
        check(
            unsafe { sys::kui_media_set_src(self.raw.as_ptr(), raw, len) },
            "Element::set_src",
        )
    }
    /// 设置 Lottie JSON 路径。
    pub fn set_lottie_src(&self, source: &str) -> Result<()> {
        let (raw, len) = bytes(source);
        check(
            unsafe { sys::kui_lottie_set_src(self.raw.as_ptr(), raw, len) },
            "Element::set_lottie_src",
        )
    }
    /// 播放 Lottie 动画。
    pub fn play_lottie(&self) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_play(self.raw.as_ptr()) },
            "Element::play_lottie",
        )
    }
    /// 调用动态 extension/3D 方法。
    pub fn invoke_extension(
        &self,
        method: &str,
        args: &[ExtensionValue],
        max_results: usize,
    ) -> Result<Vec<ExtensionValue>> {
        invoke_extension(self.document, self.raw.as_ptr(), method, args, max_results)
    }
}
impl Drop for Element<'_> {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_element_release(self.raw.as_ptr()) };
    }
}

/// Lua 脚本执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptResult {
    /// 是否成功。
    pub success: bool,
    /// Lua 错误文本。
    pub error: String,
}
/// Lua、Canvas2D 与 WebGL 共用的脚本引擎。
pub struct ScriptEngine {
    raw: NonNull<sys::KuiScriptEngine>,
    _not_send: PhantomData<Rc<()>>,
}
impl ScriptEngine {
    /// 创建脚本引擎。
    pub fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_script_engine_create(&mut raw) },
            "ScriptEngine::new",
        )?;
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "ScriptEngine::new"))?,
            _not_send: PhantomData,
        })
    }
    /// 绑定文档。
    pub fn bind_document(&self, document: &Document) -> Result<()> {
        check(
            unsafe {
                sys::kui_script_engine_bind_document(self.raw.as_ptr(), document.raw.as_ptr())
            },
            "ScriptEngine::bind_document",
        )
    }
    /// 执行 Lua。要第一个返回值用 [`ScriptEngine::eval_value`]。
    pub fn eval(&self, code: &str, source: &str) -> Result<ScriptResult> {
        self.eval_result(code, source)
    }
    /// 执行文档脚本节点。
    pub fn execute_scripts(&self) -> Result<()> {
        check(
            unsafe { sys::kui_script_engine_execute_scripts(self.raw.as_ptr()) },
            "ScriptEngine::execute_scripts",
        )
    }
}
impl Drop for ScriptEngine {
    fn drop(&mut self) {
        let owner = self.owner_key();
        let _ = unsafe { sys::kui_script_engine_release(self.raw.as_ptr()) };
        callbacks::unregister_owner(owner);
    }
}

/// HTTP 同步响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HTTPResponse {
    /// HTTP 状态码。
    pub status: i32,
    /// 响应体。
    pub body: Vec<u8>,
    /// Content-Type。
    pub content_type: String,
    /// 后端错误文本。
    pub error: String,
}
/// KUI HTTP 客户端。
pub struct HTTPClient {
    raw: NonNull<sys::KuiHttpClient>,
    _not_send: PhantomData<Rc<()>>,
}
impl HTTPClient {
    /// 创建客户端。
    pub fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_http_client_create(&mut raw) },
            "HTTPClient::new",
        )?;
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "HTTPClient::new"))?,
            _not_send: PhantomData,
        })
    }
    /// 设置 User-Agent。
    pub fn set_user_agent(&self, value: &str) -> Result<()> {
        let (raw, len) = bytes(value);
        check(
            unsafe { sys::kui_http_client_set_user_agent(self.raw.as_ptr(), raw, len) },
            "HTTPClient::set_user_agent",
        )
    }
    /// 同步请求。
    pub fn fetch(&self, url: &str, timeout_seconds: f64) -> Result<HTTPResponse> {
        let (raw_url, len) = bytes(url);
        let mut raw = ptr::null_mut();
        check(
            unsafe {
                sys::kui_http_client_fetch_sync(
                    self.raw.as_ptr(),
                    raw_url,
                    len,
                    timeout_seconds,
                    &mut raw,
                )
            },
            "HTTPClient::fetch",
        )?;
        take_http_response(raw)
    }
    /// 异步请求，`callback` 在 UI 线程的事件循环里执行，可以直接操作 DOM 与窗口（闭包不要求 `Send`）。
    ///
    /// 平台未初始化或已开始关闭时返回错误。结果因平台关闭而被丢弃、或请求在关闭之后才完成时，
    /// `callback` 不会被调用（只在 UI 线程上 drop，或随进程结束释放）。
    pub fn fetch_async_on_main_thread<F>(
        &self,
        url: &str,
        timeout_seconds: f64,
        callback: F,
    ) -> Result<()>
    where
        F: FnOnce(Result<HTTPResponse>) + 'static,
    {
        type Callback = Box<dyn FnOnce(Result<HTTPResponse>) + 'static>;
        // C ABI 保证这个函数只在 UI 线程上被调用，所以闭包不需要 Send。
        unsafe extern "C" fn trampoline(
            response: *mut sys::KuiHttpResponse,
            user_data: *mut c_void,
        ) {
            let callback = Box::from_raw(user_data as *mut Callback);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                if response.is_null() {
                    // 分配失败或关闭时被丢弃：只释放闭包，不调用。
                    drop(callback);
                } else {
                    callback(take_http_response(response));
                }
            }));
        }
        let (raw_url, len) = bytes(url);
        let raw = Box::into_raw(Box::new(Box::new(callback) as Callback));
        let status = unsafe {
            sys::kui_http_client_fetch_async_on_main_thread(
                self.raw.as_ptr(),
                raw_url,
                len,
                timeout_seconds,
                Some(trampoline),
                raw.cast(),
            )
        };
        if status != sys::KUI_OK {
            drop(unsafe { Box::from_raw(raw) });
        }
        check(status, "HTTPClient::fetch_async_on_main_thread")
    }
    /// 取消全部请求。
    pub fn cancel_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_http_client_cancel_all(self.raw.as_ptr()) },
            "HTTPClient::cancel_all",
        )
    }
}
impl Drop for HTTPClient {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_http_client_release(self.raw.as_ptr()) };
    }
}

fn take_http_response(raw: *mut sys::KuiHttpResponse) -> Result<HTTPResponse> {
    let raw = NonNull::new(raw).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "HTTPResponse"))?;
    struct Guard(NonNull<sys::KuiHttpResponse>);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = unsafe { sys::kui_http_response_release(self.0.as_ptr()) };
        }
    }
    let guard = Guard(raw);
    let mut status = 0;
    check(
        unsafe { sys::kui_http_response_get_status(raw.as_ptr(), &mut status) },
        "HTTPResponse::status",
    )?;
    let mut body_size = 0usize;
    check_buffer(
        unsafe {
            sys::kui_http_response_get_body(raw.as_ptr(), ptr::null_mut(), 0, &mut body_size)
        },
        "HTTPResponse::body",
    )?;
    // The C ABI uses the common string-buffer contract even for byte bodies:
    // capacity must include one trailing NUL while out_len excludes it.
    let mut body = vec![0u8; body_size + 1];
    if body_size != 0 {
        check(
            unsafe {
                sys::kui_http_response_get_body(
                    raw.as_ptr(),
                    body.as_mut_ptr(),
                    body.len(),
                    &mut body_size,
                )
            },
            "HTTPResponse::body",
        )?;
    }
    // 空响应体时上面没有读，缓冲区还是那 1 个 NUL 字节；无论读没读都要截到 body_size。
    body.truncate(body_size);
    let content_type = read_string("HTTPResponse::content_type", |buf, cap, out| unsafe {
        sys::kui_http_response_get_content_type(raw.as_ptr(), buf, cap, out)
    })?;
    let error_text = read_string("HTTPResponse::error", |buf, cap, out| unsafe {
        sys::kui_http_response_get_error(raw.as_ptr(), buf, cap, out)
    })?;
    drop(guard);
    Ok(HTTPResponse {
        status,
        body,
        content_type,
        error: error_text,
    })
}

/// PDF 元数据与页面选项。
#[derive(Debug, Default, Clone)]
pub struct PDFOptions {
    /// 标题。
    pub title: String,
    /// 作者。
    pub author: String,
    /// 主题。
    pub subject: String,
    /// 关键字。
    pub keywords: String,
    /// 创建者。
    pub creator: String,
    /// 创建日期。
    pub creation_date: String,
    /// 页面宽度（pt）。
    pub page_width_pt: f32,
    /// 页面高度（pt）。
    pub page_height_pt: f32,
    /// 栅格化字体。
    pub rasterize_fonts: bool,
    /// 生成 outline。
    pub generate_outline: bool,
}
impl PDFOptions {
    fn with_raw<T>(&self, call: impl FnOnce(*const sys::KuiCPDFOptions) -> Result<T>) -> Result<T> {
        let strings = [
            &self.title,
            &self.author,
            &self.subject,
            &self.keywords,
            &self.creator,
            &self.creation_date,
        ]
        .map(|value| {
            CString::new(value.as_str()).map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "PDFOptions"))
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
        let raw = sys::KuiCPDFOptions {
            title: strings[0].as_ptr(),
            author: strings[1].as_ptr(),
            subject: strings[2].as_ptr(),
            keywords: strings[3].as_ptr(),
            creator: strings[4].as_ptr(),
            creation_date: strings[5].as_ptr(),
            page_width_pt: self.page_width_pt,
            page_height_pt: self.page_height_pt,
            rasterize_fonts: bool_int(self.rasterize_fonts),
            generate_outline: bool_int(self.generate_outline),
        };
        call(&raw)
    }
}

/// Extension 参数/结果值。
#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionValue {
    /// nil。
    Nil,
    /// 整数。
    Integer(i64),
    /// 浮点。
    Number(f64),
    /// UTF-8 字符串。
    String(String),
    /// 布尔。
    Boolean(bool),
}

fn invoke_extension(
    document: &Document,
    element: *mut sys::KuiElement,
    method: &str,
    args: &[ExtensionValue],
    max_results: usize,
) -> Result<Vec<ExtensionValue>> {
    let storage: Vec<CString> = args
        .iter()
        .filter_map(|value| {
            if let ExtensionValue::String(text) = value {
                Some(
                    CString::new(text.as_str())
                        .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "invoke_extension")),
                )
            } else {
                None
            }
        })
        .collect::<Result<_>>()?;
    let mut string_index = 0usize;
    let raw_args: Vec<sys::KuiCExtensionValue> = args
        .iter()
        .map(|value| match value {
            ExtensionValue::Nil => sys::KuiCExtensionValue {
                type_: sys::KUI_C_EXT_NIL,
                int_value: 0,
                float_value: 0.0,
                string_value: ptr::null(),
                string_len: 0,
                bool_value: 0,
            },
            ExtensionValue::Integer(number) => sys::KuiCExtensionValue {
                type_: sys::KUI_C_EXT_INT,
                int_value: *number,
                float_value: 0.0,
                string_value: ptr::null(),
                string_len: 0,
                bool_value: 0,
            },
            ExtensionValue::Number(number) => sys::KuiCExtensionValue {
                type_: sys::KUI_C_EXT_FLOAT,
                int_value: 0,
                float_value: *number,
                string_value: ptr::null(),
                string_len: 0,
                bool_value: 0,
            },
            ExtensionValue::Boolean(value) => sys::KuiCExtensionValue {
                type_: sys::KUI_C_EXT_BOOL,
                int_value: 0,
                float_value: 0.0,
                string_value: ptr::null(),
                string_len: 0,
                bool_value: bool_int(*value),
            },
            ExtensionValue::String(text) => {
                let raw = &storage[string_index];
                string_index += 1;
                sys::KuiCExtensionValue {
                    type_: sys::KUI_C_EXT_STRING,
                    int_value: 0,
                    float_value: 0.0,
                    string_value: raw.as_ptr(),
                    string_len: text.len(),
                    bool_value: 0,
                }
            }
        })
        .collect();
    let (method, method_len) = bytes(method);
    let mut result = ptr::null_mut();
    check(
        unsafe {
            sys::kui_document_extension_invoke(
                document.raw.as_ptr(),
                element,
                method,
                method_len,
                raw_args.as_ptr(),
                raw_args.len(),
                max_results,
                &mut result,
            )
        },
        "invoke_extension",
    )?;
    let result =
        NonNull::new(result).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "invoke_extension"))?;
    struct Guard(NonNull<sys::KuiExtensionResult>);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = unsafe { sys::kui_extension_result_release(self.0.as_ptr()) };
        }
    }
    let _guard = Guard(result);
    // 调用本身失败（方法不存在、参数个数不对、扩展回调抛错等）时报错，不再静默返回空结果。
    let mut invoke_status = 0;
    check(
        unsafe { sys::kui_extension_result_get_status(result.as_ptr(), &mut invoke_status) },
        "invoke_extension::status",
    )?;
    if invoke_status != 0 {
        let detail = read_string("invoke_extension::error", |buf, cap, out| unsafe {
            sys::kui_extension_result_get_error(result.as_ptr(), buf, cap, out)
        })?;
        let mut failure = error(
            if invoke_status == 3 {
                sys::KUI_ERR_NOT_FOUND // 3 = method not found（取值见 kui_c.h）
            } else {
                sys::KUI_ERR_INVALID_ARG
            },
            "invoke_extension",
        );
        failure.message = format!("extension invoke status {invoke_status}: {detail}");
        return Err(failure);
    }
    let mut count = 0usize;
    check(
        unsafe { sys::kui_extension_result_get_count(result.as_ptr(), &mut count) },
        "invoke_extension::count",
    )?;
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let mut raw = sys::KuiCExtensionValue {
            type_: 0,
            int_value: 0,
            float_value: 0.0,
            string_value: ptr::null(),
            string_len: 0,
            bool_value: 0,
        };
        check(
            unsafe { sys::kui_extension_result_get_value(result.as_ptr(), index, &mut raw) },
            "invoke_extension::value",
        )?;
        values.push(match raw.type_ {
            sys::KUI_C_EXT_INT => ExtensionValue::Integer(raw.int_value),
            sys::KUI_C_EXT_FLOAT => ExtensionValue::Number(raw.float_value),
            sys::KUI_C_EXT_BOOL => ExtensionValue::Boolean(raw.bool_value != 0),
            sys::KUI_C_EXT_STRING => ExtensionValue::String(read_string(
                "invoke_extension::string",
                |buf, cap, out| unsafe {
                    sys::kui_extension_result_get_string(result.as_ptr(), index, buf, cap, out)
                },
            )?),
            _ => ExtensionValue::Nil,
        });
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_layout_matches_c_contract() {
        assert_eq!(std::mem::size_of::<sys::KuiVersion>(), 12);
        assert_eq!(std::mem::size_of::<sys::KuiRect>(), 16);
        assert_eq!(sys::KUI_ERR_INTERNAL, 99);
    }
}
