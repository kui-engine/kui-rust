// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_geometry_focus.rs; edit that file, not this copy.
//! API tour: layout geometry, scrolling, anchored placement, text measurement, focus and pointer
//! capture (examples/api_tours/geometry_focus). On start the host calls every method once,
//! checks the numbers against the CSS on the page (and against each other: offset vs bounding
//! rect, scroll offset vs the content's moved rect) and logs one row per check into #tour-log;
//! #tour-result ends as "PASS n/n". The panel buttons repeat the calls on the visible boxes.
//!
//! Mouse input for the pointer-capture checks is injected with Document::dispatch_mouse_event,
//! which runs the same dispatch path as real input.

use crate::canonical_runner::Context;
use kui::{
    Document, Element, InjectedMouse, Modifiers, MouseButton, MouseInjection, Point,
    PositioningOptions, Rect, TextMetrics, TextStyle,
};
use std::cell::{Cell, RefCell};
use std::error::Error;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug). A failure is
//            logged as "KNOWN BUG" and kept out of the verdict; once it passes it counts as ok.

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    known: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            known: Cell::new(0),
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

    fn known_bug(&self, name: &str, ok: bool, bug: &str, detail: String) {
        if ok {
            self.total.set(self.total.get() + 1);
            self.row(name, "ok", "ok");
            return;
        }
        self.known.set(self.known.get() + 1);
        eprintln!("tour known bug: {name}: {detail}");
        self.row(name, "known", &format!("KNOWN BUG: {bug} ({detail})"));
    }

    fn finish(&self) {
        let Ok(Some(result)) = self.doc.get_element_by_id("tour-result") else {
            return;
        };
        let (total, failed, known) = (self.total.get(), self.failed.get(), self.known.get());
        let mut text = if failed == 0 {
            format!("PASS {total}/{total}")
        } else {
            format!(
                "FAIL {failed}/{total}: {}",
                self.failed_names.borrow().join(", ")
            )
        };
        if known > 0 {
            text.push_str(&format!(" (known bugs: {known})"));
        }
        let _ = result.set_text_content(&text);
        let _ = result.set_attribute("data-total", &total.to_string());
        let _ = result.set_attribute("data-failed", &failed.to_string());
        let _ = result.set_attribute("data-known", &known.to_string());
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- Small readers ------------------------------------------------------------------------

fn by_id<'d>(doc: &'d Document, element_id: &str) -> Option<Element<'d>> {
    doc.get_element_by_id(element_id).ok().flatten()
}

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn round(v: f32) -> f32 {
    (v + 0.5).floor()
}

const ZERO: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 0.0,
    height: 0.0,
};

fn rect_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.bounding_rect().ok())
        .unwrap_or(ZERO)
}

fn client_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.client_rect().ok())
        .unwrap_or(ZERO)
}

fn offset_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.offset_rect().ok())
        .unwrap_or(ZERO)
}

/// Scroll offset and extent as (left, top, width, height).
fn scroll_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.scroll_metrics().ok())
        .map(|s| Rect {
            x: s.left,
            y: s.top,
            width: s.width,
            height: s.height,
        })
        .unwrap_or(ZERO)
}

fn is(r: Rect, x: f32, y: f32, w: f32, h: f32) -> bool {
    near(r.x, x, 0.01) && near(r.y, y, 0.01) && near(r.width, w, 0.01) && near(r.height, h, 0.01)
}

fn shown(r: Rect) -> String {
    format!("{:.0},{:.0} {:.0}x{:.0}", r.x, r.y, r.width, r.height)
}

fn focused(doc: &Document, element_id: &str) -> i32 {
    match by_id(doc, element_id).map(|e| e.has_focus()) {
        Some(Ok(yes)) => yes as i32,
        _ => -1,
    }
}

fn captured(doc: &Document, element_id: &str, pointer: i32) -> i32 {
    match by_id(doc, element_id).map(|e| e.has_pointer_capture(pointer)) {
        Some(Ok(yes)) => yes as i32,
        _ => -1,
    }
}

