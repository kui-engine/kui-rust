//! 弹出层、`<input>` 选择器与游戏音频（C ABI 第 12 节，3.61.8）。全部只能在 UI 线程使用。

use std::ffi::{c_char, c_void, CString};
use std::ptr;

use crate::framework::{register, release_token};
use crate::{bytes, callbacks, check, error, sys, Document, Element, MenuItem, Rect, Result};

// ---------------------------------------------------------------------------
// 弹出层
// ---------------------------------------------------------------------------

/// 弹出层种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupType {
    /// 提示框。
    Tooltip,
    /// 右键菜单。
    ContextMenu,
    /// 自定义弹出层。
    Custom,
}

impl PopupType {
    fn raw(self) -> sys::KuiCPopupType {
        match self {
            PopupType::Tooltip => sys::KUI_C_POPUP_TOOLTIP,
            PopupType::ContextMenu => sys::KUI_C_POPUP_CONTEXT_MENU,
            PopupType::Custom => sys::KUI_C_POPUP_CUSTOM,
        }
    }
}

/// 提示框相对锚点的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupPosition {
    /// 上方。
    Top,
    /// 下方。
    Bottom,
    /// 左侧。
    Left,
    /// 右侧。
    Right,
}

/// 弹出层呈现方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupMode {
    /// 画在页面上（默认）。
    Overlay,
    /// 原生弹出窗口。
    NativeWindow,
}

/// 提示框配色，颜色为 `0xRRGGBBAA`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipTheme {
    /// 背景。
    pub background: u32,
    /// 文字。
    pub foreground: u32,
    /// 边框。
    pub border: u32,
    /// 圆角半径。
    pub radius: f32,
}

type PopupMenuFn = Box<dyn FnMut(u32)>;

unsafe extern "C" fn popup_menu_activate(id: u32, user_data: *mut c_void) {
    callbacks::invoke::<PopupMenuFn>(user_data as usize, |callback| callback(id));
}

/// 文档的弹出层（提示框、右键菜单）。
pub struct Popups<'doc> {
    document: &'doc Document,
}

impl Document {
    /// 文档的弹出层管理。
    pub fn popups(&self) -> Popups<'_> {
        Popups { document: self }
    }
    /// 文档的 `<input>` 选择器。
    pub fn input_picker(&self) -> InputPicker<'_> {
        InputPicker { document: self }
    }
    /// 文档的音频管理（需要音频媒体插件）。
    pub fn audio(&self) -> Audio<'_> {
        Audio { document: self }
    }
}

