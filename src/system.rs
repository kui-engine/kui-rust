//! 窗口状态与外观、显示器、剪贴板、文件对话框、F12 开关，以及文档上的 DevTools 输入注入与盒模型。

use std::ffi::{c_char, c_void, CString};
use std::ptr;

use crate::{
    bool_int, bytes, check, error, read_string, sys, Document, Platform, Point, Rect, Result, Size,
    Window,
};

/// 显示器信息。
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    /// 显示器名称（可能为空）。
    pub name: String,
    /// 整个显示器区域（逻辑像素）。
    pub bounds: Rect,
    /// 去掉任务栏等之后的工作区。
    pub work_area: Rect,
    /// DPI 缩放比例。
    pub scale: f32,
    /// 是否主显示器。
    pub primary: bool,
}

pub(crate) fn monitor_from(raw: &sys::KuiCMonitorInfo, name: String) -> MonitorInfo {
    MonitorInfo {
        name,
        bounds: Rect {
            x: raw.bounds_x,
            y: raw.bounds_y,
            width: raw.bounds_width,
            height: raw.bounds_height,
        },
        work_area: Rect {
            x: raw.work_x,
            y: raw.work_y,
            width: raw.work_width,
            height: raw.work_height,
        },
        scale: raw.scale,
        primary: raw.is_primary != 0,
    }
}

pub(crate) fn empty_monitor() -> sys::KuiCMonitorInfo {
    sys::KuiCMonitorInfo {
        bounds_x: 0.0,
        bounds_y: 0.0,
        bounds_width: 0.0,
        bounds_height: 0.0,
        work_x: 0.0,
        work_y: 0.0,
        work_width: 0.0,
        work_height: 0.0,
        scale: 0.0,
        is_primary: 0,
    }
}

/// 文件对话框选项。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDialogOptions {
    /// 标题。
    pub title: String,
    /// 初始路径。
    pub default_path: String,
    /// 过滤器，例如 `"图片|*.png;*.jpg"`（格式与 C++ `FileDialogOptions::filters` 相同）。
    pub filters: Vec<String>,
    /// 允许多选（打开对话框）。
    pub allow_multiple: bool,
}

/// 文件对话框的结果缓冲区：Windows 路径名空间 32768 字节足够。
const DIALOG_BUFFER: usize = 32768;

fn run_dialog(
    context: &'static str,
    call: impl FnOnce(*mut c_char, usize, *mut usize) -> sys::KuiStatus,
) -> Result<Option<String>> {
    let mut buffer = vec![0u8; DIALOG_BUFFER];
    let mut len = 0usize;
    let status = call(buffer.as_mut_ptr() as *mut c_char, buffer.len(), &mut len);
    if status == sys::KUI_ERR_NOT_FOUND {
        return Ok(None); // 用户取消
    }
    check(status, context)?;
    buffer.truncate(len);
    Ok(Some(String::from_utf8_lossy(&buffer).into_owned()))
}

fn with_dialog_options<T>(
    options: &FileDialogOptions,
    context: &'static str,
    call: impl FnOnce(&sys::KuiCFileDialogOptions) -> Result<T>,
) -> Result<T> {
    let filters = options
        .filters
        .iter()
        .map(|filter| {
            CString::new(filter.as_str()).map_err(|_| error(sys::KUI_ERR_INVALID_ARG, context))
        })
        .collect::<Result<Vec<_>>>()?;
    let pointers: Vec<*const c_char> = filters.iter().map(|value| value.as_ptr()).collect();
    let (title, title_len) = bytes(&options.title);
    let (path, path_len) = bytes(&options.default_path);
    let raw = sys::KuiCFileDialogOptions {
        title,
        title_len,
        default_path: path,
        default_path_len: path_len,
        filters: if pointers.is_empty() {
            ptr::null()
        } else {
            pointers.as_ptr()
        },
        filter_count: pointers.len(),
        allow_multiple: bool_int(options.allow_multiple),
    };
    call(&raw)
}

