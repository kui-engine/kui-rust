//! C ABI 第 9 节（3.61.8）：常用窗口、DOM 与平台操作。窗口关闭 / 聚焦通知请用
//! [`Window::add_event_listener`] 注册 `"close"`（可 `prevent_default` 阻止关闭）、`"focus"`、`"blur"`。

use std::ffi::c_void;
use std::ptr;

use crate::system::MonitorInfo;
use crate::{
    bool_int, bytes, check, read_string, sys, Document, Element, Platform, Result, Window,
};

/// 光标种类，取值与原生 CursorType（CSS cursor 关键字）相同。
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Cursor {
    Auto = 0,
    Default,
    Pointer,
    Crosshair,
    Text,
    Move,
    NotAllowed,
    Grab,
    Grabbing,
    Wait,
    Progress,
    Help,
    ContextMenu,
    NResize,
    EResize,
    SResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
}

/// 窗口四个角的圆角半径（CSS px）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameRadii {
    /// 左上。
    pub top_left: f32,
    /// 右上。
    pub top_right: f32,
    /// 右下。
    pub bottom_right: f32,
    /// 左下。
    pub bottom_left: f32,
}

/// 无框窗口的三个开关。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameState {
    /// 无框。
    pub frameless: bool,
    /// 无框窗口带系统阴影。
    pub shadow: bool,
    /// 无框窗口可拖边框改尺寸。
    pub sizeable: bool,
}

/// 消息框图标。
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxType {
    Info,
    Warning,
    Error,
    Question,
}

/// 消息框按钮组合。
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

/// 用户在消息框里的选择。
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    Cancel,
    Yes,
    No,
}

