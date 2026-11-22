//! 事件、监听器、窗口回调、定时器、脚本错误回调与工作线程版异步 HTTP。
//!
//! 除 [`HTTPClient::fetch_async`] 外，这里的回调都只在 UI 线程上执行，闭包因此不要求 `Send`，
//! 可以捕获 `Rc<Document>`、`Rc<Window>` 等 UI 对象。闭包保存在 UI 线程的登记表里，
//! 移除监听 / 清除定时器 / 销毁窗口或脚本引擎时释放。

use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

use crate::callbacks::{self, TIMER_OWNER};
use crate::{
    bytes, check, read_string, sys, take_http_response, Document, Element, HTTPClient,
    HTTPResponse, Platform, Point, Result, ScriptEngine, Window,
};

/// 只在回调执行期间有效的事件。不能保存到回调之外（生命周期 `'a` 保证这一点）。
pub struct Event<'a> {
    raw: NonNull<sys::KuiEvent>,
    _callback: PhantomData<&'a ()>,
}

/// 事件的具体种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTypeTag {
    /// 普通事件（含脚本派发的合成事件）。
    Base,
    /// 鼠标事件。
    Mouse,
    /// 滚轮事件。
    Wheel,
    /// 指针事件。
    Pointer,
    /// 键盘事件。
    Keyboard,
    /// 焦点事件。
    Focus,
    /// 输入事件。
    Input,
    /// 拖放事件。
    Drag,
    /// 本版本不认识的种类（C ABI 将来追加的值）。
    Other(i32),
}

/// 鼠标按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// 无按键。
    None,
    /// 左键。
    Left,
    /// 中键。
    Middle,
    /// 右键。
    Right,
    /// 本版本不认识的值。
    Other(i32),
}

/// 拖放效果（`DataTransfer.dropEffect`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropEffect {
    /// 不允许放下。
    None,
    /// 复制。
    Copy,
    /// 移动。
    Move,
    /// 链接。
    Link,
    /// 本版本不认识的值。
    Other(i32),
}

impl DropEffect {
    fn raw(self) -> sys::KuiCDropEffect {
        match self {
            DropEffect::None => sys::KUI_C_DROP_NONE,
            DropEffect::Copy => sys::KUI_C_DROP_COPY,
            DropEffect::Move => sys::KUI_C_DROP_MOVE,
            DropEffect::Link => sys::KUI_C_DROP_LINK,
            DropEffect::Other(value) => value,
        }
    }
    fn from_raw(value: sys::KuiCDropEffect) -> Self {
        match value {
            sys::KUI_C_DROP_NONE => DropEffect::None,
            sys::KUI_C_DROP_COPY => DropEffect::Copy,
            sys::KUI_C_DROP_MOVE => DropEffect::Move,
            sys::KUI_C_DROP_LINK => DropEffect::Link,
            other => DropEffect::Other(other),
        }
    }
}

/// 拖动源允许的效果（`DataTransfer.effectAllowed`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectAllowed {
    /// 都不允许。
    None,
    /// 复制。
    Copy,
    /// 移动。
    Move,
    /// 链接。
    Link,
    /// 复制或移动。
    CopyMove,
    /// 复制或链接。
    CopyLink,
    /// 移动或链接。
    MoveLink,
    /// 全部。
    All,
    /// `dragstart` 之前的初始值。
    Uninitialized,
    /// 本版本不认识的值。
    Other(i32),
}

impl EffectAllowed {
    fn raw(self) -> sys::KuiCEffectAllowed {
        match self {
            EffectAllowed::None => sys::KUI_C_EFFECT_NONE,
            EffectAllowed::Copy => sys::KUI_C_EFFECT_COPY,
            EffectAllowed::Move => sys::KUI_C_EFFECT_MOVE,
            EffectAllowed::Link => sys::KUI_C_EFFECT_LINK,
            EffectAllowed::CopyMove => sys::KUI_C_EFFECT_COPY_MOVE,
            EffectAllowed::CopyLink => sys::KUI_C_EFFECT_COPY_LINK,
            EffectAllowed::MoveLink => sys::KUI_C_EFFECT_MOVE_LINK,
            EffectAllowed::All => sys::KUI_C_EFFECT_ALL,
            EffectAllowed::Uninitialized => sys::KUI_C_EFFECT_UNINITIALIZED,
            EffectAllowed::Other(value) => value,
        }
    }
    fn from_raw(value: sys::KuiCEffectAllowed) -> Self {
        match value {
            sys::KUI_C_EFFECT_NONE => EffectAllowed::None,
            sys::KUI_C_EFFECT_COPY => EffectAllowed::Copy,
            sys::KUI_C_EFFECT_MOVE => EffectAllowed::Move,
            sys::KUI_C_EFFECT_LINK => EffectAllowed::Link,
            sys::KUI_C_EFFECT_COPY_MOVE => EffectAllowed::CopyMove,
            sys::KUI_C_EFFECT_COPY_LINK => EffectAllowed::CopyLink,
            sys::KUI_C_EFFECT_MOVE_LINK => EffectAllowed::MoveLink,
            sys::KUI_C_EFFECT_ALL => EffectAllowed::All,
            sys::KUI_C_EFFECT_UNINITIALIZED => EffectAllowed::Uninitialized,
            other => EffectAllowed::Other(other),
        }
    }
}

