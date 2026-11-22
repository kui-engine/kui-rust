//! 应用框架（C ABI 第 11 节，3.61.8）：路由、数据绑定、动画状态、Mutation / Resize /
//! Intersection Observer。全部只能在 UI 线程使用，回调也在 UI 线程执行。
//!
//! 回调闭包放在线程局部的回调表里，KUI 用 `release_fn` 通知何时不再引用它们（移除、替换、
//! 文档销毁或最后一个引用释放）；`release_fn` 可能在文档析构途中到来，闭包因此推迟到下一个
//! 安全点才 drop。回调参数里的元素只在回调期间借用。

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::rc::Rc;

use crate::{bytes, callbacks, check, error, read_string, sys, Document, Element, Rect, Result};

/// 回调表里的项不属于任何窗口 / 元素：用一个固定的所有者编号（与 TIMER_OWNER 不同即可）。
const FRAMEWORK_OWNER: usize = 2;

pub(crate) unsafe extern "C" fn release_token(user_data: *mut c_void) {
    callbacks::release_deferred(user_data as usize);
}

pub(crate) fn register<T: 'static>(value: T) -> usize {
    callbacks::drain_released();
    callbacks::register(value, false, FRAMEWORK_OWNER, "framework")
}

fn text(raw: *const c_char, len: usize) -> String {
    if raw.is_null() || len == 0 {
        return String::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(raw.cast::<u8>(), len) };
    String::from_utf8_lossy(slice).into_owned()
}

/// 给回调期间借来的文档指针加一个引用，交给安全类型。
fn borrow_document(raw: *mut sys::KuiDocument) -> Option<Document> {
    if raw.is_null() || unsafe { sys::kui_document_retain(raw) } != sys::KUI_OK {
        return None;
    }
    Document::from_raw(raw).ok()
}

/// 给回调期间借来的元素指针加一个引用（回调返回后随 `Element` 释放）。
fn borrow_element<'doc>(
    raw: *mut sys::KuiElement,
    document: &'doc Document,
) -> Option<Element<'doc>> {
    if raw.is_null() || unsafe { sys::kui_element_retain(raw) } != sys::KUI_OK {
        return None;
    }
    Element::from_raw(raw, document).ok()
}

// ---------------------------------------------------------------------------
// 路由
// ---------------------------------------------------------------------------

/// 一次导航的目标或当前路由。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RouteInfo {
    /// 完整路径（含查询串），例如 `/users/42?tab=1`。
    pub path: String,
    /// 匹配到的模式；没有匹配时为空。
    pub pattern: String,
    /// `?` 之后的部分。
    pub query: String,
    /// `:name` 段。
    pub params: HashMap<String, String>,
    /// 查询参数。
    pub query_params: HashMap<String, String>,
}

fn route_info(raw: *const sys::KuiCRouteInfo) -> RouteInfo {
    let Some(raw) = (unsafe { raw.as_ref() }) else {
        return RouteInfo::default();
    };
    let pairs = |items: *const sys::KuiCKeyValue, count: usize| {
        let mut map = HashMap::new();
        if !items.is_null() {
            for item in unsafe { std::slice::from_raw_parts(items, count) } {
                map.insert(
                    text(item.key, item.key_len),
                    text(item.value, item.value_len),
                );
            }
        }
        map
    };
    RouteInfo {
        path: text(raw.path, raw.path_len),
        pattern: text(raw.pattern, raw.pattern_len),
        query: text(raw.query, raw.query_len),
        params: pairs(raw.params, raw.param_count),
        query_params: pairs(raw.query_params, raw.query_param_count),
    }
}

/// 守卫种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteGuardKind {
    /// 进入目标路由前。
    BeforeEnter,
    /// 离开当前路由前。
    BeforeLeave,
}

type RouteViewFn = Box<dyn for<'a> FnMut(&'a Document, &RouteInfo) -> Option<Element<'a>>>;
type RouteGuardFn = Box<dyn FnMut(&RouteInfo, &RouteInfo) -> bool>;
type RouteInfoFn = Box<dyn FnMut(&RouteInfo)>;

