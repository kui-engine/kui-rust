//! 文档与元素的其余安全接口：结构与查询、树遍历、classList、innerHTML、样式文本、几何与滚动、
//! 焦点、锚定定位、自定义元素、扩展状态，以及媒体 / Lottie 元素的读写。

use std::ffi::{c_char, c_void, CString};
use std::ptr::{self, NonNull};

use crate::callbacks;
use crate::paint::{
    CustomImeEvent, CustomKeyEvent, CustomMouseEvent, CustomMouseKind, CustomWheelEvent,
    ImeEventKind, PaintContext,
};
use crate::{bool_int, bytes, check, error, read_string, sys, Document, Element, Rect, Result};

/// 元素的滚动位置与可滚动范围（`scrollLeft` / `scrollTop` / `scrollWidth` / `scrollHeight`）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollMetrics {
    /// 当前横向滚动量。
    pub left: f32,
    /// 当前纵向滚动量。
    pub top: f32,
    /// 可滚动内容宽（整数像素）。
    pub width: f32,
    /// 可滚动内容高（整数像素）。
    pub height: f32,
}

/// [`Element::position_relative_to`] 的选项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositioningOptions {
    /// 放不下时翻到另一侧。
    pub flip: bool,
    /// 夹在视口内。
    pub clamp_to_viewport: bool,
    /// 与锚点的间距。
    pub offset: f32,
    /// 与视口边缘保留的距离。
    pub viewport_margin: f32,
}
impl Default for PositioningOptions {
    fn default() -> Self {
        Self {
            flip: true,
            clamp_to_viewport: true,
            offset: 0.0,
            viewport_margin: 0.0,
        }
    }
}

/// 动态扩展的性能统计。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExtensionPerfStats {
    /// 最近一次 tick 耗时（毫秒）。
    pub last_tick_ms: f64,
    /// 最近一次绘制耗时（毫秒）。
    pub last_paint_ms: f64,
    /// 最长 tick 耗时（毫秒）。
    pub max_tick_ms: f64,
    /// 最长绘制耗时（毫秒）。
    pub max_paint_ms: f64,
    /// tick 次数。
    pub tick_count: u64,
    /// 绘制次数。
    pub paint_count: u64,
    /// tick 总耗时（毫秒）。
    pub total_tick_ms: f64,
    /// 绘制总耗时（毫秒）。
    pub total_paint_ms: f64,
}

/// Lottie 资源的加载状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LottieLoadState {
    /// 未加载。
    Idle,
    /// 加载中。
    Loading,
    /// 已加载。
    Complete,
    /// 加载失败。
    Error,
}

/// Lottie 的播放状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LottieState {
    /// 正在播放。
    pub playing: bool,
    /// 循环。
    pub looping: bool,
    /// 自动播放。
    pub autoplay: bool,
    /// 速度倍率。
    pub speed: f32,
    /// 位置（0-1）。
    pub position: f64,
    /// 加载状态。
    pub load_state: LottieLoadState,
}

