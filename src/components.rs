//! 组件（C ABI 第 18 节，3.61.8）：内置自定义元素、主题、`VirtualList`、`ListView`、`TreeView`。
//!
//! 全部只在 UI 线程上执行，闭包因此不要求 `Send`。组件 drop 时移除它挂在容器上的监听（已渲染的行留在
//! 容器里）；闭包由 KUI 在不再引用时通知释放，推迟到下一个安全点 drop。

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::rc::Rc;

use crate::callbacks;
use crate::framework::{register, release_token};
use crate::{bytes, check, error, read_string, sys, Document, Element, Result};

impl Document {
    /// 注册 `<kui-dialog>`、`<kui-tabs>`、`<kui-tooltip>`、`<kui-badge>`、`<kui-tree>`、`<kui-table>`、
    /// `<kui-menu>` 等内置元素。
    pub fn register_builtin_components(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_register_builtin_components(self.raw.as_ptr()) },
            "Document::register_builtin_components",
        )
    }
}

// ---------------------------------------------------------------------------
// 主题
// ---------------------------------------------------------------------------

/// 具名的 CSS 自定义属性集合；内置 `"light"` 与 `"dark"`。
pub struct ThemeManager {
    raw: NonNull<sys::KuiThemeManager>,
    _not_send: PhantomData<Rc<()>>,
}

impl ThemeManager {
    /// 创建（已注册内置主题）。
    pub fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_theme_manager_create(&mut raw) },
            "ThemeManager::new",
        )?;
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "ThemeManager::new"))?,
            _not_send: PhantomData,
        })
    }
    /// 新增或替换主题；键是自定义属性名（如 `"--bg"`）。
    pub fn register_theme(&self, name: &str, variables: &HashMap<String, String>) -> Result<()> {
        let pairs: Vec<sys::KuiCKeyValue> = variables
            .iter()
            .map(|(key, value)| {
                let (key, key_len) = bytes(key);
                let (value, value_len) = bytes(value);
                sys::KuiCKeyValue {
                    key,
                    key_len,
                    value,
                    value_len,
                }
            })
            .collect();
        let (raw_name, len) = bytes(name);
        check(
            unsafe {
                sys::kui_theme_manager_register(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    if pairs.is_empty() {
                        ptr::null()
                    } else {
                        pairs.as_ptr()
                    },
                    pairs.len(),
                )
            },
            "ThemeManager::register_theme",
        )
    }
    /// 把主题变量设到 `<html>` 的内联样式上；未知主题返回 `KUI_ERR_NOT_FOUND`。
    pub fn apply(&self, name: &str, document: &Document) -> Result<()> {
        let (raw_name, len) = bytes(name);
        check(
            unsafe {
                sys::kui_theme_manager_apply(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    document.raw.as_ptr(),
                )
            },
            "ThemeManager::apply",
        )
    }
    /// 最近一次应用的主题；还没应用过为空。
    pub fn active_theme(&self) -> Result<String> {
        read_string("ThemeManager::active_theme", |buf, cap, out| unsafe {
            sys::kui_theme_manager_get_active(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 已注册的主题名。
    pub fn theme_names(&self) -> Result<Vec<String>> {
        let mut count = 0usize;
        check(
            unsafe { sys::kui_theme_manager_get_theme_count(self.raw.as_ptr(), &mut count) },
            "ThemeManager::theme_names",
        )?;
        (0..count)
            .map(|index| {
                read_string("ThemeManager::theme_names", |buf, cap, out| unsafe {
                    sys::kui_theme_manager_get_theme_name(self.raw.as_ptr(), index, buf, cap, out)
                })
            })
            .collect()
    }
    /// 主题的变量；未知主题返回 `KUI_ERR_NOT_FOUND`。
    pub fn theme(&self, name: &str) -> Result<HashMap<String, String>> {
        let (raw_name, len) = bytes(name);
        let mut count = 0usize;
        check(
            unsafe {
                sys::kui_theme_manager_get_variable_count(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    &mut count,
                )
            },
            "ThemeManager::theme",
        )?;
        let mut result = HashMap::with_capacity(count);
        for index in 0..count {
            let (mut key_len, mut value_len) = (0usize, 0usize);
            let _ = unsafe {
                sys::kui_theme_manager_get_variable(
                    self.raw.as_ptr(),
                    raw_name,
                    len,
                    index,
                    ptr::null_mut(),
                    0,
                    &mut key_len,
                    ptr::null_mut(),
                    0,
                    &mut value_len,
                )
            };
            let mut key = vec![0u8; key_len + 1];
            let mut value = vec![0u8; value_len + 1];
            check(
                unsafe {
                    sys::kui_theme_manager_get_variable(
                        self.raw.as_ptr(),
                        raw_name,
                        len,
                        index,
                        key.as_mut_ptr().cast(),
                        key.len(),
                        &mut key_len,
                        value.as_mut_ptr().cast(),
                        value.len(),
                        &mut value_len,
                    )
                },
                "ThemeManager::theme",
            )?;
            key.truncate(key_len);
            value.truncate(value_len);
            result.insert(
                String::from_utf8_lossy(&key).into_owned(),
                String::from_utf8_lossy(&value).into_owned(),
            );
        }
        Ok(result)
    }
}

impl Drop for ThemeManager {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_theme_manager_release(self.raw.as_ptr()) };
    }
}

// ---------------------------------------------------------------------------
// 共用
// ---------------------------------------------------------------------------

/// 最小滚动单位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollUnit {
    /// 按像素（默认）。
    #[default]
    Pixel,
    /// 对齐到行边界。
    Line,
}

