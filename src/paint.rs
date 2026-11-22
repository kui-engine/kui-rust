//! 自定义元素的自绘、tick 与输入（C ABI 第 17 节，3.61.8）用到的类型：绘制上下文、字体与圆角、
//! 输入事件，以及元素的重绘请求、文字测量和输入法注入。回调本身在
//! [`crate::CustomElementDefinition`] 里设置。

use std::ffi::{c_char, CString};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

use crate::{bytes, check, error, sys, Document, Element, Rect, Result};

/// 输入事件的修饰键位（与 `dispatch_mouse_event` / `dispatch_key_event` 的位不同）。
pub mod input_modifiers {
    /// Ctrl。
    pub const CTRL: i32 = 1;
    /// Shift。
    pub const SHIFT: i32 = 2;
    /// Alt。
    pub const ALT: i32 = 4;
    /// Meta（Windows 键 / Command）。
    pub const META: i32 = 8;
}

/// 自定义元素收到的键盘事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomKeyEvent {
    /// 按下（`true`）或松开。
    pub down: bool,
    /// 按键含义，例如 `"a"`、`"Enter"`。
    pub key: String,
    /// 物理键，例如 `"KeyA"`。
    pub code: String,
    /// [`input_modifiers`] 位。
    pub modifiers: i32,
    /// 是否系统自动重复。
    pub repeat: bool,
}

/// 鼠标事件的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomMouseKind {
    /// 按下。
    Down,
    /// 松开。
    Up,
    /// 移动。
    Move,
    /// 进入元素。
    Enter,
    /// 离开元素。
    Leave,
    /// 本版本不认识的值。
    Other(i32),
}

/// 自定义元素收到的鼠标事件，坐标在内容框里。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomMouseEvent {
    /// 种类。
    pub kind: CustomMouseKind,
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
    /// 0 左、1 中、2 右；移动 / 进入 / 离开为 -1。
    pub button: i32,
    /// 按下的键。
    pub buttons: i32,
    /// [`input_modifiers`] 位。
    pub modifiers: i32,
    /// 点击次数。
    pub click_count: i32,
}

/// 自定义元素收到的滚轮事件，坐标在内容框里。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomWheelEvent {
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
    /// 横向滚动量。
    pub delta_x: f32,
    /// 纵向滚动量。
    pub delta_y: f32,
    /// [`input_modifiers`] 位。
    pub modifiers: i32,
}

/// 输入法组合的阶段（也用于 [`Document::dispatch_ime_event`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeEventKind {
    /// 开始组合。
    Start,
    /// 组合文字变化。
    Update,
    /// 确认（最终文字）。
    Commit,
    /// 取消。
    Cancel,
    /// 本版本不认识的值。
    Other(i32),
}

impl ImeEventKind {
    fn raw(self) -> sys::KuiCImeEventType {
        match self {
            ImeEventKind::Start => sys::KUI_C_IME_START,
            ImeEventKind::Update => sys::KUI_C_IME_UPDATE,
            ImeEventKind::Commit => sys::KUI_C_IME_COMMIT,
            ImeEventKind::Cancel => sys::KUI_C_IME_CANCEL,
            ImeEventKind::Other(value) => value,
        }
    }
    pub(crate) fn from_raw(value: sys::KuiCImeEventType) -> Self {
        match value {
            sys::KUI_C_IME_START => ImeEventKind::Start,
            sys::KUI_C_IME_UPDATE => ImeEventKind::Update,
            sys::KUI_C_IME_COMMIT => ImeEventKind::Commit,
            sys::KUI_C_IME_CANCEL => ImeEventKind::Cancel,
            other => ImeEventKind::Other(other),
        }
    }
}

/// 自定义元素收到的输入法事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomImeEvent {
    /// 阶段。
    pub kind: ImeEventKind,
    /// 组合中的文字（`Update`）。
    pub text: String,
    /// `text` 里的字节偏移（`Update`）。
    pub caret_pos: i32,
    /// 最终文字（`Commit`）。
    pub committed_text: String,
}

/// 装饰线样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationStyle {
    /// 实线。
    Solid,
    /// 双线。
    Double,
    /// 点线。
    Dotted,
    /// 虚线。
    Dashed,
    /// 波浪线。
    Wavy,
}