unsafe extern "C" fn route_view(
    document: *mut sys::KuiDocument,
    route: *const sys::KuiCRouteInfo,
    user_data: *mut c_void,
) -> *mut sys::KuiElement {
    let info = route_info(route);
    let Some(document) = borrow_document(document) else {
        return ptr::null_mut();
    };
    let mut made = ptr::null_mut();
    callbacks::invoke::<RouteViewFn>(user_data as usize, |view| {
        if let Some(element) = view(&document, &info) {
            made = element.raw.as_ptr();
            std::mem::forget(element); // 引用转交 KUI
        }
    });
    made
}

unsafe extern "C" fn route_guard(
    from: *const sys::KuiCRouteInfo,
    to: *const sys::KuiCRouteInfo,
    user_data: *mut c_void,
) -> core::ffi::c_int {
    let (from, to) = (route_info(from), route_info(to));
    let mut allow = false; // 守卫 panic 视为取消
    callbacks::invoke::<RouteGuardFn>(user_data as usize, |guard| allow = guard(&from, &to));
    allow as core::ffi::c_int
}

unsafe extern "C" fn route_navigated(route: *const sys::KuiCRouteInfo, user_data: *mut c_void) {
    let info = route_info(route);
    callbacks::invoke::<RouteInfoFn>(user_data as usize, |callback| callback(&info));
}

unsafe extern "C" fn route_current(route: *const sys::KuiCRouteInfo, user_data: *mut c_void) {
    unsafe { *(user_data as *mut RouteInfo) = route_info(route) };
}

/// 文档的路由器（每个文档一个）。
pub struct Router<'doc> {
    document: &'doc Document,
}

impl Document {
    /// 文档的路由器。
    pub fn router(&self) -> Router<'_> {
        Router { document: self }
    }
    /// 立即写入待提交的绑定更新，不等下一帧。
    pub fn flush_bindings(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_flush_bindings(self.raw.as_ptr()) },
            "Document::flush_bindings",
        )
    }
    /// 文档里正在运行的 CSS 动画与过渡数。
    pub fn animation_counts(&self) -> Result<(u32, u32)> {
        let (mut animations, mut transitions) = (0, 0);
        check(
            unsafe {
                sys::kui_document_get_animation_counts(
                    self.raw.as_ptr(),
                    &mut animations,
                    &mut transitions,
                )
            },
            "Document::animation_counts",
        )?;
        Ok((animations, transitions))
    }
}

impl Element<'_> {
    /// 是否有正在运行的 CSS 动画 / 过渡：`(animating, transitioning)`。控制动画请改 CSS
    /// （class、内联样式、animation-play-state）；`animationend` 等事件用事件监听。
    pub fn animation_state(&self) -> Result<(bool, bool)> {
        let (mut animating, mut transitioning) = (0, 0);
        check(
            unsafe {
                sys::kui_element_get_animation_state(
                    self.raw.as_ptr(),
                    &mut animating,
                    &mut transitioning,
                )
            },
            "Element::animation_state",
        )?;
        Ok((animating != 0, transitioning != 0))
    }
}

