// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_paint.rs; edit that file, not this copy.
//! API tour: host painting into custom elements (examples/api_tours/paint). Before the page loads
//! the host defines six custom elements with a paint hook (define_custom_element_with); each hook
//! draws one group of PaintContext calls into its content box. Once frames have run, the host
//! checks the hooks ran with the element's size and every call succeeded, asks for repaints
//! (request_repaint / invalidate_rect) and checks they arrive; #tour-result ends as "PASS n/n".
//! What is drawn is checked by the case's pixel steps; the buttons change it.
//!
//! Binding note: PaintContext::width / height / device_pixel_ratio are read with
//! kui_paint_get_info when the context is created for each paint call.

use crate::canonical_runner::Context;
use kui::{
    CornerRadii, CustomElementDefinition, CustomElementHooks, DecorationStyle, Document,
    PaintContext, Rect, TextStyle,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            failed_names: RefCell::new(Vec::new()),
        }
    }

    fn row(&self, name: &str, result: &str, cell: &str) {
        let Ok(Some(log)) = self.doc.get_element_by_id("tour-log") else {
            return;
        };
        let Ok(row) = self.doc.create_element("tr") else {
            return;
        };
        let _ = row.set_attribute("data-check", name);
        let _ = row.set_attribute("data-result", result);
        if result != "ok" {
            let _ = row.class_list_add("bad");
        }
        for text in [name, cell] {
            if let Ok(td) = self.doc.create_element("td") {
                let _ = td.set_text_content(text);
                let _ = row.append_child(&td);
            }
        }
        let _ = log.append_child(&row);
    }

    fn check(&self, name: &str, ok: bool, detail: String) {
        self.total.set(self.total.get() + 1);
        if ok {
            self.row(name, "ok", "ok");
            return;
        }
        self.failed.set(self.failed.get() + 1);
        self.failed_names.borrow_mut().push(name.to_string());
        eprintln!("tour check failed: {name}: {detail}");
        self.row(name, "fail", &format!("FAIL: {detail}"));
    }

    fn finish(&self) {
        let Ok(Some(result)) = self.doc.get_element_by_id("tour-result") else {
            return;
        };
        let (total, failed) = (self.total.get(), self.failed.get());
        let text = if failed == 0 {
            format!("PASS {total}/{total}")
        } else {
            format!(
                "FAIL {failed}/{total}: {}",
                self.failed_names.borrow().join(", ")
            )
        };
        let _ = result.set_text_content(&text);
        let _ = result.set_attribute("data-total", &total.to_string());
        let _ = result.set_attribute("data-failed", &failed.to_string());
        let _ = result.set_attribute("data-known", "0");
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- What the panes draw (the buttons change these) and what the hooks saw ---------------

const PANES: usize = 6;
const SHAPES: usize = 0;
const TEXT: usize = 1;
const IMAGE: usize = 2;
const CLIP: usize = 3;
const XFORM: usize = 4;
const ALPHA: usize = 5;
const TAGS: [&str; PANES] = [
    "paint-shapes",
    "paint-text",
    "paint-image",
    "paint-clip",
    "paint-xform",
    "paint-alpha",
];
const IDS: [&str; PANES] = [
    "pv-shapes",
    "pv-text",
    "pv-image",
    "pv-clip",
    "pv-xform",
    "pv-alpha",
];
const STYLES: [(DecorationStyle, &str); 5] = [
    (DecorationStyle::Solid, "solid"),
    (DecorationStyle::Double, "double"),
    (DecorationStyle::Dotted, "dotted"),
    (DecorationStyle::Dashed, "dashed"),
    (DecorationStyle::Wavy, "wavy"),
];

struct State {
    paints: [u32; PANES],      // paint hook calls
    size: [(f32, f32); PANES], // PaintContext size, last call
    dpr: [f32; PANES],
    bad_calls: [u32; PANES], // paint calls that returned an error
    unbalanced: Option<i32>, // status of a restore without save, tried once
    fill: u32,               // shapes: first rectangle (0xRRGGBBAA)
    line_style: usize,
    line: u32,          // text: decoration line colour
    pixel_shift: usize, // image: rotates the 2x2 buffer
    clip_on: bool,
    angle: f32,
    alpha: f32,
    image_path: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        paints: [0; PANES],
        size: [(0.0, 0.0); PANES],
        dpr: [0.0; PANES],
        bad_calls: [0; PANES],
        unbalanced: None,
        fill: 0xef44_44ff,
        line_style: 0,
        line: 0xe11d_48ff,
        pixel_shift: 0,
        clip_on: true,
        angle: 45.0,
        alpha: 0.5,
        image_path: String::new(),
    });
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn ok(s: &mut State, pane: usize, result: kui::Result<()>) {
    if result.is_err() {
        s.bad_calls[pane] += 1;
    }
}