/// Injects one mouse event at the centre of an element (viewport coordinates).
fn mouse_at(doc: &Document, element_id: &str, kind: InjectedMouse, button: MouseButton) {
    let r = rect_of(doc, element_id);
    let _ = doc.dispatch_mouse_event(&MouseInjection {
        kind,
        position: Point {
            x: r.x + r.width / 2.0,
            y: r.y + r.height / 2.0,
        },
        button,
        modifiers: Modifiers::default(),
        click_count: if kind == InjectedMouse::Moved { 0 } else { 1 },
        delta: Point { x: 0.0, y: 0.0 },
    });
}

fn set_text(doc: &Document, element_id: &str, value: &str) {
    if let Some(element) = by_id(doc, element_id) {
        let _ = element.set_text_content(value);
    }
}

fn set_style(doc: &Document, element_id: &str, name: &str, value: &str) {
    if let Some(element) = by_id(doc, element_id) {
        let _ = element.set_style_property(name, value, false);
    }
}

/// Registers a listener on the element with that id (the closure holds its own Document).
fn listen<F>(
    doc: &Document,
    element_id: &str,
    name: &str,
    listener: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&kui::Event<'_>) + 'static,
{
    let element = doc
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener(name, listener)?;
    Ok(())
}

// ---- Box metrics: bounding rect, client (padding box), offset (border box vs offset parent) --

fn check_box_metrics(t: &Tour, doc: &Document) {
    let (frame, b, bb) = (
        rect_of(doc, "geo-frame"),
        rect_of(doc, "geo-box"),
        rect_of(doc, "geo-bb"),
    );
    let (client_box, client_bb) = (client_of(doc, "geo-box"), client_of(doc, "geo-bb"));
    let (offset_box, offset_bb) = (offset_of(doc, "geo-box"), offset_of(doc, "geo-bb"));

    // content-box: 200 + 2*12 padding + 2*6 border; border-box: the declared 200x50.
    t.check(
        "geo.bounding_rect-size",
        near(b.width, 236.0, 0.5)
            && near(b.height, 76.0, 0.5)
            && near(bb.width, 200.0, 0.5)
            && near(bb.height, 50.0, 0.5),
        format!(
            "content-box {:.1}x{:.1} (want 236x76), border-box {:.1}x{:.1} (want 200x50)",
            b.width, b.height, bb.width, bb.height
        ),
    );
    // Frame border 2 + frame padding 8 + box margin 10 from the frame's edge.
    t.check(
        "geo.bounding_rect-position",
        near(b.x - frame.x, 20.0, 0.5) && near(b.y - frame.y, 20.0, 0.5),
        format!(
            "box at +{:.1},+{:.1} from the frame (want +20,+20)",
            b.x - frame.x,
            b.y - frame.y
        ),
    );
    // Padding box: left/top are the border widths.
    t.check(
        "geo.client_metrics",
        is(client_box, 6.0, 6.0, 224.0, 64.0) && is(client_bb, 4.0, 4.0, 192.0, 42.0),
        format!(
            "content-box {} (want 6,6 224x64), border-box {} (want 4,4 192x42)",
            shown(client_box),
            shown(client_bb)
        ),
    );
    // Border box from the offset parent's padding edge; the boxes' 10px margins collapse.
    t.check(
        "geo.offset_metrics",
        is(offset_box, 18.0, 18.0, 236.0, 76.0) && is(offset_bb, 18.0, 104.0, 200.0, 50.0),
        format!(
            "content-box {} (want 18,18 236x76), border-box {} (want 18,104 200x50)",
            shown(offset_box),
            shown(offset_bb)
        ),
    );
    // offsetLeft/Top = the bounding rects' distance minus the parent's border, rounded.
    t.check(
        "geo.offset_vs_rect",
        offset_bb.x == round(bb.x - frame.x - 2.0)
            && offset_bb.y == round(bb.y - frame.y - 2.0)
            && offset_bb.width == round(bb.width)
            && offset_bb.height == round(bb.height),
        format!(
            "offset {:.0},{:.0} vs rects {:.2},{:.2}",
            offset_bb.x,
            offset_bb.y,
            bb.x - frame.x - 2.0,
            bb.y - frame.y - 2.0
        ),
    );
}

// ---- Scrolling: extent, set_scroll_position, scroll_by, clamping ------------------------

fn scroll_to(doc: &Document, left: f32, top: f32) {
    if let Some(e) = by_id(doc, "scroller") {
        let _ = e.set_scroll_position(left, top);
    }
}

fn scroll_by(doc: &Document, dx: f32, dy: f32) {
    if let Some(e) = by_id(doc, "scroller") {
        let _ = e.scroll_by(dx, dy);
    }
}

fn check_scrolling(t: &Tour, doc: &Document) {
    let (outer, row0) = (rect_of(doc, "scroller"), rect_of(doc, "row-0"));
    let (client, scroll) = (client_of(doc, "scroller"), scroll_of(doc, "scroller"));

    t.check(
        "scroll.extent",
        is(scroll, 0.0, 0.0, 600.0, 400.0),
        format!("scroll {} (want 0,0 600x400)", shown(scroll)),
    );
    // 1px border; a rendered scrollbar may take some of the padding box.
    t.check(
        "scroll.client",
        client.x == 1.0
            && client.y == 1.0
            && client.width <= outer.width - 2.0
            && client.width > outer.width - 24.0
            && client.height <= 118.0
            && client.height > 96.0,
        format!(
            "client {} of a {:.0}x{:.0} box",
            shown(client),
            outer.width,
            outer.height
        ),
    );

    scroll_to(doc, 0.0, 100.0);
    let (scroll, moved) = (scroll_of(doc, "scroller"), rect_of(doc, "row-0"));
    t.check(
        "scroll.set_scroll_position",
        near(scroll.y, 100.0, 0.01)
            && near(scroll.x, 0.0, 0.01)
            && near(row0.y - moved.y, 100.0, 0.5),
        format!(
            "scroll {:.1},{:.1}, row 0 moved up {:.1}",
            scroll.x,
            scroll.y,
            row0.y - moved.y
        ),
    );

    scroll_by(doc, 30.0, 50.0);
    let (scroll, moved) = (scroll_of(doc, "scroller"), rect_of(doc, "row-0"));
    t.check(
        "scroll.scroll_by",
        near(scroll.x, 30.0, 0.01)
            && near(scroll.y, 150.0, 0.01)
            && near(row0.x - moved.x, 30.0, 0.5)
            && near(row0.y - moved.y, 150.0, 0.5),
        format!(
            "scroll {:.1},{:.1}, row 0 moved {:.1},{:.1}",
            scroll.x,
            scroll.y,
            row0.x - moved.x,
            row0.y - moved.y
        ),
    );

    // Past the end clamps to extent - client size.
    scroll_to(doc, 10000.0, 10000.0);
    let scroll = scroll_of(doc, "scroller");
    t.check(
        "scroll.clamp",
        near(scroll.x, 600.0 - client.width, 0.5) && near(scroll.y, 400.0 - client.height, 0.5),
        format!(
            "scroll {:.1},{:.1} (want {:.0},{:.0})",
            scroll.x,
            scroll.y,
            600.0 - client.width,
            400.0 - client.height
        ),
    );

    scroll_to(doc, 0.0, 0.0);
    let (scroll, moved) = (scroll_of(doc, "scroller"), rect_of(doc, "row-0"));
    t.check(
        "scroll.reset",
        near(scroll.x, 0.0, 0.01) && near(scroll.y, 0.0, 0.01) && near(moved.y, row0.y, 0.5),
        format!("scroll {:.1},{:.1}", scroll.x, scroll.y),
    );
}

// ---- Anchored placement: position_relative_to with placements, flip and clamp -----------

fn place(doc: &Document, placement: &str, offset: f32, flip: bool, clamp: bool, margin: f32) {
    let (Some(popup), Some(anchor)) = (by_id(doc, "pos-popup"), by_id(doc, "pos-anchor")) else {
        return;
    };
    let options = PositioningOptions {
        flip,
        clamp_to_viewport: clamp,
        offset,
        viewport_margin: margin,
    };
    let _ = popup.position_relative_to(&anchor, placement, &options);
}

// ---- Scroll into view and inertial scrolling ----------------------------------------------------------

fn check_scroll_reveal(t: &Tour, doc: &Document, platform: &kui::Platform) {
    let (Some(scroller), Some(last)) = (by_id(doc, "scroller"), by_id(doc, "row-9")) else {
        t.check("reveal.setup", false, "missing #scroller or #row-9".into());
        return;
    };
    scroll_to(doc, 0.0, 0.0);
    let _ = last.scroll_into_view(true);
    let (scroll, box_, row) = (
        scroll_of(doc, "scroller"),
        rect_of(doc, "scroller"),
        rect_of(doc, "row-9"),
    );
    // The last row is now inside the scroll box (1px border).
    t.check(
        "scroll.scroll_into_view",
        scroll.y > 100.0 && row.y >= box_.y && row.y + row.height <= box_.y + box_.height + 1.0,
        format!(
            "scroll {:.1},{:.1}, row 9 at {:.1}+{:.1} in a box at {:.1}+{:.1}",
            scroll.x, scroll.y, row.y, row.height, box_.y, box_.height
        ),
    );

    // Inertial scroll: a velocity (px/ms) that decays frame by frame until the container stops.
    scroll_to(doc, 0.0, 0.0);
    let started = scroller.start_momentum_scroll(0.0, 0.1);
    let (mut previous, mut now_y) = (-1.0f32, 0.0f32);
    for _ in 0..80 {
        let _ = platform.wait_events(0.03);
        now_y = scroll_of(doc, "scroller").y;
        if now_y > 0.0 && now_y == previous {
            break;
        }
        previous = now_y;
    }
    t.check(
        "scroll.start_momentum_scroll",
        started.is_ok() && now_y > 0.0 && now_y < 200.0,
        format!("{started:?}, stopped at {now_y:.1}"),
    );
    scroll_to(doc, 0.0, 0.0);
}

fn check_placement(t: &Tour, doc: &Document) {
    let a = rect_of(doc, "pos-anchor");

    place(doc, "bottom-start", 6.0, false, false, 0.0);
    let p = rect_of(doc, "pos-popup");
    t.check(
        "pos.bottom-start",
        near(p.x, a.x, 0.5) && near(p.y, a.y + a.height + 6.0, 0.5),
        format!("popup {:.1},{:.1} anchor {}", p.x, p.y, shown(a)),
    );
    let left = by_id(doc, "pos-popup")
        .and_then(|e| e.style_property("left").ok())
        .unwrap_or_default();
    let value: f32 = left.trim_end_matches("px").parse().unwrap_or(-1.0);
    t.check(
        "pos.inline-left",
        left.ends_with("px") && near(value, a.x, 0.5),
        format!("inline left '{left}'"),
    );

    place(doc, "right-center", 4.0, false, false, 0.0);
    let p = rect_of(doc, "pos-popup");
    t.check(
        "pos.right-center",
        near(p.x, a.x + a.width + 4.0, 0.5)
            && near(p.y + p.height / 2.0, a.y + a.height / 2.0, 0.5),
        format!("popup {}", shown(p)),
    );

    place(doc, "left-end", 8.0, false, false, 0.0);
    let p = rect_of(doc, "pos-popup");
    t.check(
        "pos.left-end",
        near(p.x + p.width, a.x - 8.0, 0.5) && near(p.y + p.height, a.y + a.height, 0.5),
        format!("popup {}", shown(p)),
    );

    // "top" with a viewport margin below the anchor's top cannot fit: flip puts it below.
    place(doc, "top", 5.0, true, false, a.y);
    let p = rect_of(doc, "pos-popup");
    t.check(
        "pos.flip",
        near(p.y, a.y + a.height + 5.0, 0.5),
        format!("popup y {:.1} (want {:.1})", p.y, a.y + a.height + 5.0),
    );

    // Far to the left with clamping: held at the viewport margin.
    place(doc, "left", 2000.0, false, true, 7.0);
    let p = rect_of(doc, "pos-popup");
    t.check(
        "pos.clamp",
        near(p.x, 7.0, 0.5),
        format!("popup x {:.1} (want 7)", p.x),
    );

    place(doc, "bottom-start", 6.0, true, true, 4.0);
}

// ---- Text measurement --------------------------------------------------------------------

fn measure(doc: &Document, text: &str, family: &str, size: f32, weight: u16) -> TextMetrics {
    let style = TextStyle {
        font_family: family.to_string(),
        font_size: size,
        font_weight: weight,
        ..TextStyle::default()
    };
    by_id(doc, "measure-sample")
        .and_then(|e| e.measure_text(text, Some(&style)).ok())
        .unwrap_or_default()
}

fn check_measure(t: &Tour, doc: &Document) {
    let small = measure(doc, "Hello KUI", "", 12.0, 400);
    let mid = measure(doc, "Hello KUI", "", 16.0, 400);
    let large = measure(doc, "Hello KUI", "", 24.0, 400);
    let empty = measure(doc, "", "", 16.0, 400);
    let segoe = measure(doc, "Hello KUI", "Segoe UI", 16.0, 400);
    let span = rect_of(doc, "measure-span");

    t.check(
        "measure.monotonic",
        small.width > 0.0 && small.width < mid.width && mid.width < large.width,
        format!(
            "widths {:.2} / {:.2} / {:.2} at 12 / 16 / 24px",
            small.width, mid.width, large.width
        ),
    );
    t.check(
        "measure.metrics",
        mid.ascent > 0.0
            && mid.descent > 0.0
            && mid.line_height >= mid.ascent + mid.descent - 0.5
            && large.ascent > mid.ascent,
        format!(
            "ascent {:.2} descent {:.2} line height {:.2}",
            mid.ascent, mid.descent, mid.line_height
        ),
    );
    t.check(
        "measure.empty",
        empty.width == 0.0 && near(empty.ascent, mid.ascent, 0.01),
        format!("empty width {:.2} ascent {:.2}", empty.width, empty.ascent),
    );
    // Same shaping as layout: the inline-block span with the same text, font and size.
    t.check(
        "measure.matches_layout",
        near(segoe.width, span.width, 1.0),
        format!(
            "measured {:.2}, laid-out span {:.2}",
            segoe.width, span.width
        ),
    );
}

// ---- Focus: focus / blur / has_focus, and the focus / blur events they fire --------------

thread_local! {
    static FOCUS_EVENTS: Cell<u32> = const { Cell::new(0) };
    static BLUR_EVENTS: Cell<u32> = const { Cell::new(0) };
    static GOT_CAPTURE: Cell<u32> = const { Cell::new(0) };
    static LOST_CAPTURE: Cell<u32> = const { Cell::new(0) };
    static CAPTURED_MOVES: Cell<u32> = const { Cell::new(0) };
    static CAPTURE_TARGET: Cell<bool> = const { Cell::new(false) };
    static WIDE_PADDING: Cell<bool> = const { Cell::new(false) };
    static MEASURE_SIZE: Cell<u32> = const { Cell::new(16) };
    static MEASURE_BOLD: Cell<bool> = const { Cell::new(false) };
}

fn bump(counter: &'static std::thread::LocalKey<Cell<u32>>) {
    counter.with(|c| c.set(c.get() + 1));
}

fn get(counter: &'static std::thread::LocalKey<Cell<u32>>) -> u32 {
    counter.with(Cell::get)
}

fn focus_id(doc: &Document, element_id: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.focus();
    }
}