impl Router<'_> {
    fn raw(&self) -> *mut sys::KuiDocument {
        self.document.raw.as_ptr()
    }
    /// 登记一条路由。模式里用 `:name` 表示参数。`view` 生成当前路由的视图（返回 `None` 为空视图），
    /// 返回的元素交给 KUI。
    pub fn add_route<F>(&self, pattern: &str, name: &str, view: F) -> Result<()>
    where
        F: for<'a> FnMut(&'a Document, &RouteInfo) -> Option<Element<'a>> + 'static,
    {
        let token = register::<RouteViewFn>(Box::new(view));
        let (raw_pattern, pattern_len) = bytes(pattern);
        let (raw_name, name_len) = bytes(name);
        let status = unsafe {
            sys::kui_document_router_add_route(
                self.raw(),
                raw_pattern,
                pattern_len,
                raw_name,
                name_len,
                Some(route_view),
                token as *mut c_void,
                Some(release_token),
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Router::add_route")
    }
    /// 移除一条路由；没有这条时返回 `KUI_ERR_NOT_FOUND`。
    pub fn remove_route(&self, pattern: &str) -> Result<()> {
        let (raw, len) = bytes(pattern);
        let status = unsafe { sys::kui_document_router_remove_route(self.raw(), raw, len) };
        callbacks::drain_released();
        check(status, "Router::remove_route")
    }
    /// 移除全部路由。
    pub fn clear(&self) -> Result<()> {
        let status = unsafe { sys::kui_document_router_clear_routes(self.raw()) };
        callbacks::drain_released();
        check(status, "Router::clear")
    }
    /// 是否登记过这个模式。
    pub fn has_route(&self, pattern: &str) -> Result<bool> {
        let (raw, len) = bytes(pattern);
        let mut has = 0;
        check(
            unsafe { sys::kui_document_router_has_route(self.raw(), raw, len, &mut has) },
            "Router::has_route",
        )?;
        Ok(has != 0)
    }
    /// 登记的路由数。
    pub fn route_count(&self) -> Result<u32> {
        let mut count = 0;
        check(
            unsafe { sys::kui_document_router_get_route_count(self.raw(), &mut count) },
            "Router::route_count",
        )?;
        Ok(count)
    }
    /// 让 `view` 显示当前路由的视图（每次导航替换它的子节点）。页面里的 `<kui-router-view>` 会自动
    /// 登记，其他容器用本方法；换页后要为新页面的容器重新登记。
    pub fn register_view(&self, view: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_document_router_register_view(self.raw(), view.raw.as_ptr()) },
            "Router::register_view",
        )
    }
    /// 取消登记；没登记过时返回 `KUI_ERR_NOT_FOUND`。
    pub fn unregister_view(&self, view: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_document_router_unregister_view(self.raw(), view.raw.as_ptr()) },
            "Router::unregister_view",
        )
    }
    fn navigate(
        &self,
        call: unsafe extern "C" fn(
            *mut sys::KuiDocument,
            *const c_char,
            usize,
            *mut core::ffi::c_int,
        ) -> sys::KuiStatus,
        path: &str,
        context: &'static str,
    ) -> Result<bool> {
        let (raw, len) = bytes(path);
        let mut navigated = 0;
        let status = unsafe { call(self.raw(), raw, len, &mut navigated) };
        callbacks::drain_released();
        check(status, context)?;
        Ok(navigated != 0)
    }
    /// 导航到 `path` 并压入历史；守卫取消时返回 `false`。没有路由匹配也会导航（视图清空）。
    pub fn push(&self, path: &str) -> Result<bool> {
        self.navigate(sys::kui_document_router_push, path, "Router::push")
    }
    /// 导航到 `path` 并替换当前历史项。
    pub fn replace(&self, path: &str) -> Result<bool> {
        self.navigate(sys::kui_document_router_replace, path, "Router::replace")
    }
    /// 在历史里移动：-1 后退、1 前进；越界或被守卫取消时返回 `false`。
    pub fn go(&self, delta: i32) -> Result<bool> {
        let mut navigated = 0;
        let status = unsafe { sys::kui_document_router_go(self.raw(), delta, &mut navigated) };
        callbacks::drain_released();
        check(status, "Router::go")?;
        Ok(navigated != 0)
    }
    /// 等于 `go(-1)`。
    pub fn back(&self) -> Result<bool> {
        self.go(-1)
    }
    /// 等于 `go(1)`。
    pub fn forward(&self) -> Result<bool> {
        self.go(1)
    }
    /// 当前路由。
    pub fn current(&self) -> Result<RouteInfo> {
        let mut info = RouteInfo::default();
        check(
            unsafe {
                sys::kui_document_router_get_current(
                    self.raw(),
                    Some(route_current),
                    (&mut info as *mut RouteInfo).cast(),
                )
            },
            "Router::current",
        )?;
        Ok(info)
    }
    /// 当前路径；第一次导航前为空串。
    pub fn current_path(&self) -> Result<String> {
        let mut size = 0usize;
        let status = unsafe {
            sys::kui_document_router_get_current_path(self.raw(), ptr::null_mut(), 0, &mut size)
        };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(String::new());
        }
        read_string("Router::current_path", |buf, cap, out| unsafe {
            sys::kui_document_router_get_current_path(self.raw(), buf, cap, out)
        })
    }
    /// 当前路由的 `:name` 参数。
    pub fn param(&self, name: &str) -> Result<Option<String>> {
        let (raw, len) = bytes(name);
        let mut size = 0usize;
        let status = unsafe {
            sys::kui_document_router_get_param(self.raw(), raw, len, ptr::null_mut(), 0, &mut size)
        };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        read_string("Router::param", |buf, cap, out| unsafe {
            sys::kui_document_router_get_param(self.raw(), raw, len, buf, cap, out)
        })
        .map(Some)
    }
    /// `(当前历史位置, 历史长度)`；第一次导航前位置为 -1。
    pub fn history(&self) -> Result<(i32, i32)> {
        let (mut index, mut size) = (0, 0);
        check(
            unsafe { sys::kui_document_router_get_history(self.raw(), &mut index, &mut size) },
            "Router::history",
        )?;
        Ok((index, size))
    }
    /// 登记全局守卫，返回用于移除的 id。守卫返回 `false` 取消导航；不要在守卫里增删守卫。
    pub fn add_guard<F>(&self, kind: RouteGuardKind, guard: F) -> Result<u32>
    where
        F: FnMut(&RouteInfo, &RouteInfo) -> bool + 'static,
    {
        let token = register::<RouteGuardFn>(Box::new(guard));
        let raw_kind = match kind {
            RouteGuardKind::BeforeEnter => sys::KUI_C_ROUTE_GUARD_BEFORE_ENTER,
            RouteGuardKind::BeforeLeave => sys::KUI_C_ROUTE_GUARD_BEFORE_LEAVE,
        };
        let mut id = 0;
        let status = unsafe {
            sys::kui_document_router_add_guard(
                self.raw(),
                raw_kind,
                Some(route_guard),
                token as *mut c_void,
                Some(release_token),
                &mut id,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Router::add_guard")?;
        Ok(id)
    }
    /// 移除守卫（未知 id 什么也不做）。
    pub fn remove_guard(&self, id: u32) -> Result<()> {
        let status = unsafe { sys::kui_document_router_remove_guard(self.raw(), id) };
        callbacks::drain_released();
        check(status, "Router::remove_guard")
    }
    /// 每次导航成功、视图更新之后调用，返回用于移除的 id。
    pub fn on_navigated<F>(&self, callback: F) -> Result<u32>
    where
        F: FnMut(&RouteInfo) + 'static,
    {
        let token = register::<RouteInfoFn>(Box::new(callback));
        let mut id = 0;
        let status = unsafe {
            sys::kui_document_router_add_navigated_callback(
                self.raw(),
                Some(route_navigated),
                token as *mut c_void,
                Some(release_token),
                &mut id,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Router::on_navigated")?;
        Ok(id)
    }
    /// 移除 `on_navigated` 登记的回调（未知 id 什么也不做）。
    pub fn remove_navigated(&self, id: u32) -> Result<()> {
        let status = unsafe { sys::kui_document_router_remove_navigated_callback(self.raw(), id) };
        callbacks::drain_released();
        check(status, "Router::remove_navigated")
    }
}

// ---------------------------------------------------------------------------
// 数据绑定
// ---------------------------------------------------------------------------

type ChangeFn = Box<dyn FnMut()>;

unsafe extern "C" fn observable_changed(user_data: *mut c_void) {
    callbacks::invoke::<ChangeFn>(user_data as usize, |callback| callback());
}

/// 保存一个字符串值，可绑定到元素属性（`textContent`、`innerHTML`、`className`、`hidden`、
/// `disabled`、`value`、`checked`、`valueAsNumber`，其他名字按属性处理）。绑定时立即写入，之后的
/// 变化在下一帧（或 [`Document::flush_bindings`]）统一写入。drop 时解除全部绑定与订阅；它会让
/// 绑定的元素和文档一直存活到解绑。
pub struct Observable {
    raw: NonNull<sys::KuiObservable>,
    _not_send: PhantomData<Rc<()>>,
}

impl Observable {
    /// 创建初值为 `initial` 的 Observable。
    pub fn new(initial: &str) -> Result<Self> {
        let (raw, len) = bytes(initial);
        let mut out = ptr::null_mut();
        check(
            unsafe { sys::kui_observable_create(raw, len, &mut out) },
            "Observable::new",
        )?;
        Ok(Self {
            raw: NonNull::new(out)
                .ok_or_else(|| error(sys::KUI_ERR_INTERNAL, "Observable::new"))?,
            _not_send: PhantomData,
        })
    }
    /// 改值；值变了才同步通知订阅者。
    pub fn set(&self, value: &str) -> Result<()> {
        let (raw, len) = bytes(value);
        let status = unsafe { sys::kui_observable_set(self.raw.as_ptr(), raw, len) };
        callbacks::drain_released();
        check(status, "Observable::set")
    }
    /// 当前值。
    pub fn get(&self) -> Result<String> {
        read_string("Observable::get", |buf, cap, out| unsafe {
            sys::kui_observable_get(self.raw.as_ptr(), buf, cap, out)
        })
    }
    /// 每次变化后调用 `callback`，返回用于退订的 id。
    pub fn subscribe<F: FnMut() + 'static>(&self, callback: F) -> Result<u32> {
        let token = register::<ChangeFn>(Box::new(callback));
        let mut id = 0;
        let status = unsafe {
            sys::kui_observable_subscribe(
                self.raw.as_ptr(),
                Some(observable_changed),
                token as *mut c_void,
                Some(release_token),
                &mut id,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Observable::subscribe")?;
        Ok(id)
    }
    /// 退订；未知 id 返回 `KUI_ERR_NOT_FOUND`。
    pub fn unsubscribe(&self, id: u32) -> Result<()> {
        let status = unsafe { sys::kui_observable_unsubscribe(self.raw.as_ptr(), id) };
        callbacks::drain_released();
        check(status, "Observable::unsubscribe")
    }
    /// 绑定到元素属性；`two_way` 时用户输入也会写回（`value` / `checked`）。返回绑定 id。
    pub fn bind(&self, element: &Element<'_>, property: &str, two_way: bool) -> Result<u32> {
        let (raw, len) = bytes(property);
        let mut id = 0;
        check(
            unsafe {
                sys::kui_observable_bind(
                    self.raw.as_ptr(),
                    element.raw.as_ptr(),
                    raw,
                    len,
                    two_way as core::ffi::c_int,
                    &mut id,
                )
            },
            "Observable::bind",
        )?;
        Ok(id)
    }
    /// 解除一个绑定；未知 id 返回 `KUI_ERR_NOT_FOUND`。
    pub fn unbind(&self, id: u32) -> Result<()> {
        check(
            unsafe { sys::kui_observable_unbind(self.raw.as_ptr(), id) },
            "Observable::unbind",
        )
    }
    /// 解除全部绑定。
    pub fn unbind_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_observable_unbind_all(self.raw.as_ptr()) },
            "Observable::unbind_all",
        )
    }
}

impl Observable {
    /// 底层 C 句柄，给 [`crate::sys`] 里的低层函数用；它的生命周期仍由本对象管理。
    pub fn as_raw(&self) -> *mut sys::KuiObservable {
        self.raw.as_ptr()
    }
}

impl Clone for Observable {
    /// 再取一个引用（`kui_observable_retain`）；最后一个 drop 时才解除全部绑定与订阅。
    fn clone(&self) -> Self {
        unsafe {
            sys::kui_observable_retain(self.raw.as_ptr());
        }
        Self {
            raw: self.raw,
            _not_send: PhantomData,
        }
    }
}

impl Drop for Observable {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_observable_release(self.raw.as_ptr()) };
        callbacks::drain_released();
    }
}