impl Popups<'_> {
    fn raw(&self) -> *mut sys::KuiDocument {
        self.document.raw.as_ptr()
    }
    /// 在锚点矩形（视口坐标）旁显示提示框，返回 id。
    pub fn show_tooltip(&self, text: &str, anchor: Rect, position: PopupPosition) -> Result<u32> {
        let (raw, len) = bytes(text);
        let rect = sys::KuiRect {
            x: anchor.x,
            y: anchor.y,
            width: anchor.width,
            height: anchor.height,
        };
        let raw_position = match position {
            PopupPosition::Top => sys::KUI_C_POPUP_TOP,
            PopupPosition::Bottom => sys::KUI_C_POPUP_BOTTOM,
            PopupPosition::Left => sys::KUI_C_POPUP_LEFT,
            PopupPosition::Right => sys::KUI_C_POPUP_RIGHT,
        };
        let mut id = 0;
        check(
            unsafe {
                sys::kui_document_show_tooltip(self.raw(), raw, len, &rect, raw_position, &mut id)
            },
            "Popups::show_tooltip",
        )?;
        Ok(id)
    }
    /// 在 `(x, y)` 显示右键菜单（只用 `MenuItem` 的 `text` / `id` / `separator` / `disabled`），
    /// 选中某项时以它的 id 调用 `on_activate`；菜单关闭后闭包释放。
    pub fn show_context_menu<F>(
        &self,
        x: f32,
        y: f32,
        items: &[MenuItem],
        on_activate: F,
    ) -> Result<u32>
    where
        F: FnMut(u32) + 'static,
    {
        let texts = items
            .iter()
            .map(|item| {
                CString::new(item.text.as_str())
                    .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, "Popups::show_context_menu"))
            })
            .collect::<Result<Vec<_>>>()?;
        let raw_items: Vec<sys::KuiCMenuItem> = items
            .iter()
            .zip(&texts)
            .map(|(item, text)| sys::KuiCMenuItem {
                text: if item.separator {
                    ptr::null()
                } else {
                    text.as_ptr()
                },
                text_len: if item.separator { 0 } else { item.text.len() },
                shortcut_label: ptr::null(),
                shortcut_label_len: 0,
                id: item.id,
                separator: item.separator as core::ffi::c_int,
                disabled: item.disabled as core::ffi::c_int,
                checkable: 0,
                checked: 0,
                children: ptr::null(),
                child_count: 0,
            })
            .collect();
        let token = register::<PopupMenuFn>(Box::new(on_activate));
        let mut id = 0;
        let status = unsafe {
            sys::kui_document_show_context_menu(
                self.raw(),
                x,
                y,
                raw_items.as_ptr(),
                raw_items.len(),
                Some(popup_menu_activate),
                token as *mut c_void,
                Some(release_token),
                &mut id,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "Popups::show_context_menu")?;
        Ok(id)
    }
    /// 关闭一个弹出层（未知 id 什么也不做）。
    pub fn dismiss(&self, id: u32) -> Result<()> {
        let status = unsafe { sys::kui_document_dismiss_popup(self.raw(), id) };
        callbacks::drain_released();
        check(status, "Popups::dismiss")
    }
    /// 关闭全部弹出层。
    pub fn dismiss_all(&self) -> Result<()> {
        let status = unsafe { sys::kui_document_dismiss_all_popups(self.raw()) };
        callbacks::drain_released();
        check(status, "Popups::dismiss_all")
    }
    /// 关闭某一种弹出层。
    pub fn dismiss_type(&self, kind: PopupType) -> Result<()> {
        let status = unsafe { sys::kui_document_dismiss_popups_of_type(self.raw(), kind.raw()) };
        callbacks::drain_released();
        check(status, "Popups::dismiss_type")
    }
    /// 是否有弹出层在显示。
    pub fn any(&self) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_document_has_popups(self.raw(), &mut value) },
            "Popups::any",
        )?;
        Ok(value != 0)
    }
    /// 某一种弹出层是否在显示。
    pub fn has(&self, kind: PopupType) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_document_has_popup_of_type(self.raw(), kind.raw(), &mut value) },
            "Popups::has",
        )?;
        Ok(value != 0)
    }
    /// 悬停多久后显示 title 提示（毫秒，默认 500）。
    pub fn set_tooltip_delay(&self, ms: f64) -> Result<()> {
        check(
            unsafe { sys::kui_document_set_tooltip_delay(self.raw(), ms) },
            "Popups::set_tooltip_delay",
        )
    }
    /// 提示延迟（毫秒）。
    pub fn tooltip_delay(&self) -> Result<f64> {
        let mut value = 0.0;
        check(
            unsafe { sys::kui_document_get_tooltip_delay(self.raw(), &mut value) },
            "Popups::tooltip_delay",
        )?;
        Ok(value)
    }
    /// 设置提示框配色。
    pub fn set_tooltip_theme(&self, theme: TooltipTheme) -> Result<()> {
        let raw = sys::KuiCTooltipTheme {
            background: theme.background,
            foreground: theme.foreground,
            border: theme.border,
            radius: theme.radius,
        };
        check(
            unsafe { sys::kui_document_set_tooltip_theme(self.raw(), &raw) },
            "Popups::set_tooltip_theme",
        )
    }
    /// 提示框配色。
    pub fn tooltip_theme(&self) -> Result<TooltipTheme> {
        let mut raw = sys::KuiCTooltipTheme {
            background: 0,
            foreground: 0,
            border: 0,
            radius: 0.0,
        };
        check(
            unsafe { sys::kui_document_get_tooltip_theme(self.raw(), &mut raw) },
            "Popups::tooltip_theme",
        )?;
        Ok(TooltipTheme {
            background: raw.background,
            foreground: raw.foreground,
            border: raw.border,
            radius: raw.radius,
        })
    }
    /// 选择弹出层的呈现方式。
    pub fn set_mode(&self, mode: PopupMode) -> Result<()> {
        let raw = match mode {
            PopupMode::Overlay => sys::KUI_C_POPUP_MODE_OVERLAY,
            PopupMode::NativeWindow => sys::KUI_C_POPUP_MODE_NATIVE_WINDOW,
        };
        check(
            unsafe { sys::kui_document_set_popup_mode(self.raw(), raw) },
            "Popups::set_mode",
        )
    }
    /// 开关内置的默认右键菜单（输入框：剪切 / 复制 / 粘贴 / 全选；其它地方：复制 / 全选）。默认关闭：右键和
    /// 菜单键照常派发 `contextmenu` 事件，自己弹的菜单不受影响。对整个文档生效，换页后保持；页面也可以写
    /// `<html context-menu>` 开启。开了检查器而默认菜单关闭时，右键只出「检查元素」。
    pub fn set_default_context_menu_enabled(&self, enabled: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_document_set_default_context_menu_enabled(self.raw(), i32::from(enabled))
            },
            "Popups::set_default_context_menu_enabled",
        )
    }
    /// 是否用代码开启了默认右键菜单（不含页面上的 `<html context-menu>`）。
    pub fn default_context_menu_enabled(&self) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_document_get_default_context_menu_enabled(self.raw(), &mut value) },
            "Popups::default_context_menu_enabled",
        )?;
        Ok(value != 0)
    }
}