/// 绘制 / 测量文字的字体；默认 sans-serif 14px 常规。
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// CSS font-family 列表；空为 sans-serif。
    pub font_family: String,
    /// 字号（px）。
    pub font_size: f32,
    /// 字重（100..900）。
    pub font_weight: u16,
    /// 斜体。
    pub italic: bool,
    /// 字间距。
    pub letter_spacing: f32,
    /// 词间距。
    pub word_spacing: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size: 14.0,
            font_weight: 400,
            italic: false,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

/// 文字度量（CSS 像素）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextMetrics {
    /// 前进宽度。
    pub width: f32,
    /// 基线以上。
    pub ascent: f32,
    /// 基线以下。
    pub descent: f32,
    /// CSS `normal` 行高。
    pub line_height: f32,
}

/// 四个角的（横向, 纵向）半径。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CornerRadii {
    /// 左上。
    pub top_left: (f32, f32),
    /// 右上。
    pub top_right: (f32, f32),
    /// 右下。
    pub bottom_right: (f32, f32),
    /// 左下。
    pub bottom_left: (f32, f32),
}

impl CornerRadii {
    /// 四个角相同的圆角。
    pub fn uniform(radius: f32) -> Self {
        let r = (radius, radius);
        Self {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }
    fn raw(&self) -> sys::KuiCCornerRadii {
        sys::KuiCCornerRadii {
            top_left_x: self.top_left.0,
            top_left_y: self.top_left.1,
            top_right_x: self.top_right.0,
            top_right_y: self.top_right.1,
            bottom_right_x: self.bottom_right.0,
            bottom_right_y: self.bottom_right.1,
            bottom_left_x: self.bottom_left.0,
            bottom_left_y: self.bottom_left.1,
        }
    }
}

/// C 字体结构；字体名的 `CString` 要活到调用结束。
struct RawTextStyle {
    raw: sys::KuiCTextStyle,
    _family: Option<CString>,
}

impl RawTextStyle {
    fn new(style: &TextStyle, context: &'static str) -> Result<Self> {
        let family = if style.font_family.is_empty() {
            None
        } else {
            Some(
                CString::new(style.font_family.as_str())
                    .map_err(|_| error(sys::KUI_ERR_INVALID_ARG, context))?,
            )
        };
        Ok(Self {
            raw: sys::KuiCTextStyle {
                font_family: family
                    .as_ref()
                    .map_or(ptr::null(), |f| f.as_ptr() as *const c_char),
                font_size: style.font_size,
                font_weight: style.font_weight,
                italic: i32::from(style.italic),
                letter_spacing: style.letter_spacing,
                word_spacing: style.word_spacing,
            },
            _family: family,
        })
    }
}

/// 自绘回调的绘制上下文，只在回调期间有效（生命周期 `'a` 保证）。坐标是内容框里的 CSS 像素，
/// 颜色是 `0xRRGGBBAA`；回调里做的变换、裁剪与透明度在回调结束时自动撤销。
pub struct PaintContext<'a> {
    raw: NonNull<sys::KuiPaintContext>,
    /// 内容框宽度。
    pub width: f32,
    /// 内容框高度。
    pub height: f32,
    /// 设备像素比。
    pub device_pixel_ratio: f32,
    _callback: PhantomData<&'a ()>,
}