// ---- Paint hooks: one group of calls per pane --------------------------------------------

/// Filled and stroked rectangles at fixed fractions of the width (the case samples them).
fn paint_shapes(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let fill = s.fill;
    ok(
        s,
        SHAPES,
        p.fill_rect(rect(0.03 * w, 0.15 * h, 0.20 * w, 0.70 * h), fill),
    );
    ok(
        s,
        SHAPES,
        p.fill_rounded_rect(
            rect(0.28 * w, 0.15 * h, 0.20 * w, 0.70 * h),
            &CornerRadii::uniform(16.0),
            0x22c5_5eff,
        ),
    );
    ok(
        s,
        SHAPES,
        p.stroke_rect(
            rect(0.53 * w, 0.15 * h, 0.20 * w, 0.70 * h),
            8.0,
            0x3b82_f6ff,
        ),
    );
    ok(
        s,
        SHAPES,
        p.stroke_rounded_rect(
            rect(0.78 * w, 0.15 * h, 0.19 * w, 0.70 * h),
            &CornerRadii::uniform(14.0),
            8.0,
            0xa855_f7ff,
        ),
    );
}

/// Bold text hanging from y (the top of its line box, not the baseline), the decoration line band at 0.6h..0.6h+8 and a wavy line below.
fn paint_text(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let style = TextStyle {
        font_size: 26.0,
        font_weight: 700,
        ..TextStyle::default()
    };
    let (line_style, line) = (STYLES[s.line_style].0, s.line);
    ok(
        s,
        TEXT,
        p.draw_text("Hello paint", 10.0, 0.45 * h, Some(&style), 0x0f17_2aff),
    );
    ok(
        s,
        TEXT,
        p.draw_decoration_line(10.0, 0.6 * h, 0.9 * w, 8.0, line_style, line),
    );
    ok(
        s,
        TEXT,
        p.draw_decoration_line(
            10.0,
            0.82 * h,
            0.9 * w,
            2.0,
            DecorationStyle::Wavy,
            0x0ea5_e9ff,
        ),
    );
}

/// A 2x2 premultiplied 0xAARRGGBB buffer scaled into the left box, an image file into the right.
fn paint_image(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let colors = [0xffff_0000u32, 0xff00_ff00, 0xff00_00ff, 0xffff_ff00];
    let pixels: Vec<u32> = (0..4).map(|i| colors[(i + s.pixel_shift) % 4]).collect();
    ok(
        s,
        IMAGE,
        p.draw_pixels(&pixels, 2, 2, rect(0.03 * w, 0.1 * h, 0.4 * w, 0.8 * h)),
    );
    let path = s.image_path.clone();
    ok(
        s,
        IMAGE,
        p.draw_image(
            &path,
            rect(0.0, 0.0, 8.0, 8.0),
            rect(0.55 * w, 0.1 * h, 0.4 * w, 0.8 * h),
        ),
    );
}

/// Top band clipped to the left half; bottom band clipped to a rounded rectangle.
fn paint_clip(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let clip_on = s.clip_on;
    ok(s, CLIP, p.save());
    if clip_on {
        ok(s, CLIP, p.clip_rect(rect(0.0, 0.0, 0.5 * w, h)));
    }
    ok(
        s,
        CLIP,
        p.fill_rect(rect(0.0, 0.0, w, 0.45 * h), 0xf973_16ff),
    );
    ok(s, CLIP, p.restore());
    ok(s, CLIP, p.save());
    if clip_on {
        ok(
            s,
            CLIP,
            p.clip_rounded_rect(
                rect(0.1 * w, 0.55 * h, 0.8 * w, 0.4 * h),
                &CornerRadii::uniform(24.0),
            ),
        );
    }
    ok(
        s,
        CLIP,
        p.fill_rect(rect(0.0, 0.5 * h, w, 0.5 * h), 0x14b8_a6ff),
    );
    ok(s, CLIP, p.restore());
}