// ---------------------------------------------------------------------------
// <input> 选择器
// ---------------------------------------------------------------------------

/// 选择器面板种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    /// 关闭；打开时表示按 input 的 type 选面板。
    None,
    /// 日期。
    Date,
    /// 时间。
    Time,
    /// 本地日期时间。
    DateTimeLocal,
    /// 颜色。
    Color,
}

impl PickerMode {
    fn raw(self) -> sys::KuiCPickerMode {
        match self {
            PickerMode::None => sys::KUI_C_PICKER_NONE,
            PickerMode::Date => sys::KUI_C_PICKER_DATE,
            PickerMode::Time => sys::KUI_C_PICKER_TIME,
            PickerMode::DateTimeLocal => sys::KUI_C_PICKER_DATETIME_LOCAL,
            PickerMode::Color => sys::KUI_C_PICKER_COLOR,
        }
    }
    fn from_raw(raw: sys::KuiCPickerMode) -> Self {
        match raw {
            sys::KUI_C_PICKER_DATE => PickerMode::Date,
            sys::KUI_C_PICKER_TIME => PickerMode::Time,
            sys::KUI_C_PICKER_DATETIME_LOCAL => PickerMode::DateTimeLocal,
            sys::KUI_C_PICKER_COLOR => PickerMode::Color,
            _ => PickerMode::None,
        }
    }
}

/// 选择器当前状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputPickerState {
    /// 是否打开。
    pub open: bool,
    /// 是否启用。
    pub enabled: bool,
    /// 当前面板。
    pub mode: PickerMode,
    /// 日历页年份。
    pub visible_year: i32,
    /// 日历页月份（1-12）。
    pub visible_month: i32,
    /// 色相（0-360）。
    pub color_h: f32,
    /// 饱和度（0-1）。
    pub color_s: f32,
    /// 明度（0-1）。
    pub color_v: f32,
    /// 时。
    pub hour: i32,
    /// 分。
    pub minute: i32,
}

impl Element<'_> {
    /// 为 `<input>` 打开选择器；`PickerMode::None` 按 type 选面板。没有面板的类型或选择器被禁用时返回
    /// `KUI_ERR_NOT_SUPPORTED`，非 `<input>` 元素返回 `KUI_ERR_INVALID_ARG`。
    pub fn open_input_picker(&self, mode: PickerMode) -> Result<()> {
        check(
            unsafe { sys::kui_element_open_input_picker(self.raw.as_ptr(), mode.raw()) },
            "Element::open_input_picker",
        )
    }
}