impl Platform {
    /// 显示器个数。
    pub fn monitor_count(&self) -> Result<usize> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_platform_get_monitor_count(&mut count) },
            "Platform::monitor_count",
        )?;
        Ok(count)
    }
    /// 第 `index` 个显示器。
    pub fn monitor(&self, index: usize) -> Result<MonitorInfo> {
        let mut raw = empty_monitor();
        let name = read_string("Platform::monitor", |buf, cap, out| unsafe {
            sys::kui_platform_get_monitor(index, &mut raw, buf, cap, out)
        })?;
        Ok(monitor_from(&raw, name))
    }
    /// 全部显示器。
    pub fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        (0..self.monitor_count()?)
            .map(|index| self.monitor(index))
            .collect()
    }
    /// 主显示器（名称为空）。
    pub fn primary_monitor(&self) -> Result<MonitorInfo> {
        let mut raw = empty_monitor();
        check(
            unsafe { sys::kui_platform_get_primary_monitor(&mut raw) },
            "Platform::primary_monitor",
        )?;
        Ok(monitor_from(&raw, String::new()))
    }
    /// 主显示器的物理像素尺寸。
    pub fn primary_monitor_physical_size(&self) -> Result<Size> {
        let (mut width, mut height) = (0.0f32, 0.0f32);
        check(
            unsafe { sys::kui_platform_get_primary_monitor_physical_size(&mut width, &mut height) },
            "Platform::primary_monitor_physical_size",
        )?;
        Ok(Size { width, height })
    }
    /// 剪贴板里是否有文本。
    pub fn has_clipboard_text(&self) -> Result<bool> {
        let mut has = 0;
        check(
            unsafe { sys::kui_platform_has_clipboard_text(&mut has) },
            "Platform::has_clipboard_text",
        )?;
        Ok(has != 0)
    }
    /// 开关 F12 打开 DevTools。
    pub fn set_f12_devtools_enabled(&self, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_platform_set_f12_devtools_enabled(bool_int(enabled)) },
            "Platform::set_f12_devtools_enabled",
        )
    }
    /// 打开文件对话框；用户取消时返回 `None`。多选时多个路径的分隔方式与 C++ SDK 相同。
    pub fn open_file_dialog(&self, options: &FileDialogOptions) -> Result<Option<String>> {
        with_dialog_options(options, "Platform::open_file_dialog", |raw| {
            run_dialog("Platform::open_file_dialog", |buf, cap, out| unsafe {
                sys::kui_platform_open_file_dialog(raw, buf, cap, out)
            })
        })
    }
    /// 保存文件对话框；用户取消时返回 `None`。
    pub fn save_file_dialog(&self, options: &FileDialogOptions) -> Result<Option<String>> {
        with_dialog_options(options, "Platform::save_file_dialog", |raw| {
            run_dialog("Platform::save_file_dialog", |buf, cap, out| unsafe {
                sys::kui_platform_save_file_dialog(raw, buf, cap, out)
            })
        })
    }
    /// 选择文件夹；用户取消时返回 `None`。
    pub fn select_folder_dialog(&self, title: &str) -> Result<Option<String>> {
        let (raw, len) = bytes(title);
        run_dialog("Platform::select_folder_dialog", |buf, cap, out| unsafe {
            sys::kui_platform_select_folder_dialog(raw, len, buf, cap, out)
        })
    }
}