// ---------------------------------------------------------------------------
// Observer
// ---------------------------------------------------------------------------

/// DOM 变化的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    /// 子节点增删。
    ChildList,
    /// 属性变化。
    Attributes,
    /// 文本内容变化。
    CharacterData,
}

/// 一次 DOM 变化。元素只在回调期间借用；`*_node_type` 是 DOM nodeType（0 无、1 元素、3 文本、
/// 8 注释），文本节点对应的元素字段为 `None`。`CharacterData` 的 `target` 是文本节点的父元素。
pub struct MutationRecord<'doc> {
    /// 种类。
    pub kind: MutationType,
    /// 目标元素。
    pub target: Option<Element<'doc>>,
    /// 目标节点类型。
    pub target_node_type: i32,
    /// 新增的子元素。
    pub added: Option<Element<'doc>>,
    /// 新增节点类型。
    pub added_node_type: i32,
    /// 移除的子元素。
    pub removed: Option<Element<'doc>>,
    /// 移除节点类型。
    pub removed_node_type: i32,
    /// 前一个兄弟元素。
    pub previous_sibling: Option<Element<'doc>>,
    /// 后一个兄弟元素。
    pub next_sibling: Option<Element<'doc>>,
    /// 属性名（`Attributes`）。
    pub attribute_name: String,
    /// 旧值（选项里要求时）。
    pub old_value: String,
}