fn blur_id(doc: &Document, element_id: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.blur();
    }
}

fn check_focus(t: &Tour, doc: &Document) -> Result<(), Box<dyn Error>> {
    listen(doc, "f-div", "focus", |_| bump(&FOCUS_EVENTS))?;
    listen(doc, "f-input", "blur", |_| bump(&BLUR_EVENTS))?;

    focus_id(doc, "f-btn");
    t.check(
        "focus.button",
        focused(doc, "f-btn") == 1,
        format!("has_focus(button) = {}", focused(doc, "f-btn")),
    );

    focus_id(doc, "f-input");
    t.check(
        "focus.moves",
        focused(doc, "f-input") == 1 && focused(doc, "f-btn") == 0,
        format!(
            "input {}, button {}",
            focused(doc, "f-input"),
            focused(doc, "f-btn")
        ),
    );

    blur_id(doc, "f-input");
    t.check(
        "focus.blur",
        focused(doc, "f-input") == 0 && get(&BLUR_EVENTS) == 1,
        format!(
            "has_focus(input) = {}, blur events {}",
            focused(doc, "f-input"),
            get(&BLUR_EVENTS)
        ),
    );

    focus_id(doc, "f-div");
    t.check(
        "focus.tabindex",
        focused(doc, "f-div") == 1 && get(&FOCUS_EVENTS) == 1,
        format!(
            "has_focus(div) = {}, focus events {}",
            focused(doc, "f-div"),
            get(&FOCUS_EVENTS)
        ),
    );

    // A div without tabindex is not focusable (HTML focus()): focus changes nothing.
    focus_id(doc, "f-plain");
    t.known_bug(
        "focus.not_focusable",
        focused(doc, "f-plain") == 0 && focused(doc, "f-div") == 1,
        "focus() also focuses an element that is not focusable (div without tabindex)",
        format!(
            "plain {}, tabindex div {}",
            focused(doc, "f-plain"),
            focused(doc, "f-div")
        ),
    );

    blur_id(doc, "f-div");
    Ok(())
}

