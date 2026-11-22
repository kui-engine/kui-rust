//! C ABI 第 10 节（3.61.8）：原生菜单栏、托盘、窗口级快捷键、Toast、截图。
//! 回调都在 UI 线程执行，闭包不要求 `Send`；替换、移除或窗口销毁时释放。

use std::ffi::{c_char, c_void, CString};
use std::ptr;

use crate::callbacks;
use crate::{bool_int, bytes, check, error, sys, Document, Result, Window};

/// 一个菜单项。`id` 由调用方指定，被点击时传给激活回调；`children` 非空即为子菜单。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuItem {
    /// 文字，`&` 标记助记键，例如 `"&Open"`。
    pub text: String,
    /// 只用于显示的快捷键文字，例如 `"Ctrl+O"`；真正的快捷键用 [`Window::bind_shortcut`]。
    pub shortcut: String,
    /// 激活回调收到的 id。
    pub id: u32,
    /// 分隔线。
    pub separator: bool,
    /// 禁用。
    pub disabled: bool,
    /// 可勾选。
    pub checkable: bool,
    /// 已勾选。
    pub checked: bool,
    /// 子菜单。
    pub children: Vec<MenuItem>,
}

/// 菜单栏里的一个顶层菜单。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Menu {
    /// 标题，例如 `"&File"`。
    pub text: String,
    /// 菜单项。
    pub items: Vec<MenuItem>,
}

/// 托盘图标上的用户操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// 单击。
    Click,
    /// 双击。
    DoubleClick,
    /// 右键。
    RightClick,
    /// 点击气泡通知。
    BalloonClick,
}

/// Toast 的样式级别。
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// Toast 选项（对应原生 `ToastConfig`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastOptions {
    /// 正文（必填）。
    pub message: String,
    /// 标题。
    pub title: String,
    /// 级别。
    pub level: ToastLevel,
    /// 自动关闭的毫秒数；0 一直显示到被关闭。
    pub duration_ms: i32,
    /// 位置：`top-left`、`top-center`、`top-right`、`bottom-left`、`bottom-center`、`bottom-right`。
    pub position: String,
    /// 显示关闭按钮。
    pub closable: bool,
    /// 图标字符；空时按级别。
    pub icon: String,
}
impl Default for ToastOptions {
    fn default() -> Self {
        Self {
            message: String::new(),
            title: String::new(),
            level: ToastLevel::Info,
            duration_ms: 3000,
            position: "bottom-right".into(),
            closable: true,
            icon: String::new(),
        }
    }
}

type ActivateFn = Box<dyn FnMut(u32)>;
type TrayFn = Box<dyn FnMut(TrayEvent)>;
type ShortcutFn = Box<dyn FnMut()>;

const MENU_GROUP: &str = "\u{0}menu";
const TRAY_MENU_GROUP: &str = "\u{0}tray-menu";
const TRAY_GROUP: &str = "\u{0}tray";

fn shortcut_group(id: u32) -> String {
    format!("\u{0}shortcut:{id}")
}

unsafe extern "C" fn activate_trampoline(id: u32, user_data: *mut c_void) {
    callbacks::invoke::<ActivateFn>(user_data as usize, |callback| callback(id));
}

unsafe extern "C" fn tray_trampoline(event: sys::KuiCTrayEvent, user_data: *mut c_void) {
    let event = match event {
        sys::KUI_C_TRAY_CLICK => TrayEvent::Click,
        sys::KUI_C_TRAY_DOUBLE_CLICK => TrayEvent::DoubleClick,
        sys::KUI_C_TRAY_RIGHT_CLICK => TrayEvent::RightClick,
        _ => TrayEvent::BalloonClick,
    };
    callbacks::invoke::<TrayFn>(user_data as usize, |callback| callback(event));
}

unsafe extern "C" fn shortcut_trampoline(user_data: *mut c_void) {
    callbacks::invoke::<ShortcutFn>(user_data as usize, |callback| callback());
}

/// 菜单的 C 结构与字符串：C 层会复制，调用返回后即可丢弃。
#[derive(Default)]
struct MenuArena {
    strings: Vec<CString>,
    lists: Vec<Vec<sys::KuiCMenuItem>>,
}