type ElementCallback = Box<dyn FnMut(&Element<'_>)>;
type AttributeCallback = Box<dyn FnMut(&Element<'_>, &str, &str, &str)>;
type PaintCallback = Box<dyn FnMut(&Element<'_>, &mut PaintContext<'_>)>;
type TickCallback = Box<dyn FnMut(&Element<'_>, f64)>;
type NeedsTickCallback = Box<dyn FnMut(&Element<'_>) -> bool>;
type KeyCallback = Box<dyn FnMut(&Element<'_>, &CustomKeyEvent) -> bool>;
type MouseCallback = Box<dyn FnMut(&Element<'_>, &CustomMouseEvent) -> bool>;
type WheelCallback = Box<dyn FnMut(&Element<'_>, &CustomWheelEvent) -> bool>;
type ImeCallback = Box<dyn FnMut(&Element<'_>, &CustomImeEvent) -> bool>;

/// 自定义元素定义。回调在 UI 线程执行，参数里的元素只在回调期间借用。
#[derive(Default)]
pub struct CustomElementDefinition {
    /// 需要触发 `attribute_changed` 的属性名。
    pub observed_attributes: Vec<String>,
    /// 元素接入文档时。
    pub connected: Option<ElementCallback>,
    /// 元素离开文档时。
    pub disconnected: Option<ElementCallback>,
    /// 被观察的属性变化时：`(元素, 属性名, 旧值, 新值)`。
    pub attribute_changed: Option<AttributeCallback>,
}

/// 自定义元素的自绘、tick 与输入回调（3.61.8，C ABI 第 17 节），与
/// [`CustomElementDefinition`] 一起交给 [`Document::define_custom_element_with`]。
/// 同一个定义的回调不会嵌套执行（在回调里触发同一定义的另一个回调时，后者被跳过）。
#[derive(Default)]
pub struct CustomElementHooks {
    /// 自绘：在内容框里画，见 [`PaintContext`]。
    pub paint: Option<PaintCallback>,
    /// tick：参数是距上次 tick 的秒数。元素约每 250 ms tick 一次；tick 里调用了
    /// [`Element::request_repaint`] / [`Element::invalidate_rect`] 时下一帧也会 tick。
    pub tick: Option<TickCallback>,
    /// 是否需要 tick；不设置时一直 tick。
    pub needs_tick: Option<NeedsTickCallback>,
    /// 键盘（获得焦点的元素）；返回 `true` 表示已处理。
    pub key: Option<KeyCallback>,
    /// 鼠标（内容框坐标）；返回 `true` 表示已处理。
    pub mouse: Option<MouseCallback>,
    /// 滚轮（内容框坐标）；返回 `true` 表示已处理。
    pub wheel: Option<WheelCallback>,
    /// 输入法（获得焦点的元素）；返回 `true` 表示已处理。
    pub ime: Option<ImeCallback>,
}

struct CustomElementState {
    document: NonNull<sys::KuiDocument>,
    connected: Option<ElementCallback>,
    disconnected: Option<ElementCallback>,
    attribute_changed: Option<AttributeCallback>,
    paint: Option<PaintCallback>,
    tick: Option<TickCallback>,
    needs_tick: Option<NeedsTickCallback>,
    key: Option<KeyCallback>,
    mouse: Option<MouseCallback>,
    wheel: Option<WheelCallback>,
    ime: Option<ImeCallback>,
}

fn c_text(raw: *const c_char, len: usize) -> String {
    if raw.is_null() || len == 0 {
        String::new()
    } else {
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(raw as *const u8, len) })
            .into_owned()
    }
}

unsafe extern "C" fn custom_paint(
    element: *mut sys::KuiElement,
    paint: *mut sys::KuiPaintContext,
    user_data: *mut c_void,
) {
    let Some(paint) = NonNull::new(paint) else {
        return;
    };
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        if let Some(callback) = state.paint.as_mut() {
            with_borrowed_element(document, element, |element| {
                let mut context = PaintContext::new(paint);
                callback(element, &mut context);
            });
        }
    });
}

unsafe extern "C" fn custom_tick(
    element: *mut sys::KuiElement,
    delta: f64,
    user_data: *mut c_void,
) {
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        if let Some(callback) = state.tick.as_mut() {
            with_borrowed_element(document, element, |element| callback(element, delta));
        }
    });
}

/// 输入类回调的公共部分：返回 1 表示已处理。
fn consumed(
    user_data: *mut c_void,
    element: *mut sys::KuiElement,
    call: impl FnOnce(&mut CustomElementState, &Element<'_>) -> bool,
) -> i32 {
    let mut result = false;
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        with_borrowed_element(document, element, |element| result = call(state, element));
    });
    i32::from(result)
}

unsafe extern "C" fn custom_needs_tick(
    element: *mut sys::KuiElement,
    user_data: *mut c_void,
) -> i32 {
    consumed(user_data, element, |state, element| {
        state
            .needs_tick
            .as_mut()
            .is_none_or(|callback| callback(element))
    })
}

unsafe extern "C" fn custom_key(
    element: *mut sys::KuiElement,
    event: *const sys::KuiCCustomKeyEvent,
    user_data: *mut c_void,
) -> i32 {
    let event = unsafe { &*event };
    let data = CustomKeyEvent {
        down: event.type_ == sys::KUI_C_CUSTOM_KEY_DOWN,
        key: c_text(event.key, event.key_len),
        code: c_text(event.code, event.code_len),
        modifiers: event.modifiers,
        repeat: event.is_repeat != 0,
    };
    consumed(user_data, element, |state, element| {
        state
            .key
            .as_mut()
            .is_some_and(|callback| callback(element, &data))
    })
}

unsafe extern "C" fn custom_mouse(
    element: *mut sys::KuiElement,
    event: *const sys::KuiCCustomMouseEvent,
    user_data: *mut c_void,
) -> i32 {
    let event = unsafe { &*event };
    let data = CustomMouseEvent {
        kind: match event.type_ {
            sys::KUI_C_CUSTOM_MOUSE_DOWN => CustomMouseKind::Down,
            sys::KUI_C_CUSTOM_MOUSE_UP => CustomMouseKind::Up,
            sys::KUI_C_CUSTOM_MOUSE_MOVE => CustomMouseKind::Move,
            sys::KUI_C_CUSTOM_MOUSE_ENTER => CustomMouseKind::Enter,
            sys::KUI_C_CUSTOM_MOUSE_LEAVE => CustomMouseKind::Leave,
            other => CustomMouseKind::Other(other),
        },
        x: event.x,
        y: event.y,
        button: event.button,
        buttons: event.buttons,
        modifiers: event.modifiers,
        click_count: event.click_count,
    };
    consumed(user_data, element, |state, element| {
        state
            .mouse
            .as_mut()
            .is_some_and(|callback| callback(element, &data))
    })
}

unsafe extern "C" fn custom_wheel(
    element: *mut sys::KuiElement,
    event: *const sys::KuiCCustomWheelEvent,
    user_data: *mut c_void,
) -> i32 {
    let event = unsafe { &*event };
    let data = CustomWheelEvent {
        x: event.x,
        y: event.y,
        delta_x: event.delta_x,
        delta_y: event.delta_y,
        modifiers: event.modifiers,
    };
    consumed(user_data, element, |state, element| {
        state
            .wheel
            .as_mut()
            .is_some_and(|callback| callback(element, &data))
    })
}

unsafe extern "C" fn custom_ime(
    element: *mut sys::KuiElement,
    event: *const sys::KuiCCustomImeEvent,
    user_data: *mut c_void,
) -> i32 {
    let event = unsafe { &*event };
    let data = CustomImeEvent {
        kind: ImeEventKind::from_raw(event.type_),
        text: c_text(event.text, event.text_len),
        caret_pos: event.caret_pos,
        committed_text: c_text(event.committed_text, event.committed_len),
    };
    consumed(user_data, element, |state, element| {
        state
            .ime
            .as_mut()
            .is_some_and(|callback| callback(element, &data))
    })
}

/// 在回调期间给借用来的文档 / 元素指针各临时加一个引用，交给安全类型；回调返回后释放。
fn with_borrowed_element(
    document: NonNull<sys::KuiDocument>,
    element: *mut sys::KuiElement,
    call: impl FnOnce(&Element<'_>),
) {
    if element.is_null() {
        return;
    }
    unsafe {
        if sys::kui_document_retain(document.as_ptr()) != sys::KUI_OK {
            return;
        }
    }
    let Ok(document) = Document::from_raw(document.as_ptr()) else {
        return;
    };
    unsafe {
        if sys::kui_element_retain(element) != sys::KUI_OK {
            return;
        }
    }
    let Ok(element) = Element::from_raw(element, &document) else {
        return;
    };
    call(&element);
}

unsafe extern "C" fn custom_connected(element: *mut sys::KuiElement, user_data: *mut c_void) {
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        if let Some(callback) = state.connected.as_mut() {
            with_borrowed_element(document, element, |element| callback(element));
        }
    });
}