// ---- Pointer capture: set / has / release, the capture events, routing, implicit release -

fn capture(doc: &Document, on: bool) {
    if let Some(e) = by_id(doc, "cap-box") {
        let _ = if on {
            e.set_pointer_capture(1)
        } else {
            e.release_pointer_capture(1)
        };
    }
}

fn check_capture(t: &Tour, doc: &Document) -> Result<(), Box<dyn Error>> {
    let owner = doc.clone();
    listen(doc, "cap-box", "gotpointercapture", move |event| {
        bump(&GOT_CAPTURE);
        let is_box = matches!(event.current_target(&owner), Ok(Some(e)) if e.id().unwrap_or_default() == "cap-box");
        CAPTURE_TARGET.with(|c| c.set(is_box));
        if let Some(e) = by_id(&owner, "cap-box") {
            let _ = e.set_class_name("captured");
        }
    })?;
    let owner = doc.clone();
    listen(doc, "cap-box", "lostpointercapture", move |_| {
        bump(&LOST_CAPTURE);
        if let Some(e) = by_id(&owner, "cap-box") {
            let _ = e.set_class_name("");
        }
    })?;
    listen(doc, "cap-box", "pointermove", |_| bump(&CAPTURED_MOVES))?;

    t.check(
        "cap.initial",
        captured(doc, "cap-box", 1) == 0,
        format!("has_pointer_capture(1) = {}", captured(doc, "cap-box", 1)),
    );

    capture(doc, true);
    t.check(
        "cap.set",
        captured(doc, "cap-box", 1) == 1
            && captured(doc, "cap-box", 2) == 0
            && get(&GOT_CAPTURE) == 1,
        format!(
            "pointer 1 {}, pointer 2 {}, gotpointercapture {}",
            captured(doc, "cap-box", 1),
            captured(doc, "cap-box", 2),
            get(&GOT_CAPTURE)
        ),
    );
    // The capture events are dispatched at the capturing element.
    let target = CAPTURE_TARGET.with(Cell::get);
    t.known_bug(
        "cap.event_target",
        target,
        "gotpointercapture / lostpointercapture have no target or current target",
        format!("current target is #cap-box: {target}"),
    );

    // A move over another element is delivered to the capturing box.
    let moves = get(&CAPTURED_MOVES);
    mouse_at(doc, "cap-other", InjectedMouse::Moved, MouseButton::None);
    t.check(
        "cap.routes_moves",
        get(&CAPTURED_MOVES) == moves + 1,
        format!(
            "pointermove on the box {} -> {}",
            moves,
            get(&CAPTURED_MOVES)
        ),
    );

    capture(doc, false);
    t.check(
        "cap.release",
        captured(doc, "cap-box", 1) == 0 && get(&LOST_CAPTURE) == 1,
        format!(
            "has_pointer_capture(1) = {}, lostpointercapture {}",
            captured(doc, "cap-box", 1),
            get(&LOST_CAPTURE)
        ),
    );

    // Releasing the button ends a capture by itself (implicit release).
    capture(doc, true);
    mouse_at(doc, "cap-other", InjectedMouse::Pressed, MouseButton::Left);
    mouse_at(doc, "cap-other", InjectedMouse::Released, MouseButton::Left);
    t.check(
        "cap.implicit_release",
        captured(doc, "cap-box", 1) == 0 && get(&LOST_CAPTURE) == 2,
        format!(
            "has_pointer_capture(1) = {}, lostpointercapture {}",
            captured(doc, "cap-box", 1),
            get(&LOST_CAPTURE)
        ),
    );
    mouse_at(doc, "cap-other", InjectedMouse::Moved, MouseButton::None);
    Ok(())
}