/// 文档的 `<input>` 日期 / 时间 / 取色面板。
pub struct InputPicker<'doc> {
    document: &'doc Document,
}

impl<'doc> InputPicker<'doc> {
    fn raw(&self) -> *mut sys::KuiDocument {
        self.document.raw.as_ptr()
    }
    /// 关闭选择器。
    pub fn close(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_close_input_picker(self.raw()) },
            "InputPicker::close",
        )
    }
    /// 把面板的值写回 input（触发 input / change）并关闭。
    pub fn commit(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_commit_input_picker(self.raw()) },
            "InputPicker::commit",
        )
    }
    /// 当前状态。
    pub fn state(&self) -> Result<InputPickerState> {
        let mut raw = sys::KuiCInputPickerState {
            open: 0,
            enabled: 0,
            mode: sys::KUI_C_PICKER_NONE,
            visible_year: 0,
            visible_month: 0,
            color_h: 0.0,
            color_s: 0.0,
            color_v: 0.0,
            hour: 0,
            minute: 0,
        };
        check(
            unsafe { sys::kui_document_get_input_picker_state(self.raw(), &mut raw) },
            "InputPicker::state",
        )?;
        Ok(InputPickerState {
            open: raw.open != 0,
            enabled: raw.enabled != 0,
            mode: PickerMode::from_raw(raw.mode),
            visible_year: raw.visible_year,
            visible_month: raw.visible_month,
            color_h: raw.color_h,
            color_s: raw.color_s,
            color_v: raw.color_v,
            hour: raw.hour,
            minute: raw.minute,
        })
    }
    /// 选择器所属的 input；关闭时为 `None`。
    pub fn target(&self) -> Result<Option<Element<'doc>>> {
        let mut raw = ptr::null_mut();
        let status = unsafe { sys::kui_document_get_input_picker_target(self.raw(), &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "InputPicker::target")?;
        Element::from_raw(raw, self.document).map(Some)
    }
    /// 启用或禁用选择器。
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_document_set_input_picker_enabled(self.raw(), enabled as core::ffi::c_int)
            },
            "InputPicker::set_enabled",
        )
    }
    /// 切换日历页（月份 1-12）。
    pub fn set_month(&self, year: i32, month: i32) -> Result<()> {
        check(
            unsafe { sys::kui_document_set_input_picker_month(self.raw(), year, month) },
            "InputPicker::set_month",
        )
    }
    /// 设置取色面板的颜色（色相 0-360，饱和度 / 明度 0-1）。
    pub fn set_color_hsv(&self, h: f32, s: f32, v: f32) -> Result<()> {
        check(
            unsafe { sys::kui_document_set_input_picker_color_hsv(self.raw(), h, s, v) },
            "InputPicker::set_color_hsv",
        )
    }
    /// 设置时间面板（0-23 时、0-59 分）。
    pub fn set_time(&self, hour: i32, minute: i32) -> Result<()> {
        check(
            unsafe { sys::kui_document_set_input_picker_time(self.raw(), hour, minute) },
            "InputPicker::set_time",
        )
    }
}

// ---------------------------------------------------------------------------
// 游戏音频
// ---------------------------------------------------------------------------

/// 音频分组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioGroup {
    /// 主音量。
    Master,
    /// 音乐。
    Music,
    /// 音效。
    Sfx,
    /// 界面音。
    Ui,
}

impl AudioGroup {
    fn raw(self) -> sys::KuiCAudioGroup {
        match self {
            AudioGroup::Master => sys::KUI_C_AUDIO_MASTER,
            AudioGroup::Music => sys::KUI_C_AUDIO_MUSIC,
            AudioGroup::Sfx => sys::KUI_C_AUDIO_SFX,
            AudioGroup::Ui => sys::KUI_C_AUDIO_UI,
        }
    }
    fn from_raw(raw: sys::KuiCAudioGroup) -> Self {
        match raw {
            sys::KUI_C_AUDIO_MASTER => AudioGroup::Master,
            sys::KUI_C_AUDIO_MUSIC => AudioGroup::Music,
            sys::KUI_C_AUDIO_UI => AudioGroup::Ui,
            _ => AudioGroup::Sfx,
        }
    }
}