impl MenuArena {
    fn text(&mut self, value: &str, context: &'static str) -> Result<(*const c_char, usize)> {
        let owned = CString::new(value).map_err(|_| error(sys::KUI_ERR_INVALID_ARG, context))?;
        let raw = owned.as_ptr();
        self.strings.push(owned);
        Ok((raw, value.len()))
    }
    fn items(
        &mut self,
        items: &[MenuItem],
        context: &'static str,
    ) -> Result<*const sys::KuiCMenuItem> {
        if items.is_empty() {
            return Ok(ptr::null());
        }
        let mut list = Vec::with_capacity(items.len());
        for item in items {
            let (text, text_len) = if item.separator {
                (ptr::null(), 0)
            } else {
                self.text(&item.text, context)?
            };
            let (shortcut_label, shortcut_label_len) = if item.shortcut.is_empty() {
                (ptr::null(), 0)
            } else {
                self.text(&item.shortcut, context)?
            };
            let children = self.items(&item.children, context)?;
            list.push(sys::KuiCMenuItem {
                text,
                text_len,
                shortcut_label,
                shortcut_label_len,
                id: item.id,
                separator: bool_int(item.separator),
                disabled: bool_int(item.disabled),
                checkable: bool_int(item.checkable),
                checked: bool_int(item.checked),
                children,
                child_count: item.children.len(),
            });
        }
        // Vec 的堆缓冲在 push 进 self.lists 之后地址不变。
        let raw = list.as_ptr();
        self.lists.push(list);
        Ok(raw)
    }
}