// ---- Panel buttons ----------------------------------------------------------------------

fn show_geo(doc: &Document) {
    let (client, offset, r) = (
        client_of(doc, "geo-box"),
        offset_of(doc, "geo-box"),
        rect_of(doc, "geo-box"),
    );
    set_text(
        doc,
        "geo-out",
        &format!(
            "rect {:.0}x{:.0} | client {:.0},{:.0} {:.0}x{:.0} | offset {:.0},{:.0}",
            r.width, r.height, client.x, client.y, client.width, client.height, offset.x, offset.y
        ),
    );
}

fn show_scroll(doc: &Document) {
    let s = scroll_of(doc, "scroller");
    set_text(
        doc,
        "scroll-out",
        &format!("left={:.0} top={:.0}", s.x, s.y),
    );
}

fn show_pos(doc: &Document, placement: &str) {
    let (a, p) = (rect_of(doc, "pos-anchor"), rect_of(doc, "pos-popup"));
    set_text(
        doc,
        "pos-out",
        &format!("{placement}: dx={:.0} dy={:.0}", p.x - a.x, p.y - a.y),
    );
}

fn show_measure(doc: &Document) {
    let size = MEASURE_SIZE.with(Cell::get);
    let bold = MEASURE_BOLD.with(Cell::get);
    let weight: u16 = if bold { 700 } else { 400 };
    let m = measure(doc, "Hello KUI", "Segoe UI", size as f32, weight);
    set_style(doc, "measure-sample", "font-size", &format!("{size}px"));
    set_style(doc, "measure-sample", "font-weight", &weight.to_string());
    set_text(
        doc,
        "measure-out",
        &format!(
            "size={size}{} width={:.1} ascent={:.1} descent={:.1} line={:.1}",
            if bold { " bold" } else { "" },
            m.width,
            m.ascent,
            m.descent,
            m.line_height
        ),
    );
}