impl ScrollUnit {
    fn raw(self) -> sys::KuiCScrollUnit {
        match self {
            ScrollUnit::Pixel => sys::KUI_C_SCROLL_PIXEL,
            ScrollUnit::Line => sys::KUI_C_SCROLL_LINE,
        }
    }
}

/// 虚拟列表的当前状态：已渲染的行是 `visible_start..visible_end`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualListState {
    /// 第一个已渲染的行。
    pub visible_start: u32,
    /// 最后一个已渲染的行之后。
    pub visible_end: u32,
    /// 总行数。
    pub total_items: u32,
    /// 回收池里的元素数。
    pub recycle_pool_size: u32,
    /// 键盘焦点行（-1 为没有）。
    pub focused_index: i32,
    /// 总高度。
    pub total_height: f32,
    /// 固定行高。
    pub item_height: f32,
    /// 滚动单位。
    pub scroll_unit: ScrollUnit,
}

fn state_from(raw: &sys::KuiCVirtualListState) -> VirtualListState {
    VirtualListState {
        visible_start: raw.visible_start,
        visible_end: raw.visible_end,
        total_items: raw.total_items,
        recycle_pool_size: raw.recycle_pool_size,
        focused_index: raw.focused_index,
        total_height: raw.total_height,
        item_height: raw.item_height,
        scroll_unit: if raw.scroll_unit == sys::KUI_C_SCROLL_LINE {
            ScrollUnit::Line
        } else {
            ScrollUnit::Pixel
        },
    }
}

fn empty_state() -> sys::KuiCVirtualListState {
    unsafe { std::mem::zeroed() }
}

type IndexFn = Box<dyn FnMut(u32)>;

fn color(value: &str, context: &'static str) -> Result<Option<CString>> {
    if value.is_empty() {
        Ok(None)
    } else {
        CString::new(value)
            .map(Some)
            .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, context))
    }
}

fn color_ptr(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(ptr::null(), |c| c.as_ptr())
}

fn read_indices(
    context: &'static str,
    call: impl Fn(*mut u32, usize, *mut usize) -> sys::KuiStatus,
) -> Result<Vec<u32>> {
    let mut count = 0usize;
    check(call(ptr::null_mut(), 0, &mut count), context)?;
    let mut values = vec![0u32; count];
    if count > 0 {
        check(call(values.as_mut_ptr(), values.len(), &mut count), context)?;
        values.truncate(count);
    }
    Ok(values)
}

// ---------------------------------------------------------------------------
// VirtualList
// ---------------------------------------------------------------------------

/// 渲染一行：返回新元素（`create_element` 等），KUI 接管它。
pub type RenderItemFn = Box<dyn for<'a> FnMut(&'a Document, u32) -> Option<Element<'a>>>;

/// 虚拟列表配置；`render_item` 必填。
#[derive(Default)]
pub struct VirtualListConfig {
    /// 总行数。
    pub total_items: u32,
    /// 固定行高（px），0 为 32。
    pub item_height: f32,
    /// 视口上下多渲染的行数。
    pub overscan: u32,
    /// 回收池上限，0 为自动。
    pub recycle_pool_max: u32,
    /// 滚动单位。
    pub scroll_unit: ScrollUnit,
    /// 方向键 / Home / End / PageUp / PageDown / Enter。
    pub enable_keyboard: bool,
    /// 渲染一行。
    pub render_item: Option<RenderItemFn>,
    /// 逐行高度；不设时每行 `item_height`。
    pub height_for_item: Option<Box<dyn FnMut(u32) -> f32>>,
    /// 键盘焦点变化：`(旧, 新)`。
    pub on_focus_change: Option<Box<dyn FnMut(i32, i32)>>,
    /// 焦点行上按 Enter / Space。
    pub on_activate: Option<IndexFn>,
}