unsafe extern "C" fn custom_disconnected(element: *mut sys::KuiElement, user_data: *mut c_void) {
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        if let Some(callback) = state.disconnected.as_mut() {
            with_borrowed_element(document, element, |element| callback(element));
        }
    });
}

unsafe extern "C" fn custom_attribute(
    element: *mut sys::KuiElement,
    name: *const c_char,
    name_len: usize,
    old_value: *const c_char,
    old_len: usize,
    new_value: *const c_char,
    new_len: usize,
    user_data: *mut c_void,
) {
    let text = |raw: *const c_char, len: usize| {
        if raw.is_null() || len == 0 {
            String::new()
        } else {
            String::from_utf8_lossy(std::slice::from_raw_parts(raw as *const u8, len)).into_owned()
        }
    };
    let (name, old_value, new_value) = (
        text(name, name_len),
        text(old_value, old_len),
        text(new_value, new_len),
    );
    callbacks::invoke::<CustomElementState>(user_data as usize, |state| {
        let document = state.document;
        if let Some(callback) = state.attribute_changed.as_mut() {
            with_borrowed_element(document, element, |element| {
                callback(element, &name, &old_value, &new_value)
            });
        }
    });
}

pub(crate) fn collect_elements<'d>(
    document: &'d Document,
    context: &'static str,
    mut call: impl FnMut(*mut *mut sys::KuiElement, usize, *mut usize) -> sys::KuiStatus,
) -> Result<Vec<Element<'d>>> {
    let mut count = 0usize;
    check(call(ptr::null_mut(), 0, &mut count), context)?;
    loop {
        let mut buffer: Vec<*mut sys::KuiElement> = vec![ptr::null_mut(); count];
        let mut written = 0usize;
        let status = call(buffer.as_mut_ptr(), buffer.len(), &mut written);
        if status == sys::KUI_ERR_BUFFER_TOO_SMALL && written > count {
            count = written; // 两次调用之间 DOM 变了：按新的数目重试
            continue;
        }
        check(status, context)?;
        buffer.truncate(written);
        return buffer
            .into_iter()
            .map(|raw| Element::from_raw(raw, document))
            .collect();
    }
}

impl Document {
    /// 底层 C 句柄，给 [`crate::sys`] 里的低层函数用；它的生命周期仍由本对象管理。
    pub fn as_raw(&self) -> *mut sys::KuiDocument {
        self.raw.as_ptr()
    }
}

impl Element<'_> {
    /// 底层 C 句柄，给 [`crate::sys`] 里的低层函数用；它的生命周期仍由本对象管理。
    pub fn as_raw(&self) -> *mut sys::KuiElement {
        self.raw.as_ptr()
    }
}

impl Clone for Document {
    /// 再取一个引用（`kui_document_retain`）。
    fn clone(&self) -> Self {
        unsafe {
            sys::kui_document_retain(self.raw.as_ptr());
        }
        Self {
            raw: self.raw,
            _not_send: std::marker::PhantomData,
        }
    }
}

impl Clone for Element<'_> {
    /// 再取一个引用（`kui_element_retain`）。
    fn clone(&self) -> Self {
        unsafe {
            sys::kui_element_retain(self.raw.as_ptr());
        }
        Self {
            raw: self.raw,
            document: self.document,
            _not_send: std::marker::PhantomData,
        }
    }
}