fn group_or_all(group: Option<AudioGroup>) -> core::ffi::c_int {
    group.map_or(-1, |g| g.raw())
}

/// 播放选项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioPlayOptions {
    /// 分组。
    pub group: AudioGroup,
    /// 音量（0-1，分组 / 主音量缩放之前）。
    pub volume: f32,
    /// 声像（-1 左到 1 右）。
    pub pan: f32,
    /// 播放速度倍率。
    pub pitch: f32,
    /// 循环。
    pub looping: bool,
    /// 只加载不播放。
    pub preload_only: bool,
}

impl Default for AudioPlayOptions {
    fn default() -> Self {
        Self {
            group: AudioGroup::Sfx,
            volume: 1.0,
            pan: 0.0,
            pitch: 1.0,
            looping: false,
            preload_only: false,
        }
    }
}

/// 一个声音的状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundInfo {
    /// 分组。
    pub group: AudioGroup,
    /// 音量。
    pub volume: f32,
    /// 声像。
    pub pan: f32,
    /// 音高。
    pub pitch: f32,
    /// 正在播放。
    pub playing: bool,
    /// 循环。
    pub looping: bool,
    /// 当前位置（秒）。
    pub current_time: f64,
    /// 时长（秒）。
    pub duration: f64,
}

/// 文档的音频管理。
pub struct Audio<'doc> {
    document: &'doc Document,
}