impl Window {
    fn flag(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiWindow, *mut i32) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<bool> {
        let mut value = 0;
        check(unsafe { call(self.raw.as_ptr(), &mut value) }, context)?;
        Ok(value != 0)
    }
    fn action(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiWindow) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<()> {
        check(unsafe { call(self.raw.as_ptr()) }, context)
    }
    /// 窗口尺寸。
    pub fn size(&self) -> Result<(i32, i32)> {
        let (mut width, mut height) = (0, 0);
        check(
            unsafe { sys::kui_window_get_size(self.raw.as_ptr(), &mut width, &mut height) },
            "Window::size",
        )?;
        Ok((width, height))
    }
    /// 窗口位置。
    pub fn position(&self) -> Result<(i32, i32)> {
        let (mut x, mut y) = (0, 0);
        check(
            unsafe { sys::kui_window_get_position(self.raw.as_ptr(), &mut x, &mut y) },
            "Window::position",
        )?;
        Ok((x, y))
    }
    /// 是否可见。
    pub fn is_visible(&self) -> Result<bool> {
        self.flag(sys::kui_window_is_visible, "Window::is_visible")
    }
    /// 是否最小化。
    pub fn is_minimized(&self) -> Result<bool> {
        self.flag(sys::kui_window_is_minimized, "Window::is_minimized")
    }
    /// 是否最大化。
    pub fn is_maximized(&self) -> Result<bool> {
        self.flag(sys::kui_window_is_maximized, "Window::is_maximized")
    }
    /// 是否已请求关闭。
    pub fn should_close(&self) -> Result<bool> {
        self.flag(sys::kui_window_should_close, "Window::should_close")
    }
    /// 最小化。
    pub fn minimize(&self) -> Result<()> {
        self.action(sys::kui_window_minimize, "Window::minimize")
    }
    /// 最大化。
    pub fn maximize(&self) -> Result<()> {
        self.action(sys::kui_window_maximize, "Window::maximize")
    }
    /// 从最小化 / 最大化恢复。
    pub fn restore(&self) -> Result<()> {
        self.action(sys::kui_window_restore, "Window::restore")
    }
    /// 总在最前。
    pub fn set_always_on_top(&self, on_top: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_always_on_top(self.raw.as_ptr(), bool_int(on_top)) },
            "Window::set_always_on_top",
        )
    }
    /// 窗口不透明度（0～1）。
    pub fn set_opacity(&self, opacity: f32) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_opacity(self.raw.as_ptr(), opacity) },
            "Window::set_opacity",
        )
    }
    /// 设备像素比。
    pub fn device_pixel_ratio(&self) -> Result<f32> {
        let mut ratio = 0.0f32;
        check(
            unsafe { sys::kui_window_get_device_pixel_ratio(self.raw.as_ptr(), &mut ratio) },
            "Window::device_pixel_ratio",
        )?;
        Ok(ratio)
    }
    /// 无框窗口是否允许拖边框改尺寸。
    pub fn set_frameless_sizeable(&self, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_frameless_sizeable(self.raw.as_ptr(), bool_int(enabled)) },
            "Window::set_frameless_sizeable",
        )
    }
    /// 窗口圆角半径（四角相同）。
    pub fn set_frame_radius(&self, radius: f32) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_frame_radius(self.raw.as_ptr(), radius) },
            "Window::set_frame_radius",
        )
    }
    /// 清除窗口圆角。
    pub fn clear_frame_radius(&self) -> Result<()> {
        self.action(
            sys::kui_window_clear_frame_radius,
            "Window::clear_frame_radius",
        )
    }
    /// DevTools 端口；没有启动 DevTools 时为 `None`。
    pub fn devtools_port(&self) -> Result<Option<u16>> {
        let mut port = 0u16;
        let status = unsafe { sys::kui_window_get_devtools_port(self.raw.as_ptr(), &mut port) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Window::devtools_port")?;
        Ok(Some(port))
    }
    /// DevTools 前端地址；没有启动 DevTools 时为 `None`。
    pub fn devtools_url(&self) -> Result<Option<String>> {
        match read_string("Window::devtools_url", |buf, cap, out| unsafe {
            sys::kui_window_get_devtools_url(self.raw.as_ptr(), buf, cap, out)
        }) {
            Ok(url) => Ok(Some(url)),
            Err(failure) if failure.status() == sys::KUI_ERR_NOT_FOUND => Ok(None),
            Err(failure) => Err(failure),
        }
    }
    /// 原生窗口句柄：Windows 上是 HWND，macOS 上是窗口的内容视图 `NSView*`（不是 `NSWindow*`），
    /// Linux 上是 X11 Window（XID）。
    pub fn native_handle(&self) -> Result<*mut c_void> {
        let mut handle = ptr::null_mut();
        check(
            unsafe { sys::kui_window_get_native_handle(self.raw.as_ptr(), &mut handle) },
            "Window::native_handle",
        )?;
        Ok(handle)
    }
}

/// DevTools 注入的鼠标事件种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedMouse {
    /// 移动。
    Moved,
    /// 按下。
    Pressed,
    /// 松开。
    Released,
    /// 滚轮。
    Wheel,
}

/// DevTools 注入的键盘事件种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedKey {
    /// 按下。
    Down,
    /// 松开。
    Up,
    /// 字符输入。
    Char,
}

/// DevTools 注入鼠标事件的参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseInjection {
    /// 种类。
    pub kind: InjectedMouse,
    /// 视口坐标。
    pub position: Point,
    /// 按键。
    pub button: crate::MouseButton,
    /// 修饰键。
    pub modifiers: crate::Modifiers,
    /// 点击次数（1 单击，2 双击）。
    pub click_count: i32,
    /// 滚轮增量（只对 `Wheel` 有效）。
    pub delta: Point,
}