impl Document {
    /// 设置文档解析相对资源路径（图片 / 样式表 / 脚本）的目录，以路径分隔符结尾；不会重新加载。
    pub fn set_base_path(&self, path: &str) -> Result<()> {
        let (raw, len) = bytes(path);
        check(
            unsafe { sys::kui_document_set_base_path(self.raw.as_ptr(), raw, len) },
            "Document::set_base_path",
        )
    }
    /// 读回基准路径。
    pub fn base_path(&self) -> Result<String> {
        read_string("Document::base_path", |buf, cap, out| unsafe {
            sys::kui_document_get_base_path(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 设置渲染后端的设备像素比（页面光栅化的倍率）；文档还没有渲染器（没有窗口）时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn set_render_device_pixel_ratio(&self, ratio: f32) -> Result<()> {
        check(
            unsafe { sys::kui_document_set_render_device_pixel_ratio(self.raw.as_ptr(), ratio) },
            "Document::set_render_device_pixel_ratio",
        )
    }
    /// 从文件加载 HTML。
    pub fn load_html_file(&mut self, path: &str) -> Result<()> {
        let (raw, len) = bytes(path);
        check(
            unsafe { sys::kui_document_load_html_file(self.raw.as_ptr(), raw, len) },
            "Document::load_html_file",
        )
    }
    /// 清除通过 [`Document::add_stylesheet`] 加入的样式表。
    pub fn clear_stylesheets(&mut self) -> Result<()> {
        check(
            unsafe { sys::kui_document_clear_stylesheets(self.raw.as_ptr()) },
            "Document::clear_stylesheets",
        )
    }
    fn structural(
        &self,
        call: unsafe extern "C" fn(
            *mut sys::KuiDocument,
            *mut *mut sys::KuiElement,
        ) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<Option<Element<'_>>> {
        let mut raw = ptr::null_mut();
        let status = unsafe { call(self.raw.as_ptr(), &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, context)?;
        Element::from_raw(raw, self).map(Some)
    }
    /// 根元素（`<html>`）。
    pub fn root(&self) -> Result<Option<Element<'_>>> {
        self.structural(sys::kui_document_root, "Document::root")
    }
    /// `<body>`。
    pub fn body(&self) -> Result<Option<Element<'_>>> {
        self.structural(sys::kui_document_body, "Document::body")
    }
    /// `<head>`。
    pub fn head(&self) -> Result<Option<Element<'_>>> {
        self.structural(sys::kui_document_head, "Document::head")
    }
    /// 全部匹配 selector 的元素。
    pub fn query_selector_all(&self, selector: &str) -> Result<Vec<Element<'_>>> {
        let (raw, len) = bytes(selector);
        collect_elements(
            self,
            "Document::query_selector_all",
            |buf, cap, out| unsafe {
                sys::kui_document_query_selector_all(self.raw.as_ptr(), raw, len, buf, cap, out)
            },
        )
    }
    /// 定义自定义元素。回调闭包保存在 UI 线程，随文档的生命周期存在。
    pub fn define_custom_element(
        &self,
        tag: &str,
        definition: CustomElementDefinition,
    ) -> Result<()> {
        self.define_custom_element_with(tag, definition, CustomElementHooks::default())
    }
    /// 定义带自绘、tick 与输入回调的自定义元素（3.61.8）。
    pub fn define_custom_element_with(
        &self,
        tag: &str,
        definition: CustomElementDefinition,
        hooks: CustomElementHooks,
    ) -> Result<()> {
        let names = definition
            .observed_attributes
            .iter()
            .map(|name| {
                CString::new(name.as_str())
                    .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "Document::define_custom_element"))
            })
            .collect::<Result<Vec<_>>>()?;
        let pointers: Vec<*const c_char> = names.iter().map(|name| name.as_ptr()).collect();
        let has_connected = definition.connected.is_some();
        let has_disconnected = definition.disconnected.is_some();
        let has_attribute = definition.attribute_changed.is_some();
        let has_paint = hooks.paint.is_some();
        let has_tick = hooks.tick.is_some();
        let has_needs_tick = hooks.needs_tick.is_some();
        let has_key = hooks.key.is_some();
        let has_mouse = hooks.mouse.is_some();
        let has_wheel = hooks.wheel.is_some();
        let has_ime = hooks.ime.is_some();
        let token = callbacks::register(
            CustomElementState {
                document: self.raw,
                connected: definition.connected,
                disconnected: definition.disconnected,
                attribute_changed: definition.attribute_changed,
                paint: hooks.paint,
                tick: hooks.tick,
                needs_tick: hooks.needs_tick,
                key: hooks.key,
                mouse: hooks.mouse,
                wheel: hooks.wheel,
                ime: hooks.ime,
            },
            false,
            self.raw.as_ptr() as usize,
            tag,
        );
        // 闭包随文档注销（所有者是文档），不交给 release_fn。
        let raw = sys::KuiCCustomElementDefinition {
            callbacks: sys::KuiCCustomElementCallbacks {
                connected: has_connected.then_some(custom_connected as _),
                disconnected: has_disconnected.then_some(custom_disconnected as _),
                attribute_changed: has_attribute.then_some(custom_attribute as _),
                observed_attributes: if pointers.is_empty() {
                    ptr::null()
                } else {
                    pointers.as_ptr()
                },
                observed_attribute_count: pointers.len(),
                user_data: token as *mut c_void,
            },
            paint: has_paint.then_some(custom_paint as _),
            tick: has_tick.then_some(custom_tick as _),
            needs_tick: has_needs_tick.then_some(custom_needs_tick as _),
            key: has_key.then_some(custom_key as _),
            mouse: has_mouse.then_some(custom_mouse as _),
            wheel: has_wheel.then_some(custom_wheel as _),
            ime: has_ime.then_some(custom_ime as _),
            release_fn: None,
        };
        let (raw_tag, len) = bytes(tag);
        let status = unsafe {
            sys::kui_document_define_custom_element_ex(self.raw.as_ptr(), raw_tag, len, &raw)
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Document::define_custom_element")
    }
    /// 自定义元素是否已定义。
    pub fn custom_element_is_defined(&self, tag: &str) -> Result<bool> {
        let (raw, len) = bytes(tag);
        let mut defined = 0;
        check(
            unsafe {
                sys::kui_document_custom_element_is_defined(
                    self.raw.as_ptr(),
                    raw,
                    len,
                    &mut defined,
                )
            },
            "Document::custom_element_is_defined",
        )?;
        Ok(defined != 0)
    }
    /// 某个标签的动态扩展是否已加载。
    pub fn extension_is_loaded(&self, tag: &str) -> Result<bool> {
        let (raw, len) = bytes(tag);
        let mut loaded = 0;
        check(
            unsafe {
                sys::kui_document_extension_is_loaded(self.raw.as_ptr(), raw, len, &mut loaded)
            },
            "Document::extension_is_loaded",
        )?;
        Ok(loaded != 0)
    }
    /// 某个标签的动态扩展的性能统计。
    pub fn extension_perf_stats(&self, tag: &str) -> Result<ExtensionPerfStats> {
        let (raw, len) = bytes(tag);
        let mut stats = sys::KuiCExtensionPerfStats {
            last_tick_ms: 0.0,
            last_paint_ms: 0.0,
            max_tick_ms: 0.0,
            max_paint_ms: 0.0,
            tick_count: 0,
            paint_count: 0,
            total_tick_ms: 0.0,
            total_paint_ms: 0.0,
        };
        check(
            unsafe {
                sys::kui_document_extension_get_perf_stats(self.raw.as_ptr(), raw, len, &mut stats)
            },
            "Document::extension_perf_stats",
        )?;
        Ok(ExtensionPerfStats {
            last_tick_ms: stats.last_tick_ms,
            last_paint_ms: stats.last_paint_ms,
            max_tick_ms: stats.max_tick_ms,
            max_paint_ms: stats.max_paint_ms,
            tick_count: stats.tick_count,
            paint_count: stats.paint_count,
            total_tick_ms: stats.total_tick_ms,
            total_paint_ms: stats.total_paint_ms,
        })
    }
    /// 清零全部扩展的性能统计。
    pub fn reset_extension_perf_stats(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_extension_reset_perf_stats(self.raw.as_ptr()) },
            "Document::reset_extension_perf_stats",
        )
    }
}