/// 事件的三个标志。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventFlags {
    /// 是否冒泡。
    pub bubbles: bool,
    /// 是否可取消。
    pub cancelable: bool,
    /// 是否已被 `prevent_default`。
    pub default_prevented: bool,
}

/// 键盘修饰键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Ctrl。
    pub ctrl: bool,
    /// Shift。
    pub shift: bool,
    /// Alt。
    pub alt: bool,
    /// Meta（Windows 键 / Command）。
    pub meta: bool,
}

/// 已登记的监听器。移除时需要它和事件名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerId {
    id: sys::KuiListenerId,
    token: usize,
}
impl ListenerId {
    /// C ABI 的原始监听器 id。
    pub fn raw(&self) -> u64 {
        self.id
    }
}

/// 平台定时器 id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(sys::KuiTimerId);
impl TimerId {
    /// C ABI 的原始定时器 id。
    pub fn raw(&self) -> u64 {
        self.0
    }
}

type ListenerFn = Box<dyn FnMut(&Event<'_>)>;
type ResizeFn = Box<dyn FnMut(u32, u32)>;
type TaskOnce = Option<Box<dyn FnOnce()>>;
type TaskRepeat = Box<dyn FnMut()>;
type ScriptErrorFn = Box<dyn FnMut(&str, &str, i32)>;
type HttpWorkerFn = Box<dyn FnOnce(Result<HTTPResponse>) + Send + 'static>;

const RESIZE_GROUP: &str = "\u{0}resize";
const SCRIPT_ERROR_GROUP: &str = "\u{0}script-error";

unsafe extern "C" fn listener_trampoline(event: *mut sys::KuiEvent, user_data: *mut c_void) {
    let Some(raw) = NonNull::new(event) else {
        return;
    };
    let event = Event {
        raw,
        _callback: PhantomData,
    };
    callbacks::invoke::<ListenerFn>(user_data as usize, |callback| callback(&event));
}

unsafe extern "C" fn resize_trampoline(width: u32, height: u32, user_data: *mut c_void) {
    callbacks::invoke::<ResizeFn>(user_data as usize, |callback| callback(width, height));
}

unsafe extern "C" fn task_once_trampoline(user_data: *mut c_void) {
    callbacks::invoke::<TaskOnce>(user_data as usize, |task| {
        if let Some(task) = task.take() {
            task();
        }
    });
}

unsafe extern "C" fn task_repeat_trampoline(user_data: *mut c_void) {
    callbacks::invoke::<TaskRepeat>(user_data as usize, |task| task());
}

unsafe extern "C" fn script_error_trampoline(
    message: *const std::ffi::c_char,
    message_len: usize,
    source: *const std::ffi::c_char,
    source_len: usize,
    line: std::ffi::c_int,
    user_data: *mut c_void,
) {
    let text = |raw: *const std::ffi::c_char, len: usize| {
        if raw.is_null() || len == 0 {
            String::new()
        } else {
            String::from_utf8_lossy(std::slice::from_raw_parts(raw as *const u8, len)).into_owned()
        }
    };
    let (message, source) = (text(message, message_len), text(source, source_len));
    callbacks::invoke::<ScriptErrorFn>(user_data as usize, |callback| {
        callback(&message, &source, line)
    });
}

unsafe extern "C" fn http_worker_trampoline(
    response: *mut sys::KuiHttpResponse,
    user_data: *mut c_void,
) {
    let callback = Box::from_raw(user_data as *mut HttpWorkerFn);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        callback(take_http_response(response))
    }));
}