impl Window {
    fn register_activate<F>(
        &self,
        group: &str,
        on_activate: Option<F>,
    ) -> (sys::KuiMenuActivateFn, usize)
    where
        F: FnMut(u32) + 'static,
    {
        match on_activate {
            Some(callback) => {
                let token = callbacks::register::<ActivateFn>(
                    Box::new(callback),
                    false,
                    self.owner_key(),
                    group,
                );
                (Some(activate_trampoline), token)
            }
            None => (None, 0),
        }
    }
    /// 替换窗口的原生菜单栏；`on_activate` 以菜单项 id 在 UI 线程被调用。
    pub fn set_menu_bar<F>(&self, menus: &[Menu], on_activate: Option<F>) -> Result<()>
    where
        F: FnMut(u32) + 'static,
    {
        let mut arena = MenuArena::default();
        let mut raw_menus = Vec::with_capacity(menus.len());
        for menu in menus {
            let (text, text_len) = arena.text(&menu.text, "Window::set_menu_bar")?;
            let items = arena.items(&menu.items, "Window::set_menu_bar")?;
            raw_menus.push(sys::KuiCMenu {
                text,
                text_len,
                items,
                item_count: menu.items.len(),
            });
        }
        let (fn_, token) = self.register_activate("\u{0}menu-pending", on_activate);
        let status = unsafe {
            sys::kui_window_set_menu_bar(
                self.raw.as_ptr(),
                if raw_menus.is_empty() {
                    ptr::null()
                } else {
                    raw_menus.as_ptr()
                },
                raw_menus.len(),
                fn_,
                token as *mut c_void,
            )
        };
        self.commit_callback(MENU_GROUP, token, status, "Window::set_menu_bar")
    }
    /// 新回调登记成功后再释放旧的（失败时释放新的、保留旧的）。
    fn commit_callback(
        &self,
        group: &str,
        token: usize,
        status: sys::KuiStatus,
        context: &'static str,
    ) -> Result<()> {
        if status != sys::KUI_OK {
            if token != 0 {
                callbacks::unregister(token);
            }
            return check(status, context);
        }
        callbacks::unregister_group(self.owner_key(), group);
        if token != 0 {
            callbacks::regroup(token, group);
        }
        Ok(())
    }
    /// 移除菜单栏。
    pub fn clear_menu_bar(&self) -> Result<()> {
        let status = unsafe { sys::kui_window_clear_menu_bar(self.raw.as_ptr()) };
        callbacks::unregister_group(self.owner_key(), MENU_GROUP);
        check(status, "Window::clear_menu_bar")
    }
    /// 勾选 / 取消勾选菜单项并重建菜单；id 不存在时返回 `KUI_ERR_NOT_FOUND`。
    pub fn set_menu_item_checked(&self, id: u32, checked: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_menu_set_checked(self.raw.as_ptr(), id, bool_int(checked)) },
            "Window::set_menu_item_checked",
        )
    }
    /// 启用 / 禁用菜单项并重建菜单。
    pub fn set_menu_item_enabled(&self, id: u32, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_menu_set_enabled(self.raw.as_ptr(), id, bool_int(enabled)) },
            "Window::set_menu_item_enabled",
        )
    }

    /// 创建托盘图标；`icon_path` 为空用应用图标。平台不支持时返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn create_tray(&self, icon_path: &str, tooltip: &str, visible: bool) -> Result<()> {
        let (icon, icon_len) = bytes(icon_path);
        let (tip, tip_len) = bytes(tooltip);
        check(
            unsafe {
                sys::kui_window_tray_create(
                    self.raw.as_ptr(),
                    icon,
                    icon_len,
                    tip,
                    tip_len,
                    bool_int(visible),
                )
            },
            "Window::create_tray",
        )
    }
    /// 移除托盘图标。
    pub fn destroy_tray(&self) -> Result<()> {
        let status = unsafe { sys::kui_window_tray_destroy(self.raw.as_ptr()) };
        callbacks::unregister_group(self.owner_key(), TRAY_MENU_GROUP);
        check(status, "Window::destroy_tray")
    }
    fn tray_flag(
        &self,
        call: unsafe extern "C" fn(*mut sys::KuiWindow, *mut i32) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<bool> {
        let mut value = 0;
        check(unsafe { call(self.raw.as_ptr(), &mut value) }, context)?;
        Ok(value != 0)
    }
    /// 托盘图标是否已创建。
    pub fn tray_active(&self) -> Result<bool> {
        self.tray_flag(sys::kui_window_tray_is_active, "Window::tray_active")
    }
    /// 托盘图标是否可见。
    pub fn tray_visible(&self) -> Result<bool> {
        self.tray_flag(sys::kui_window_tray_is_visible, "Window::tray_visible")
    }
    /// 显示 / 隐藏托盘图标。
    pub fn set_tray_visible(&self, visible: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_tray_set_visible(self.raw.as_ptr(), bool_int(visible)) },
            "Window::set_tray_visible",
        )
    }
    /// 托盘提示文字。
    pub fn set_tray_tooltip(&self, tooltip: &str) -> Result<()> {
        let (raw, len) = bytes(tooltip);
        check(
            unsafe { sys::kui_window_tray_set_tooltip(self.raw.as_ptr(), raw, len) },
            "Window::set_tray_tooltip",
        )
    }
    /// 托盘图标文件。
    pub fn set_tray_icon(&self, icon_path: &str) -> Result<()> {
        let (raw, len) = bytes(icon_path);
        check(
            unsafe { sys::kui_window_tray_set_icon(self.raw.as_ptr(), raw, len) },
            "Window::set_tray_icon",
        )
    }
    /// 托盘右键菜单；`on_activate` 以菜单项 id 被调用。
    pub fn set_tray_context_menu<F>(&self, items: &[MenuItem], on_activate: Option<F>) -> Result<()>
    where
        F: FnMut(u32) + 'static,
    {
        let mut arena = MenuArena::default();
        let raw = arena.items(items, "Window::set_tray_context_menu")?;
        let (fn_, token) = self.register_activate("\u{0}tray-menu-pending", on_activate);
        let status = unsafe {
            sys::kui_window_tray_set_context_menu(
                self.raw.as_ptr(),
                raw,
                items.len(),
                fn_,
                token as *mut c_void,
            )
        };
        self.commit_callback(
            TRAY_MENU_GROUP,
            token,
            status,
            "Window::set_tray_context_menu",
        )
    }
    /// 立即弹出托盘右键菜单。
    pub fn popup_tray_context_menu(&self) -> Result<()> {
        check(
            unsafe { sys::kui_window_tray_popup_context_menu(self.raw.as_ptr()) },
            "Window::popup_tray_context_menu",
        )
    }
    /// 托盘气泡通知；`icon` 0 无、1 信息、2 警告、3 错误。
    pub fn show_tray_balloon(
        &self,
        title: &str,
        message: &str,
        icon: i32,
        duration_ms: i32,
    ) -> Result<()> {
        let (raw_title, title_len) = bytes(title);
        let (raw_message, message_len) = bytes(message);
        check(
            unsafe {
                sys::kui_window_tray_show_balloon(
                    self.raw.as_ptr(),
                    raw_title,
                    title_len,
                    raw_message,
                    message_len,
                    icon,
                    duration_ms,
                )
            },
            "Window::show_tray_balloon",
        )
    }
    /// 托盘点击 / 双击 / 右键 / 气泡点击的回调（替换上一个）。
    pub fn on_tray_event<F>(&self, handler: F) -> Result<()>
    where
        F: FnMut(TrayEvent) + 'static,
    {
        let token = callbacks::register::<TrayFn>(
            Box::new(handler),
            false,
            self.owner_key(),
            "\u{0}tray-pending",
        );
        let status = unsafe {
            sys::kui_window_tray_set_event_callback(
                self.raw.as_ptr(),
                Some(tray_trampoline),
                token as *mut c_void,
            )
        };
        self.commit_callback(TRAY_GROUP, token, status, "Window::on_tray_event")
    }
    /// 清除托盘事件回调。
    pub fn clear_tray_event(&self) -> Result<()> {
        let status = unsafe {
            sys::kui_window_tray_set_event_callback(self.raw.as_ptr(), None, ptr::null_mut())
        };
        callbacks::unregister_group(self.owner_key(), TRAY_GROUP);
        check(status, "Window::clear_tray_event")
    }
    /// 像用户操作一样触发托盘事件（自动化与测试）。
    pub fn simulate_tray_event(&self, event: TrayEvent) -> Result<()> {
        let raw = match event {
            TrayEvent::Click => sys::KUI_C_TRAY_CLICK,
            TrayEvent::DoubleClick => sys::KUI_C_TRAY_DOUBLE_CLICK,
            TrayEvent::RightClick => sys::KUI_C_TRAY_RIGHT_CLICK,
            TrayEvent::BalloonClick => sys::KUI_C_TRAY_BALLOON_CLICK,
        };
        check(
            unsafe { sys::kui_window_tray_simulate_event(self.raw.as_ptr(), raw) },
            "Window::simulate_tray_event",
        )
    }

    /// 绑定窗口级快捷键（`"Ctrl+S"`、`"Ctrl+Shift+F5"`……）；同一组合再绑会替换回调并返回同一 id。
    pub fn bind_shortcut<F>(&self, shortcut: &str, callback: F) -> Result<u32>
    where
        F: FnMut() + 'static,
    {
        let token = callbacks::register::<ShortcutFn>(
            Box::new(callback),
            false,
            self.owner_key(),
            "\u{0}shortcut-pending",
        );
        let (raw, len) = bytes(shortcut);
        let mut id = 0u32;
        let status = unsafe {
            sys::kui_window_bind_shortcut(
                self.raw.as_ptr(),
                raw,
                len,
                Some(shortcut_trampoline),
                token as *mut c_void,
                &mut id,
            )
        };
        self.commit_callback(&shortcut_group(id), token, status, "Window::bind_shortcut")?;
        Ok(id)
    }
    /// 解除一个快捷键。
    pub fn unbind_shortcut(&self, id: u32) -> Result<()> {
        let status = unsafe { sys::kui_window_unbind_shortcut(self.raw.as_ptr(), id) };
        callbacks::unregister_group(self.owner_key(), &shortcut_group(id));
        check(status, "Window::unbind_shortcut")
    }
    /// 解除全部快捷键。
    pub fn unbind_all_shortcuts(&self) -> Result<()> {
        let status = unsafe { sys::kui_window_unbind_all_shortcuts(self.raw.as_ptr()) };
        callbacks::unregister_where_group_starts_with(self.owner_key(), "\u{0}shortcut:");
        check(status, "Window::unbind_all_shortcuts")
    }
    /// 整体启用 / 停用快捷键。
    pub fn set_shortcuts_enabled(&self, enabled: bool) -> Result<()> {
        check(
            unsafe { sys::kui_window_set_shortcuts_enabled(self.raw.as_ptr(), bool_int(enabled)) },
            "Window::set_shortcuts_enabled",
        )
    }
    /// 把窗口当前内容写成 PNG；写不出时返回 `false`。
    pub fn save_screenshot(&self, path: &str) -> Result<bool> {
        let (raw, len) = bytes(path);
        let mut saved = 0;
        check(
            unsafe { sys::kui_window_save_screenshot(self.raw.as_ptr(), raw, len, &mut saved) },
            "Window::save_screenshot",
        )?;
        Ok(saved != 0)
    }
}