impl Window {
    /// 启用 / 禁用窗口的鼠标键盘输入（模态父窗口）；仅 Windows，其他平台返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_enabled(self.raw.as_ptr(), bool_int(enabled)) },
            "Window::set_enabled",
        )
    }
    /// 窗口是否启用输入；仅 Windows。
    pub fn is_enabled(&self) -> Result<bool> {
        let mut enabled = 0;
        check(
            unsafe { sys::kui_window_is_enabled(self.raw.as_ptr(), &mut enabled) },
            "Window::is_enabled",
        )?;
        Ok(enabled != 0)
    }
    /// 立即处理待处理事件、更新文档并绘制一帧（平台循环每帧做的事）；典型用法：窗口还隐藏时先渲染首帧再 `show`。
    pub fn process_frame(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_process_frame(self.raw.as_ptr()) },
            "Window::process_frame",
        )
    }
    /// 打开 / 关闭 DevTools WebSocket 的令牌校验；DevTools 服务必须已启动，否则 `KUI_ERR_NOT_FOUND`。
    /// 关掉之后任何本机客户端都能连，只给无法带令牌的工具用。
    pub fn set_devtools_token_auth(&self, enabled: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_window_set_devtools_token_auth(self.raw.as_ptr(), bool_int(enabled))
            },
            "Window::set_devtools_token_auth",
        )
    }
    fn int_flag(
        &self,
        call: impl FnOnce(*mut sys::KuiWindow, *mut i32) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<bool> {
        let mut value = 0;
        check(call(self.raw.as_ptr(), &mut value), context)?;
        Ok(value != 0)
    }
    /// 把窗口带到前台并获得焦点。
    pub fn focus(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_focus(self.raw.as_ptr()) },
            "Window::focus",
        )
    }
    /// 窗口是否有焦点。
    pub fn has_focus(&self) -> Result<bool> {
        self.int_flag(
            |w, out| unsafe { sys::kui_window_has_focus(w, out) },
            "Window::has_focus",
        )
    }
    /// 立即切换光标；指针下一次进入别的内容时，CSS `cursor` 会重新生效。
    pub fn set_cursor(&self, cursor: Cursor) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_cursor(self.raw.as_ptr(), cursor as i32) },
            "Window::set_cursor",
        )
    }
    /// 显示或隐藏光标。
    pub fn set_cursor_visible(&self, visible: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_cursor_visible(self.raw.as_ptr(), bool_int(visible)) },
            "Window::set_cursor_visible",
        )
    }
    /// 进入或退出全屏。
    pub fn set_fullscreen(&self, fullscreen: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_fullscreen(self.raw.as_ptr(), bool_int(fullscreen)) },
            "Window::set_fullscreen",
        )
    }
    /// 是否全屏。
    pub fn is_fullscreen(&self) -> Result<bool> {
        self.int_flag(
            |w, out| unsafe { sys::kui_window_is_fullscreen(w, out) },
            "Window::is_fullscreen",
        )
    }
    /// 创建后切换无框模式。
    pub fn set_frameless(&self, frameless: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_frameless(self.raw.as_ptr(), bool_int(frameless)) },
            "Window::set_frameless",
        )
    }
    /// 无框窗口是否带系统阴影。
    pub fn set_frameless_shadow(&self, shadow: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_frameless_shadow(self.raw.as_ptr(), bool_int(shadow)) },
            "Window::set_frameless_shadow",
        )
    }
    /// 无框 / 阴影 / 可调尺寸三个开关的当前值。
    pub fn frame_state(&self) -> Result<FrameState> {
        let (mut frameless, mut shadow, mut sizeable) = (0, 0, 0);
        check(
            unsafe {
                sys::kui_window_get_frame_state(
                    self.raw.as_ptr(),
                    &mut frameless,
                    &mut shadow,
                    &mut sizeable,
                )
            },
            "Window::frame_state",
        )?;
        Ok(FrameState {
            frameless: frameless != 0,
            shadow: shadow != 0,
            sizeable: sizeable != 0,
        })
    }
    /// 四个角分别设置圆角。
    pub fn set_frame_radii(&self, radii: FrameRadii) -> Result<()> {
        check(
            unsafe {
                sys::kui_window_set_frame_radii(
                    self.raw.as_ptr(),
                    radii.top_left,
                    radii.top_right,
                    radii.bottom_right,
                    radii.bottom_left,
                )
            },
            "Window::set_frame_radii",
        )
    }
    /// 当前圆角；没有设置时为 `None`。
    pub fn frame_radii(&self) -> Result<Option<FrameRadii>> {
        let mut radii = FrameRadii::default();
        let status = unsafe {
            sys::kui_window_get_frame_radii(
                self.raw.as_ptr(),
                &mut radii.top_left,
                &mut radii.top_right,
                &mut radii.bottom_right,
                &mut radii.bottom_left,
            )
        };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Window::frame_radii")?;
        Ok(Some(radii))
    }
    /// 输入法候选框跟随自绘光标的位置（视口坐标）。
    pub fn set_ime_candidate_pos(&self, x: f32, y: f32) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_ime_candidate_pos(self.raw.as_ptr(), x, y) },
            "Window::set_ime_candidate_pos",
        )
    }
}