/// 要观察的变化；`child_list` / `attributes` / `character_data` 至少一个。
#[derive(Debug, Default, Clone)]
pub struct MutationOptions {
    /// 子节点增删。
    pub child_list: bool,
    /// 属性变化。
    pub attributes: bool,
    /// 文本内容变化。
    pub character_data: bool,
    /// 包括后代。
    pub subtree: bool,
    /// 记录属性旧值。
    pub attribute_old_value: bool,
    /// 记录文本旧值。
    pub character_data_old_value: bool,
    /// 只观察这些属性；空 = 全部。
    pub attribute_filter: Vec<String>,
}

/// 一次尺寸变化（CSS px）。
pub struct ResizeEntry<'doc> {
    /// 目标元素。
    pub target: Option<Element<'doc>>,
    /// 内容盒宽。
    pub content_width: f32,
    /// 内容盒高。
    pub content_height: f32,
    /// 边框盒宽。
    pub border_width: f32,
    /// 边框盒高。
    pub border_height: f32,
    /// 内容盒在边框盒里的 X 偏移。
    pub content_x: f32,
    /// 内容盒在边框盒里的 Y 偏移。
    pub content_y: f32,
}

/// IntersectionObserver 的配置；`root` 为 `None` 表示视口，`thresholds` 为空等于 `[0]`。
#[derive(Default)]
pub struct IntersectionOptions<'a> {
    /// 根元素。
    pub root: Option<&'a Element<'a>>,
    /// 根边界的扩展（负值收缩）：上。
    pub margin_top: f32,
    /// 右。
    pub margin_right: f32,
    /// 下。
    pub margin_bottom: f32,
    /// 左。
    pub margin_left: f32,
    /// 触发回调的相交比例。
    pub thresholds: Vec<f32>,
}