impl<'doc> Element<'doc> {
    fn relative(
        &self,
        call: unsafe extern "C" fn(
            *mut sys::KuiElement,
            *mut *mut sys::KuiElement,
        ) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<Option<Element<'doc>>> {
        let mut raw = ptr::null_mut();
        let status = unsafe { call(self.raw.as_ptr(), &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, context)?;
        Element::from_raw(raw, self.document).map(Some)
    }
    /// 父元素。
    pub fn parent(&self) -> Result<Option<Element<'doc>>> {
        self.relative(sys::kui_element_parent, "Element::parent")
    }
    /// 第一个子元素。
    pub fn first_child(&self) -> Result<Option<Element<'doc>>> {
        self.relative(sys::kui_element_first_child, "Element::first_child")
    }
    /// 最后一个子元素。
    pub fn last_child(&self) -> Result<Option<Element<'doc>>> {
        self.relative(sys::kui_element_last_child, "Element::last_child")
    }
    /// 下一个兄弟元素。
    pub fn next_sibling(&self) -> Result<Option<Element<'doc>>> {
        self.relative(sys::kui_element_next_sibling, "Element::next_sibling")
    }
    /// 上一个兄弟元素。
    pub fn previous_sibling(&self) -> Result<Option<Element<'doc>>> {
        self.relative(
            sys::kui_element_previous_sibling,
            "Element::previous_sibling",
        )
    }
    /// 子元素个数。
    pub fn child_count(&self) -> Result<usize> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_element_child_count(self.raw.as_ptr(), &mut count) },
            "Element::child_count",
        )?;
        Ok(count)
    }
    /// 子树里第一个匹配 selector 的元素。
    pub fn query_selector(&self, selector: &str) -> Result<Option<Element<'doc>>> {
        let (raw, len) = bytes(selector);
        let mut out = ptr::null_mut();
        let status =
            unsafe { sys::kui_element_query_selector(self.raw.as_ptr(), raw, len, &mut out) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Element::query_selector")?;
        Element::from_raw(out, self.document).map(Some)
    }
    /// 子树里全部匹配 selector 的元素。
    pub fn query_selector_all(&self, selector: &str) -> Result<Vec<Element<'doc>>> {
        let (raw, len) = bytes(selector);
        collect_elements(
            self.document,
            "Element::query_selector_all",
            |buf, cap, out| unsafe {
                sys::kui_element_query_selector_all(self.raw.as_ptr(), raw, len, buf, cap, out)
            },
        )
    }
    /// 从父元素上移除自己。
    pub fn remove(&self) -> Result<()> {
        check(
            unsafe { sys::kui_element_remove(self.raw.as_ptr()) },
            "Element::remove",
        )
    }
    /// 移除子元素；不是自己的子元素时返回 `KUI_ERR_NOT_FOUND`。
    pub fn remove_child(&self, child: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_element_remove_child(self.raw.as_ptr(), child.raw.as_ptr()) },
            "Element::remove_child",
        )
    }
    /// id 属性。
    pub fn id(&self) -> Result<String> {
        read_string("Element::id", |buf, cap, out| unsafe {
            sys::kui_element_get_id(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// class 属性。
    pub fn class_name(&self) -> Result<String> {
        read_string("Element::class_name", |buf, cap, out| unsafe {
            sys::kui_element_get_class_name(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 设置 class 属性。
    pub fn set_class_name(&self, value: &str) -> Result<()> {
        let (raw, len) = bytes(value);
        check(
            unsafe { sys::kui_element_set_class_name(self.raw.as_ptr(), raw, len) },
            "Element::set_class_name",
        )
    }
    fn class_token(
        &self,
        token: &str,
        call: unsafe extern "C" fn(*mut sys::KuiElement, *const c_char, usize) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<()> {
        let (raw, len) = bytes(token);
        check(unsafe { call(self.raw.as_ptr(), raw, len) }, context)
    }
    /// `classList.add`。
    pub fn class_list_add(&self, token: &str) -> Result<()> {
        self.class_token(
            token,
            sys::kui_element_class_list_add,
            "Element::class_list_add",
        )
    }
    /// `classList.remove`。
    pub fn class_list_remove(&self, token: &str) -> Result<()> {
        self.class_token(
            token,
            sys::kui_element_class_list_remove,
            "Element::class_list_remove",
        )
    }
    /// `classList.toggle`。
    pub fn class_list_toggle(&self, token: &str) -> Result<()> {
        self.class_token(
            token,
            sys::kui_element_class_list_toggle,
            "Element::class_list_toggle",
        )
    }
    /// `classList.contains`。
    pub fn class_list_contains(&self, token: &str) -> Result<bool> {
        let (raw, len) = bytes(token);
        let mut contains = 0;
        check(
            unsafe {
                sys::kui_element_class_list_contains(self.raw.as_ptr(), raw, len, &mut contains)
            },
            "Element::class_list_contains",
        )?;
        Ok(contains != 0)
    }
    /// 是否有某个属性。
    pub fn has_attribute(&self, name: &str) -> Result<bool> {
        let (raw, len) = bytes(name);
        let mut has = 0;
        check(
            unsafe { sys::kui_element_has_attribute(self.raw.as_ptr(), raw, len, &mut has) },
            "Element::has_attribute",
        )?;
        Ok(has != 0)
    }
    /// 移除属性。
    pub fn remove_attribute(&self, name: &str) -> Result<()> {
        let (raw, len) = bytes(name);
        check(
            unsafe { sys::kui_element_remove_attribute(self.raw.as_ptr(), raw, len) },
            "Element::remove_attribute",
        )
    }
    /// 以速度（px/ms，y 为正向下）启动惯性滚动。
    pub fn start_momentum_scroll(&self, velocity_x: f32, velocity_y: f32) -> Result<()> {
        check(
            unsafe {
                sys::kui_element_start_momentum_scroll(self.raw.as_ptr(), velocity_x, velocity_y)
            },
            "Element::start_momentum_scroll",
        )
    }
    /// 滚动最近的可滚动祖先让元素可见；`align_to_top` 为真时顶对齐，否则底对齐。
    pub fn scroll_into_view(&self, align_to_top: bool) -> Result<()> {
        check(
            unsafe { sys::kui_element_scroll_into_view(self.raw.as_ptr(), bool_int(align_to_top)) },
            "Element::scroll_into_view",
        )
    }
    /// 表单控件的当前值（用户键入 / 选择的），不是内容属性 `value="…"` 的初值（那个用 `attribute("value")`）：
    /// `<input>` / `<textarea>` / `<select>` / `<output>` 取 value，`<progress>` / `<meter>` 取数值的十进制文本。
    /// 其他元素返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn value(&self) -> Result<String> {
        read_string("Element::value", |buf, cap, out| unsafe {
            sys::kui_element_get_value(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 写控件的当前值；不派发 input / change 事件（与给 DOM 属性赋值一致）。
    pub fn set_value(&self, value: &str) -> Result<()> {
        let (raw, len) = bytes(value);
        check(
            unsafe { sys::kui_element_set_value(self.raw.as_ptr(), raw, len) },
            "Element::set_value",
        )
    }
    /// `<input>` 的选中状态（复选框 / 单选有意义）。其他元素返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn checked(&self) -> Result<bool> {
        let mut checked = 0;
        check(
            unsafe { sys::kui_element_get_checked(self.raw.as_ptr(), &mut checked) },
            "Element::checked",
        )?;
        Ok(checked != 0)
    }
    /// 写 `<input>` 的选中状态；不派发事件。
    pub fn set_checked(&self, checked: bool) -> Result<()> {
        check(
            unsafe { sys::kui_element_set_checked(self.raw.as_ptr(), bool_int(checked)) },
            "Element::set_checked",
        )
    }
    /// `<select>` 当前选中的选项下标（-1 表示没有）。其他元素返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn selected_index(&self) -> Result<i32> {
        let mut index = 0;
        check(
            unsafe { sys::kui_element_get_selected_index(self.raw.as_ptr(), &mut index) },
            "Element::selected_index",
        )?;
        Ok(index)
    }
    /// 写 `<select>` 当前选项（越界按 DOM 规则钳制）；不派发 change 事件。
    pub fn set_selected_index(&self, index: i32) -> Result<()> {
        check(
            unsafe { sys::kui_element_set_selected_index(self.raw.as_ptr(), index) },
            "Element::set_selected_index",
        )
    }
    /// innerHTML。
    pub fn inner_html(&self) -> Result<String> {
        read_string("Element::inner_html", |buf, cap, out| unsafe {
            sys::kui_element_get_inner_html(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 设置 innerHTML。
    pub fn set_inner_html(&self, html: &str) -> Result<()> {
        let (raw, len) = bytes(html);
        check(
            unsafe { sys::kui_element_set_inner_html(self.raw.as_ptr(), raw, len) },
            "Element::set_inner_html",
        )
    }
    /// 内联样式整段文本（`style.cssText`）。
    pub fn style_text(&self) -> Result<String> {
        read_string("Element::style_text", |buf, cap, out| unsafe {
            sys::kui_element_get_style_text(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 整段替换内联样式。
    pub fn set_style_text(&self, css: &str) -> Result<()> {
        let (raw, len) = bytes(css);
        check(
            unsafe { sys::kui_element_set_style_text(self.raw.as_ptr(), raw, len) },
            "Element::set_style_text",
        )
    }
    /// 移除一个内联样式属性。
    pub fn remove_style_property(&self, name: &str) -> Result<()> {
        let (raw, len) = bytes(name);
        check(
            unsafe { sys::kui_element_remove_style_property(self.raw.as_ptr(), raw, len) },
            "Element::remove_style_property",
        )
    }
    /// 内联样式属性是否带 `!important`。
    pub fn style_property_is_important(&self, name: &str) -> Result<bool> {
        let (raw, len) = bytes(name);
        let mut important = 0;
        check(
            unsafe {
                sys::kui_element_style_property_is_important(
                    self.raw.as_ptr(),
                    raw,
                    len,
                    &mut important,
                )
            },
            "Element::style_property_is_important",
        )?;
        Ok(important != 0)
    }
    /// 内联样式里出现的属性名（按声明顺序）。
    pub fn style_property_names(&self) -> Result<Vec<String>> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_element_get_style_property_count(self.raw.as_ptr(), &mut count) },
            "Element::style_property_names",
        )?;
        (0..count)
            .map(|index| {
                read_string("Element::style_property_names", |buf, cap, out| unsafe {
                    sys::kui_element_get_style_property_name(
                        self.raw.as_ptr(),
                        index,
                        buf,
                        cap,
                        out,
                    )
                })
            })
            .collect()
    }
    fn metrics(
        &self,
        call: unsafe extern "C" fn(
            *mut sys::KuiElement,
            *mut f32,
            *mut f32,
            *mut f32,
            *mut f32,
        ) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<[f32; 4]> {
        let mut values = [0.0f32; 4];
        let [a, b, c, d] = &mut values;
        check(unsafe { call(self.raw.as_ptr(), a, b, c, d) }, context)?;
        Ok(values)
    }
    /// client 区域（`clientLeft/Top/Width/Height`，整数像素）。
    pub fn client_rect(&self) -> Result<Rect> {
        let [x, y, width, height] =
            self.metrics(sys::kui_element_get_client_metrics, "Element::client_rect")?;
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }
    /// 相对 offset parent 的边框盒（`offsetLeft/Top/Width/Height`，整数像素）。
    pub fn offset_rect(&self) -> Result<Rect> {
        let [x, y, width, height] =
            self.metrics(sys::kui_element_get_offset_metrics, "Element::offset_rect")?;
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }
    /// 滚动位置与可滚动范围。
    pub fn scroll_metrics(&self) -> Result<ScrollMetrics> {
        let [left, top, width, height] = self.metrics(
            sys::kui_element_get_scroll_metrics,
            "Element::scroll_metrics",
        )?;
        Ok(ScrollMetrics {
            left,
            top,
            width,
            height,
        })
    }
    /// 滚动到指定位置。
    pub fn set_scroll_position(&self, left: f32, top: f32) -> Result<()> {
        check(
            unsafe { sys::kui_element_set_scroll_position(self.raw.as_ptr(), left, top) },
            "Element::set_scroll_position",
        )
    }
    /// 相对当前位置滚动。
    pub fn scroll_by(&self, dx: f32, dy: f32) -> Result<()> {
        check(
            unsafe { sys::kui_element_scroll_by(self.raw.as_ptr(), dx, dy) },
            "Element::scroll_by",
        )
    }
    /// 获得焦点。
    pub fn focus(&self) -> Result<()> {
        check(
            unsafe { sys::kui_element_focus(self.raw.as_ptr()) },
            "Element::focus",
        )
    }
    /// 失去焦点。
    pub fn blur(&self) -> Result<()> {
        check(
            unsafe { sys::kui_element_blur(self.raw.as_ptr()) },
            "Element::blur",
        )
    }
    /// 把自己定位到 `anchor` 旁边（`placement` 例如 `"bottom-start"`），用于下拉、提示框。
    pub fn position_relative_to(
        &self,
        anchor: &Element<'_>,
        placement: &str,
        options: &PositioningOptions,
    ) -> Result<()> {
        let (raw, len) = bytes(placement);
        let raw_options = sys::KuiCPositioningOptions {
            flip: bool_int(options.flip),
            clamp_to_viewport: bool_int(options.clamp_to_viewport),
            offset: options.offset,
            viewport_margin: options.viewport_margin,
        };
        check(
            unsafe {
                sys::kui_element_position_relative_to(
                    self.raw.as_ptr(),
                    anchor.raw.as_ptr(),
                    raw,
                    len,
                    &raw_options,
                )
            },
            "Element::position_relative_to",
        )
    }

    // --- 媒体元素（<audio> / <video>）---------------------------------------
    fn media_f64(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiElement, *mut f64) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<f64> {
        let mut value = 0.0f64;
        check(unsafe { call(self.raw.as_ptr(), &mut value) }, context)?;
        Ok(value)
    }
    fn media_int(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiElement, *mut i32) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<i32> {
        let mut value = 0i32;
        check(unsafe { call(self.raw.as_ptr(), &mut value) }, context)?;
        Ok(value)
    }
    /// 媒体后端是否可用（对应插件已加载）。
    pub fn media_is_available(&self) -> Result<bool> {
        Ok(self.media_int(sys::kui_media_is_available, "Element::media_is_available")? != 0)
    }
    /// 当前播放位置（秒）。
    pub fn current_time(&self) -> Result<f64> {
        self.media_f64(sys::kui_media_get_current_time, "Element::current_time")
    }
    /// 跳到指定位置（秒）。
    pub fn set_current_time(&self, seconds: f64) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_current_time(self.raw.as_ptr(), seconds) },
            "Element::set_current_time",
        )
    }
    /// 总时长（秒）。
    pub fn duration(&self) -> Result<f64> {
        self.media_f64(sys::kui_media_get_duration, "Element::duration")
    }
    /// 是否暂停。
    pub fn paused(&self) -> Result<bool> {
        Ok(self.media_int(sys::kui_media_get_paused, "Element::paused")? != 0)
    }
    /// 音量（0～1）。
    pub fn volume(&self) -> Result<f32> {
        let mut value = 0.0f32;
        check(
            unsafe { sys::kui_media_get_volume(self.raw.as_ptr(), &mut value) },
            "Element::volume",
        )?;
        Ok(value)
    }
    /// 设置音量（0～1）。
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_volume(self.raw.as_ptr(), volume) },
            "Element::set_volume",
        )
    }
    /// 是否静音。
    pub fn muted(&self) -> Result<bool> {
        Ok(self.media_int(sys::kui_media_get_muted, "Element::muted")? != 0)
    }
    /// 设置静音。
    pub fn set_muted(&self, muted: bool) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_muted(self.raw.as_ptr(), bool_int(muted)) },
            "Element::set_muted",
        )
    }
    /// 显示或隐藏原生控件。
    pub fn set_controls(&self, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_controls(self.raw.as_ptr(), bool_int(enabled)) },
            "Element::set_controls",
        )
    }
    /// `readyState`（0～4）。
    pub fn ready_state(&self) -> Result<i32> {
        self.media_int(sys::kui_media_get_ready_state, "Element::ready_state")
    }
    /// `networkState`（0～3）。
    pub fn network_state(&self) -> Result<i32> {
        self.media_int(sys::kui_media_get_network_state, "Element::network_state")
    }

    // --- Lottie ------------------------------------------------------------
    /// 暂停 Lottie 动画。
    pub fn pause_lottie(&self) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_pause(self.raw.as_ptr()) },
            "Element::pause_lottie",
        )
    }
    /// Lottie 资源是否已加载完成。
    pub fn lottie_complete(&self) -> Result<bool> {
        let mut complete = 0;
        check(
            unsafe { sys::kui_lottie_get_complete(self.raw.as_ptr(), &mut complete) },
            "Element::lottie_complete",
        )?;
        Ok(complete != 0)
    }
    /// 停止 Lottie 并回到第一帧。
    pub fn stop_lottie(&self) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_stop(self.raw.as_ptr()) },
            "Element::stop_lottie",
        )
    }
    /// 跳到 0-1 之间的位置（暂停时也生效）。
    pub fn seek_lottie(&self, position: f64) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_seek(self.raw.as_ptr(), position) },
            "Element::seek_lottie",
        )
    }
    /// 播放速度倍率（≥ 0）。
    pub fn set_lottie_speed(&self, speed: f32) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_set_speed(self.raw.as_ptr(), speed) },
            "Element::set_lottie_speed",
        )
    }
    /// 是否循环。
    pub fn set_lottie_loop(&self, looping: bool) -> Result<()> {
        check(
            unsafe { sys::kui_lottie_set_loop(self.raw.as_ptr(), looping as core::ffi::c_int) },
            "Element::set_lottie_loop",
        )
    }
    /// Lottie 的播放状态。
    pub fn lottie_state(&self) -> Result<crate::LottieState> {
        let mut raw = sys::KuiCLottieState {
            playing: 0,
            looping: 0,
            autoplay: 0,
            speed: 0.0,
            position: 0.0,
            load_state: sys::KUI_C_LOTTIE_IDLE,
        };
        check(
            unsafe { sys::kui_lottie_get_state(self.raw.as_ptr(), &mut raw) },
            "Element::lottie_state",
        )?;
        Ok(crate::LottieState {
            playing: raw.playing != 0,
            looping: raw.looping != 0,
            autoplay: raw.autoplay != 0,
            speed: raw.speed,
            position: raw.position,
            load_state: match raw.load_state {
                sys::KUI_C_LOTTIE_LOADING => crate::LottieLoadState::Loading,
                sys::KUI_C_LOTTIE_COMPLETE => crate::LottieLoadState::Complete,
                sys::KUI_C_LOTTIE_ERROR => crate::LottieLoadState::Error,
                _ => crate::LottieLoadState::Idle,
            },
        })
    }
    /// 重新加载当前媒体 src（`HTMLMediaElement.load()`）。
    pub fn load_media(&self) -> Result<()> {
        check(
            unsafe { sys::kui_media_load(self.raw.as_ptr()) },
            "Element::load_media",
        )
    }
    /// 媒体是否循环。
    pub fn set_media_loop(&self, looping: bool) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_loop(self.raw.as_ptr(), looping as core::ffi::c_int) },
            "Element::set_media_loop",
        )
    }
    /// 媒体是否自动播放。
    pub fn set_media_autoplay(&self, autoplay: bool) -> Result<()> {
        check(
            unsafe { sys::kui_media_set_autoplay(self.raw.as_ptr(), autoplay as core::ffi::c_int) },
            "Element::set_media_autoplay",
        )
    }
    /// 媒体的 `(loop, autoplay, ended)`。
    pub fn media_playback_flags(&self) -> Result<(bool, bool, bool)> {
        let (mut looping, mut autoplay, mut ended) = (0, 0, 0);
        check(
            unsafe {
                sys::kui_media_get_playback_flags(
                    self.raw.as_ptr(),
                    &mut looping,
                    &mut autoplay,
                    &mut ended,
                )
            },
            "Element::media_playback_flags",
        )?;
        Ok((looping != 0, autoplay != 0, ended != 0))
    }
    /// Lottie 动画的固有尺寸。
    pub fn lottie_natural_size(&self) -> Result<crate::Size> {
        let (mut width, mut height) = (0.0f32, 0.0f32);
        check(
            unsafe { sys::kui_lottie_get_natural_size(self.raw.as_ptr(), &mut width, &mut height) },
            "Element::lottie_natural_size",
        )?;
        Ok(crate::Size { width, height })
    }
}