impl<'doc> Element<'doc> {
    /// 把 `child` 插到 `reference` 前面；`reference` 为 `None` 时追加到末尾。已在树里的 `child` 会被移动。
    /// `reference` 不是自己的子元素时返回 `KUI_ERR_NOT_FOUND`；插入自己的祖先返回 `KUI_ERR_INVALID_ARG`。
    pub fn insert_before(
        &self,
        child: &Element<'_>,
        reference: Option<&Element<'_>>,
    ) -> Result<()> {
        check(
            unsafe {
                sys::kui_element_insert_before(
                    self.raw.as_ptr(),
                    child.raw.as_ptr(),
                    reference.map_or(ptr::null_mut(), |value| value.raw.as_ptr()),
                )
            },
            "Element::insert_before",
        )
    }
    /// 用 `new_child` 替换子元素 `old_child`。
    pub fn replace_child(&self, new_child: &Element<'_>, old_child: &Element<'_>) -> Result<()> {
        check(
            unsafe {
                sys::kui_element_replace_child(
                    self.raw.as_ptr(),
                    new_child.raw.as_ptr(),
                    old_child.raw.as_ptr(),
                )
            },
            "Element::replace_child",
        )
    }
    /// DOM `cloneNode`：脱离文档的新元素，`deep` 时连子树一起复制。（`Clone` 只是同一元素的另一个句柄。）
    pub fn clone_node(&self, deep: bool) -> Result<Element<'doc>> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_element_clone(self.raw.as_ptr(), bool_int(deep), &mut raw) },
            "Element::clone_node",
        )?;
        Element::from_raw(raw, self.document)
    }
    /// 是否匹配 selector。
    pub fn matches(&self, selector: &str) -> Result<bool> {
        let (raw, len) = bytes(selector);
        let mut value = 0;
        check(
            unsafe { sys::kui_element_matches(self.raw.as_ptr(), raw, len, &mut value) },
            "Element::matches",
        )?;
        Ok(value != 0)
    }
    /// 最近的匹配祖先（含自己）。
    pub fn closest(&self, selector: &str) -> Result<Option<Element<'doc>>> {
        let (raw, len) = bytes(selector);
        let mut out = ptr::null_mut();
        let status = unsafe { sys::kui_element_closest(self.raw.as_ptr(), raw, len, &mut out) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Element::closest")?;
        Element::from_raw(out, self.document).map(Some)
    }
    /// outerHTML。
    pub fn outer_html(&self) -> Result<String> {
        read_string("Element::outer_html", |buf, cap, out| unsafe {
            sys::kui_element_get_outer_html(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// `getComputedStyle(el).getPropertyValue(name)`：按 CSS 属性名（短横线写法）读计算后的值，
    /// 与 Lua 的 `getComputedStyle` 同口径；逻辑属性按 `direction` 映射。未知属性名或不在文档里的
    /// 元素返回空串。
    pub fn computed_style_value(&self, property: &str) -> Result<String> {
        let (raw, len) = bytes(property);
        read_string("Element::computed_style_value", |buf, cap, out| unsafe {
            sys::kui_element_get_computed_style_value(self.raw.as_ptr(), raw, len, buf, cap, out)
        })
    }
    /// `position` 为 `"beforebegin"`、`"afterbegin"`、`"beforeend"` 或 `"afterend"`。
    pub fn insert_adjacent_html(&self, position: &str, html: &str) -> Result<()> {
        let (raw_position, position_len) = bytes(position);
        let (raw_html, html_len) = bytes(html);
        check(
            unsafe {
                sys::kui_element_insert_adjacent_html(
                    self.raw.as_ptr(),
                    raw_position,
                    position_len,
                    raw_html,
                    html_len,
                )
            },
            "Element::insert_adjacent_html",
        )
    }
    /// 追加一个文本节点。
    pub fn append_text(&self, text: &str) -> Result<()> {
        let (raw, len) = bytes(text);
        check(
            unsafe { sys::kui_element_append_text(self.raw.as_ptr(), raw, len) },
            "Element::append_text",
        )
    }
    /// 元素是否有焦点。
    pub fn has_focus(&self) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_element_has_focus(self.raw.as_ptr(), &mut value) },
            "Element::has_focus",
        )?;
        Ok(value != 0)
    }
    /// 捕获指针：`pointer_id` 的后续移动与松开都发给这个元素，直到释放。鼠标的 id 是 1。
    pub fn set_pointer_capture(&self, pointer_id: i32) -> Result<()> {
        check(
            unsafe { sys::kui_element_set_pointer_capture(self.raw.as_ptr(), pointer_id) },
            "Element::set_pointer_capture",
        )
    }
    /// 释放指针捕获。
    pub fn release_pointer_capture(&self, pointer_id: i32) -> Result<()> {
        check(
            unsafe { sys::kui_element_release_pointer_capture(self.raw.as_ptr(), pointer_id) },
            "Element::release_pointer_capture",
        )
    }
    /// 是否捕获着 `pointer_id`。
    pub fn has_pointer_capture(&self, pointer_id: i32) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe {
                sys::kui_element_has_pointer_capture(self.raw.as_ptr(), pointer_id, &mut value)
            },
            "Element::has_pointer_capture",
        )?;
        Ok(value != 0)
    }
}