/// A 60x60 square rotated about (0.25w, 0.5h); a 10x10 square scaled x4 at (0.6w, 0.2h); after
/// the last restore a marker in the bottom-right corner (it lands there only if the transforms
/// were undone).
fn paint_xform(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let angle = s.angle;
    ok(s, XFORM, p.save());
    ok(s, XFORM, p.translate(0.25 * w, 0.5 * h));
    ok(s, XFORM, p.rotate(angle));
    ok(
        s,
        XFORM,
        p.fill_rect(rect(-30.0, -30.0, 60.0, 60.0), 0x6366_f1ff),
    );
    ok(s, XFORM, p.restore());
    ok(s, XFORM, p.save());
    ok(s, XFORM, p.translate(0.6 * w, 0.2 * h));
    ok(s, XFORM, p.scale(4.0, 4.0));
    ok(
        s,
        XFORM,
        p.fill_rect(rect(0.0, 0.0, 10.0, 10.0), 0xf59e_0bff),
    );
    ok(s, XFORM, p.restore());
    ok(
        s,
        XFORM,
        p.fill_rect(rect(w - 20.0, h - 20.0, 16.0, 16.0), 0x16a3_4aff),
    );
    if s.unbalanced.is_none() {
        // restore without a matching save is KUI_ERR_INVALID_ARG (kui_c.h).
        s.unbalanced = Some(match p.restore() {
            Ok(()) => kui::sys::KUI_OK,
            Err(error) => error.status(),
        });
    }
}

/// Blue at the pane's opacity on the left; 0.5 inside 0.5 on the right.
fn paint_alpha(s: &mut State, p: &mut PaintContext<'_>, w: f32, h: f32) {
    let alpha = s.alpha;
    ok(s, ALPHA, p.save());
    ok(s, ALPHA, p.set_opacity(alpha));
    ok(
        s,
        ALPHA,
        p.fill_rect(rect(0.05 * w, 0.15 * h, 0.4 * w, 0.7 * h), 0x0000_ffff),
    );
    ok(s, ALPHA, p.restore());
    ok(s, ALPHA, p.save());
    ok(s, ALPHA, p.set_opacity(0.5));
    ok(s, ALPHA, p.save());
    ok(s, ALPHA, p.set_opacity(0.5));
    ok(
        s,
        ALPHA,
        p.fill_rect(rect(0.55 * w, 0.15 * h, 0.4 * w, 0.7 * h), 0x0000_ffff),
    );
    ok(s, ALPHA, p.restore());
    ok(s, ALPHA, p.restore());
}

fn hooks_for(pane: usize) -> CustomElementHooks {
    let draw = [
        paint_shapes,
        paint_text,
        paint_image,
        paint_clip,
        paint_xform,
        paint_alpha,
    ][pane];
    CustomElementHooks {
        paint: Some(Box::new(move |_element, p| {
            STATE.with(|state| {
                let s = &mut *state.borrow_mut();
                s.paints[pane] += 1;
                s.size[pane] = (p.width, p.height);
                s.dpr[pane] = p.device_pixel_ratio;
                let (w, h) = (p.width, p.height);
                draw(s, p, w, h);
            });
        })),
        ..CustomElementHooks::default()
    }
}

// ---- Before the page loads: the image file and the six definitions -----------------------