struct VirtualListCallbacks {
    document: Document,
    render: RenderItemFn,
    height: Option<Box<dyn FnMut(u32) -> f32>>,
    focus: Option<Box<dyn FnMut(i32, i32)>>,
    activate: Option<IndexFn>,
}

unsafe extern "C" fn vlist_render(
    _: *mut sys::KuiDocument,
    index: u32,
    user_data: *mut c_void,
) -> *mut sys::KuiElement {
    let mut result = ptr::null_mut();
    callbacks::invoke::<VirtualListCallbacks>(user_data as usize, |state| {
        if let Some(element) = (state.render)(&state.document, index) {
            result = element.raw.as_ptr();
            std::mem::forget(element); // KUI 接管这个引用
        }
    });
    result
}

unsafe extern "C" fn vlist_height(index: u32, user_data: *mut c_void) -> f32 {
    let mut result = 0.0;
    callbacks::invoke::<VirtualListCallbacks>(user_data as usize, |state| {
        if let Some(height) = state.height.as_mut() {
            result = height(index);
        }
    });
    result
}

unsafe extern "C" fn vlist_focus(old_index: i32, new_index: i32, user_data: *mut c_void) {
    callbacks::invoke::<VirtualListCallbacks>(user_data as usize, |state| {
        if let Some(focus) = state.focus.as_mut() {
            focus(old_index, new_index);
        }
    });
}

unsafe extern "C" fn vlist_activate(index: u32, user_data: *mut c_void) {
    callbacks::invoke::<VirtualListCallbacks>(user_data as usize, |state| {
        if let Some(activate) = state.activate.as_mut() {
            activate(index);
        }
    });
}

/// 只把视口里的行（加上 overscan）放进 DOM 的列表。
pub struct VirtualList {
    raw: NonNull<sys::KuiVirtualList>,
    _not_send: PhantomData<Rc<()>>,
}

impl VirtualList {
    /// 把 `container` 设为虚拟滚动容器（会改它的 position / overflow），返回前先渲染首屏。
    pub fn new(container: &Element<'_>, config: VirtualListConfig) -> Result<Self> {
        let Some(render) = config.render_item else {
            return Err(error(sys::KUI_ERR_INVALID_ARG, "VirtualList::new"));
        };
        let has_height = config.height_for_item.is_some();
        let has_focus = config.on_focus_change.is_some();
        let has_activate = config.on_activate.is_some();
        let token = register(VirtualListCallbacks {
            document: container.document.clone(),
            render,
            height: config.height_for_item,
            focus: config.on_focus_change,
            activate: config.on_activate,
        });
        let raw_config = sys::KuiCVirtualListConfig {
            total_items: config.total_items,
            item_height: config.item_height,
            overscan: config.overscan,
            recycle_pool_max: config.recycle_pool_max,
            scroll_unit: config.scroll_unit.raw(),
            enable_keyboard: i32::from(config.enable_keyboard),
            render_item: Some(vlist_render),
            height_for_item: has_height.then_some(vlist_height as _),
            on_focus_change: has_focus.then_some(vlist_focus as _),
            on_activate: has_activate.then_some(vlist_activate as _),
            user_data: token as *mut c_void,
            release_fn: Some(release_token),
        };
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::kui_virtual_list_create(container.raw.as_ptr(), &raw_config, &mut raw) };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "VirtualList::new")?;
        Ok(Self {
            raw: NonNull::new(raw)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "VirtualList::new"))?,
            _not_send: PhantomData,
        })
    }
    fn p(&self) -> *mut sys::KuiVirtualList {
        self.raw.as_ptr()
    }
    /// 当前状态。
    pub fn state(&self) -> Result<VirtualListState> {
        let mut raw = empty_state();
        check(
            unsafe { sys::kui_virtual_list_get_state(self.p(), &mut raw) },
            "VirtualList::state",
        )?;
        Ok(state_from(&raw))
    }
    /// 改总行数。
    pub fn set_total_items(&self, count: u32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_set_total_items(self.p(), count) },
            "VirtualList::set_total_items",
        )
    }
    /// 在 `index` 处插入行。
    pub fn insert_items(&self, index: u32, count: u32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_insert_items(self.p(), index, count) },
            "VirtualList::insert_items",
        )
    }
    /// 删除行；越界返回 `KUI_ERR_INVALID_ARG`。
    pub fn remove_items(&self, index: u32, count: u32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_remove_items(self.p(), index, count) },
            "VirtualList::remove_items",
        )
    }
    /// 重新渲染可见的行（数据变了之后）。
    pub fn refresh(&self) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_refresh(self.p()) },
            "VirtualList::refresh",
        )
    }
    /// 滚到某行。
    pub fn scroll_to_index(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_scroll_to_index(self.p(), index) },
            "VirtualList::scroll_to_index",
        )
    }
    /// 滚到顶。
    pub fn scroll_to_top(&self) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_scroll_to_top(self.p()) },
            "VirtualList::scroll_to_top",
        )
    }
    /// 滚到底。
    pub fn scroll_to_bottom(&self) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_scroll_to_bottom(self.p()) },
            "VirtualList::scroll_to_bottom",
        )
    }
    /// 改滚动单位。
    pub fn set_scroll_unit(&self, unit: ScrollUnit) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_set_scroll_unit(self.p(), unit.raw()) },
            "VirtualList::set_scroll_unit",
        )
    }
    /// 设键盘焦点行（越界会被夹住，-1 清除），变化时调用 `on_focus_change`。
    pub fn set_focused_index(&self, index: i32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_set_focused_index(self.p(), index) },
            "VirtualList::set_focused_index",
        )
    }
    /// 相对移动焦点。
    pub fn move_focus(&self, delta: i32) -> Result<()> {
        check(
            unsafe { sys::kui_virtual_list_move_focus(self.p(), delta) },
            "VirtualList::move_focus",
        )
    }
}