impl<'a> Event<'a> {
    /// 事件名，例如 `"click"`。
    pub fn event_type(&self) -> Result<String> {
        read_string("Event::event_type", |buf, cap, out| unsafe {
            sys::kui_event_get_type(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 事件种类。
    pub fn type_tag(&self) -> Result<EventTypeTag> {
        let mut tag: sys::KuiCEventTypeTag = 0;
        check(
            unsafe { sys::kui_event_get_type_tag(self.raw.as_ptr(), &mut tag) },
            "Event::type_tag",
        )?;
        Ok(match tag {
            sys::KUI_C_EVENT_BASE => EventTypeTag::Base,
            sys::KUI_C_EVENT_MOUSE => EventTypeTag::Mouse,
            sys::KUI_C_EVENT_WHEEL => EventTypeTag::Wheel,
            sys::KUI_C_EVENT_POINTER => EventTypeTag::Pointer,
            sys::KUI_C_EVENT_KEYBOARD => EventTypeTag::Keyboard,
            sys::KUI_C_EVENT_FOCUS => EventTypeTag::Focus,
            sys::KUI_C_EVENT_INPUT => EventTypeTag::Input,
            sys::KUI_C_EVENT_DRAG => EventTypeTag::Drag,
            other => EventTypeTag::Other(other),
        })
    }
    fn element<'d>(
        &self,
        document: &'d Document,
        call: unsafe extern "C" fn(*mut sys::KuiEvent, *mut *mut sys::KuiElement) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<Option<Element<'d>>> {
        let mut raw = ptr::null_mut();
        let status = unsafe { call(self.raw.as_ptr(), &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, context)?;
        Element::from_raw(raw, document).map(Some)
    }
    /// 事件目标元素；目标不是元素（例如文本节点）时为 `None`。`document` 是目标所在的文档，
    /// 返回的元素借用它的生命周期（C ABI 没有「取元素所属文档」的入口）。
    pub fn target<'d>(&self, document: &'d Document) -> Result<Option<Element<'d>>> {
        self.element(document, sys::kui_event_get_target, "Event::target")
    }
    /// 当前正在处理事件的元素。
    pub fn current_target<'d>(&self, document: &'d Document) -> Result<Option<Element<'d>>> {
        self.element(
            document,
            sys::kui_event_get_current_target,
            "Event::current_target",
        )
    }
    /// 鼠标事件（mouseover / mouseout / mouseenter / mouseleave）与焦点事件（focus / blur）的另一端；
    /// `<kui-context-menu>` 的 `menu-open` 里是被右键的元素。没有时为 `None`。
    pub fn related_target<'d>(&self, document: &'d Document) -> Result<Option<Element<'d>>> {
        self.element(
            document,
            sys::kui_event_get_related_target,
            "Event::related_target",
        )
    }
    /// 阻止默认行为。
    pub fn prevent_default(&self) -> Result<()> {
        check(
            unsafe { sys::kui_event_prevent_default(self.raw.as_ptr()) },
            "Event::prevent_default",
        )
    }
    /// 停止传播。
    pub fn stop_propagation(&self) -> Result<()> {
        check(
            unsafe { sys::kui_event_stop_propagation(self.raw.as_ptr()) },
            "Event::stop_propagation",
        )
    }
    /// 冒泡 / 可取消 / 已阻止默认行为三个标志。
    pub fn flags(&self) -> Result<EventFlags> {
        let (mut bubbles, mut cancelable, mut prevented) = (0, 0, 0);
        check(
            unsafe {
                sys::kui_event_get_flags(
                    self.raw.as_ptr(),
                    &mut bubbles,
                    &mut cancelable,
                    &mut prevented,
                )
            },
            "Event::flags",
        )?;
        Ok(EventFlags {
            bubbles: bubbles != 0,
            cancelable: cancelable != 0,
            default_prevented: prevented != 0,
        })
    }
    fn point(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiEvent, *mut f32, *mut f32) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<Point> {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        check(unsafe { call(self.raw.as_ptr(), &mut x, &mut y) }, context)?;
        Ok(Point { x, y })
    }
    /// 视口坐标。不是鼠标事件时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn mouse_client(&self) -> Result<Point> {
        self.point(sys::kui_event_mouse_get_client_xy, "Event::mouse_client")
    }
    /// 页面坐标（含滚动偏移）。
    pub fn mouse_page(&self) -> Result<Point> {
        self.point(sys::kui_event_mouse_get_page_xy, "Event::mouse_page")
    }
    /// 相对目标 padding box 的坐标。
    pub fn mouse_offset(&self) -> Result<Point> {
        self.point(sys::kui_event_mouse_get_offset_xy, "Event::mouse_offset")
    }
    /// `"wheel"` 事件的滚动量；不是滚轮事件返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn wheel_delta(&self) -> Result<Point> {
        self.point(sys::kui_event_wheel_get_delta, "Event::wheel_delta")
    }
    /// 鼠标按键。
    pub fn mouse_button(&self) -> Result<MouseButton> {
        let mut button = 0;
        check(
            unsafe { sys::kui_event_mouse_get_button(self.raw.as_ptr(), &mut button) },
            "Event::mouse_button",
        )?;
        Ok(match button {
            0 => MouseButton::None,
            1 => MouseButton::Left,
            2 => MouseButton::Middle,
            3 => MouseButton::Right,
            other => MouseButton::Other(other),
        })
    }
    /// 按键含义，例如 `"a"`、`"Enter"`。不是键盘事件时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn key(&self) -> Result<String> {
        read_string("Event::key", |buf, cap, out| unsafe {
            sys::kui_event_key_get_key(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 物理按键，例如 `"KeyA"`。
    pub fn code(&self) -> Result<String> {
        read_string("Event::code", |buf, cap, out| unsafe {
            sys::kui_event_key_get_code(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 修饰键。
    pub fn modifiers(&self) -> Result<Modifiers> {
        let (mut ctrl, mut shift, mut alt, mut meta) = (0, 0, 0, 0);
        check(
            unsafe {
                sys::kui_event_key_get_modifiers(
                    self.raw.as_ptr(),
                    &mut ctrl,
                    &mut shift,
                    &mut alt,
                    &mut meta,
                )
            },
            "Event::modifiers",
        )?;
        Ok(Modifiers {
            ctrl: ctrl != 0,
            shift: shift != 0,
            alt: alt != 0,
            meta: meta != 0,
        })
    }
    /// 拖入的文件路径。不是拖放事件时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn drag_files(&self) -> Result<Vec<String>> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_event_drag_get_file_count(self.raw.as_ptr(), &mut count) },
            "Event::drag_files",
        )?;
        (0..count)
            .map(|index| {
                read_string("Event::drag_files", |buf, cap, out| unsafe {
                    sys::kui_event_drag_get_file(self.raw.as_ptr(), index, buf, cap, out)
                })
            })
            .collect()
    }
    /// 在 `dragstart` 里设置拖动数据，`format` 是 MIME 类型（如 `"text/plain"`）。
    /// 不是拖放事件时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn set_drag_data(&self, format: &str, data: &str) -> Result<()> {
        let (format_ptr, format_len) = bytes(format);
        let (data_ptr, data_len) = bytes(data);
        check(
            unsafe {
                sys::kui_event_drag_set_data(
                    self.raw.as_ptr(),
                    format_ptr,
                    format_len,
                    data_ptr,
                    data_len,
                )
            },
            "Event::set_drag_data",
        )
    }
    /// 读取拖动数据；没设置过该格式时返回 `None`。
    pub fn drag_data(&self, format: &str) -> Result<Option<String>> {
        let (format_ptr, format_len) = bytes(format);
        let mut size = 0usize;
        let status = unsafe {
            sys::kui_event_drag_get_data(
                self.raw.as_ptr(),
                format_ptr,
                format_len,
                ptr::null_mut(),
                0,
                &mut size,
            )
        };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        read_string("Event::drag_data", |buf, cap, out| unsafe {
            sys::kui_event_drag_get_data(self.raw.as_ptr(), format_ptr, format_len, buf, cap, out)
        })
        .map(Some)
    }
    /// 清除拖动数据：`Some(format)` 清一种，`None` 清全部。
    pub fn clear_drag_data(&self, format: Option<&str>) -> Result<()> {
        let (format_ptr, format_len) = match format {
            Some(format) => bytes(format),
            None => (ptr::null(), 0),
        };
        check(
            unsafe { sys::kui_event_drag_clear_data(self.raw.as_ptr(), format_ptr, format_len) },
            "Event::clear_drag_data",
        )
    }
    /// 已设置的数据格式，按设置顺序（`DataTransfer.types`）。
    pub fn drag_types(&self) -> Result<Vec<String>> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_event_drag_get_type_count(self.raw.as_ptr(), &mut count) },
            "Event::drag_types",
        )?;
        (0..count)
            .map(|index| {
                read_string("Event::drag_types", |buf, cap, out| unsafe {
                    sys::kui_event_drag_get_type(self.raw.as_ptr(), index, buf, cap, out)
                })
            })
            .collect()
    }
    /// 设置拖放效果（在目标的 `dragover` 里）。
    pub fn set_drop_effect(&self, effect: DropEffect) -> Result<()> {
        check(
            unsafe { sys::kui_event_drag_set_drop_effect(self.raw.as_ptr(), effect.raw()) },
            "Event::set_drop_effect",
        )
    }
    /// 当前拖放效果。
    pub fn drop_effect(&self) -> Result<DropEffect> {
        let mut value = sys::KUI_C_DROP_NONE;
        check(
            unsafe { sys::kui_event_drag_get_drop_effect(self.raw.as_ptr(), &mut value) },
            "Event::drop_effect",
        )?;
        Ok(DropEffect::from_raw(value))
    }
    /// 设置拖动源允许的效果（在 `dragstart` 里）。
    pub fn set_effect_allowed(&self, effect: EffectAllowed) -> Result<()> {
        check(
            unsafe { sys::kui_event_drag_set_effect_allowed(self.raw.as_ptr(), effect.raw()) },
            "Event::set_effect_allowed",
        )
    }
    /// 拖动源允许的效果。
    pub fn effect_allowed(&self) -> Result<EffectAllowed> {
        let mut value = sys::KUI_C_EFFECT_NONE;
        check(
            unsafe { sys::kui_event_drag_get_effect_allowed(self.raw.as_ptr(), &mut value) },
            "Event::effect_allowed",
        )?;
        Ok(EffectAllowed::from_raw(value))
    }
}