/// 元素盒模型（四个盒各四个顶点，视口坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxModel {
    /// 内容盒。
    pub content: [Point; 4],
    /// padding 盒。
    pub padding: [Point; 4],
    /// 边框盒。
    pub border: [Point; 4],
    /// margin 盒。
    pub margin: [Point; 4],
    /// 内容宽。
    pub width: f32,
    /// 内容高。
    pub height: f32,
}

fn modifier_bits(modifiers: crate::Modifiers) -> i32 {
    // 与 C ABI 一致：1=Alt，2=Ctrl，4=Meta，8=Shift。
    (modifiers.alt as i32)
        | (modifiers.ctrl as i32) << 1
        | (modifiers.meta as i32) << 2
        | (modifiers.shift as i32) << 3
}

impl Document {
    /// 经 DevTools 注入鼠标事件（与真实输入走同一条派发路径）。
    pub fn dispatch_mouse_event(&self, event: &MouseInjection) -> Result<()> {
        let kind = match event.kind {
            InjectedMouse::Moved => sys::KUI_C_MOUSE_MOVED,
            InjectedMouse::Pressed => sys::KUI_C_MOUSE_PRESSED,
            InjectedMouse::Released => sys::KUI_C_MOUSE_RELEASED,
            InjectedMouse::Wheel => sys::KUI_C_MOUSE_WHEEL,
        };
        let button = match event.button {
            crate::MouseButton::None => sys::KUI_C_MOUSE_BUTTON_NONE,
            crate::MouseButton::Left => sys::KUI_C_MOUSE_BUTTON_LEFT,
            crate::MouseButton::Middle => sys::KUI_C_MOUSE_BUTTON_MIDDLE,
            crate::MouseButton::Right => sys::KUI_C_MOUSE_BUTTON_RIGHT,
            crate::MouseButton::Other(value) => value,
        };
        check(
            unsafe {
                sys::kui_devtools_dispatch_mouse_event(
                    self.raw.as_ptr(),
                    kind,
                    event.position.x,
                    event.position.y,
                    button,
                    modifier_bits(event.modifiers),
                    event.click_count,
                    event.delta.x,
                    event.delta.y,
                )
            },
            "Document::dispatch_mouse_event",
        )
    }
    /// 经 DevTools 注入键盘事件。`text` 只对 `Char` 有意义。
    pub fn dispatch_key_event(
        &self,
        kind: InjectedKey,
        key: &str,
        code: &str,
        modifiers: crate::Modifiers,
        text: &str,
        auto_repeat: bool,
    ) -> Result<()> {
        let kind = match kind {
            InjectedKey::Down => sys::KUI_C_KEY_DOWN,
            InjectedKey::Up => sys::KUI_C_KEY_UP,
            InjectedKey::Char => sys::KUI_C_KEY_CHAR,
        };
        let (key, key_len) = bytes(key);
        let (code, code_len) = bytes(code);
        let (text, text_len) = bytes(text);
        check(
            unsafe {
                sys::kui_devtools_dispatch_key_event(
                    self.raw.as_ptr(),
                    kind,
                    key,
                    key_len,
                    code,
                    code_len,
                    modifier_bits(modifiers),
                    text,
                    text_len,
                    bool_int(auto_repeat),
                )
            },
            "Document::dispatch_key_event",
        )
    }
    /// 第一个匹配 selector 的元素的盒模型；没有匹配时返回 `None`。
    pub fn box_model(&self, selector: &str) -> Result<Option<BoxModel>> {
        let selector = CString::new(selector)
            .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "Document::box_model"))?;
        let mut quads = [0.0f32; 32];
        let (mut width, mut height) = (0.0f32, 0.0f32);
        let status = unsafe {
            sys::kui_devtools_get_box_model(
                self.raw.as_ptr(),
                selector.as_ptr(),
                quads.as_mut_ptr(),
                &mut width,
                &mut height,
            )
        };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Document::box_model")?;
        let quad = |offset: usize| {
            [0, 1, 2, 3].map(|corner| Point {
                x: quads[offset + corner * 2],
                y: quads[offset + corner * 2 + 1],
            })
        };
        Ok(Some(BoxModel {
            content: quad(0),
            padding: quad(8),
            border: quad(16),
            margin: quad(24),
            width,
            height,
        }))
    }
}