/// 一次相交状态变化。
pub struct IntersectionEntry<'doc> {
    /// 目标元素。
    pub target: Option<Element<'doc>>,
    /// 目标边框盒（视口坐标）。
    pub bounding_client_rect: Rect,
    /// 相交部分。
    pub intersection_rect: Rect,
    /// 根边界。
    pub root_bounds: Rect,
    /// 相交比例。
    pub intersection_ratio: f32,
    /// 是否相交。
    pub is_intersecting: bool,
}

type MutationFn = Box<dyn for<'a> FnMut(&[MutationRecord<'a>])>;
type ResizeFn = Box<dyn for<'a> FnMut(&[ResizeEntry<'a>])>;
type IntersectionFn = Box<dyn for<'a> FnMut(&[IntersectionEntry<'a>])>;

struct ObserverState<F> {
    document: Document,
    callback: F,
}

fn rect(raw: sys::KuiRect) -> Rect {
    Rect {
        x: raw.x,
        y: raw.y,
        width: raw.width,
        height: raw.height,
    }
}

unsafe extern "C" fn mutation_records(
    records: *const sys::KuiCMutationRecord,
    count: usize,
    user_data: *mut c_void,
) {
    if records.is_null() {
        return;
    }
    let raw = unsafe { std::slice::from_raw_parts(records, count) };
    callbacks::invoke::<ObserverState<MutationFn>>(user_data as usize, |state| {
        let document = &state.document;
        let list: Vec<MutationRecord<'_>> = raw
            .iter()
            .map(|r| MutationRecord {
                kind: match r.type_ {
                    sys::KUI_C_MUTATION_ATTRIBUTES => MutationType::Attributes,
                    sys::KUI_C_MUTATION_CHARACTER_DATA => MutationType::CharacterData,
                    _ => MutationType::ChildList,
                },
                target: borrow_element(r.target, document),
                target_node_type: r.target_node_type,
                added: borrow_element(r.added, document),
                added_node_type: r.added_node_type,
                removed: borrow_element(r.removed, document),
                removed_node_type: r.removed_node_type,
                previous_sibling: borrow_element(r.previous_sibling, document),
                next_sibling: borrow_element(r.next_sibling, document),
                attribute_name: text(r.attribute_name, r.attribute_name_len),
                old_value: text(r.old_value, r.old_value_len),
            })
            .collect();
        (state.callback)(&list);
    });
}