/// Writes an 8x8 24-bit BMP filled with #14b8a6 (teal).
fn write_image() -> std::io::Result<String> {
    let path = std::env::temp_dir().join("kui_api_paint_rust.bmp");
    let mut data = vec![0u8; 54 + 8 * 24];
    let le = |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    data[0] = b'B';
    data[1] = b'M';
    le(&mut data, 2, (54 + 8 * 24) as u32);
    le(&mut data, 10, 54);
    le(&mut data, 14, 40);
    le(&mut data, 18, 8);
    le(&mut data, 22, 8);
    data[26] = 1; // planes
    data[28] = 24; // bits per pixel
    le(&mut data, 34, 8 * 24);
    for pixel in data[54..].chunks_mut(3) {
        pixel.copy_from_slice(&[0xa6, 0xb8, 0x14]); // blue, green, red
    }
    std::fs::write(&path, data)?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn before_load(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    match write_image() {
        Ok(path) => STATE.with(|state| state.borrow_mut().image_path = path),
        Err(error) => eprintln!("kui_api_paint: cannot write the image: {error}"),
    }
    for (pane, tag) in TAGS.iter().enumerate() {
        ctx.document.define_custom_element_with(
            tag,
            CustomElementDefinition::default(),
            hooks_for(pane),
        )?;
    }
    Ok(())
}

// ---- Self-check: definitions now, hooks and repaints once frames have run ---------------

fn check_definitions(t: &Tour) {
    let all = TAGS
        .iter()
        .all(|tag| matches!(t.doc.custom_element_is_defined(tag), Ok(true)));
    let again = t.doc.define_custom_element_with(
        "paint-shapes",
        CustomElementDefinition::default(),
        hooks_for(SHAPES),
    );
    let status = again.as_ref().err().map(|e| e.status());
    t.check(
        "define_custom_element_ex",
        all && status == Some(kui::sys::KUI_ERR_INVALID_ARG),
        format!("all defined {all}, second definition {again:?}"),
    );
}

fn phase_painted(t: Rc<Tour>, platform: Rc<kui::Platform>) {
    let s = STATE.with(|state| {
        let s = state.borrow();
        (s.paints, s.size, s.dpr, s.bad_calls, s.unbalanced)
    });
    let (paints, size, dpr, bad_calls, unbalanced) = s;
    let mut sizes_match = true;
    let mut shown = Vec::new();
    for pane in 0..PANES {
        let element = t.doc.get_element_by_id(IDS[pane]).ok().flatten();
        let r = element
            .and_then(|e| e.bounding_rect().ok())
            .unwrap_or(rect(0.0, 0.0, 0.0, 0.0));
        let (w, h) = size[pane];
        sizes_match &= dpr[pane] > 0.0
            && r.width > 0.0
            && (w - r.width).abs() < 0.5
            && (h - r.height).abs() < 0.5;
        shown.push(format!("{w:.0}x{h:.0}/{:.0}x{:.0}", r.width, r.height));
    }
    let bad: u32 = bad_calls.iter().sum();
    t.check(
        "paint.callbacks_run",
        paints.iter().all(|&n| n > 0),
        format!("paints {paints:?}"),
    );
    t.check(
        "paint_get_info.size",
        sizes_match,
        format!("info/element {}, dpr {:.2}", shown.join(" "), dpr[0]),
    );
    t.check(
        "paint.calls_ok",
        bad == 0,
        format!("{bad} calls returned an error"),
    );
    t.check(
        "paint_restore.unbalanced",
        unbalanced == Some(kui::sys::KUI_ERR_INVALID_ARG),
        format!("status {unbalanced:?}"),
    );

    if let Ok(Some(e)) = t.doc.get_element_by_id("pv-shapes") {
        let _ = e.request_repaint();
    }
    if let Ok(Some(e)) = t.doc.get_element_by_id("pv-text") {
        let _ = e.invalidate_rect(rect(0.0, 60.0, 300.0, 20.0));
    }
    let _ = platform.set_timeout(300, move || {
        let now = STATE.with(|state| state.borrow().paints);
        t.check(
            "request_repaint",
            now[SHAPES] > paints[SHAPES],
            format!("paints {} -> {}", paints[SHAPES], now[SHAPES]),
        );
        t.check(
            "invalidate_rect",
            now[TEXT] > paints[TEXT],
            format!("paints {} -> {}", paints[TEXT], now[TEXT]),
        );
        t.finish();
    });
}

// ---- Panel buttons: change what a pane draws, then ask for its repaint -------------------

fn repaint(doc: &Document, pane: usize) {
    if let Ok(Some(e)) = doc.get_element_by_id(IDS[pane]) {
        let _ = e.request_repaint();
    }
}

/// Repaints only the decoration line's band of the text pane.
fn invalidate_line(doc: &Document) {
    let (w, h) = STATE.with(|state| state.borrow().size[TEXT]);
    let (w, h) = if w > 0.0 && h > 0.0 {
        (w, h)
    } else {
        (300.0, 120.0)
    };
    if let Ok(Some(e)) = doc.get_element_by_id("pv-text") {
        let _ = e.invalidate_rect(rect(0.0, 0.6 * h - 2.0, w, 12.0));
    }
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(e)) = doc.get_element_by_id(id) {
        let _ = e.set_text_content(text);
    }
}