impl Drop for VirtualList {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_virtual_list_release(self.raw.as_ptr()) };
    }
}

// ---------------------------------------------------------------------------
// ListView
// ---------------------------------------------------------------------------

/// 选择模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// 不可选。
    None,
    /// 单选（默认）。
    #[default]
    Single,
    /// 多选。
    Multi,
}

/// 列表的一行。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListViewItem {
    /// 主文字。
    pub text: String,
    /// 第二行（可空）。
    pub subtitle: String,
    /// 图标文字（emoji 或字符，可空）。
    pub icon: String,
    /// 禁用。
    pub disabled: bool,
}

/// 列表配置；`item_provider` 必填，颜色为空时用默认（深色主题）。
#[derive(Default)]
pub struct ListViewConfig {
    /// 行数。
    pub item_count: u32,
    /// 行高（px），0 为 36。
    pub item_height: f32,
    /// 视口上下多渲染的行数。
    pub overscan: u32,
    /// 滚动单位。
    pub scroll_unit: ScrollUnit,
    /// 选择模式。
    pub selection_mode: SelectionMode,
    /// 行背景。
    pub item_bg: String,
    /// 悬停背景。
    pub item_hover_bg: String,
    /// 选中背景。
    pub item_selected_bg: String,
    /// 文字颜色。
    pub item_text_color: String,
    /// 第二行颜色。
    pub item_sub_color: String,
    /// 一行的数据。
    pub item_provider: Option<Box<dyn FnMut(u32) -> ListViewItem>>,
    /// 点击 / 键盘选择。
    pub on_select: Option<IndexFn>,
    /// 双击 / Enter。
    pub on_activate: Option<IndexFn>,
}

struct ListViewCallbacks {
    provider: Box<dyn FnMut(u32) -> ListViewItem>,
    select: Option<IndexFn>,
    activate: Option<IndexFn>,
    // 上一次的行数据：KUI 在回调返回后立刻复制。
    last: ListViewItem,
}

unsafe extern "C" fn list_item(
    index: u32,
    out: *mut sys::KuiCListViewItem,
    user_data: *mut c_void,
) {
    let out = unsafe { &mut *out };
    callbacks::invoke::<ListViewCallbacks>(user_data as usize, |state| {
        state.last = (state.provider)(index);
        (out.text, out.text_len) = bytes(&state.last.text);
        (out.subtitle, out.subtitle_len) = bytes(&state.last.subtitle);
        (out.icon, out.icon_len) = bytes(&state.last.icon);
        out.disabled = i32::from(state.last.disabled);
    });
}

unsafe extern "C" fn list_select(index: u32, user_data: *mut c_void) {
    callbacks::invoke::<ListViewCallbacks>(user_data as usize, |state| {
        if let Some(select) = state.select.as_mut() {
            select(index);
        }
    });
}

unsafe extern "C" fn list_activate(index: u32, user_data: *mut c_void) {
    callbacks::invoke::<ListViewCallbacks>(user_data as usize, |state| {
        if let Some(activate) = state.activate.as_mut() {
            activate(index);
        }
    });
}

/// 建在虚拟列表上的可选择列表。
pub struct ListView {
    raw: NonNull<sys::KuiListView>,
    _not_send: PhantomData<Rc<()>>,
}