unsafe extern "C" fn resize_entries(
    entries: *const sys::KuiCResizeEntry,
    count: usize,
    user_data: *mut c_void,
) {
    if entries.is_null() {
        return;
    }
    let raw = unsafe { std::slice::from_raw_parts(entries, count) };
    callbacks::invoke::<ObserverState<ResizeFn>>(user_data as usize, |state| {
        let document = &state.document;
        let list: Vec<ResizeEntry<'_>> = raw
            .iter()
            .map(|e| ResizeEntry {
                target: borrow_element(e.target, document),
                content_width: e.content_width,
                content_height: e.content_height,
                border_width: e.border_width,
                border_height: e.border_height,
                content_x: e.content_x,
                content_y: e.content_y,
            })
            .collect();
        (state.callback)(&list);
    });
}

unsafe extern "C" fn intersection_entries(
    entries: *const sys::KuiCIntersectionEntry,
    count: usize,
    user_data: *mut c_void,
) {
    if entries.is_null() {
        return;
    }
    let raw = unsafe { std::slice::from_raw_parts(entries, count) };
    callbacks::invoke::<ObserverState<IntersectionFn>>(user_data as usize, |state| {
        let document = &state.document;
        let list: Vec<IntersectionEntry<'_>> = raw
            .iter()
            .map(|e| IntersectionEntry {
                target: borrow_element(e.target, document),
                bounding_client_rect: rect(e.bounding_client_rect),
                intersection_rect: rect(e.intersection_rect),
                root_bounds: rect(e.root_bounds),
                intersection_ratio: e.intersection_ratio,
                is_intersecting: e.is_intersecting != 0,
            })
            .collect();
        (state.callback)(&list);
    });
}

/// Mutation / Resize / Intersection Observer，由 `Document::create_*_observer` 创建。被观察的元素在
/// 观察期间保持存活；drop 时断开（回调里 drop 也可以）。
pub struct Observer {
    raw: NonNull<sys::KuiObserver>,
    _not_send: PhantomData<Rc<()>>,
}

impl Document {
    fn retained(&self) -> Result<Document> {
        check(
            unsafe { sys::kui_document_retain(self.raw.as_ptr()) },
            "Document::retain",
        )?;
        Document::from_raw(self.raw.as_ptr())
    }

    fn create_observer<F: 'static>(
        &self,
        callback: F,
        create: impl FnOnce(*mut c_void, *mut *mut sys::KuiObserver) -> sys::KuiStatus,
        context: &'static str,
    ) -> Result<Observer> {
        let token = register(ObserverState {
            document: self.retained()?,
            callback,
        });
        let mut out = ptr::null_mut();
        let status = create(token as *mut c_void, &mut out);
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, context)?;
        Ok(Observer {
            raw: NonNull::new(out).ok_or_else(|| error(sys::KUI_ERR_INTERNAL, context))?,
            _not_send: PhantomData,
        })
    }

    /// 创建 MutationObserver；变化发生后同步回调。回调里的元素借用本文档。
    pub fn create_mutation_observer<F>(&self, callback: F) -> Result<Observer>
    where
        F: for<'a> FnMut(&[MutationRecord<'a>]) + 'static,
    {
        self.create_observer::<MutationFn>(
            Box::new(callback),
            |user, out| unsafe {
                sys::kui_mutation_observer_create(
                    Some(mutation_records),
                    user,
                    Some(release_token),
                    out,
                )
            },
            "Document::create_mutation_observer",
        )
    }

    /// 创建 ResizeObserver；在渲染帧排版之后回调（首次观察总会报告一次）。
    pub fn create_resize_observer<F>(&self, callback: F) -> Result<Observer>
    where
        F: for<'a> FnMut(&[ResizeEntry<'a>]) + 'static,
    {
        self.create_observer::<ResizeFn>(
            Box::new(callback),
            |user, out| unsafe {
                sys::kui_resize_observer_create(
                    Some(resize_entries),
                    user,
                    Some(release_token),
                    out,
                )
            },
            "Document::create_resize_observer",
        )
    }

    /// 创建 IntersectionObserver。
    pub fn create_intersection_observer<F>(
        &self,
        options: &IntersectionOptions<'_>,
        callback: F,
    ) -> Result<Observer>
    where
        F: for<'a> FnMut(&[IntersectionEntry<'a>]) + 'static,
    {
        let raw = sys::KuiCIntersectionOptions {
            root: options
                .root
                .map_or(ptr::null_mut(), |root| root.raw.as_ptr()),
            margin_top: options.margin_top,
            margin_right: options.margin_right,
            margin_bottom: options.margin_bottom,
            margin_left: options.margin_left,
            thresholds: if options.thresholds.is_empty() {
                ptr::null()
            } else {
                options.thresholds.as_ptr()
            },
            threshold_count: options.thresholds.len(),
        };
        self.create_observer::<IntersectionFn>(
            Box::new(callback),
            |user, out| unsafe {
                sys::kui_intersection_observer_create(
                    &raw,
                    Some(intersection_entries),
                    user,
                    Some(release_token),
                    out,
                )
            },
            "Document::create_intersection_observer",
        )
    }
}