impl PaintContext<'_> {
    pub(crate) fn new(raw: NonNull<sys::KuiPaintContext>) -> Self {
        let (mut width, mut height, mut dpr) = (0.0, 0.0, 1.0);
        let _ = unsafe { sys::kui_paint_get_info(raw.as_ptr(), &mut width, &mut height, &mut dpr) };
        Self {
            raw,
            width,
            height,
            device_pixel_ratio: dpr,
            _callback: PhantomData,
        }
    }
    fn p(&self) -> *mut sys::KuiPaintContext {
        self.raw.as_ptr()
    }
    /// 填充矩形。
    pub fn fill_rect(&mut self, rect: Rect, rgba: u32) -> Result<()> {
        check(
            unsafe {
                sys::kui_paint_fill_rect(self.p(), rect.x, rect.y, rect.width, rect.height, rgba)
            },
            "PaintContext::fill_rect",
        )
    }
    /// 填充圆角矩形。
    pub fn fill_rounded_rect(&mut self, rect: Rect, radii: &CornerRadii, rgba: u32) -> Result<()> {
        let radii = radii.raw();
        check(
            unsafe {
                sys::kui_paint_fill_rounded_rect(
                    self.p(),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    &radii,
                    rgba,
                )
            },
            "PaintContext::fill_rounded_rect",
        )
    }
    /// 描边矩形。
    pub fn stroke_rect(&mut self, rect: Rect, line_width: f32, rgba: u32) -> Result<()> {
        check(
            unsafe {
                sys::kui_paint_stroke_rect(
                    self.p(),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    line_width,
                    rgba,
                )
            },
            "PaintContext::stroke_rect",
        )
    }
    /// 描边圆角矩形。
    pub fn stroke_rounded_rect(
        &mut self,
        rect: Rect,
        radii: &CornerRadii,
        line_width: f32,
        rgba: u32,
    ) -> Result<()> {
        let radii = radii.raw();
        check(
            unsafe {
                sys::kui_paint_stroke_rounded_rect(
                    self.p(),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    &radii,
                    line_width,
                    rgba,
                )
            },
            "PaintContext::stroke_rounded_rect",
        )
    }
    /// 在 `(x, y)` 画文字：`y` 是文字行盒的顶部（不是基线），字形在 `y` 下方，基线在 `y + ascent`
    /// （ascent 取自同一 style 的 `Element::measure_text`）；`style` 为 `None` 用 sans-serif 14px。
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        style: Option<&TextStyle>,
        rgba: u32,
    ) -> Result<()> {
        let style = style
            .map(|s| RawTextStyle::new(s, "PaintContext::draw_text"))
            .transpose()?;
        let (raw, len) = bytes(text);
        check(
            unsafe {
                sys::kui_paint_draw_text(
                    self.p(),
                    raw,
                    len,
                    x,
                    y,
                    style.as_ref().map_or(ptr::null(), |s| &s.raw),
                    rgba,
                )
            },
            "PaintContext::draw_text",
        )
    }
    /// 画 `width × height` 个预乘的 `0xAARRGGBB` 像素，缩放到 `dest`。
    pub fn draw_pixels(
        &mut self,
        pixels: &[u32],
        width: u32,
        height: u32,
        dest: Rect,
    ) -> Result<()> {
        if (pixels.len() as u64) < u64::from(width) * u64::from(height) {
            return Err(error(sys::KUI_ERR_INVALID_ARG, "PaintContext::draw_pixels"));
        }
        check(
            unsafe {
                sys::kui_paint_draw_pixels(
                    self.p(),
                    pixels.as_ptr(),
                    width,
                    height,
                    dest.x,
                    dest.y,
                    dest.width,
                    dest.height,
                )
            },
            "PaintContext::draw_pixels",
        )
    }
    /// 画一个图片文件（与 `<img>` 同一套加载与缓存）；`src` 是图片像素坐标。
    pub fn draw_image(&mut self, path: &str, src: Rect, dest: Rect) -> Result<()> {
        let (raw, len) = bytes(path);
        let from = sys::KuiRect {
            x: src.x,
            y: src.y,
            width: src.width,
            height: src.height,
        };
        let to = sys::KuiRect {
            x: dest.x,
            y: dest.y,
            width: dest.width,
            height: dest.height,
        };
        check(
            unsafe { sys::kui_paint_draw_image(self.p(), raw, len, &from, &to) },
            "PaintContext::draw_image",
        )
    }
    /// 画装饰线（下划线、波浪线等）。
    pub fn draw_decoration_line(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        thickness: f32,
        style: DecorationStyle,
        rgba: u32,
    ) -> Result<()> {
        let style = match style {
            DecorationStyle::Solid => sys::KUI_C_DECORATION_SOLID,
            DecorationStyle::Double => sys::KUI_C_DECORATION_DOUBLE,
            DecorationStyle::Dotted => sys::KUI_C_DECORATION_DOTTED,
            DecorationStyle::Dashed => sys::KUI_C_DECORATION_DASHED,
            DecorationStyle::Wavy => sys::KUI_C_DECORATION_WAVY,
        };
        check(
            unsafe {
                sys::kui_paint_draw_decoration_line(self.p(), x, y, width, thickness, style, rgba)
            },
            "PaintContext::draw_decoration_line",
        )
    }
    /// 保存变换、裁剪与透明度。
    pub fn save(&mut self) -> Result<()> {
        check(
            unsafe { sys::kui_paint_save(self.p()) },
            "PaintContext::save",
        )
    }
    /// 撤销最近一次 `save`；没有对应的 `save` 时返回 `KUI_ERR_INVALID_ARG`。
    pub fn restore(&mut self) -> Result<()> {
        check(
            unsafe { sys::kui_paint_restore(self.p()) },
            "PaintContext::restore",
        )
    }
    /// 平移。
    pub fn translate(&mut self, dx: f32, dy: f32) -> Result<()> {
        check(
            unsafe { sys::kui_paint_translate(self.p(), dx, dy) },
            "PaintContext::translate",
        )
    }
    /// 缩放。
    pub fn scale(&mut self, sx: f32, sy: f32) -> Result<()> {
        check(
            unsafe { sys::kui_paint_scale(self.p(), sx, sy) },
            "PaintContext::scale",
        )
    }
    /// 旋转（度）。
    pub fn rotate(&mut self, degrees: f32) -> Result<()> {
        check(
            unsafe { sys::kui_paint_rotate(self.p(), degrees) },
            "PaintContext::rotate",
        )
    }
    /// 之后画的内容乘以 `opacity`（0..1），到对应的 `restore` 为止。
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        check(
            unsafe { sys::kui_paint_set_opacity(self.p(), opacity) },
            "PaintContext::set_opacity",
        )
    }
    /// 裁剪到矩形。
    pub fn clip_rect(&mut self, rect: Rect) -> Result<()> {
        check(
            unsafe { sys::kui_paint_clip_rect(self.p(), rect.x, rect.y, rect.width, rect.height) },
            "PaintContext::clip_rect",
        )
    }
    /// 裁剪到圆角矩形。
    pub fn clip_rounded_rect(&mut self, rect: Rect, radii: &CornerRadii) -> Result<()> {
        let radii = radii.raw();
        check(
            unsafe {
                sys::kui_paint_clip_rounded_rect(
                    self.p(),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    &radii,
                )
            },
            "PaintContext::clip_rounded_rect",
        )
    }
}