fn show_focus(doc: &Document, note: &str) {
    set_text(
        doc,
        "focus-out",
        &format!(
            "{note} | button={} input={} div={} plain={}",
            focused(doc, "f-btn"),
            focused(doc, "f-input"),
            focused(doc, "f-div"),
            focused(doc, "f-plain")
        ),
    );
}

fn show_capture(doc: &Document) {
    let state = if captured(doc, "cap-box", 1) == 1 {
        "yes"
    } else {
        "no"
    };
    set_text(
        doc,
        "cap-out",
        &format!(
            "captured={state} | got={} lost={} moves={}",
            get(&GOT_CAPTURE),
            get(&LOST_CAPTURE),
            get(&CAPTURED_MOVES)
        ),
    );
}

fn on<F>(ctx: &Context<'_>, element_id: &str, handler: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(&Document) + 'static,
{
    let doc = ctx.document.clone();
    listen(ctx.document, element_id, "click", move |_| handler(&doc))
}

fn bind_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-geo-show", show_geo)?;
    on(ctx, "btn-geo-pad", |doc| {
        let wide = !WIDE_PADDING.with(Cell::get);
        WIDE_PADDING.with(|c| c.set(wide));
        set_style(
            doc,
            "geo-box",
            "padding",
            if wide { "20px" } else { "12px" },
        );
        show_geo(doc);
    })?;
    on(ctx, "btn-scroll-down", |doc| {
        scroll_by(doc, 0.0, 40.0);
        show_scroll(doc);
    })?;
    on(ctx, "btn-scroll-right", |doc| {
        scroll_by(doc, 50.0, 0.0);
        show_scroll(doc);
    })?;
    on(ctx, "btn-scroll-end", |doc| {
        scroll_to(doc, 10000.0, 10000.0);
        show_scroll(doc);
    })?;
    on(ctx, "btn-scroll-top", |doc| {
        scroll_to(doc, 0.0, 0.0);
        show_scroll(doc);
    })?;
    for (button, placement) in [
        ("btn-pos-below", "bottom-start"),
        ("btn-pos-right", "right-center"),
        ("btn-pos-left", "left-end"),
    ] {
        on(ctx, button, move |doc| {
            place(doc, placement, 6.0, true, true, 4.0);
            show_pos(doc, placement);
        })?;
    }
    on(ctx, "btn-measure-smaller", |doc| {
        MEASURE_SIZE.with(|c| c.set(if c.get() > 8 { c.get() - 4 } else { c.get() }));
        show_measure(doc);
    })?;
    on(ctx, "btn-measure-larger", |doc| {
        MEASURE_SIZE.with(|c| c.set(if c.get() < 40 { c.get() + 4 } else { c.get() }));
        show_measure(doc);
    })?;
    on(ctx, "btn-measure-bold", |doc| {
        MEASURE_BOLD.with(|c| c.set(!c.get()));
        show_measure(doc);
    })?;
    on(ctx, "btn-focus-input", |doc| {
        focus_id(doc, "f-input");
        show_focus(doc, "focus(input)");
    })?;
    on(ctx, "btn-focus-div", |doc| {
        focus_id(doc, "f-div");
        show_focus(doc, "focus(div)");
    })?;
    on(ctx, "btn-focus-plain", |doc| {
        focus_id(doc, "f-plain");
        show_focus(doc, "focus(plain)");
    })?;
    on(ctx, "btn-focus-blur", |doc| {
        focus_id(doc, "f-input");
        let before = focused(doc, "f-input");
        blur_id(doc, "f-input");
        show_focus(
            doc,
            &format!("focus+blur(input) {before}->{}", focused(doc, "f-input")),
        );
    })?;
    on(ctx, "btn-cap-set", |doc| {
        capture(doc, true);
        show_capture(doc);
    })?;
    on(ctx, "btn-cap-release", |doc| {
        capture(doc, false);
        show_capture(doc);
    })?;
    on(ctx, "btn-cap-state", show_capture)
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document;
    let t = Tour::new(doc);
    check_box_metrics(&t, doc);
    check_scrolling(&t, doc);
    check_scroll_reveal(&t, doc, &*ctx.platform);
    check_placement(&t, doc);
    check_measure(&t, doc);
    check_focus(&t, doc)?;
    check_capture(&t, doc)?;
    t.finish();
    bind_buttons(ctx)?;
    show_measure(doc);
    Ok(())
}