impl ListView {
    /// 在 `container` 里建列表。
    pub fn new(container: &Element<'_>, config: ListViewConfig) -> Result<Self> {
        const CONTEXT: &str = "ListView::new";
        let Some(provider) = config.item_provider else {
            return Err(error(sys::KUI_ERR_INVALID_ARG, CONTEXT));
        };
        let colors = [
            color(&config.item_bg, CONTEXT)?,
            color(&config.item_hover_bg, CONTEXT)?,
            color(&config.item_selected_bg, CONTEXT)?,
            color(&config.item_text_color, CONTEXT)?,
            color(&config.item_sub_color, CONTEXT)?,
        ];
        let has_select = config.on_select.is_some();
        let has_activate = config.on_activate.is_some();
        let token = register(ListViewCallbacks {
            provider,
            select: config.on_select,
            activate: config.on_activate,
            last: ListViewItem::default(),
        });
        let raw_config = sys::KuiCListViewConfig {
            item_count: config.item_count,
            item_height: config.item_height,
            overscan: config.overscan,
            scroll_unit: config.scroll_unit.raw(),
            selection_mode: match config.selection_mode {
                SelectionMode::None => sys::KUI_C_SELECTION_NONE,
                SelectionMode::Single => sys::KUI_C_SELECTION_SINGLE,
                SelectionMode::Multi => sys::KUI_C_SELECTION_MULTI,
            },
            item_bg: color_ptr(&colors[0]),
            item_hover_bg: color_ptr(&colors[1]),
            item_selected_bg: color_ptr(&colors[2]),
            item_text_color: color_ptr(&colors[3]),
            item_sub_color: color_ptr(&colors[4]),
            item_provider: Some(list_item),
            on_select: has_select.then_some(list_select as _),
            on_activate: has_activate.then_some(list_activate as _),
            user_data: token as *mut c_void,
            release_fn: Some(release_token),
        };
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sys::kui_list_view_create(container.raw.as_ptr(), &raw_config, &mut raw) };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, CONTEXT)?;
        Ok(Self {
            raw: NonNull::new(raw).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, CONTEXT))?,
            _not_send: PhantomData,
        })
    }
    fn p(&self) -> *mut sys::KuiListView {
        self.raw.as_ptr()
    }
    /// 选中一行（单选模式会替换选择）。
    pub fn select(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_select(self.p(), index) },
            "ListView::select",
        )
    }
    /// 取消选中一行。
    pub fn deselect(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_deselect(self.p(), index) },
            "ListView::deselect",
        )
    }
    /// 清空选择。
    pub fn clear_selection(&self) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_clear_selection(self.p()) },
            "ListView::clear_selection",
        )
    }
    /// 把 `start..end` 加进选择。
    pub fn select_range(&self, start: u32, end: u32) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_select_range(self.p(), start, end) },
            "ListView::select_range",
        )
    }
    /// 是否选中。
    pub fn is_selected(&self, index: u32) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_list_view_is_selected(self.p(), index, &mut value) },
            "ListView::is_selected",
        )?;
        Ok(value != 0)
    }
    /// 选中的行，升序。
    pub fn selected(&self) -> Result<Vec<u32>> {
        read_indices("ListView::selected", |buf, cap, out| unsafe {
            sys::kui_list_view_get_selected(self.p(), buf, cap, out)
        })
    }
    /// 焦点行（-1 为没有）。
    pub fn focused_index(&self) -> Result<i32> {
        let mut value = -1;
        check(
            unsafe { sys::kui_list_view_get_focused_index(self.p(), &mut value) },
            "ListView::focused_index",
        )?;
        Ok(value)
    }
    /// 改行数。
    pub fn set_item_count(&self, count: u32) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_set_item_count(self.p(), count) },
            "ListView::set_item_count",
        )
    }
    /// 重新渲染可见的行。
    pub fn refresh(&self) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_refresh(self.p()) },
            "ListView::refresh",
        )
    }
    /// 滚到某行。
    pub fn scroll_to_index(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_scroll_to_index(self.p(), index) },
            "ListView::scroll_to_index",
        )
    }
    /// 滚到顶。
    pub fn scroll_to_top(&self) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_scroll_to_top(self.p()) },
            "ListView::scroll_to_top",
        )
    }
    /// 滚到底。
    pub fn scroll_to_bottom(&self) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_scroll_to_bottom(self.p()) },
            "ListView::scroll_to_bottom",
        )
    }
    /// 改滚动单位。
    pub fn set_scroll_unit(&self, unit: ScrollUnit) -> Result<()> {
        check(
            unsafe { sys::kui_list_view_set_scroll_unit(self.p(), unit.raw()) },
            "ListView::set_scroll_unit",
        )
    }
    /// 列表底下虚拟列表的状态。
    pub fn state(&self) -> Result<VirtualListState> {
        let mut raw = empty_state();
        check(
            unsafe { sys::kui_list_view_get_state(self.p(), &mut raw) },
            "ListView::state",
        )?;
        Ok(state_from(&raw))
    }
}

impl Drop for ListView {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_list_view_release(self.raw.as_ptr()) };
    }
}

