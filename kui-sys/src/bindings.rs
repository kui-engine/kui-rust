// 由 scripts/generate_rust_bindings.py 生成；禁止手改。
// lint 例外定义在 kui-sys/src/lib.rs，避免 include! 中出现非法内部属性。

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiVersion {
    pub major: core::ffi::c_int,
    pub minor: core::ffi::c_int,
    pub patch: core::ffi::c_int,
}

pub type KuiStatus = core::ffi::c_int;
pub const KUI_OK: KuiStatus = 0;
pub const KUI_ERR_INVALID_ARG: KuiStatus = 1;
pub const KUI_ERR_OUT_OF_MEMORY: KuiStatus = 2;
pub const KUI_ERR_NOT_FOUND: KuiStatus = 3;
pub const KUI_ERR_NOT_SUPPORTED: KuiStatus = 4;
pub const KUI_ERR_PARSE: KuiStatus = 5;
pub const KUI_ERR_IO: KuiStatus = 6;
pub const KUI_ERR_PLUGIN_MISSING: KuiStatus = 7;
pub const KUI_ERR_BUFFER_TOO_SMALL: KuiStatus = 8;
pub const KUI_ERR_INTERNAL: KuiStatus = 99;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiWindow {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiDocument {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiElement {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiEvent {
    _private: [u8; 0],
}

pub type KuiTaskFn = Option<unsafe extern "C" fn(*mut core::ffi::c_void)>;

pub type KuiTimerId = u64;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCMonitorInfo {
    pub bounds_x: f32,
    pub bounds_y: f32,
    pub bounds_width: f32,
    pub bounds_height: f32,
    pub work_x: f32,
    pub work_y: f32,
    pub work_width: f32,
    pub work_height: f32,
    pub scale: f32,
    pub is_primary: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiWindowProps {
    pub title: *const core::ffi::c_char,
    pub title_len: usize,
    pub width: core::ffi::c_int,
    pub height: core::ffi::c_int,
    pub min_width: core::ffi::c_int,
    pub min_height: core::ffi::c_int,
    pub max_width: core::ffi::c_int,
    pub max_height: core::ffi::c_int,
    pub no_resize: core::ffi::c_int,
    pub no_decorations: core::ffi::c_int,
    pub transparent: core::ffi::c_int,
    pub always_on_top: core::ffi::c_int,
    pub maximized: core::ffi::c_int,
    pub centered: core::ffi::c_int,
    pub frameless: core::ffi::c_int,
    pub frameless_shadow: core::ffi::c_int,
    pub opacity: f32,
}

pub type KuiWindowResizeFn = Option<unsafe extern "C" fn(u32, u32, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub type KuiEventListenerFn = Option<unsafe extern "C" fn(*mut KuiEvent, *mut core::ffi::c_void)>;

pub type KuiListenerId = u64;

pub type KuiCMouseEventType = core::ffi::c_int;
pub const KUI_C_MOUSE_MOVED: KuiCMouseEventType = 0;
pub const KUI_C_MOUSE_PRESSED: KuiCMouseEventType = 1;
pub const KUI_C_MOUSE_RELEASED: KuiCMouseEventType = 2;
pub const KUI_C_MOUSE_WHEEL: KuiCMouseEventType = 3;

pub type KuiCMouseButton = core::ffi::c_int;
pub const KUI_C_MOUSE_BUTTON_NONE: KuiCMouseButton = 0;
pub const KUI_C_MOUSE_BUTTON_LEFT: KuiCMouseButton = 1;
pub const KUI_C_MOUSE_BUTTON_MIDDLE: KuiCMouseButton = 2;
pub const KUI_C_MOUSE_BUTTON_RIGHT: KuiCMouseButton = 3;

pub type KuiCKeyEventType = core::ffi::c_int;
pub const KUI_C_KEY_DOWN: KuiCKeyEventType = 0;
pub const KUI_C_KEY_UP: KuiCKeyEventType = 1;
pub const KUI_C_KEY_CHAR: KuiCKeyEventType = 2;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiScriptEngine {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiHttpClient {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiHttpResponse {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiExtensionResult {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCFileDialogOptions {
    pub title: *const core::ffi::c_char,
    pub title_len: usize,
    pub default_path: *const core::ffi::c_char,
    pub default_path_len: usize,
    pub filters: *const *const core::ffi::c_char,
    pub filter_count: usize,
    pub allow_multiple: core::ffi::c_int,
}

pub type KuiWindowEventFn = Option<unsafe extern "C" fn(*mut KuiEvent, *mut core::ffi::c_void)>;

pub type KuiCEventTypeTag = core::ffi::c_int;
pub const KUI_C_EVENT_BASE: KuiCEventTypeTag = 0;
pub const KUI_C_EVENT_MOUSE: KuiCEventTypeTag = 1;
pub const KUI_C_EVENT_WHEEL: KuiCEventTypeTag = 2;
pub const KUI_C_EVENT_POINTER: KuiCEventTypeTag = 3;
pub const KUI_C_EVENT_KEYBOARD: KuiCEventTypeTag = 4;
pub const KUI_C_EVENT_FOCUS: KuiCEventTypeTag = 5;
pub const KUI_C_EVENT_INPUT: KuiCEventTypeTag = 6;
pub const KUI_C_EVENT_DRAG: KuiCEventTypeTag = 7;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCPositioningOptions {
    pub flip: core::ffi::c_int,
    pub clamp_to_viewport: core::ffi::c_int,
    pub offset: f32,
    pub viewport_margin: f32,
}

pub type KuiScriptErrorFn = Option<unsafe extern "C" fn(*const core::ffi::c_char, usize, *const core::ffi::c_char, usize, core::ffi::c_int, *mut core::ffi::c_void)>;

pub type KuiHttpResponseFn = Option<unsafe extern "C" fn(*mut KuiHttpResponse, *mut core::ffi::c_void)>;

pub type KuiCExtensionValueType = core::ffi::c_int;
pub const KUI_C_EXT_NIL: KuiCExtensionValueType = 0;
pub const KUI_C_EXT_INT: KuiCExtensionValueType = 1;
pub const KUI_C_EXT_FLOAT: KuiCExtensionValueType = 2;
pub const KUI_C_EXT_STRING: KuiCExtensionValueType = 3;
pub const KUI_C_EXT_BOOL: KuiCExtensionValueType = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCExtensionValue {
    pub type_: KuiCExtensionValueType,
    pub int_value: i64,
    pub float_value: f64,
    pub string_value: *const core::ffi::c_char,
    pub string_len: usize,
    pub bool_value: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCExtensionPerfStats {
    pub last_tick_ms: f64,
    pub last_paint_ms: f64,
    pub max_tick_ms: f64,
    pub max_paint_ms: f64,
    pub tick_count: u64,
    pub paint_count: u64,
    pub total_tick_ms: f64,
    pub total_paint_ms: f64,
}

pub type KuiCustomElementLifecycleFn = Option<unsafe extern "C" fn(*mut KuiElement, *mut core::ffi::c_void)>;

pub type KuiCustomElementAttributeFn = Option<unsafe extern "C" fn(*mut KuiElement, *const core::ffi::c_char, usize, *const core::ffi::c_char, usize, *const core::ffi::c_char, usize, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomElementCallbacks {
    pub connected: KuiCustomElementLifecycleFn,
    pub disconnected: KuiCustomElementLifecycleFn,
    pub attribute_changed: KuiCustomElementAttributeFn,
    pub observed_attributes: *const *const core::ffi::c_char,
    pub observed_attribute_count: usize,
    pub user_data: *mut core::ffi::c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCPDFOptions {
    pub title: *const core::ffi::c_char,
    pub author: *const core::ffi::c_char,
    pub subject: *const core::ffi::c_char,
    pub keywords: *const core::ffi::c_char,
    pub creator: *const core::ffi::c_char,
    pub creation_date: *const core::ffi::c_char,
    pub page_width_pt: f32,
    pub page_height_pt: f32,
    pub rasterize_fonts: core::ffi::c_int,
    pub generate_outline: core::ffi::c_int,
}

pub type KuiCCursor = core::ffi::c_int;
pub const KUI_C_CURSOR_AUTO: KuiCCursor = 0;
pub const KUI_C_CURSOR_DEFAULT: KuiCCursor = 1;
pub const KUI_C_CURSOR_POINTER: KuiCCursor = 2;
pub const KUI_C_CURSOR_CROSSHAIR: KuiCCursor = 3;
pub const KUI_C_CURSOR_TEXT: KuiCCursor = 4;
pub const KUI_C_CURSOR_MOVE: KuiCCursor = 5;
pub const KUI_C_CURSOR_NOT_ALLOWED: KuiCCursor = 6;
pub const KUI_C_CURSOR_GRAB: KuiCCursor = 7;
pub const KUI_C_CURSOR_GRABBING: KuiCCursor = 8;
pub const KUI_C_CURSOR_WAIT: KuiCCursor = 9;
pub const KUI_C_CURSOR_PROGRESS: KuiCCursor = 10;
pub const KUI_C_CURSOR_HELP: KuiCCursor = 11;
pub const KUI_C_CURSOR_CONTEXT_MENU: KuiCCursor = 12;
pub const KUI_C_CURSOR_N_RESIZE: KuiCCursor = 13;
pub const KUI_C_CURSOR_E_RESIZE: KuiCCursor = 14;
pub const KUI_C_CURSOR_S_RESIZE: KuiCCursor = 15;
pub const KUI_C_CURSOR_W_RESIZE: KuiCCursor = 16;
pub const KUI_C_CURSOR_NE_RESIZE: KuiCCursor = 17;
pub const KUI_C_CURSOR_NW_RESIZE: KuiCCursor = 18;
pub const KUI_C_CURSOR_SE_RESIZE: KuiCCursor = 19;
pub const KUI_C_CURSOR_SW_RESIZE: KuiCCursor = 20;
pub const KUI_C_CURSOR_EW_RESIZE: KuiCCursor = 21;
pub const KUI_C_CURSOR_NS_RESIZE: KuiCCursor = 22;
pub const KUI_C_CURSOR_NESW_RESIZE: KuiCCursor = 23;
pub const KUI_C_CURSOR_NWSE_RESIZE: KuiCCursor = 24;
pub const KUI_C_CURSOR_COL_RESIZE: KuiCCursor = 25;
pub const KUI_C_CURSOR_ROW_RESIZE: KuiCCursor = 26;

pub type KuiCMessageBoxType = core::ffi::c_int;
pub const KUI_C_MESSAGE_BOX_INFO: KuiCMessageBoxType = 0;
pub const KUI_C_MESSAGE_BOX_WARNING: KuiCMessageBoxType = 1;
pub const KUI_C_MESSAGE_BOX_ERROR: KuiCMessageBoxType = 2;
pub const KUI_C_MESSAGE_BOX_QUESTION: KuiCMessageBoxType = 3;

pub type KuiCMessageBoxButtons = core::ffi::c_int;
pub const KUI_C_MESSAGE_BOX_OK: KuiCMessageBoxButtons = 0;
pub const KUI_C_MESSAGE_BOX_OK_CANCEL: KuiCMessageBoxButtons = 1;
pub const KUI_C_MESSAGE_BOX_YES_NO: KuiCMessageBoxButtons = 2;
pub const KUI_C_MESSAGE_BOX_YES_NO_CANCEL: KuiCMessageBoxButtons = 3;

pub type KuiCMessageBoxResult = core::ffi::c_int;
pub const KUI_C_MESSAGE_BOX_RESULT_OK: KuiCMessageBoxResult = 0;
pub const KUI_C_MESSAGE_BOX_RESULT_CANCEL: KuiCMessageBoxResult = 1;
pub const KUI_C_MESSAGE_BOX_RESULT_YES: KuiCMessageBoxResult = 2;
pub const KUI_C_MESSAGE_BOX_RESULT_NO: KuiCMessageBoxResult = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCMenuItem {
    pub text: *const core::ffi::c_char,
    pub text_len: usize,
    pub shortcut_label: *const core::ffi::c_char,
    pub shortcut_label_len: usize,
    pub id: u32,
    pub separator: core::ffi::c_int,
    pub disabled: core::ffi::c_int,
    pub checkable: core::ffi::c_int,
    pub checked: core::ffi::c_int,
    pub children: *const KuiCMenuItem,
    pub child_count: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCMenu {
    pub text: *const core::ffi::c_char,
    pub text_len: usize,
    pub items: *const KuiCMenuItem,
    pub item_count: usize,
}

pub type KuiMenuActivateFn = Option<unsafe extern "C" fn(u32, *mut core::ffi::c_void)>;

pub type KuiCTrayEvent = core::ffi::c_int;
pub const KUI_C_TRAY_CLICK: KuiCTrayEvent = 0;
pub const KUI_C_TRAY_DOUBLE_CLICK: KuiCTrayEvent = 1;
pub const KUI_C_TRAY_RIGHT_CLICK: KuiCTrayEvent = 2;
pub const KUI_C_TRAY_BALLOON_CLICK: KuiCTrayEvent = 3;

pub type KuiTrayEventFn = Option<unsafe extern "C" fn(KuiCTrayEvent, *mut core::ffi::c_void)>;

pub type KuiCToastLevel = core::ffi::c_int;
pub const KUI_C_TOAST_INFO: KuiCToastLevel = 0;
pub const KUI_C_TOAST_SUCCESS: KuiCToastLevel = 1;
pub const KUI_C_TOAST_WARNING: KuiCToastLevel = 2;
pub const KUI_C_TOAST_ERROR: KuiCToastLevel = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCToastOptions {
    pub message: *const core::ffi::c_char,
    pub message_len: usize,
    pub title: *const core::ffi::c_char,
    pub title_len: usize,
    pub level: KuiCToastLevel,
    pub duration_ms: core::ffi::c_int,
    pub position: *const core::ffi::c_char,
    pub position_len: usize,
    pub closable: core::ffi::c_int,
    pub icon: *const core::ffi::c_char,
    pub icon_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCKeyValue {
    pub key: *const core::ffi::c_char,
    pub key_len: usize,
    pub value: *const core::ffi::c_char,
    pub value_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCRouteInfo {
    pub path: *const core::ffi::c_char,
    pub path_len: usize,
    pub pattern: *const core::ffi::c_char,
    pub pattern_len: usize,
    pub query: *const core::ffi::c_char,
    pub query_len: usize,
    pub params: *const KuiCKeyValue,
    pub param_count: usize,
    pub query_params: *const KuiCKeyValue,
    pub query_param_count: usize,
}

pub type KuiRouteViewFn = Option<unsafe extern "C" fn(*mut KuiDocument, *const KuiCRouteInfo, *mut core::ffi::c_void) -> *mut KuiElement>;

pub type KuiRouteGuardFn = Option<unsafe extern "C" fn(*const KuiCRouteInfo, *const KuiCRouteInfo, *mut core::ffi::c_void) -> core::ffi::c_int>;

pub type KuiRouteInfoFn = Option<unsafe extern "C" fn(*const KuiCRouteInfo, *mut core::ffi::c_void)>;

pub type KuiCRouteGuardKind = core::ffi::c_int;
pub const KUI_C_ROUTE_GUARD_BEFORE_ENTER: KuiCRouteGuardKind = 0;
pub const KUI_C_ROUTE_GUARD_BEFORE_LEAVE: KuiCRouteGuardKind = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiObservable {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiObserver {
    _private: [u8; 0],
}

pub type KuiCMutationType = core::ffi::c_int;
pub const KUI_C_MUTATION_CHILD_LIST: KuiCMutationType = 0;
pub const KUI_C_MUTATION_ATTRIBUTES: KuiCMutationType = 1;
pub const KUI_C_MUTATION_CHARACTER_DATA: KuiCMutationType = 2;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCMutationRecord {
    pub type_: KuiCMutationType,
    pub target: *mut KuiElement,
    pub target_node_type: core::ffi::c_int,
    pub added: *mut KuiElement,
    pub added_node_type: core::ffi::c_int,
    pub removed: *mut KuiElement,
    pub removed_node_type: core::ffi::c_int,
    pub previous_sibling: *mut KuiElement,
    pub next_sibling: *mut KuiElement,
    pub attribute_name: *const core::ffi::c_char,
    pub attribute_name_len: usize,
    pub old_value: *const core::ffi::c_char,
    pub old_value_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCMutationOptions {
    pub child_list: core::ffi::c_int,
    pub attributes: core::ffi::c_int,
    pub character_data: core::ffi::c_int,
    pub subtree: core::ffi::c_int,
    pub attribute_old_value: core::ffi::c_int,
    pub character_data_old_value: core::ffi::c_int,
    pub attribute_filter: *const *const core::ffi::c_char,
    pub attribute_filter_count: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCResizeEntry {
    pub target: *mut KuiElement,
    pub content_width: f32,
    pub content_height: f32,
    pub border_width: f32,
    pub border_height: f32,
    pub content_x: f32,
    pub content_y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCIntersectionOptions {
    pub root: *mut KuiElement,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub thresholds: *const f32,
    pub threshold_count: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCIntersectionEntry {
    pub target: *mut KuiElement,
    pub bounding_client_rect: KuiRect,
    pub intersection_rect: KuiRect,
    pub root_bounds: KuiRect,
    pub intersection_ratio: f32,
    pub is_intersecting: core::ffi::c_int,
}

pub type KuiMutationFn = Option<unsafe extern "C" fn(*const KuiCMutationRecord, usize, *mut core::ffi::c_void)>;

pub type KuiResizeFn = Option<unsafe extern "C" fn(*const KuiCResizeEntry, usize, *mut core::ffi::c_void)>;

pub type KuiIntersectionFn = Option<unsafe extern "C" fn(*const KuiCIntersectionEntry, usize, *mut core::ffi::c_void)>;

pub type KuiCPopupType = core::ffi::c_int;
pub const KUI_C_POPUP_TOOLTIP: KuiCPopupType = 0;
pub const KUI_C_POPUP_CONTEXT_MENU: KuiCPopupType = 1;
pub const KUI_C_POPUP_CUSTOM: KuiCPopupType = 2;

pub type KuiCPopupPosition = core::ffi::c_int;
pub const KUI_C_POPUP_TOP: KuiCPopupPosition = 0;
pub const KUI_C_POPUP_BOTTOM: KuiCPopupPosition = 1;
pub const KUI_C_POPUP_LEFT: KuiCPopupPosition = 2;
pub const KUI_C_POPUP_RIGHT: KuiCPopupPosition = 3;

pub type KuiCPopupMode = core::ffi::c_int;
pub const KUI_C_POPUP_MODE_OVERLAY: KuiCPopupMode = 0;
pub const KUI_C_POPUP_MODE_NATIVE_WINDOW: KuiCPopupMode = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCTooltipTheme {
    pub background: u32,
    pub foreground: u32,
    pub border: u32,
    pub radius: f32,
}

pub type KuiCPickerMode = core::ffi::c_int;
pub const KUI_C_PICKER_NONE: KuiCPickerMode = 0;
pub const KUI_C_PICKER_DATE: KuiCPickerMode = 1;
pub const KUI_C_PICKER_TIME: KuiCPickerMode = 2;
pub const KUI_C_PICKER_DATETIME_LOCAL: KuiCPickerMode = 3;
pub const KUI_C_PICKER_COLOR: KuiCPickerMode = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCInputPickerState {
    pub open: core::ffi::c_int,
    pub enabled: core::ffi::c_int,
    pub mode: KuiCPickerMode,
    pub visible_year: core::ffi::c_int,
    pub visible_month: core::ffi::c_int,
    pub color_h: f32,
    pub color_s: f32,
    pub color_v: f32,
    pub hour: core::ffi::c_int,
    pub minute: core::ffi::c_int,
}

pub type KuiCAudioGroup = core::ffi::c_int;
pub const KUI_C_AUDIO_MASTER: KuiCAudioGroup = 0;
pub const KUI_C_AUDIO_MUSIC: KuiCAudioGroup = 1;
pub const KUI_C_AUDIO_SFX: KuiCAudioGroup = 2;
pub const KUI_C_AUDIO_UI: KuiCAudioGroup = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCAudioPlayOptions {
    pub group: KuiCAudioGroup,
    pub volume: f32,
    pub pan: f32,
    pub pitch: f32,
    pub loop_: core::ffi::c_int,
    pub preload_only: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCSoundInfo {
    pub group: KuiCAudioGroup,
    pub volume: f32,
    pub pan: f32,
    pub pitch: f32,
    pub playing: core::ffi::c_int,
    pub looping: core::ffi::c_int,
    pub current_time: f64,
    pub duration: f64,
}

pub type KuiCLottieLoadState = core::ffi::c_int;
pub const KUI_C_LOTTIE_IDLE: KuiCLottieLoadState = 0;
pub const KUI_C_LOTTIE_LOADING: KuiCLottieLoadState = 1;
pub const KUI_C_LOTTIE_COMPLETE: KuiCLottieLoadState = 2;
pub const KUI_C_LOTTIE_ERROR: KuiCLottieLoadState = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCLottieState {
    pub playing: core::ffi::c_int,
    pub looping: core::ffi::c_int,
    pub autoplay: core::ffi::c_int,
    pub speed: f32,
    pub position: f64,
    pub load_state: KuiCLottieLoadState,
}

pub type KuiCDropEffect = core::ffi::c_int;
pub const KUI_C_DROP_NONE: KuiCDropEffect = 0;
pub const KUI_C_DROP_COPY: KuiCDropEffect = 1;
pub const KUI_C_DROP_MOVE: KuiCDropEffect = 2;
pub const KUI_C_DROP_LINK: KuiCDropEffect = 3;

pub type KuiCEffectAllowed = core::ffi::c_int;
pub const KUI_C_EFFECT_NONE: KuiCEffectAllowed = 0;
pub const KUI_C_EFFECT_COPY: KuiCEffectAllowed = 1;
pub const KUI_C_EFFECT_MOVE: KuiCEffectAllowed = 2;
pub const KUI_C_EFFECT_LINK: KuiCEffectAllowed = 3;
pub const KUI_C_EFFECT_COPY_MOVE: KuiCEffectAllowed = 4;
pub const KUI_C_EFFECT_COPY_LINK: KuiCEffectAllowed = 5;
pub const KUI_C_EFFECT_MOVE_LINK: KuiCEffectAllowed = 6;
pub const KUI_C_EFFECT_ALL: KuiCEffectAllowed = 7;
pub const KUI_C_EFFECT_UNINITIALIZED: KuiCEffectAllowed = 8;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiScriptFunction {
    _private: [u8; 0],
}

pub type KuiCScriptValueType = core::ffi::c_int;
pub const KUI_C_SCRIPT_NIL: KuiCScriptValueType = 0;
pub const KUI_C_SCRIPT_BOOL: KuiCScriptValueType = 1;
pub const KUI_C_SCRIPT_INT: KuiCScriptValueType = 2;
pub const KUI_C_SCRIPT_FLOAT: KuiCScriptValueType = 3;
pub const KUI_C_SCRIPT_STRING: KuiCScriptValueType = 4;
pub const KUI_C_SCRIPT_FUNCTION: KuiCScriptValueType = 5;
pub const KUI_C_SCRIPT_USERDATA: KuiCScriptValueType = 6;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCScriptValue {
    pub type_: KuiCScriptValueType,
    pub bool_value: core::ffi::c_int,
    pub int_value: i64,
    pub float_value: f64,
    pub string_value: *const core::ffi::c_char,
    pub string_len: usize,
    pub function_value: *mut KuiScriptFunction,
    pub userdata: *mut core::ffi::c_void,
}

pub type KuiScriptFunctionFn = Option<unsafe extern "C" fn(*const KuiCScriptValue, usize, *mut KuiCScriptValue, *mut core::ffi::c_void) -> KuiStatus>;

pub type KuiScriptPrintFn = Option<unsafe extern "C" fn(*const core::ffi::c_char, usize, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiPaintContext {
    _private: [u8; 0],
}

pub type KuiCInputModifier = core::ffi::c_int;
pub const KUI_C_INPUT_MOD_CTRL: KuiCInputModifier = 1;
pub const KUI_C_INPUT_MOD_SHIFT: KuiCInputModifier = 2;
pub const KUI_C_INPUT_MOD_ALT: KuiCInputModifier = 4;
pub const KUI_C_INPUT_MOD_META: KuiCInputModifier = 8;

pub type KuiCCustomKeyType = core::ffi::c_int;
pub const KUI_C_CUSTOM_KEY_DOWN: KuiCCustomKeyType = 0;
pub const KUI_C_CUSTOM_KEY_UP: KuiCCustomKeyType = 1;

pub type KuiCCustomMouseType = core::ffi::c_int;
pub const KUI_C_CUSTOM_MOUSE_DOWN: KuiCCustomMouseType = 0;
pub const KUI_C_CUSTOM_MOUSE_UP: KuiCCustomMouseType = 1;
pub const KUI_C_CUSTOM_MOUSE_MOVE: KuiCCustomMouseType = 2;
pub const KUI_C_CUSTOM_MOUSE_ENTER: KuiCCustomMouseType = 3;
pub const KUI_C_CUSTOM_MOUSE_LEAVE: KuiCCustomMouseType = 4;

pub type KuiCImeEventType = core::ffi::c_int;
pub const KUI_C_IME_START: KuiCImeEventType = 0;
pub const KUI_C_IME_UPDATE: KuiCImeEventType = 1;
pub const KUI_C_IME_COMMIT: KuiCImeEventType = 2;
pub const KUI_C_IME_CANCEL: KuiCImeEventType = 3;

pub type KuiCDecorationStyle = core::ffi::c_int;
pub const KUI_C_DECORATION_SOLID: KuiCDecorationStyle = 0;
pub const KUI_C_DECORATION_DOUBLE: KuiCDecorationStyle = 1;
pub const KUI_C_DECORATION_DOTTED: KuiCDecorationStyle = 2;
pub const KUI_C_DECORATION_DASHED: KuiCDecorationStyle = 3;
pub const KUI_C_DECORATION_WAVY: KuiCDecorationStyle = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomKeyEvent {
    pub type_: KuiCCustomKeyType,
    pub key: *const core::ffi::c_char,
    pub key_len: usize,
    pub code: *const core::ffi::c_char,
    pub code_len: usize,
    pub modifiers: core::ffi::c_int,
    pub is_repeat: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomMouseEvent {
    pub type_: KuiCCustomMouseType,
    pub x: f32,
    pub y: f32,
    pub button: core::ffi::c_int,
    pub buttons: core::ffi::c_int,
    pub modifiers: core::ffi::c_int,
    pub click_count: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomWheelEvent {
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub modifiers: core::ffi::c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomImeEvent {
    pub type_: KuiCImeEventType,
    pub text: *const core::ffi::c_char,
    pub text_len: usize,
    pub caret_pos: core::ffi::c_int,
    pub committed_text: *const core::ffi::c_char,
    pub committed_len: usize,
}

pub type KuiCustomElementPaintFn = Option<unsafe extern "C" fn(*mut KuiElement, *mut KuiPaintContext, *mut core::ffi::c_void)>;

pub type KuiCustomElementTickFn = Option<unsafe extern "C" fn(*mut KuiElement, f64, *mut core::ffi::c_void)>;

pub type KuiCustomElementNeedsTickFn = Option<unsafe extern "C" fn(*mut KuiElement, *mut core::ffi::c_void) -> core::ffi::c_int>;

pub type KuiCustomElementKeyFn = Option<unsafe extern "C" fn(*mut KuiElement, *const KuiCCustomKeyEvent, *mut core::ffi::c_void) -> core::ffi::c_int>;

pub type KuiCustomElementMouseFn = Option<unsafe extern "C" fn(*mut KuiElement, *const KuiCCustomMouseEvent, *mut core::ffi::c_void) -> core::ffi::c_int>;

pub type KuiCustomElementWheelFn = Option<unsafe extern "C" fn(*mut KuiElement, *const KuiCCustomWheelEvent, *mut core::ffi::c_void) -> core::ffi::c_int>;

pub type KuiCustomElementImeFn = Option<unsafe extern "C" fn(*mut KuiElement, *const KuiCCustomImeEvent, *mut core::ffi::c_void) -> core::ffi::c_int>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCustomElementDefinition {
    pub callbacks: KuiCCustomElementCallbacks,
    pub paint: KuiCustomElementPaintFn,
    pub tick: KuiCustomElementTickFn,
    pub needs_tick: KuiCustomElementNeedsTickFn,
    pub key: KuiCustomElementKeyFn,
    pub mouse: KuiCustomElementMouseFn,
    pub wheel: KuiCustomElementWheelFn,
    pub ime: KuiCustomElementImeFn,
    pub release_fn: KuiTaskFn,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCTextStyle {
    pub font_family: *const core::ffi::c_char,
    pub font_size: f32,
    pub font_weight: u16,
    pub italic: core::ffi::c_int,
    pub letter_spacing: f32,
    pub word_spacing: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCornerRadii {
    pub top_left_x: f32,
    pub top_left_y: f32,
    pub top_right_x: f32,
    pub top_right_y: f32,
    pub bottom_right_x: f32,
    pub bottom_right_y: f32,
    pub bottom_left_x: f32,
    pub bottom_left_y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCTextMetrics {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiThemeManager {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiVirtualList {
    _private: [u8; 0],
}

pub type KuiCScrollUnit = core::ffi::c_int;
pub const KUI_C_SCROLL_PIXEL: KuiCScrollUnit = 0;
pub const KUI_C_SCROLL_LINE: KuiCScrollUnit = 1;

pub type KuiVirtualListRenderFn = Option<unsafe extern "C" fn(*mut KuiDocument, u32, *mut core::ffi::c_void) -> *mut KuiElement>;

pub type KuiVirtualListHeightFn = Option<unsafe extern "C" fn(u32, *mut core::ffi::c_void) -> f32>;

pub type KuiVirtualListFocusFn = Option<unsafe extern "C" fn(i32, i32, *mut core::ffi::c_void)>;

pub type KuiIndexFn = Option<unsafe extern "C" fn(u32, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCVirtualListConfig {
    pub total_items: u32,
    pub item_height: f32,
    pub overscan: u32,
    pub recycle_pool_max: u32,
    pub scroll_unit: KuiCScrollUnit,
    pub enable_keyboard: core::ffi::c_int,
    pub render_item: KuiVirtualListRenderFn,
    pub height_for_item: KuiVirtualListHeightFn,
    pub on_focus_change: KuiVirtualListFocusFn,
    pub on_activate: KuiIndexFn,
    pub user_data: *mut core::ffi::c_void,
    pub release_fn: KuiTaskFn,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCVirtualListState {
    pub visible_start: u32,
    pub visible_end: u32,
    pub total_items: u32,
    pub recycle_pool_size: u32,
    pub focused_index: i32,
    pub total_height: f32,
    pub item_height: f32,
    pub scroll_unit: KuiCScrollUnit,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiListView {
    _private: [u8; 0],
}

pub type KuiCSelectionMode = core::ffi::c_int;
pub const KUI_C_SELECTION_NONE: KuiCSelectionMode = 0;
pub const KUI_C_SELECTION_SINGLE: KuiCSelectionMode = 1;
pub const KUI_C_SELECTION_MULTI: KuiCSelectionMode = 2;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCListViewItem {
    pub text: *const core::ffi::c_char,
    pub text_len: usize,
    pub subtitle: *const core::ffi::c_char,
    pub subtitle_len: usize,
    pub icon: *const core::ffi::c_char,
    pub icon_len: usize,
    pub disabled: core::ffi::c_int,
}

pub type KuiListViewItemFn = Option<unsafe extern "C" fn(u32, *mut KuiCListViewItem, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCListViewConfig {
    pub item_count: u32,
    pub item_height: f32,
    pub overscan: u32,
    pub scroll_unit: KuiCScrollUnit,
    pub selection_mode: KuiCSelectionMode,
    pub item_bg: *const core::ffi::c_char,
    pub item_hover_bg: *const core::ffi::c_char,
    pub item_selected_bg: *const core::ffi::c_char,
    pub item_text_color: *const core::ffi::c_char,
    pub item_sub_color: *const core::ffi::c_char,
    pub item_provider: KuiListViewItemFn,
    pub on_select: KuiIndexFn,
    pub on_activate: KuiIndexFn,
    pub user_data: *mut core::ffi::c_void,
    pub release_fn: KuiTaskFn,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiTreeView {
    _private: [u8; 0],
}

pub type KuiTreeToggleFn = Option<unsafe extern "C" fn(u32, core::ffi::c_int, *mut core::ffi::c_void)>;

pub type KuiTreeSelectionFn = Option<unsafe extern "C" fn(*const u32, usize, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCTreeViewConfig {
    pub item_height: f32,
    pub indent_width: f32,
    pub overscan: u32,
    pub scroll_unit: KuiCScrollUnit,
    pub item_bg: *const core::ffi::c_char,
    pub item_hover_bg: *const core::ffi::c_char,
    pub item_selected_bg: *const core::ffi::c_char,
    pub item_text_color: *const core::ffi::c_char,
    pub multi_select: core::ffi::c_int,
    pub on_select: KuiIndexFn,
    pub on_toggle: KuiTreeToggleFn,
    pub on_selection_changed: KuiTreeSelectionFn,
    pub user_data: *mut core::ffi::c_void,
    pub release_fn: KuiTaskFn,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCTreeNode {
    pub text: *const core::ffi::c_char,
    pub text_len: usize,
    pub icon: *const core::ffi::c_char,
    pub icon_len: usize,
    pub expanded: core::ffi::c_int,
    pub disabled: core::ffi::c_int,
    pub parent: i32,
    pub child_count: usize,
}

pub type KuiCStyleProfile = core::ffi::c_int;
pub const KUI_C_STYLE_NATIVE_GUI: KuiCStyleProfile = 0;
pub const KUI_C_STYLE_BROWSER_COMPAT: KuiCStyleProfile = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiWindowPropsEx {
    pub struct_size: usize,
    pub base: KuiWindowProps,
    pub sizeable: core::ffi::c_int,
    pub hidden: core::ffi::c_int,
    pub has_frame_radius: core::ffi::c_int,
    pub frame_radius: f32,
}

pub type KuiCLogLevel = core::ffi::c_int;
pub const KUI_C_LOG_DEBUG: KuiCLogLevel = 0;
pub const KUI_C_LOG_INFO: KuiCLogLevel = 1;
pub const KUI_C_LOG_WARN: KuiCLogLevel = 2;
pub const KUI_C_LOG_ERROR: KuiCLogLevel = 3;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCStackTraceConfig {
    pub struct_size: usize,
    pub output_mode: core::ffi::c_uint,
    pub log_file_path: *const core::ffi::c_char,
    pub max_frames: core::ffi::c_int,
    pub app_name: *const core::ffi::c_char,
    pub write_mini_dump: core::ffi::c_int,
    pub dump_dir: *const core::ffi::c_char,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KuiCCrashReporterConfig {
    pub struct_size: usize,
    pub handler_exe: *const core::ffi::c_char,
    pub dump_dir: *const core::ffi::c_char,
    pub app_name: *const core::ffi::c_char,
    pub app_version: *const core::ffi::c_char,
    pub report_url: *const core::ffi::c_char,
    pub enable_upload: core::ffi::c_int,
}

#[link(name = "kui_c")]
unsafe extern "C" {
    pub fn kui_get_version() -> KuiVersion;
    pub fn kui_status_message(s: KuiStatus) -> *const core::ffi::c_char;
    pub fn kui_platform_init() -> KuiStatus;
    pub fn kui_platform_shutdown() -> KuiStatus;
    pub fn kui_platform_run(out_exit_code: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_platform_quit(exit_code: core::ffi::c_int) -> KuiStatus;
    pub fn kui_platform_poll_events() -> KuiStatus;
    pub fn kui_platform_wait_events(timeout_seconds: f64) -> KuiStatus;
    pub fn kui_platform_set_timeout(fn_: KuiTaskFn, user_data: *mut core::ffi::c_void, delay_ms: u32, out_id: *mut KuiTimerId) -> KuiStatus;
    pub fn kui_platform_set_interval(fn_: KuiTaskFn, user_data: *mut core::ffi::c_void, interval_ms: u32, out_id: *mut KuiTimerId) -> KuiStatus;
    pub fn kui_platform_clear_timer(id: KuiTimerId) -> KuiStatus;
    pub fn kui_platform_request_idle(fn_: KuiTaskFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_platform_post_to_main_thread(fn_: KuiTaskFn, discard_fn: KuiTaskFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_platform_set_f12_devtools_enabled(enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_platform_get_primary_monitor(out_monitor: *mut KuiCMonitorInfo) -> KuiStatus;
    pub fn kui_window_create(props: *const KuiWindowProps, out: *mut *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_release(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_show(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_hide(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_is_visible(w: *mut KuiWindow, out_visible: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_get_document(w: *mut KuiWindow, out: *mut *mut KuiDocument) -> KuiStatus;
    pub fn kui_window_get_title(w: *mut KuiWindow, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_window_set_title(w: *mut KuiWindow, title: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_window_get_size(w: *mut KuiWindow, out_width: *mut core::ffi::c_int, out_height: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_resize(w: *mut KuiWindow, width: core::ffi::c_int, height: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_get_position(w: *mut KuiWindow, out_x: *mut core::ffi::c_int, out_y: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_position(w: *mut KuiWindow, x: core::ffi::c_int, y: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_minimize(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_maximize(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_restore(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_set_always_on_top(w: *mut KuiWindow, on_top: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_opacity(w: *mut KuiWindow, opacity: f32) -> KuiStatus;
    pub fn kui_window_get_device_pixel_ratio(w: *mut KuiWindow, out_ratio: *mut f32) -> KuiStatus;
    pub fn kui_window_is_minimized(w: *mut KuiWindow, out_minimized: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_is_maximized(w: *mut KuiWindow, out_maximized: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_close(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_should_close(w: *mut KuiWindow, out_should_close: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_on_resize(w: *mut KuiWindow, fn_: KuiWindowResizeFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_window_start_devtools(w: *mut KuiWindow, port: u16, out_started: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_get_devtools_url(w: *mut KuiWindow, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_window_get_native_handle(w: *mut KuiWindow, out_handle: *mut *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_document_create(out: *mut *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_retain(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_release(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_load_html(doc: *mut KuiDocument, html: *const core::ffi::c_char, html_len: usize) -> KuiStatus;
    pub fn kui_document_load_html_file(doc: *mut KuiDocument, path: *const core::ffi::c_char, path_len: usize) -> KuiStatus;
    pub fn kui_document_add_stylesheet(doc: *mut KuiDocument, css: *const core::ffi::c_char, css_len: usize) -> KuiStatus;
    pub fn kui_document_clear_stylesheets(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_root(doc: *mut KuiDocument, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_body(doc: *mut KuiDocument, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_head(doc: *mut KuiDocument, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_get_element_by_id(doc: *mut KuiDocument, id: *const core::ffi::c_char, id_len: usize, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_query_selector(doc: *mut KuiDocument, sel: *const core::ffi::c_char, sel_len: usize, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_query_selector_all(doc: *mut KuiDocument, sel: *const core::ffi::c_char, sel_len: usize, out_buf: *mut *mut KuiElement, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_document_create_element(doc: *mut KuiDocument, tag: *const core::ffi::c_char, tag_len: usize, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_retain(el: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_release(el: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_get_tag_name(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_get_id(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_get_class_name(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_set_class_name(el: *mut KuiElement, value: *const core::ffi::c_char, value_len: usize) -> KuiStatus;
    pub fn kui_element_set_attribute(el: *mut KuiElement, name: *const core::ffi::c_char, name_len: usize, value: *const core::ffi::c_char, value_len: usize) -> KuiStatus;
    pub fn kui_element_get_attribute(el: *mut KuiElement, name: *const core::ffi::c_char, name_len: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_has_attribute(el: *mut KuiElement, name: *const core::ffi::c_char, name_len: usize, out_has: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_remove_attribute(el: *mut KuiElement, name: *const core::ffi::c_char, name_len: usize) -> KuiStatus;
    pub fn kui_element_get_value(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_set_value(el: *mut KuiElement, value: *const core::ffi::c_char, value_len: usize) -> KuiStatus;
    pub fn kui_element_get_checked(el: *mut KuiElement, out_checked: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_set_checked(el: *mut KuiElement, checked: core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_get_selected_index(el: *mut KuiElement, out_index: *mut i32) -> KuiStatus;
    pub fn kui_element_set_selected_index(el: *mut KuiElement, index: i32) -> KuiStatus;
    pub fn kui_element_set_text_content(el: *mut KuiElement, text: *const core::ffi::c_char, text_len: usize) -> KuiStatus;
    pub fn kui_element_get_text_content(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_set_inner_html(el: *mut KuiElement, html: *const core::ffi::c_char, html_len: usize) -> KuiStatus;
    pub fn kui_element_get_inner_html(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_class_list_add(el: *mut KuiElement, token: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_element_class_list_remove(el: *mut KuiElement, token: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_element_class_list_toggle(el: *mut KuiElement, token: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_element_class_list_contains(el: *mut KuiElement, token: *const core::ffi::c_char, len: usize, out_contains: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_query_selector(el: *mut KuiElement, sel: *const core::ffi::c_char, sel_len: usize, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_query_selector_all(el: *mut KuiElement, sel: *const core::ffi::c_char, sel_len: usize, out_buf: *mut *mut KuiElement, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_element_parent(el: *mut KuiElement, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_first_child(el: *mut KuiElement, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_last_child(el: *mut KuiElement, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_next_sibling(el: *mut KuiElement, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_previous_sibling(el: *mut KuiElement, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_child_count(el: *mut KuiElement, out_count: *mut usize) -> KuiStatus;
    pub fn kui_element_append_child(parent: *mut KuiElement, child: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_remove_child(parent: *mut KuiElement, child: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_remove(el: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_set_scroll_position(el: *mut KuiElement, left: f32, top: f32) -> KuiStatus;
    pub fn kui_element_scroll_by(el: *mut KuiElement, dx: f32, dy: f32) -> KuiStatus;
    pub fn kui_element_focus(el: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_blur(el: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_add_event_listener(el: *mut KuiElement, event_name: *const core::ffi::c_char, len: usize, fn_: KuiEventListenerFn, user_data: *mut core::ffi::c_void, out_id: *mut KuiListenerId) -> KuiStatus;
    pub fn kui_element_add_event_listener_ex(el: *mut KuiElement, event_name: *const core::ffi::c_char, len: usize, fn_: KuiEventListenerFn, user_data: *mut core::ffi::c_void, capture: core::ffi::c_int, out_id: *mut KuiListenerId) -> KuiStatus;
    pub fn kui_element_remove_event_listener(el: *mut KuiElement, event_name: *const core::ffi::c_char, len: usize, id: KuiListenerId) -> KuiStatus;
    pub fn kui_element_remove_event_listeners(el: *mut KuiElement, event_name: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_element_dispatch_event(el: *mut KuiElement, event_name: *const core::ffi::c_char, len: usize, bubbles: core::ffi::c_int, cancelable: core::ffi::c_int, out_not_canceled: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_event_get_type(evt: *mut KuiEvent, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_event_get_target(evt: *mut KuiEvent, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_event_get_current_target(evt: *mut KuiEvent, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_event_get_related_target(evt: *mut KuiEvent, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_event_prevent_default(evt: *mut KuiEvent) -> KuiStatus;
    pub fn kui_event_stop_propagation(evt: *mut KuiEvent) -> KuiStatus;
    pub fn kui_event_get_flags(evt: *mut KuiEvent, out_bubbles: *mut core::ffi::c_int, out_cancelable: *mut core::ffi::c_int, out_default_prevented: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_event_mouse_get_client_xy(evt: *mut KuiEvent, x: *mut f32, y: *mut f32) -> KuiStatus;
    pub fn kui_event_mouse_get_page_xy(evt: *mut KuiEvent, x: *mut f32, y: *mut f32) -> KuiStatus;
    pub fn kui_event_mouse_get_offset_xy(evt: *mut KuiEvent, x: *mut f32, y: *mut f32) -> KuiStatus;
    pub fn kui_event_mouse_get_button(evt: *mut KuiEvent, out_button: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_event_key_get_key(evt: *mut KuiEvent, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_event_key_get_code(evt: *mut KuiEvent, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_event_key_get_modifiers(evt: *mut KuiEvent, out_ctrl: *mut core::ffi::c_int, out_shift: *mut core::ffi::c_int, out_alt: *mut core::ffi::c_int, out_meta: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_set_style_property(el: *mut KuiElement, prop: *const core::ffi::c_char, prop_len: usize, value: *const core::ffi::c_char, value_len: usize, important: core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_get_style_property(el: *mut KuiElement, prop: *const core::ffi::c_char, prop_len: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_remove_style_property(el: *mut KuiElement, prop: *const core::ffi::c_char, prop_len: usize) -> KuiStatus;
    pub fn kui_element_style_property_is_important(el: *mut KuiElement, prop: *const core::ffi::c_char, prop_len: usize, out_important: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_get_style_text(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_set_style_text(el: *mut KuiElement, css: *const core::ffi::c_char, css_len: usize) -> KuiStatus;
    pub fn kui_element_get_style_property_count(el: *mut KuiElement, out_count: *mut usize) -> KuiStatus;
    pub fn kui_element_get_style_property_name(el: *mut KuiElement, index: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_get_bounding_rect(el: *mut KuiElement, out: *mut KuiRect) -> KuiStatus;
    pub fn kui_element_get_client_metrics(el: *mut KuiElement, out_left: *mut f32, out_top: *mut f32, out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_element_get_offset_metrics(el: *mut KuiElement, out_left: *mut f32, out_top: *mut f32, out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_element_get_scroll_metrics(el: *mut KuiElement, out_left: *mut f32, out_top: *mut f32, out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_devtools_dispatch_mouse_event(doc: *mut KuiDocument, type_: KuiCMouseEventType, x: f32, y: f32, button: KuiCMouseButton, modifiers: core::ffi::c_int, click_count: core::ffi::c_int, delta_x: f32, delta_y: f32) -> KuiStatus;
    pub fn kui_devtools_dispatch_key_event(doc: *mut KuiDocument, type_: KuiCKeyEventType, key: *const core::ffi::c_char, key_len: usize, code: *const core::ffi::c_char, code_len: usize, modifiers: core::ffi::c_int, text: *const core::ffi::c_char, text_len: usize, is_auto_repeat: core::ffi::c_int) -> KuiStatus;
    pub fn kui_devtools_get_box_model(doc: *mut KuiDocument, selector: *const core::ffi::c_char, out_quads: *mut f32, out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_devtools_dispatch_command(doc: *mut KuiDocument, json_command: *const core::ffi::c_char, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_get_monitor_count(out_count: *mut usize) -> KuiStatus;
    pub fn kui_platform_get_monitor(index: usize, out_monitor: *mut KuiCMonitorInfo, name_buf: *mut core::ffi::c_char, name_capacity: usize, out_name_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_get_primary_monitor_physical_size(out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_platform_get_clipboard_text(buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_set_clipboard_text(text: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_platform_has_clipboard_text(out_has_text: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_platform_get_os_name(buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_get_os_version(buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_open_file_dialog(options: *const KuiCFileDialogOptions, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_save_file_dialog(options: *const KuiCFileDialogOptions, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_select_folder_dialog(title: *const core::ffi::c_char, title_len: usize, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_window_set_frameless_sizeable(window: *mut KuiWindow, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_frame_radius(window: *mut KuiWindow, radius: f32) -> KuiStatus;
    pub fn kui_window_clear_frame_radius(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_get_devtools_port(window: *mut KuiWindow, out_port: *mut u16) -> KuiStatus;
    pub fn kui_window_add_event_listener(window: *mut KuiWindow, event_name: *const core::ffi::c_char, len: usize, fn_: KuiWindowEventFn, user_data: *mut core::ffi::c_void, once: core::ffi::c_int, out_id: *mut KuiListenerId) -> KuiStatus;
    pub fn kui_window_remove_event_listener(window: *mut KuiWindow, event_name: *const core::ffi::c_char, len: usize, id: KuiListenerId) -> KuiStatus;
    pub fn kui_window_remove_event_listeners(window: *mut KuiWindow, event_name: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_event_get_type_tag(event: *mut KuiEvent, out_tag: *mut KuiCEventTypeTag) -> KuiStatus;
    pub fn kui_event_drag_get_file_count(event: *mut KuiEvent, out_count: *mut usize) -> KuiStatus;
    pub fn kui_event_drag_get_file(event: *mut KuiEvent, index: usize, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_position_relative_to(element: *mut KuiElement, anchor: *mut KuiElement, placement: *const core::ffi::c_char, placement_len: usize, options: *const KuiCPositioningOptions) -> KuiStatus;
    pub fn kui_media_play(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_media_pause(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_media_set_src(element: *mut KuiElement, src: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_media_set_controls(element: *mut KuiElement, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_set_current_time(element: *mut KuiElement, seconds: f64) -> KuiStatus;
    pub fn kui_media_get_current_time(element: *mut KuiElement, out_seconds: *mut f64) -> KuiStatus;
    pub fn kui_media_get_duration(element: *mut KuiElement, out_seconds: *mut f64) -> KuiStatus;
    pub fn kui_media_set_volume(element: *mut KuiElement, volume: f32) -> KuiStatus;
    pub fn kui_media_get_volume(element: *mut KuiElement, out_volume: *mut f32) -> KuiStatus;
    pub fn kui_media_set_muted(element: *mut KuiElement, muted: core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_get_muted(element: *mut KuiElement, out_muted: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_get_paused(element: *mut KuiElement, out_paused: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_get_ready_state(element: *mut KuiElement, out_state: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_get_network_state(element: *mut KuiElement, out_state: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_is_available(element: *mut KuiElement, out_available: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_lottie_set_src(element: *mut KuiElement, src: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_lottie_play(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_lottie_pause(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_lottie_get_complete(element: *mut KuiElement, out_complete: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_lottie_get_natural_size(element: *mut KuiElement, out_width: *mut f32, out_height: *mut f32) -> KuiStatus;
    pub fn kui_script_engine_create(out_engine: *mut *mut KuiScriptEngine) -> KuiStatus;
    pub fn kui_script_engine_release(engine: *mut KuiScriptEngine) -> KuiStatus;
    pub fn kui_script_engine_bind_document(engine: *mut KuiScriptEngine, document: *mut KuiDocument) -> KuiStatus;
    pub fn kui_script_engine_unbind_document(engine: *mut KuiScriptEngine) -> KuiStatus;
    pub fn kui_script_engine_eval(engine: *mut KuiScriptEngine, code: *const core::ffi::c_char, code_len: usize, source: *const core::ffi::c_char, source_len: usize, out_success: *mut core::ffi::c_int, error_buf: *mut core::ffi::c_char, error_capacity: usize, out_error_len: *mut usize) -> KuiStatus;
    pub fn kui_script_engine_execute_scripts(engine: *mut KuiScriptEngine) -> KuiStatus;
    pub fn kui_script_engine_set_error_handler(engine: *mut KuiScriptEngine, fn_: KuiScriptErrorFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_window_set_script_engine(window: *mut KuiWindow, engine: *mut KuiScriptEngine) -> KuiStatus;
    pub fn kui_http_client_create(out_client: *mut *mut KuiHttpClient) -> KuiStatus;
    pub fn kui_http_client_release(client: *mut KuiHttpClient) -> KuiStatus;
    pub fn kui_http_client_set_user_agent(client: *mut KuiHttpClient, user_agent: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_http_client_fetch_sync(client: *mut KuiHttpClient, url: *const core::ffi::c_char, len: usize, timeout_seconds: f64, out_response: *mut *mut KuiHttpResponse) -> KuiStatus;
    pub fn kui_http_client_fetch_async(client: *mut KuiHttpClient, url: *const core::ffi::c_char, len: usize, timeout_seconds: f64, fn_: KuiHttpResponseFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_http_client_fetch_async_on_main_thread(client: *mut KuiHttpClient, url: *const core::ffi::c_char, len: usize, timeout_seconds: f64, fn_: KuiHttpResponseFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_http_client_cancel_all(client: *mut KuiHttpClient) -> KuiStatus;
    pub fn kui_http_response_release(response: *mut KuiHttpResponse) -> KuiStatus;
    pub fn kui_http_response_get_status(response: *mut KuiHttpResponse, out_status_code: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_http_response_get_body(response: *mut KuiHttpResponse, buf: *mut u8, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_http_response_get_content_type(response: *mut KuiHttpResponse, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_http_response_get_error(response: *mut KuiHttpResponse, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_document_init_extensions(document: *mut KuiDocument, directory: *const core::ffi::c_char, len: usize, out_loaded_count: *mut usize) -> KuiStatus;
    pub fn kui_document_extension_is_loaded(document: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, out_loaded: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_extension_invoke(document: *mut KuiDocument, element: *mut KuiElement, method: *const core::ffi::c_char, method_len: usize, args: *const KuiCExtensionValue, arg_count: usize, max_results: usize, out_result: *mut *mut KuiExtensionResult) -> KuiStatus;
    pub fn kui_extension_result_release(result: *mut KuiExtensionResult) -> KuiStatus;
    pub fn kui_extension_result_get_status(result: *mut KuiExtensionResult, out_status: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_extension_result_get_count(result: *mut KuiExtensionResult, out_count: *mut usize) -> KuiStatus;
    pub fn kui_extension_result_get_value(result: *mut KuiExtensionResult, index: usize, out_value: *mut KuiCExtensionValue) -> KuiStatus;
    pub fn kui_extension_result_get_string(result: *mut KuiExtensionResult, index: usize, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_extension_result_get_error(result: *mut KuiExtensionResult, buf: *mut core::ffi::c_char, capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_document_extension_get_perf_stats(document: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, out_stats: *mut KuiCExtensionPerfStats) -> KuiStatus;
    pub fn kui_document_extension_reset_perf_stats(document: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_define_custom_element(document: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, callbacks: *const KuiCCustomElementCallbacks) -> KuiStatus;
    pub fn kui_document_custom_element_is_defined(document: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, out_defined: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_print_to_pdf(document: *mut KuiDocument, path: *const core::ffi::c_char, path_len: usize, options: *const KuiCPDFOptions, out_written: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_focus(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_has_focus(window: *mut KuiWindow, out_focused: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_cursor(window: *mut KuiWindow, cursor: KuiCCursor) -> KuiStatus;
    pub fn kui_window_set_cursor_visible(window: *mut KuiWindow, visible: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_fullscreen(window: *mut KuiWindow, fullscreen: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_is_fullscreen(window: *mut KuiWindow, out_fullscreen: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_frameless(window: *mut KuiWindow, frameless: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_frameless_shadow(window: *mut KuiWindow, shadow: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_get_frame_state(window: *mut KuiWindow, out_frameless: *mut core::ffi::c_int, out_shadow: *mut core::ffi::c_int, out_sizeable: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_set_frame_radii(window: *mut KuiWindow, top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> KuiStatus;
    pub fn kui_window_get_frame_radii(window: *mut KuiWindow, out_top_left: *mut f32, out_top_right: *mut f32, out_bottom_right: *mut f32, out_bottom_left: *mut f32) -> KuiStatus;
    pub fn kui_window_set_ime_candidate_pos(window: *mut KuiWindow, x: f32, y: f32) -> KuiStatus;
    pub fn kui_element_insert_before(parent: *mut KuiElement, child: *mut KuiElement, ref_child: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_replace_child(parent: *mut KuiElement, new_child: *mut KuiElement, old_child: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_clone(el: *mut KuiElement, deep: core::ffi::c_int, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_matches(el: *mut KuiElement, sel: *const core::ffi::c_char, sel_len: usize, out_matches: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_closest(el: *mut KuiElement, sel: *const core::ffi::c_char, sel_len: usize, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_element_get_outer_html(el: *mut KuiElement, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_get_computed_style_value(el: *mut KuiElement, name: *const core::ffi::c_char, name_len: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_element_insert_adjacent_html(el: *mut KuiElement, position: *const core::ffi::c_char, position_len: usize, html: *const core::ffi::c_char, html_len: usize) -> KuiStatus;
    pub fn kui_element_append_text(el: *mut KuiElement, text: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_element_has_focus(el: *mut KuiElement, out_focused: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_set_pointer_capture(el: *mut KuiElement, pointer_id: core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_release_pointer_capture(el: *mut KuiElement, pointer_id: core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_has_pointer_capture(el: *mut KuiElement, pointer_id: core::ffi::c_int, out_captured: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_element_from_point(doc: *mut KuiDocument, x: f32, y: f32, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_get_elements_by_class_name(doc: *mut KuiDocument, class_name: *const core::ffi::c_char, len: usize, out_buf: *mut *mut KuiElement, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_document_get_elements_by_tag_name(doc: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, out_buf: *mut *mut KuiElement, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_platform_message_box(title: *const core::ffi::c_char, title_len: usize, message: *const core::ffi::c_char, message_len: usize, type_: KuiCMessageBoxType, buttons: KuiCMessageBoxButtons, out_result: *mut KuiCMessageBoxResult) -> KuiStatus;
    pub fn kui_platform_open_url(url: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_platform_now(out_ms: *mut f64) -> KuiStatus;
    pub fn kui_platform_get_monitor_from_point(x: f32, y: f32, out_monitor: *mut KuiCMonitorInfo, name_buf: *mut core::ffi::c_char, name_capacity: usize, out_name_len: *mut usize) -> KuiStatus;
    pub fn kui_platform_get_monitor_from_window(native_window: *mut core::ffi::c_void, out_monitor: *mut KuiCMonitorInfo, name_buf: *mut core::ffi::c_char, name_capacity: usize, out_name_len: *mut usize) -> KuiStatus;
    pub fn kui_window_set_menu_bar(window: *mut KuiWindow, menus: *const KuiCMenu, menu_count: usize, fn_: KuiMenuActivateFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_window_clear_menu_bar(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_menu_set_checked(window: *mut KuiWindow, item_id: u32, checked: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_menu_set_enabled(window: *mut KuiWindow, item_id: u32, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_create(window: *mut KuiWindow, icon_path: *const core::ffi::c_char, icon_path_len: usize, tooltip: *const core::ffi::c_char, tooltip_len: usize, visible: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_destroy(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_tray_is_active(window: *mut KuiWindow, out_active: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_set_visible(window: *mut KuiWindow, visible: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_is_visible(window: *mut KuiWindow, out_visible: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_set_tooltip(window: *mut KuiWindow, tooltip: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_window_tray_set_icon(window: *mut KuiWindow, icon_path: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_window_tray_set_context_menu(window: *mut KuiWindow, items: *const KuiCMenuItem, item_count: usize, fn_: KuiMenuActivateFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_window_tray_popup_context_menu(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_tray_show_balloon(window: *mut KuiWindow, title: *const core::ffi::c_char, title_len: usize, message: *const core::ffi::c_char, message_len: usize, icon_type: core::ffi::c_int, duration_ms: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_tray_set_event_callback(window: *mut KuiWindow, fn_: KuiTrayEventFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_window_tray_simulate_event(window: *mut KuiWindow, event: KuiCTrayEvent) -> KuiStatus;
    pub fn kui_window_bind_shortcut(window: *mut KuiWindow, shortcut: *const core::ffi::c_char, len: usize, fn_: KuiTaskFn, user_data: *mut core::ffi::c_void, out_id: *mut u32) -> KuiStatus;
    pub fn kui_window_unbind_shortcut(window: *mut KuiWindow, id: u32) -> KuiStatus;
    pub fn kui_window_unbind_all_shortcuts(window: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_set_shortcuts_enabled(window: *mut KuiWindow, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_show_toast(doc: *mut KuiDocument, options: *const KuiCToastOptions, out_id: *mut u32) -> KuiStatus;
    pub fn kui_document_dismiss_toast(doc: *mut KuiDocument, id: u32) -> KuiStatus;
    pub fn kui_document_dismiss_all_toasts(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_get_toast_count(doc: *mut KuiDocument, out_count: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_save_screenshot(window: *mut KuiWindow, path: *const core::ffi::c_char, path_len: usize, out_saved: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_add_route(doc: *mut KuiDocument, pattern: *const core::ffi::c_char, pattern_len: usize, name: *const core::ffi::c_char, name_len: usize, fn_: KuiRouteViewFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn) -> KuiStatus;
    pub fn kui_document_router_remove_route(doc: *mut KuiDocument, pattern: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_document_router_clear_routes(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_router_register_view(doc: *mut KuiDocument, el: *mut KuiElement) -> KuiStatus;
    pub fn kui_document_router_unregister_view(doc: *mut KuiDocument, el: *mut KuiElement) -> KuiStatus;
    pub fn kui_document_router_has_route(doc: *mut KuiDocument, pattern: *const core::ffi::c_char, len: usize, out_has: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_get_route_count(doc: *mut KuiDocument, out_count: *mut u32) -> KuiStatus;
    pub fn kui_document_router_push(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, out_navigated: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_replace(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, out_navigated: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_go(doc: *mut KuiDocument, delta: core::ffi::c_int, out_navigated: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_get_current(doc: *mut KuiDocument, fn_: KuiRouteInfoFn, user_data: *mut core::ffi::c_void) -> KuiStatus;
    pub fn kui_document_router_get_current_path(doc: *mut KuiDocument, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_document_router_get_param(doc: *mut KuiDocument, name: *const core::ffi::c_char, name_len: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_document_router_get_history(doc: *mut KuiDocument, out_index: *mut core::ffi::c_int, out_size: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_router_add_guard(doc: *mut KuiDocument, kind: KuiCRouteGuardKind, fn_: KuiRouteGuardFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out_id: *mut u32) -> KuiStatus;
    pub fn kui_document_router_remove_guard(doc: *mut KuiDocument, id: u32) -> KuiStatus;
    pub fn kui_document_router_add_navigated_callback(doc: *mut KuiDocument, fn_: KuiRouteInfoFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out_id: *mut u32) -> KuiStatus;
    pub fn kui_document_router_remove_navigated_callback(doc: *mut KuiDocument, id: u32) -> KuiStatus;
    pub fn kui_observable_create(initial: *const core::ffi::c_char, len: usize, out: *mut *mut KuiObservable) -> KuiStatus;
    pub fn kui_observable_retain(obs: *mut KuiObservable) -> KuiStatus;
    pub fn kui_observable_release(obs: *mut KuiObservable) -> KuiStatus;
    pub fn kui_observable_set(obs: *mut KuiObservable, value: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_observable_get(obs: *mut KuiObservable, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_observable_subscribe(obs: *mut KuiObservable, fn_: KuiTaskFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out_id: *mut u32) -> KuiStatus;
    pub fn kui_observable_unsubscribe(obs: *mut KuiObservable, id: u32) -> KuiStatus;
    pub fn kui_observable_bind(obs: *mut KuiObservable, el: *mut KuiElement, property: *const core::ffi::c_char, property_len: usize, two_way: core::ffi::c_int, out_binding_id: *mut u32) -> KuiStatus;
    pub fn kui_observable_unbind(obs: *mut KuiObservable, binding_id: u32) -> KuiStatus;
    pub fn kui_observable_unbind_all(obs: *mut KuiObservable) -> KuiStatus;
    pub fn kui_document_flush_bindings(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_element_get_animation_state(el: *mut KuiElement, out_animating: *mut core::ffi::c_int, out_transitioning: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_get_animation_counts(doc: *mut KuiDocument, out_animations: *mut u32, out_transitions: *mut u32) -> KuiStatus;
    pub fn kui_mutation_observer_create(fn_: KuiMutationFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out: *mut *mut KuiObserver) -> KuiStatus;
    pub fn kui_resize_observer_create(fn_: KuiResizeFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out: *mut *mut KuiObserver) -> KuiStatus;
    pub fn kui_intersection_observer_create(options: *const KuiCIntersectionOptions, fn_: KuiIntersectionFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out: *mut *mut KuiObserver) -> KuiStatus;
    pub fn kui_mutation_observer_observe(obs: *mut KuiObserver, target: *mut KuiElement, options: *const KuiCMutationOptions) -> KuiStatus;
    pub fn kui_observer_observe(obs: *mut KuiObserver, target: *mut KuiElement) -> KuiStatus;
    pub fn kui_observer_unobserve(obs: *mut KuiObserver, target: *mut KuiElement) -> KuiStatus;
    pub fn kui_observer_disconnect(obs: *mut KuiObserver) -> KuiStatus;
    pub fn kui_observer_retain(obs: *mut KuiObserver) -> KuiStatus;
    pub fn kui_observer_release(obs: *mut KuiObserver) -> KuiStatus;
    pub fn kui_document_show_tooltip(doc: *mut KuiDocument, text: *const core::ffi::c_char, len: usize, anchor: *const KuiRect, position: KuiCPopupPosition, out_id: *mut u32) -> KuiStatus;
    pub fn kui_document_show_context_menu(doc: *mut KuiDocument, x: f32, y: f32, items: *const KuiCMenuItem, item_count: usize, fn_: KuiMenuActivateFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out_id: *mut u32) -> KuiStatus;
    pub fn kui_document_dismiss_popup(doc: *mut KuiDocument, id: u32) -> KuiStatus;
    pub fn kui_document_dismiss_all_popups(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_dismiss_popups_of_type(doc: *mut KuiDocument, type_: KuiCPopupType) -> KuiStatus;
    pub fn kui_document_has_popups(doc: *mut KuiDocument, out_any: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_has_popup_of_type(doc: *mut KuiDocument, type_: KuiCPopupType, out_has: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_set_tooltip_delay(doc: *mut KuiDocument, ms: f64) -> KuiStatus;
    pub fn kui_document_get_tooltip_delay(doc: *mut KuiDocument, out_ms: *mut f64) -> KuiStatus;
    pub fn kui_document_set_tooltip_theme(doc: *mut KuiDocument, theme: *const KuiCTooltipTheme) -> KuiStatus;
    pub fn kui_document_get_tooltip_theme(doc: *mut KuiDocument, out_theme: *mut KuiCTooltipTheme) -> KuiStatus;
    pub fn kui_document_set_popup_mode(doc: *mut KuiDocument, mode: KuiCPopupMode) -> KuiStatus;
    pub fn kui_document_set_default_context_menu_enabled(doc: *mut KuiDocument, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_get_default_context_menu_enabled(doc: *mut KuiDocument, out_enabled: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_element_open_input_picker(el: *mut KuiElement, mode: KuiCPickerMode) -> KuiStatus;
    pub fn kui_document_close_input_picker(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_commit_input_picker(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_get_input_picker_state(doc: *mut KuiDocument, out_state: *mut KuiCInputPickerState) -> KuiStatus;
    pub fn kui_document_get_input_picker_target(doc: *mut KuiDocument, out: *mut *mut KuiElement) -> KuiStatus;
    pub fn kui_document_set_input_picker_enabled(doc: *mut KuiDocument, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_set_input_picker_month(doc: *mut KuiDocument, year: core::ffi::c_int, month: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_set_input_picker_color_hsv(doc: *mut KuiDocument, h: f32, s: f32, v: f32) -> KuiStatus;
    pub fn kui_document_set_input_picker_time(doc: *mut KuiDocument, hour: core::ffi::c_int, minute: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_is_available(doc: *mut KuiDocument, out_available: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_play(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, options: *const KuiCAudioPlayOptions, out_handle: *mut u32) -> KuiStatus;
    pub fn kui_document_audio_stop(doc: *mut KuiDocument, handle: u32) -> KuiStatus;
    pub fn kui_document_audio_set_paused(doc: *mut KuiDocument, handle: u32, paused: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_set_volume(doc: *mut KuiDocument, handle: u32, volume: f32) -> KuiStatus;
    pub fn kui_document_audio_set_pan(doc: *mut KuiDocument, handle: u32, pan: f32) -> KuiStatus;
    pub fn kui_document_audio_set_pitch(doc: *mut KuiDocument, handle: u32, pitch: f32) -> KuiStatus;
    pub fn kui_document_audio_seek(doc: *mut KuiDocument, handle: u32, seconds: f64) -> KuiStatus;
    pub fn kui_document_audio_get_info(doc: *mut KuiDocument, handle: u32, out_info: *mut KuiCSoundInfo) -> KuiStatus;
    pub fn kui_document_audio_set_group(doc: *mut KuiDocument, group: KuiCAudioGroup, volume: f32, muted: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_get_group(doc: *mut KuiDocument, group: KuiCAudioGroup, out_volume: *mut f32, out_muted: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_stop_all(doc: *mut KuiDocument, group: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_set_all_paused(doc: *mut KuiDocument, group: core::ffi::c_int, paused: core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_audio_preload(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, count: u32, out_loaded: *mut u32) -> KuiStatus;
    pub fn kui_document_audio_unload(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_document_audio_get_counts(doc: *mut KuiDocument, group: core::ffi::c_int, out_active: *mut u32, out_pooled: *mut u32) -> KuiStatus;
    pub fn kui_media_load(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_media_set_loop(element: *mut KuiElement, loop_: core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_set_autoplay(element: *mut KuiElement, autoplay: core::ffi::c_int) -> KuiStatus;
    pub fn kui_media_get_playback_flags(element: *mut KuiElement, out_loop: *mut core::ffi::c_int, out_autoplay: *mut core::ffi::c_int, out_ended: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_lottie_stop(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_lottie_seek(element: *mut KuiElement, position: f64) -> KuiStatus;
    pub fn kui_lottie_set_speed(element: *mut KuiElement, speed: f32) -> KuiStatus;
    pub fn kui_lottie_set_loop(element: *mut KuiElement, loop_: core::ffi::c_int) -> KuiStatus;
    pub fn kui_lottie_get_state(element: *mut KuiElement, out_state: *mut KuiCLottieState) -> KuiStatus;
    pub fn kui_document_load_extension(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, out_loaded: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_unload_extension(doc: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_document_reload_extension(doc: *mut KuiDocument, path: *const core::ffi::c_char, len: usize, out_reloaded: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_document_unload_all_extensions(doc: *mut KuiDocument) -> KuiStatus;
    pub fn kui_document_watch_extensions(doc: *mut KuiDocument, directory: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_document_get_extension_state(doc: *mut KuiDocument, out_count: *mut usize, out_watching: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_stop_devtools(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_get_devtools_state(w: *mut KuiWindow, out_running: *mut core::ffi::c_int, out_port: *mut u16, out_clients: *mut u32) -> KuiStatus;
    pub fn kui_event_drag_set_data(event: *mut KuiEvent, format: *const core::ffi::c_char, format_len: usize, data: *const core::ffi::c_char, data_len: usize) -> KuiStatus;
    pub fn kui_event_drag_get_data(event: *mut KuiEvent, format: *const core::ffi::c_char, format_len: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_event_drag_clear_data(event: *mut KuiEvent, format: *const core::ffi::c_char, format_len: usize) -> KuiStatus;
    pub fn kui_event_drag_get_type_count(event: *mut KuiEvent, out_count: *mut usize) -> KuiStatus;
    pub fn kui_event_drag_get_type(event: *mut KuiEvent, index: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_event_drag_set_drop_effect(event: *mut KuiEvent, effect: KuiCDropEffect) -> KuiStatus;
    pub fn kui_event_drag_get_drop_effect(event: *mut KuiEvent, out_effect: *mut KuiCDropEffect) -> KuiStatus;
    pub fn kui_event_drag_set_effect_allowed(event: *mut KuiEvent, effect: KuiCEffectAllowed) -> KuiStatus;
    pub fn kui_event_drag_get_effect_allowed(event: *mut KuiEvent, out_effect: *mut KuiCEffectAllowed) -> KuiStatus;
    pub fn kui_script_engine_eval_value(engine: *mut KuiScriptEngine, code: *const core::ffi::c_char, code_len: usize, source: *const core::ffi::c_char, source_len: usize, out_success: *mut core::ffi::c_int, out_value: *mut KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_engine_eval_file(engine: *mut KuiScriptEngine, path: *const core::ffi::c_char, path_len: usize, out_success: *mut core::ffi::c_int, out_value: *mut KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_engine_call_function(engine: *mut KuiScriptEngine, name: *const core::ffi::c_char, name_len: usize, args: *const KuiCScriptValue, arg_count: usize, out_success: *mut core::ffi::c_int, out_value: *mut KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_engine_get_last_error(engine: *mut KuiScriptEngine, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_script_engine_register_function(engine: *mut KuiScriptEngine, name: *const core::ffi::c_char, name_len: usize, fn_: KuiScriptFunctionFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn) -> KuiStatus;
    pub fn kui_script_engine_set_global(engine: *mut KuiScriptEngine, name: *const core::ffi::c_char, name_len: usize, value: *const KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_engine_get_global(engine: *mut KuiScriptEngine, name: *const core::ffi::c_char, name_len: usize, out_value: *mut KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_engine_set_print_handler(engine: *mut KuiScriptEngine, fn_: KuiScriptPrintFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn) -> KuiStatus;
    pub fn kui_script_engine_set_memory_limit(engine: *mut KuiScriptEngine, bytes: usize) -> KuiStatus;
    pub fn kui_script_engine_set_instruction_limit(engine: *mut KuiScriptEngine, max_instructions: u64) -> KuiStatus;
    pub fn kui_script_engine_get_resource_usage(engine: *mut KuiScriptEngine, out_memory_bytes: *mut usize, out_memory_limit: *mut usize, out_instruction_limit: *mut u64) -> KuiStatus;
    pub fn kui_script_function_create(engine: *mut KuiScriptEngine, fn_: KuiScriptFunctionFn, user_data: *mut core::ffi::c_void, release_fn: KuiTaskFn, out_function: *mut *mut KuiScriptFunction) -> KuiStatus;
    pub fn kui_script_function_call(function: *mut KuiScriptFunction, args: *const KuiCScriptValue, arg_count: usize, out_success: *mut core::ffi::c_int, out_value: *mut KuiCScriptValue) -> KuiStatus;
    pub fn kui_script_function_retain(function: *mut KuiScriptFunction, out_owned: *mut *mut KuiScriptFunction) -> KuiStatus;
    pub fn kui_script_function_release(function: *mut KuiScriptFunction) -> KuiStatus;
    pub fn kui_document_define_custom_element_ex(document: *mut KuiDocument, tag_name: *const core::ffi::c_char, len: usize, definition: *const KuiCCustomElementDefinition) -> KuiStatus;
    pub fn kui_paint_get_info(paint: *mut KuiPaintContext, out_width: *mut f32, out_height: *mut f32, out_device_pixel_ratio: *mut f32) -> KuiStatus;
    pub fn kui_paint_fill_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32, rgba: u32) -> KuiStatus;
    pub fn kui_paint_fill_rounded_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32, radii: *const KuiCCornerRadii, rgba: u32) -> KuiStatus;
    pub fn kui_paint_stroke_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32, line_width: f32, rgba: u32) -> KuiStatus;
    pub fn kui_paint_stroke_rounded_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32, radii: *const KuiCCornerRadii, line_width: f32, rgba: u32) -> KuiStatus;
    pub fn kui_paint_draw_text(paint: *mut KuiPaintContext, text: *const core::ffi::c_char, len: usize, x: f32, baseline_y: f32, style: *const KuiCTextStyle, rgba: u32) -> KuiStatus;
    pub fn kui_paint_draw_pixels(paint: *mut KuiPaintContext, pixels: *const u32, width: u32, height: u32, x: f32, y: f32, dest_width: f32, dest_height: f32) -> KuiStatus;
    pub fn kui_paint_draw_image(paint: *mut KuiPaintContext, path: *const core::ffi::c_char, len: usize, src: *const KuiRect, dest: *const KuiRect) -> KuiStatus;
    pub fn kui_paint_draw_decoration_line(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, thickness: f32, style: KuiCDecorationStyle, rgba: u32) -> KuiStatus;
    pub fn kui_paint_save(paint: *mut KuiPaintContext) -> KuiStatus;
    pub fn kui_paint_restore(paint: *mut KuiPaintContext) -> KuiStatus;
    pub fn kui_paint_translate(paint: *mut KuiPaintContext, dx: f32, dy: f32) -> KuiStatus;
    pub fn kui_paint_scale(paint: *mut KuiPaintContext, sx: f32, sy: f32) -> KuiStatus;
    pub fn kui_paint_rotate(paint: *mut KuiPaintContext, degrees: f32) -> KuiStatus;
    pub fn kui_paint_set_opacity(paint: *mut KuiPaintContext, opacity: f32) -> KuiStatus;
    pub fn kui_paint_clip_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32) -> KuiStatus;
    pub fn kui_paint_clip_rounded_rect(paint: *mut KuiPaintContext, x: f32, y: f32, width: f32, height: f32, radii: *const KuiCCornerRadii) -> KuiStatus;
    pub fn kui_element_measure_text(element: *mut KuiElement, text: *const core::ffi::c_char, len: usize, style: *const KuiCTextStyle, out_metrics: *mut KuiCTextMetrics) -> KuiStatus;
    pub fn kui_element_request_repaint(element: *mut KuiElement) -> KuiStatus;
    pub fn kui_element_invalidate_rect(element: *mut KuiElement, x: f32, y: f32, width: f32, height: f32) -> KuiStatus;
    pub fn kui_devtools_dispatch_ime_event(document: *mut KuiDocument, type_: KuiCImeEventType, text: *const core::ffi::c_char, len: usize) -> KuiStatus;
    pub fn kui_document_register_builtin_components(document: *mut KuiDocument) -> KuiStatus;
    pub fn kui_theme_manager_create(out_manager: *mut *mut KuiThemeManager) -> KuiStatus;
    pub fn kui_theme_manager_release(manager: *mut KuiThemeManager) -> KuiStatus;
    pub fn kui_theme_manager_register(manager: *mut KuiThemeManager, name: *const core::ffi::c_char, name_len: usize, variables: *const KuiCKeyValue, variable_count: usize) -> KuiStatus;
    pub fn kui_theme_manager_apply(manager: *mut KuiThemeManager, name: *const core::ffi::c_char, name_len: usize, document: *mut KuiDocument) -> KuiStatus;
    pub fn kui_theme_manager_get_active(manager: *mut KuiThemeManager, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_theme_manager_get_theme_count(manager: *mut KuiThemeManager, out_count: *mut usize) -> KuiStatus;
    pub fn kui_theme_manager_get_theme_name(manager: *mut KuiThemeManager, index: usize, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_theme_manager_get_variable_count(manager: *mut KuiThemeManager, theme: *const core::ffi::c_char, theme_len: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_theme_manager_get_variable(manager: *mut KuiThemeManager, theme: *const core::ffi::c_char, theme_len: usize, index: usize, name_buf: *mut core::ffi::c_char, name_capacity: usize, out_name_len: *mut usize, value_buf: *mut core::ffi::c_char, value_capacity: usize, out_value_len: *mut usize) -> KuiStatus;
    pub fn kui_virtual_list_create(container: *mut KuiElement, config: *const KuiCVirtualListConfig, out_list: *mut *mut KuiVirtualList) -> KuiStatus;
    pub fn kui_virtual_list_release(list: *mut KuiVirtualList) -> KuiStatus;
    pub fn kui_virtual_list_get_state(list: *mut KuiVirtualList, out_state: *mut KuiCVirtualListState) -> KuiStatus;
    pub fn kui_virtual_list_set_total_items(list: *mut KuiVirtualList, count: u32) -> KuiStatus;
    pub fn kui_virtual_list_insert_items(list: *mut KuiVirtualList, index: u32, count: u32) -> KuiStatus;
    pub fn kui_virtual_list_remove_items(list: *mut KuiVirtualList, index: u32, count: u32) -> KuiStatus;
    pub fn kui_virtual_list_refresh(list: *mut KuiVirtualList) -> KuiStatus;
    pub fn kui_virtual_list_scroll_to_index(list: *mut KuiVirtualList, index: u32) -> KuiStatus;
    pub fn kui_virtual_list_scroll_to_top(list: *mut KuiVirtualList) -> KuiStatus;
    pub fn kui_virtual_list_scroll_to_bottom(list: *mut KuiVirtualList) -> KuiStatus;
    pub fn kui_virtual_list_set_scroll_unit(list: *mut KuiVirtualList, unit: KuiCScrollUnit) -> KuiStatus;
    pub fn kui_virtual_list_set_focused_index(list: *mut KuiVirtualList, index: i32) -> KuiStatus;
    pub fn kui_virtual_list_move_focus(list: *mut KuiVirtualList, delta: i32) -> KuiStatus;
    pub fn kui_list_view_create(container: *mut KuiElement, config: *const KuiCListViewConfig, out_list: *mut *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_release(list: *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_select(list: *mut KuiListView, index: u32) -> KuiStatus;
    pub fn kui_list_view_deselect(list: *mut KuiListView, index: u32) -> KuiStatus;
    pub fn kui_list_view_clear_selection(list: *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_select_range(list: *mut KuiListView, start: u32, end: u32) -> KuiStatus;
    pub fn kui_list_view_is_selected(list: *mut KuiListView, index: u32, out_selected: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_list_view_get_selected(list: *mut KuiListView, buf: *mut u32, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_list_view_get_focused_index(list: *mut KuiListView, out_index: *mut i32) -> KuiStatus;
    pub fn kui_list_view_set_item_count(list: *mut KuiListView, count: u32) -> KuiStatus;
    pub fn kui_list_view_refresh(list: *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_scroll_to_index(list: *mut KuiListView, index: u32) -> KuiStatus;
    pub fn kui_list_view_scroll_to_top(list: *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_scroll_to_bottom(list: *mut KuiListView) -> KuiStatus;
    pub fn kui_list_view_set_scroll_unit(list: *mut KuiListView, unit: KuiCScrollUnit) -> KuiStatus;
    pub fn kui_list_view_get_state(list: *mut KuiListView, out_state: *mut KuiCVirtualListState) -> KuiStatus;
    pub fn kui_tree_view_create(out_tree: *mut *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_release(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_add_node(tree: *mut KuiTreeView, text: *const core::ffi::c_char, text_len: usize, icon: *const core::ffi::c_char, icon_len: usize, out_index: *mut u32) -> KuiStatus;
    pub fn kui_tree_view_add_child(tree: *mut KuiTreeView, parent: u32, text: *const core::ffi::c_char, text_len: usize, icon: *const core::ffi::c_char, icon_len: usize, out_index: *mut u32) -> KuiStatus;
    pub fn kui_tree_view_get_node_count(tree: *mut KuiTreeView, out_count: *mut u32) -> KuiStatus;
    pub fn kui_tree_view_get_node(tree: *mut KuiTreeView, index: u32, out_node: *mut KuiCTreeNode) -> KuiStatus;
    pub fn kui_tree_view_get_node_child(tree: *mut KuiTreeView, index: u32, position: usize, out_child: *mut u32) -> KuiStatus;
    pub fn kui_tree_view_set_node(tree: *mut KuiTreeView, index: u32, text: *const core::ffi::c_char, text_len: usize, icon: *const core::ffi::c_char, icon_len: usize, disabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_tree_view_setup(tree: *mut KuiTreeView, container: *mut KuiElement, config: *const KuiCTreeViewConfig) -> KuiStatus;
    pub fn kui_tree_view_expand(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_collapse(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_toggle(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_expand_all(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_collapse_all(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_select(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_clear_selection(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_toggle_select(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_select_range(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_select_all(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_is_selected(tree: *mut KuiTreeView, index: u32, out_selected: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_tree_view_get_selected_index(tree: *mut KuiTreeView, out_index: *mut i32) -> KuiStatus;
    pub fn kui_tree_view_get_selected(tree: *mut KuiTreeView, buf: *mut u32, buf_capacity: usize, out_count: *mut usize) -> KuiStatus;
    pub fn kui_tree_view_refresh(tree: *mut KuiTreeView) -> KuiStatus;
    pub fn kui_tree_view_scroll_to_node(tree: *mut KuiTreeView, index: u32) -> KuiStatus;
    pub fn kui_tree_view_set_scroll_unit(tree: *mut KuiTreeView, unit: KuiCScrollUnit) -> KuiStatus;
    pub fn kui_tree_view_get_visible_row_count(tree: *mut KuiTreeView, out_count: *mut u32) -> KuiStatus;
    pub fn kui_document_set_style_profile(document: *mut KuiDocument, profile: KuiCStyleProfile) -> KuiStatus;
    pub fn kui_document_get_style_profile(document: *mut KuiDocument, out_profile: *mut KuiCStyleProfile) -> KuiStatus;
    pub fn kui_element_start_momentum_scroll(el: *mut KuiElement, velocity_x: f32, velocity_y: f32) -> KuiStatus;
    pub fn kui_element_scroll_into_view(el: *mut KuiElement, align_to_top: core::ffi::c_int) -> KuiStatus;
    pub fn kui_event_wheel_get_delta(evt: *mut KuiEvent, out_delta_x: *mut f32, out_delta_y: *mut f32) -> KuiStatus;
    pub fn kui_script_engine_tick_timers(engine: *mut KuiScriptEngine, now_ms: f64) -> KuiStatus;
    pub fn kui_document_set_base_path(doc: *mut KuiDocument, path: *const core::ffi::c_char, path_len: usize) -> KuiStatus;
    pub fn kui_document_get_base_path(doc: *mut KuiDocument, buf: *mut core::ffi::c_char, buf_capacity: usize, out_len: *mut usize) -> KuiStatus;
    pub fn kui_document_set_render_device_pixel_ratio(doc: *mut KuiDocument, dpr: f32) -> KuiStatus;
    pub fn kui_window_create_ex(props: *const KuiWindowPropsEx, out: *mut *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_set_enabled(w: *mut KuiWindow, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_is_enabled(w: *mut KuiWindow, out_enabled: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_window_process_frame(w: *mut KuiWindow) -> KuiStatus;
    pub fn kui_window_set_devtools_token_auth(w: *mut KuiWindow, enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_platform_disable_ime() -> KuiStatus;
    pub fn kui_console_set_enabled(enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_console_set_stderr_output(enabled: core::ffi::c_int) -> KuiStatus;
    pub fn kui_console_set_min_level(level: KuiCLogLevel) -> KuiStatus;
    pub fn kui_stack_trace_install(config: *const KuiCStackTraceConfig, out_installed: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_stack_trace_uninstall() -> KuiStatus;
    pub fn kui_crash_reporter_install(config: *const KuiCCrashReporterConfig, out_installed: *mut core::ffi::c_int) -> KuiStatus;
    pub fn kui_crash_reporter_uninstall() -> KuiStatus;
}