fn on<F>(ctx: &Context<'_>, id: &str, mut handler: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Document, &mut State) + 'static,
{
    let doc = ctx.document.clone();
    let element = ctx
        .document
        .get_element_by_id(id)?
        .ok_or(format!("page has no #{id}"))?;
    element.add_event_listener("click", move |_event| {
        // Change the state first, then repaint outside the borrow (paint hooks read it).
        STATE.with(|state| handler(&doc, &mut state.borrow_mut()));
    })?;
    Ok(())
}

fn bind_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-shapes-color", |doc, s| {
        s.fill = if s.fill == 0xef44_44ff {
            0xf973_16ff
        } else {
            0xef44_44ff
        };
        set_text(doc, "shapes-out", &format!("fill=#{:06x}", s.fill >> 8));
    })?;
    on(ctx, "btn-paint-info", |doc, s| {
        let p = s.paints;
        let (w, h) = s.size[SHAPES];
        set_text(
            doc,
            "shapes-out",
            &format!(
                "paints={},{},{},{},{},{} | {w:.0}x{h:.0} @{:.2}",
                p[0], p[1], p[2], p[3], p[4], p[5], s.dpr[SHAPES]
            ),
        );
    })?;
    on(ctx, "btn-text-style", |doc, s| {
        s.line_style = (s.line_style + 1) % STYLES.len();
        set_text(
            doc,
            "text-out",
            &format!("style={}", STYLES[s.line_style].1),
        );
    })?;
    on(ctx, "btn-text-color", |doc, s| {
        s.line = if s.line == 0xe11d_48ff {
            0x2563_ebff
        } else {
            0xe11d_48ff
        };
        set_text(doc, "text-out", &format!("line=#{:06x}", s.line >> 8));
    })?;
    on(ctx, "btn-image-rotate", |doc, s| {
        s.pixel_shift = (s.pixel_shift + 1) % 4;
        let names = ["#ff0000", "#00ff00", "#0000ff", "#ffff00"];
        set_text(
            doc,
            "image-out",
            &format!("top-left={}", names[s.pixel_shift]),
        );
    })?;
    on(ctx, "btn-clip-toggle", |doc, s| {
        s.clip_on = !s.clip_on;
        set_text(
            doc,
            "clip-out",
            if s.clip_on { "clip=on" } else { "clip=off" },
        );
    })?;
    on(ctx, "btn-xform-rotate", |doc, s| {
        s.angle = if s.angle == 0.0 { 45.0 } else { 0.0 };
        set_text(doc, "xform-out", &format!("angle={}", s.angle));
    })?;
    on(ctx, "btn-alpha-cycle", |doc, s| {
        s.alpha = if s.alpha == 0.5 {
            1.0
        } else if s.alpha == 1.0 {
            0.25
        } else {
            0.5
        };
        set_text(doc, "alpha-out", &format!("opacity={}", s.alpha));
    })?;
    // Each button's repaint request, registered after the state change above.
    let repaints: [(&str, Option<usize>); 7] = [
        ("btn-shapes-color", Some(SHAPES)),
        ("btn-text-style", None),
        ("btn-text-color", None),
        ("btn-image-rotate", Some(IMAGE)),
        ("btn-clip-toggle", Some(CLIP)),
        ("btn-xform-rotate", Some(XFORM)),
        ("btn-alpha-cycle", Some(ALPHA)),
    ];
    for (id, pane) in repaints {
        let doc = ctx.document.clone();
        let element = ctx
            .document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?;
        element.add_event_listener("click", move |_event| match pane {
            Some(pane) => repaint(&doc, pane),
            None => invalidate_line(&doc),
        })?;
    }
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let t = Rc::new(Tour::new(ctx.document));
    check_definitions(&t);
    // Paint hooks run with frames: check them once a few have been drawn.
    let platform = ctx.platform.clone();
    ctx.platform
        .set_timeout(500, move || phase_painted(t, platform))?;
    bind_buttons(ctx)
}