// ---------------------------------------------------------------------------
// TreeView
// ---------------------------------------------------------------------------

/// [`TreeView::update_node`] 的改动：为 `None` 的字段保持原值。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeNodeUpdate<'a> {
    /// 新文字；`None` 保持原值。
    pub text: Option<&'a str>,
    /// 新图标；`None` 保持原值。
    pub icon: Option<&'a str>,
    /// 新的禁用状态；`None` 保持原值。
    pub disabled: Option<bool>,
}

/// 树的一个节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// 文字。
    pub text: String,
    /// 图标文字。
    pub icon: String,
    /// 是否展开。
    pub expanded: bool,
    /// 是否禁用。
    pub disabled: bool,
    /// 父节点（-1 为根）。
    pub parent: i32,
    /// 子节点。
    pub children: Vec<u32>,
}

/// 树配置；颜色为空时用默认。
#[derive(Default)]
pub struct TreeViewConfig {
    /// 行高（px），0 为 28。
    pub item_height: f32,
    /// 每层缩进（px），0 为 20。
    pub indent_width: f32,
    /// 视口上下多渲染的行数。
    pub overscan: u32,
    /// 滚动单位。
    pub scroll_unit: ScrollUnit,
    /// 行背景。
    pub item_bg: String,
    /// 悬停背景。
    pub item_hover_bg: String,
    /// 选中背景。
    pub item_selected_bg: String,
    /// 文字颜色。
    pub item_text_color: String,
    /// 多选（Ctrl+单击切换，Shift+单击选区间）。
    pub multi_select: bool,
    /// 选中节点。
    pub on_select: Option<IndexFn>,
    /// 展开 / 收起：`(节点, 是否展开)`。
    pub on_toggle: Option<Box<dyn FnMut(u32, bool)>>,
    /// 多选集合变化（升序）。
    pub on_selection_changed: Option<Box<dyn FnMut(&[u32])>>,
}

struct TreeViewCallbacks {
    select: Option<IndexFn>,
    toggle: Option<Box<dyn FnMut(u32, bool)>>,
    selection: Option<Box<dyn FnMut(&[u32])>>,
}

unsafe extern "C" fn tree_select(index: u32, user_data: *mut c_void) {
    callbacks::invoke::<TreeViewCallbacks>(user_data as usize, |state| {
        if let Some(select) = state.select.as_mut() {
            select(index);
        }
    });
}

unsafe extern "C" fn tree_toggle(index: u32, expanded: i32, user_data: *mut c_void) {
    callbacks::invoke::<TreeViewCallbacks>(user_data as usize, |state| {
        if let Some(toggle) = state.toggle.as_mut() {
            toggle(index, expanded != 0);
        }
    });
}

unsafe extern "C" fn tree_selection(selected: *const u32, count: usize, user_data: *mut c_void) {
    let values = if selected.is_null() || count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(selected, count) }
    };
    callbacks::invoke::<TreeViewCallbacks>(user_data as usize, |state| {
        if let Some(selection) = state.selection.as_mut() {
            selection(values);
        }
    });
}

/// 建在虚拟列表上的可展开树。先加节点再 [`TreeView::setup`]（之后加的节点在 `refresh` 后出现）。
pub struct TreeView {
    raw: NonNull<sys::KuiTreeView>,
    _not_send: PhantomData<Rc<()>>,
}