impl Observer {
    /// 让 MutationObserver 观察 `target`；再次观察同一目标会替换选项。
    pub fn observe_mutations(&self, target: &Element<'_>, options: &MutationOptions) -> Result<()> {
        let names = options
            .attribute_filter
            .iter()
            .map(|name| {
                CString::new(name.as_str())
                    .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "Observer::observe_mutations"))
            })
            .collect::<Result<Vec<_>>>()?;
        let pointers: Vec<*const c_char> = names.iter().map(|name| name.as_ptr()).collect();
        let raw = sys::KuiCMutationOptions {
            child_list: options.child_list as core::ffi::c_int,
            attributes: options.attributes as core::ffi::c_int,
            character_data: options.character_data as core::ffi::c_int,
            subtree: options.subtree as core::ffi::c_int,
            attribute_old_value: options.attribute_old_value as core::ffi::c_int,
            character_data_old_value: options.character_data_old_value as core::ffi::c_int,
            attribute_filter: if pointers.is_empty() {
                ptr::null()
            } else {
                pointers.as_ptr()
            },
            attribute_filter_count: pointers.len(),
        };
        check(
            unsafe {
                sys::kui_mutation_observer_observe(self.raw.as_ptr(), target.raw.as_ptr(), &raw)
            },
            "Observer::observe_mutations",
        )
    }
    /// 让 Resize / Intersection Observer 观察 `target`（MutationObserver 请用 `observe_mutations`）。
    pub fn observe(&self, target: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_observer_observe(self.raw.as_ptr(), target.raw.as_ptr()) },
            "Observer::observe",
        )
    }
    /// 停止观察一个目标（MutationObserver 返回 `KUI_ERR_NOT_SUPPORTED`）。
    pub fn unobserve(&self, target: &Element<'_>) -> Result<()> {
        check(
            unsafe { sys::kui_observer_unobserve(self.raw.as_ptr(), target.raw.as_ptr()) },
            "Observer::unobserve",
        )
    }
    /// 停止观察全部目标并丢弃未投递的记录。
    pub fn disconnect(&self) -> Result<()> {
        check(
            unsafe { sys::kui_observer_disconnect(self.raw.as_ptr()) },
            "Observer::disconnect",
        )
    }
}

impl Observer {
    /// 底层 C 句柄，给 [`crate::sys`] 里的低层函数用；它的生命周期仍由本对象管理。
    pub fn as_raw(&self) -> *mut sys::KuiObserver {
        self.raw.as_ptr()
    }
}

impl Clone for Observer {
    /// 再取一个引用（`kui_observer_retain`）；最后一个 drop 时才断开观察。
    fn clone(&self) -> Self {
        unsafe {
            sys::kui_observer_retain(self.raw.as_ptr());
        }
        Self {
            raw: self.raw,
            _not_send: PhantomData,
        }
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_observer_release(self.raw.as_ptr()) };
        callbacks::drain_released();
    }
}