impl Document {
    /// 带有全部给定类名（空格分隔）的元素。
    pub fn get_elements_by_class_name(&self, class_name: &str) -> Result<Vec<Element<'_>>> {
        let (raw, len) = bytes(class_name);
        crate::dom::collect_elements(
            self,
            "Document::get_elements_by_class_name",
            |buf, cap, out| unsafe {
                sys::kui_document_get_elements_by_class_name(
                    self.raw.as_ptr(),
                    raw,
                    len,
                    buf,
                    cap,
                    out,
                )
            },
        )
    }
    /// 指定标签名的元素。
    pub fn get_elements_by_tag_name(&self, tag_name: &str) -> Result<Vec<Element<'_>>> {
        let (raw, len) = bytes(tag_name);
        crate::dom::collect_elements(
            self,
            "Document::get_elements_by_tag_name",
            |buf, cap, out| unsafe {
                sys::kui_document_get_elements_by_tag_name(
                    self.raw.as_ptr(),
                    raw,
                    len,
                    buf,
                    cap,
                    out,
                )
            },
        )
    }
    /// 视口坐标处最上层的元素。
    pub fn element_from_point(&self, x: f32, y: f32) -> Result<Option<Element<'_>>> {
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::kui_document_element_from_point(self.raw.as_ptr(), x, y, &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Document::element_from_point")?;
        Element::from_raw(raw, self).map(Some)
    }
}

impl Platform {
    /// 模态消息框，等待用户回答。
    pub fn message_box(
        &self,
        title: &str,
        message: &str,
        kind: MessageBoxType,
        buttons: MessageBoxButtons,
    ) -> Result<MessageBoxResult> {
        let (raw_title, title_len) = bytes(title);
        let (raw_message, message_len) = bytes(message);
        let mut result = 0;
        check(
            unsafe {
                sys::kui_platform_message_box(
                    raw_title,
                    title_len,
                    raw_message,
                    message_len,
                    kind as i32,
                    buttons as i32,
                    &mut result,
                )
            },
            "Platform::message_box",
        )?;
        Ok(match result {
            1 => MessageBoxResult::Cancel,
            2 => MessageBoxResult::Yes,
            3 => MessageBoxResult::No,
            _ => MessageBoxResult::Ok,
        })
    }
    /// 用系统默认程序打开网址。
    pub fn open_url(&self, url: &str) -> Result<()> {
        let (raw, len) = bytes(url);
        check(
            unsafe { sys::kui_platform_open_url(raw, len) },
            "Platform::open_url",
        )
    }
    /// 单调时钟毫秒数（起点任意，与平台定时器同一时钟）。
    pub fn now(&self) -> Result<f64> {
        let mut value = 0.0f64;
        check(
            unsafe { sys::kui_platform_now(&mut value) },
            "Platform::now",
        )?;
        Ok(value)
    }
    /// 包含虚拟屏幕坐标 `(x, y)` 的显示器。
    pub fn monitor_from_point(&self, x: f32, y: f32) -> Result<MonitorInfo> {
        let mut raw = crate::system::empty_monitor();
        let name = read_string("Platform::monitor_from_point", |buf, cap, out| unsafe {
            sys::kui_platform_get_monitor_from_point(x, y, &mut raw, buf, cap, out)
        })?;
        Ok(crate::system::monitor_from(&raw, name))
    }
    /// 窗口所在的显示器。
    pub fn monitor_from_window(&self, window: &Window) -> Result<MonitorInfo> {
        let handle: *mut c_void = window.native_handle()?;
        let mut raw = crate::system::empty_monitor();
        let name = read_string("Platform::monitor_from_window", |buf, cap, out| unsafe {
            sys::kui_platform_get_monitor_from_window(handle, &mut raw, buf, cap, out)
        })?;
        Ok(crate::system::monitor_from(&raw, name))
    }
}