impl TreeView {
    /// 空树。
    pub fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        check(
            unsafe { sys::kui_tree_view_create(&mut raw) },
            "TreeView::new",
        )?;
        Ok(Self {
            raw: NonNull::new(raw).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "TreeView::new"))?,
            _not_send: PhantomData,
        })
    }
    fn p(&self) -> *mut sys::KuiTreeView {
        self.raw.as_ptr()
    }
    /// 加根节点，返回节点编号。
    pub fn add_node(&self, text: &str, icon: &str) -> Result<u32> {
        let (raw_text, text_len) = bytes(text);
        let (raw_icon, icon_len) = bytes(icon);
        let mut index = 0;
        check(
            unsafe {
                sys::kui_tree_view_add_node(
                    self.p(),
                    raw_text,
                    text_len,
                    raw_icon,
                    icon_len,
                    &mut index,
                )
            },
            "TreeView::add_node",
        )?;
        Ok(index)
    }
    /// 加子节点；父节点不存在返回 `KUI_ERR_INVALID_ARG`。
    pub fn add_child(&self, parent: u32, text: &str, icon: &str) -> Result<u32> {
        let (raw_text, text_len) = bytes(text);
        let (raw_icon, icon_len) = bytes(icon);
        let mut index = 0;
        check(
            unsafe {
                sys::kui_tree_view_add_child(
                    self.p(),
                    parent,
                    raw_text,
                    text_len,
                    raw_icon,
                    icon_len,
                    &mut index,
                )
            },
            "TreeView::add_child",
        )?;
        Ok(index)
    }
    /// 节点数。
    pub fn node_count(&self) -> Result<u32> {
        let mut count = 0;
        check(
            unsafe { sys::kui_tree_view_get_node_count(self.p(), &mut count) },
            "TreeView::node_count",
        )?;
        Ok(count)
    }
    /// 节点数据。
    pub fn node(&self, index: u32) -> Result<TreeNode> {
        let mut raw: sys::KuiCTreeNode = unsafe { std::mem::zeroed() };
        check(
            unsafe { sys::kui_tree_view_get_node(self.p(), index, &mut raw) },
            "TreeView::node",
        )?;
        let text = |p: *const c_char, len: usize| {
            if p.is_null() || len == 0 {
                String::new()
            } else {
                String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(p.cast::<u8>(), len) })
                    .into_owned()
            }
        };
        let mut node = TreeNode {
            text: text(raw.text, raw.text_len),
            icon: text(raw.icon, raw.icon_len),
            expanded: raw.expanded != 0,
            disabled: raw.disabled != 0,
            parent: raw.parent,
            children: Vec::with_capacity(raw.child_count),
        };
        for position in 0..raw.child_count {
            let mut child = 0;
            check(
                unsafe { sys::kui_tree_view_get_node_child(self.p(), index, position, &mut child) },
                "TreeView::node",
            )?;
            node.children.push(child);
        }
        Ok(node)
    }
    /// 只改 `update` 里设了的字段（`None` 保持原值）；`refresh` 后显示。
    pub fn update_node(&self, index: u32, update: &TreeNodeUpdate<'_>) -> Result<()> {
        let disabled = match update.disabled {
            Some(value) => value,
            None => self.node(index)?.disabled,
        };
        let text = update.text.map(bytes);
        let icon = update.icon.map(bytes);
        check(
            unsafe {
                sys::kui_tree_view_set_node(
                    self.p(),
                    index,
                    text.map_or(ptr::null(), |(raw, _)| raw),
                    text.map_or(0, |(_, len)| len),
                    icon.map_or(ptr::null(), |(raw, _)| raw),
                    icon.map_or(0, |(_, len)| len),
                    i32::from(disabled),
                )
            },
            "TreeView::update_node",
        )
    }
    /// 改节点的文字、图标与禁用状态；`refresh` 后显示。要保持其中某项原值用 [`TreeView::update_node`]。
    pub fn set_node(&self, index: u32, text: &str, icon: &str, disabled: bool) -> Result<()> {
        let (raw_text, text_len) = bytes(text);
        let (raw_icon, icon_len) = bytes(icon);
        check(
            unsafe {
                sys::kui_tree_view_set_node(
                    self.p(),
                    index,
                    raw_text,
                    text_len,
                    raw_icon,
                    icon_len,
                    i32::from(disabled),
                )
            },
            "TreeView::set_node",
        )
    }
    /// 在 `container` 里显示（再次调用会替换配置与回调）。
    pub fn setup(&self, container: &Element<'_>, config: TreeViewConfig) -> Result<()> {
        const CONTEXT: &str = "TreeView::setup";
        let colors = [
            color(&config.item_bg, CONTEXT)?,
            color(&config.item_hover_bg, CONTEXT)?,
            color(&config.item_selected_bg, CONTEXT)?,
            color(&config.item_text_color, CONTEXT)?,
        ];
        let has_select = config.on_select.is_some();
        let has_toggle = config.on_toggle.is_some();
        let has_selection = config.on_selection_changed.is_some();
        let token = register(TreeViewCallbacks {
            select: config.on_select,
            toggle: config.on_toggle,
            selection: config.on_selection_changed,
        });
        let raw_config = sys::KuiCTreeViewConfig {
            item_height: config.item_height,
            indent_width: config.indent_width,
            overscan: config.overscan,
            scroll_unit: config.scroll_unit.raw(),
            item_bg: color_ptr(&colors[0]),
            item_hover_bg: color_ptr(&colors[1]),
            item_selected_bg: color_ptr(&colors[2]),
            item_text_color: color_ptr(&colors[3]),
            multi_select: i32::from(config.multi_select),
            on_select: has_select.then_some(tree_select as _),
            on_toggle: has_toggle.then_some(tree_toggle as _),
            on_selection_changed: has_selection.then_some(tree_selection as _),
            user_data: token as *mut c_void,
            release_fn: Some(release_token),
        };
        let status =
            unsafe { sys::kui_tree_view_setup(self.p(), container.raw.as_ptr(), &raw_config) };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, CONTEXT)
    }
    /// 展开。
    pub fn expand(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_expand(self.p(), index) },
            "TreeView::expand",
        )
    }
    /// 收起。
    pub fn collapse(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_collapse(self.p(), index) },
            "TreeView::collapse",
        )
    }
    /// 切换展开。
    pub fn toggle(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_toggle(self.p(), index) },
            "TreeView::toggle",
        )
    }
    /// 全部展开。
    pub fn expand_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_expand_all(self.p()) },
            "TreeView::expand_all",
        )
    }
    /// 全部收起。
    pub fn collapse_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_collapse_all(self.p()) },
            "TreeView::collapse_all",
        )
    }
    /// 选中节点。
    pub fn select(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_select(self.p(), index) },
            "TreeView::select",
        )
    }
    /// 清空选择。
    pub fn clear_selection(&self) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_clear_selection(self.p()) },
            "TreeView::clear_selection",
        )
    }
    /// 多选：切换一个节点。
    pub fn toggle_select(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_toggle_select(self.p(), index) },
            "TreeView::toggle_select",
        )
    }
    /// 多选：从锚点选到这里。
    pub fn select_range(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_select_range(self.p(), index) },
            "TreeView::select_range",
        )
    }
    /// 全选。
    pub fn select_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_select_all(self.p()) },
            "TreeView::select_all",
        )
    }
    /// 是否选中。
    pub fn is_selected(&self, index: u32) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_tree_view_is_selected(self.p(), index, &mut value) },
            "TreeView::is_selected",
        )?;
        Ok(value != 0)
    }
    /// 最近选中的节点（-1 为没有）。
    pub fn selected_index(&self) -> Result<i32> {
        let mut value = -1;
        check(
            unsafe { sys::kui_tree_view_get_selected_index(self.p(), &mut value) },
            "TreeView::selected_index",
        )?;
        Ok(value)
    }
    /// 与 `is_selected` 口径相同：多选树是选中的集合，单选树是那一个节点；升序。
    pub fn selected(&self) -> Result<Vec<u32>> {
        read_indices("TreeView::selected", |buf, cap, out| unsafe {
            sys::kui_tree_view_get_selected(self.p(), buf, cap, out)
        })
    }
    /// 重建显示的行。
    pub fn refresh(&self) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_refresh(self.p()) },
            "TreeView::refresh",
        )
    }
    /// 滚到节点。
    pub fn scroll_to_node(&self, index: u32) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_scroll_to_node(self.p(), index) },
            "TreeView::scroll_to_node",
        )
    }
    /// 改滚动单位。
    pub fn set_scroll_unit(&self, unit: ScrollUnit) -> Result<()> {
        check(
            unsafe { sys::kui_tree_view_set_scroll_unit(self.p(), unit.raw()) },
            "TreeView::set_scroll_unit",
        )
    }
    /// 当前显示的行数。
    pub fn visible_row_count(&self) -> Result<u32> {
        let mut count = 0;
        check(
            unsafe { sys::kui_tree_view_get_visible_row_count(self.p(), &mut count) },
            "TreeView::visible_row_count",
        )?;
        Ok(count)
    }
}