impl Document {
    /// 显示一个 Toast，返回它的 id。
    pub fn show_toast(&self, options: &ToastOptions) -> Result<u32> {
        let (message, message_len) = bytes(&options.message);
        let (title, title_len) = bytes(&options.title);
        let (position, position_len) = bytes(&options.position);
        let (icon, icon_len) = if options.icon.is_empty() {
            (ptr::null(), 0)
        } else {
            bytes(&options.icon)
        };
        let raw = sys::KuiCToastOptions {
            message,
            message_len,
            title,
            title_len,
            level: options.level as i32,
            duration_ms: options.duration_ms,
            position,
            position_len,
            closable: bool_int(options.closable),
            icon,
            icon_len,
        };
        let mut id = 0u32;
        check(
            unsafe { sys::kui_document_show_toast(self.raw.as_ptr(), &raw, &mut id) },
            "Document::show_toast",
        )?;
        Ok(id)
    }
    /// 关闭一个 Toast。
    pub fn dismiss_toast(&self, id: u32) -> Result<()> {
        check(
            unsafe { sys::kui_document_dismiss_toast(self.raw.as_ptr(), id) },
            "Document::dismiss_toast",
        )
    }
    /// 关闭全部 Toast。
    pub fn dismiss_all_toasts(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_dismiss_all_toasts(self.raw.as_ptr()) },
            "Document::dismiss_all_toasts",
        )
    }
    /// 当前 Toast 个数（含正在离场的）。
    pub fn toast_count(&self) -> Result<i32> {
        let mut count = 0;
        check(
            unsafe { sys::kui_document_get_toast_count(self.raw.as_ptr(), &mut count) },
            "Document::toast_count",
        )?;
        Ok(count)
    }
}