impl<'doc> Element<'doc> {
    fn owner_key(&self) -> usize {
        self.raw.as_ptr() as usize
    }
    /// 登记事件监听。闭包在 UI 线程执行，可以捕获 `Rc` 等对象；移除监听时释放。
    pub fn add_event_listener<F>(&self, name: &str, listener: F) -> Result<ListenerId>
    where
        F: FnMut(&Event<'_>) + 'static,
    {
        self.add_listener(name, listener, false, "Element::add_event_listener")
    }
    /// 登记捕获阶段监听：事件从根往下走时执行（在目标自己身上最先执行，早于普通监听）；在里面
    /// `stop_propagation` 会终止下行。用 [`Element::remove_event_listener`] 摘除。
    pub fn add_capture_event_listener<F>(&self, name: &str, listener: F) -> Result<ListenerId>
    where
        F: FnMut(&Event<'_>) + 'static,
    {
        self.add_listener(name, listener, true, "Element::add_capture_event_listener")
    }
    fn add_listener<F>(
        &self,
        name: &str,
        listener: F,
        capture: bool,
        context: &'static str,
    ) -> Result<ListenerId>
    where
        F: FnMut(&Event<'_>) + 'static,
    {
        let token =
            callbacks::register::<ListenerFn>(Box::new(listener), false, self.owner_key(), name);
        let (raw_name, len) = bytes(name);
        let mut id: sys::KuiListenerId = 0;
        let status = unsafe {
            if capture {
                sys::kui_element_add_event_listener_ex(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    Some(listener_trampoline),
                    token as *mut c_void,
                    1,
                    &mut id,
                )
            } else {
                sys::kui_element_add_event_listener(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    Some(listener_trampoline),
                    token as *mut c_void,
                    &mut id,
                )
            }
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, context)?;
        Ok(ListenerId { id, token })
    }
    /// 移除一个监听（回调里移除自己也可以）。
    pub fn remove_event_listener(&self, name: &str, listener: ListenerId) -> Result<()> {
        let (raw_name, len) = bytes(name);
        let status = unsafe {
            sys::kui_element_remove_event_listener(self.raw.as_ptr(), raw_name, len, listener.id)
        };
        callbacks::unregister(listener.token);
        check(status, "Element::remove_event_listener")
    }
    /// 移除某个事件名下的全部监听。
    pub fn remove_event_listeners(&self, name: &str) -> Result<()> {
        let (raw_name, len) = bytes(name);
        let status =
            unsafe { sys::kui_element_remove_event_listeners(self.raw.as_ptr(), raw_name, len) };
        callbacks::unregister_group(self.owner_key(), name);
        check(status, "Element::remove_event_listeners")
    }
    /// 派发合成事件，走正常的捕获 / 冒泡；返回没有监听器调用 `prevent_default` 时为 `true`。
    /// 元素必须已挂到文档上。
    pub fn dispatch_event(&self, name: &str, bubbles: bool, cancelable: bool) -> Result<bool> {
        let (raw_name, len) = bytes(name);
        let mut not_canceled = 0;
        check(
            unsafe {
                sys::kui_element_dispatch_event(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    bubbles as i32,
                    cancelable as i32,
                    &mut not_canceled,
                )
            },
            "Element::dispatch_event",
        )?;
        Ok(not_canceled != 0)
    }
    /// 所属文档（构造时借用的那一个）。
    pub fn document(&self) -> &'doc Document {
        self.document
    }
}

impl Window {
    pub(crate) fn owner_key(&self) -> usize {
        self.raw.as_ptr() as usize
    }
    /// 设置窗口尺寸变化回调（替换上一个）。
    pub fn on_resize<F>(&self, callback: F) -> Result<()>
    where
        F: FnMut(u32, u32) + 'static,
    {
        callbacks::unregister_group(self.owner_key(), RESIZE_GROUP);
        let token = callbacks::register::<ResizeFn>(
            Box::new(callback),
            false,
            self.owner_key(),
            RESIZE_GROUP,
        );
        let status = unsafe {
            sys::kui_window_on_resize(
                self.raw.as_ptr(),
                Some(resize_trampoline),
                token as *mut c_void,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Window::on_resize")
    }
    /// 清除尺寸变化回调。
    pub fn clear_on_resize(&self) -> Result<()> {
        let status = unsafe { sys::kui_window_on_resize(self.raw.as_ptr(), None, ptr::null_mut()) };
        callbacks::unregister_group(self.owner_key(), RESIZE_GROUP);
        check(status, "Window::clear_on_resize")
    }
    /// 登记窗口事件监听（例如 `"close"`，监听器可以 `prevent_default` 阻止关闭）。
    /// `once` 为真时触发一次后自动移除。
    pub fn add_event_listener<F>(&self, name: &str, once: bool, listener: F) -> Result<ListenerId>
    where
        F: FnMut(&Event<'_>) + 'static,
    {
        let token =
            callbacks::register::<ListenerFn>(Box::new(listener), once, self.owner_key(), name);
        let (raw_name, len) = bytes(name);
        let mut id: sys::KuiListenerId = 0;
        let status = unsafe {
            sys::kui_window_add_event_listener(
                self.raw.as_ptr(),
                raw_name,
                len,
                Some(listener_trampoline),
                token as *mut c_void,
                once as i32,
                &mut id,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Window::add_event_listener")?;
        Ok(ListenerId { id, token })
    }
    /// 移除一个窗口事件监听。
    pub fn remove_event_listener(&self, name: &str, listener: ListenerId) -> Result<()> {
        let (raw_name, len) = bytes(name);
        let status = unsafe {
            sys::kui_window_remove_event_listener(self.raw.as_ptr(), raw_name, len, listener.id)
        };
        callbacks::unregister(listener.token);
        check(status, "Window::remove_event_listener")
    }
    /// 移除某个事件名下的全部窗口事件监听。
    pub fn remove_event_listeners(&self, name: &str) -> Result<()> {
        let (raw_name, len) = bytes(name);
        let status =
            unsafe { sys::kui_window_remove_event_listeners(self.raw.as_ptr(), raw_name, len) };
        callbacks::unregister_group(self.owner_key(), name);
        check(status, "Window::remove_event_listeners")
    }
}

impl Platform {
    fn add_timer(&self, delay_ms: u32, repeat: bool, token: usize) -> Result<TimerId> {
        let mut id: sys::KuiTimerId = 0;
        let status = unsafe {
            if repeat {
                sys::kui_platform_set_interval(
                    Some(task_repeat_trampoline),
                    token as *mut c_void,
                    delay_ms,
                    &mut id,
                )
            } else {
                sys::kui_platform_set_timeout(
                    Some(task_once_trampoline),
                    token as *mut c_void,
                    delay_ms,
                    &mut id,
                )
            }
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(
            status,
            if repeat {
                "Platform::set_interval"
            } else {
                "Platform::set_timeout"
            },
        )?;
        callbacks::regroup(token, &id.to_string());
        Ok(TimerId(id))
    }
    /// `delay_ms` 毫秒后在 UI 线程执行一次。
    pub fn set_timeout<F>(&self, delay_ms: u32, task: F) -> Result<TimerId>
    where
        F: FnOnce() + 'static,
    {
        let token = callbacks::register::<TaskOnce>(Some(Box::new(task)), true, TIMER_OWNER, "");
        self.add_timer(delay_ms, false, token)
    }
    /// 每隔 `interval_ms` 毫秒在 UI 线程执行一次，直到 [`Platform::clear_timer`]。
    pub fn set_interval<F>(&self, interval_ms: u32, task: F) -> Result<TimerId>
    where
        F: FnMut() + 'static,
    {
        let token = callbacks::register::<TaskRepeat>(Box::new(task), false, TIMER_OWNER, "");
        self.add_timer(interval_ms, true, token)
    }
    /// 清除定时器（未触发的闭包随之释放）。
    pub fn clear_timer(&self, id: TimerId) -> Result<()> {
        let status = unsafe { sys::kui_platform_clear_timer(id.0) };
        callbacks::unregister_group(TIMER_OWNER, &id.0.to_string());
        check(status, "Platform::clear_timer")
    }
    /// 事件循环下次空闲时在 UI 线程执行一次。跨线程请用 [`crate::post_to_main_thread`]。
    pub fn request_idle<F>(&self, task: F) -> Result<()>
    where
        F: FnOnce() + 'static,
    {
        let token =
            callbacks::register::<TaskOnce>(Some(Box::new(task)), true, TIMER_OWNER, "\u{0}idle");
        let status = unsafe {
            sys::kui_platform_request_idle(Some(task_once_trampoline), token as *mut c_void)
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Platform::request_idle")
    }
}

impl ScriptEngine {
    pub(crate) fn owner_key(&self) -> usize {
        self.raw.as_ptr() as usize
    }
    /// 设置 Lua 运行时错误回调 `(错误信息, 来源, 行号)`（替换上一个）。
    pub fn set_error_handler<F>(&self, handler: F) -> Result<()>
    where
        F: FnMut(&str, &str, i32) + 'static,
    {
        callbacks::unregister_group(self.owner_key(), SCRIPT_ERROR_GROUP);
        let token = callbacks::register::<ScriptErrorFn>(
            Box::new(handler),
            false,
            self.owner_key(),
            SCRIPT_ERROR_GROUP,
        );
        let status = unsafe {
            sys::kui_script_engine_set_error_handler(
                self.raw.as_ptr(),
                Some(script_error_trampoline),
                token as *mut c_void,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "ScriptEngine::set_error_handler")
    }
    /// 清除错误回调。
    pub fn clear_error_handler(&self) -> Result<()> {
        let status = unsafe {
            sys::kui_script_engine_set_error_handler(self.raw.as_ptr(), None, ptr::null_mut())
        };
        callbacks::unregister_group(self.owner_key(), SCRIPT_ERROR_GROUP);
        check(status, "ScriptEngine::clear_error_handler")
    }
    /// 解除文档绑定。
    pub fn unbind_document(&self) -> Result<()> {
        check(
            unsafe { sys::kui_script_engine_unbind_document(self.raw.as_ptr()) },
            "ScriptEngine::unbind_document",
        )
    }
}

impl HTTPClient {
    /// 异步请求；`callback` 在 **HTTP 工作线程** 上执行，所以必须是 `Send`。要操作界面请用
    /// [`HTTPClient::fetch_async_on_main_thread`]，或在回调里 [`crate::post_to_main_thread`]。
    /// 被取消或失败的请求也会回调一次（错误在响应里）。
    pub fn fetch_async<F>(&self, url: &str, timeout_seconds: f64, callback: F) -> Result<()>
    where
        F: FnOnce(Result<HTTPResponse>) + Send + 'static,
    {
        let (raw_url, len) = bytes(url);
        let raw = Box::into_raw(Box::new(Box::new(callback) as HttpWorkerFn));
        let status = unsafe {
            sys::kui_http_client_fetch_async(
                self.raw.as_ptr(),
                raw_url,
                len,
                timeout_seconds,
                Some(http_worker_trampoline),
                raw.cast(),
            )
        };
        if status != sys::KUI_OK {
            drop(unsafe { Box::from_raw(raw) });
        }
        check(status, "HTTPClient::fetch_async")
    }
}