impl Drop for TreeView {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_tree_view_release(self.raw.as_ptr()) };
    }
}

// ---------------------------------------------------------------------------
// 文档样式配置（C ABI 第 19 节）
// ---------------------------------------------------------------------------

/// 文档级的呈现策略：`NativeGui`（默认）保持 KUI 的 GUI 默认值，`BrowserCompat` 用浏览器的文档默认值
/// （默认字体为 serif）；作者样式照常覆盖。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleProfile {
    /// KUI 的 GUI 默认值。
    #[default]
    NativeGui,
    /// 浏览器的文档默认值。
    BrowserCompat,
}

impl Document {
    /// 设置样式配置；在加载页面之前设，改动会重载 UA 样式表。
    pub fn set_style_profile(&self, profile: StyleProfile) -> Result<()> {
        let raw = match profile {
            StyleProfile::NativeGui => sys::KUI_C_STYLE_NATIVE_GUI,
            StyleProfile::BrowserCompat => sys::KUI_C_STYLE_BROWSER_COMPAT,
        };
        check(
            unsafe { sys::kui_document_set_style_profile(self.raw.as_ptr(), raw) },
            "Document::set_style_profile",
        )
    }
    /// 当前样式配置。
    pub fn style_profile(&self) -> Result<StyleProfile> {
        let mut raw = sys::KUI_C_STYLE_NATIVE_GUI;
        check(
            unsafe { sys::kui_document_get_style_profile(self.raw.as_ptr(), &mut raw) },
            "Document::style_profile",
        )?;
        Ok(if raw == sys::KUI_C_STYLE_BROWSER_COMPAT {
            StyleProfile::BrowserCompat
        } else {
            StyleProfile::NativeGui
        })
    }
}