impl Audio<'_> {
    fn raw(&self) -> *mut sys::KuiDocument {
        self.document.raw.as_ptr()
    }
    /// 是否有音频设备与媒体插件。
    pub fn available(&self) -> Result<bool> {
        let mut value = 0;
        check(
            unsafe { sys::kui_document_audio_is_available(self.raw(), &mut value) },
            "Audio::available",
        )?;
        Ok(value != 0)
    }
    /// 播放文件并返回句柄。音频不可用返回 `KUI_ERR_NOT_SUPPORTED`，文件放不了返回 `KUI_ERR_IO`。
    pub fn play(&self, path: &str, options: &AudioPlayOptions) -> Result<u32> {
        let (raw, len) = bytes(path);
        let raw_options = sys::KuiCAudioPlayOptions {
            group: options.group.raw(),
            volume: options.volume,
            pan: options.pan,
            pitch: options.pitch,
            loop_: options.looping as core::ffi::c_int,
            preload_only: options.preload_only as core::ffi::c_int,
        };
        let mut handle = 0;
        check(
            unsafe {
                sys::kui_document_audio_play(self.raw(), raw, len, &raw_options, &mut handle)
            },
            "Audio::play",
        )?;
        Ok(handle)
    }
    /// 停止一个声音（未知句柄什么也不做）。
    pub fn stop(&self, handle: u32) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_stop(self.raw(), handle) },
            "Audio::stop",
        )
    }
    /// 暂停或继续一个声音。
    pub fn set_paused(&self, handle: u32, paused: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_document_audio_set_paused(self.raw(), handle, paused as core::ffi::c_int)
            },
            "Audio::set_paused",
        )
    }
    /// 设置音量。
    pub fn set_volume(&self, handle: u32, volume: f32) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_set_volume(self.raw(), handle, volume) },
            "Audio::set_volume",
        )
    }
    /// 设置声像。
    pub fn set_pan(&self, handle: u32, pan: f32) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_set_pan(self.raw(), handle, pan) },
            "Audio::set_pan",
        )
    }
    /// 设置音高（播放速度倍率）。
    pub fn set_pitch(&self, handle: u32, pitch: f32) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_set_pitch(self.raw(), handle, pitch) },
            "Audio::set_pitch",
        )
    }
    /// 跳到某个时刻（秒）。
    pub fn seek(&self, handle: u32, seconds: f64) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_seek(self.raw(), handle, seconds) },
            "Audio::seek",
        )
    }
    /// 一个声音的状态；未知或已结束的句柄为 `None`。
    pub fn info(&self, handle: u32) -> Result<Option<SoundInfo>> {
        let mut raw = sys::KuiCSoundInfo {
            group: sys::KUI_C_AUDIO_SFX,
            volume: 0.0,
            pan: 0.0,
            pitch: 0.0,
            playing: 0,
            looping: 0,
            current_time: 0.0,
            duration: 0.0,
        };
        let status = unsafe { sys::kui_document_audio_get_info(self.raw(), handle, &mut raw) };
        if status == sys::KUI_ERR_NOT_FOUND {
            return Ok(None);
        }
        check(status, "Audio::info")?;
        Ok(Some(SoundInfo {
            group: AudioGroup::from_raw(raw.group),
            volume: raw.volume,
            pan: raw.pan,
            pitch: raw.pitch,
            playing: raw.playing != 0,
            looping: raw.looping != 0,
            current_time: raw.current_time,
            duration: raw.duration,
        }))
    }
    /// 设置一个分组的音量（0-1）与静音。
    pub fn set_group(&self, group: AudioGroup, volume: f32, muted: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_document_audio_set_group(
                    self.raw(),
                    group.raw(),
                    volume,
                    muted as core::ffi::c_int,
                )
            },
            "Audio::set_group",
        )
    }
    /// 一个分组的 `(音量, 是否静音)`。
    pub fn group(&self, group: AudioGroup) -> Result<(f32, bool)> {
        let (mut volume, mut muted) = (0.0f32, 0);
        check(
            unsafe {
                sys::kui_document_audio_get_group(self.raw(), group.raw(), &mut volume, &mut muted)
            },
            "Audio::group",
        )?;
        Ok((volume, muted != 0))
    }
    /// 停止一个分组（`None` = 全部分组）的全部声音。
    pub fn stop_all(&self, group: Option<AudioGroup>) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_stop_all(self.raw(), group_or_all(group)) },
            "Audio::stop_all",
        )
    }
    /// 暂停或继续一个分组（`None` = 全部分组）的全部声音。
    pub fn set_all_paused(&self, group: Option<AudioGroup>, paused: bool) -> Result<()> {
        check(
            unsafe {
                sys::kui_document_audio_set_all_paused(
                    self.raw(),
                    group_or_all(group),
                    paused as core::ffi::c_int,
                )
            },
            "Audio::set_all_paused",
        )
    }
    /// 预加载 `count` 个实例，返回成功的个数。
    pub fn preload(&self, path: &str, count: u32) -> Result<u32> {
        let (raw, len) = bytes(path);
        let mut loaded = 0;
        check(
            unsafe { sys::kui_document_audio_preload(self.raw(), raw, len, count, &mut loaded) },
            "Audio::preload",
        )?;
        Ok(loaded)
    }
    /// 卸载一个文件的全部实例。
    pub fn unload(&self, path: &str) -> Result<()> {
        let (raw, len) = bytes(path);
        check(
            unsafe { sys::kui_document_audio_unload(self.raw(), raw, len) },
            "Audio::unload",
        )
    }
    /// 卸载全部。
    pub fn unload_all(&self) -> Result<()> {
        check(
            unsafe { sys::kui_document_audio_unload(self.raw(), ptr::null::<c_char>(), 0) },
            "Audio::unload_all",
        )
    }
    /// 一个分组（`None` = 全部分组）正在播放的数量，以及整个实例池的大小。
    pub fn counts(&self, group: Option<AudioGroup>) -> Result<(u32, u32)> {
        let (mut active, mut pooled) = (0, 0);
        check(
            unsafe {
                sys::kui_document_audio_get_counts(
                    self.raw(),
                    group_or_all(group),
                    &mut active,
                    &mut pooled,
                )
            },
            "Audio::counts",
        )?;
        Ok((active, pooled))
    }
}