impl Element<'_> {
    /// 用与 KUI 自己相同的排版测量文字；窗口创建之前返回 `KUI_ERR_NOT_SUPPORTED`。
    pub fn measure_text(&self, text: &str, style: Option<&TextStyle>) -> Result<TextMetrics> {
        let style = style
            .map(|s| RawTextStyle::new(s, "Element::measure_text"))
            .transpose()?;
        let (raw, len) = bytes(text);
        let mut out = sys::KuiCTextMetrics {
            width: 0.0,
            ascent: 0.0,
            descent: 0.0,
            line_height: 0.0,
        };
        check(
            unsafe {
                sys::kui_element_measure_text(
                    self.raw.as_ptr(),
                    raw,
                    len,
                    style.as_ref().map_or(ptr::null(), |s| &s.raw),
                    &mut out,
                )
            },
            "Element::measure_text",
        )?;
        Ok(TextMetrics {
            width: out.width,
            ascent: out.ascent,
            descent: out.descent,
            line_height: out.line_height,
        })
    }
    /// 下一帧重绘（自绘元素的状态变了之后调用）。
    pub fn request_repaint(&self) -> Result<()> {
        check(
            unsafe { sys::kui_element_request_repaint(self.raw.as_ptr()) },
            "Element::request_repaint",
        )
    }
    /// 只重绘内容框里的这块矩形。
    pub fn invalidate_rect(&self, rect: Rect) -> Result<()> {
        check(
            unsafe {
                sys::kui_element_invalidate_rect(
                    self.raw.as_ptr(),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                )
            },
            "Element::invalidate_rect",
        )
    }
}

impl Document {
    /// 向获得焦点的元素注入一步输入法组合（与系统输入法走同一条路径）。
    pub fn dispatch_ime_event(&self, kind: ImeEventKind, text: &str) -> Result<()> {
        let (raw, len) = bytes(text);
        check(
            unsafe {
                sys::kui_devtools_dispatch_ime_event(self.raw.as_ptr(), kind.raw(), raw, len)
            },
            "Document::dispatch_ime_event",
        )
    }
}
