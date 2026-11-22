// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_events.rs; edit that file, not this copy.
//! API tour: DOM events from the host (examples/api_tours/events). On start the host registers
//! listeners, dispatches synthetic events (Element::dispatch_event) and injects real mouse and
//! keyboard input (Document::dispatch_mouse_event / dispatch_key_event, the same dispatch path as
//! the OS), then checks what every listener saw: propagation order, current target,
//! stop_propagation, flags and prevent_default, listener removal, mouse button and coordinates,
//! related targets, key / code / modifiers, focus events and the whole drag-and-drop data API.
//! One row per check goes into #tour-log; #tour-result ends as "PASS n/n". Afterwards the panels
//! react to real input: every listener writes what it read into the panel's output line.

use crate::canonical_runner::Context;
use kui::{
    Document, DropEffect, EffectAllowed, Element, Event, EventTypeTag, InjectedKey, InjectedMouse,
    ListenerId, Modifiers, MouseButton, MouseInjection, Point, Rect,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

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

// ---- Small helpers ------------------------------------------------------------------------

fn by_id<'d>(doc: &'d Document, element_id: &str) -> Option<Element<'d>> {
    doc.get_element_by_id(element_id).ok().flatten()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.51
}

fn id_of(element: kui::Result<Option<Element<'_>>>) -> String {
    element
        .ok()
        .flatten()
        .and_then(|e| e.id().ok())
        .unwrap_or_default()
}

fn or_dash(text: &str) -> &str {
    if text.is_empty() {
        "-"
    } else {
        text
    }
}

/// The KuiStatus of a failed call (0 when it succeeded).
fn status<T>(result: &kui::Result<T>) -> i32 {
    result.as_ref().err().map(|e| e.status()).unwrap_or(0)
}

const NOT_SUPPORTED: i32 = kui::sys::KUI_ERR_NOT_SUPPORTED;

fn set_text(doc: &Document, element_id: &str, value: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_text_content(value);
    }
}

fn set_class(doc: &Document, element_id: &str, value: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_class_name(value);
    }
}

fn rect_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.bounding_rect().ok())
        .unwrap_or_default()
}

/// Dispatches a synthetic event; Some(not canceled) when the dispatch itself worked.
fn dispatch(
    doc: &Document,
    element_id: &str,
    name: &str,
    bubbles: bool,
    cancelable: bool,
) -> Option<bool> {
    by_id(doc, element_id).and_then(|e| e.dispatch_event(name, bubbles, cancelable).ok())
}

/// Registers a listener on the element with that id; the closure gets its own Document.
fn listen<F>(
    doc: &Document,
    element_id: &str,
    name: &str,
    mut listener: F,
) -> Result<ListenerId, Box<dyn Error>>
where
    F: FnMut(&Document, &Event<'_>) + 'static,
{
    let owner = doc.clone();
    let element = doc
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    Ok(element.add_event_listener(name, move |event| listener(&owner, event))?)
}

/// Injected real input, in viewport coordinates.
fn mouse(doc: &Document, kind: InjectedMouse, x: f32, y: f32, button: MouseButton, clicks: i32) {
    let _ = doc.dispatch_mouse_event(&MouseInjection {
        kind,
        position: Point { x, y },
        button,
        modifiers: Modifiers::default(),
        click_count: clicks,
        delta: Point::default(),
    });
}

fn click_at(doc: &Document, element_id: &str, button: MouseButton, clicks: i32) {
    let r = rect_of(doc, element_id);
    let (x, y) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    mouse(doc, InjectedMouse::Moved, x, y, MouseButton::None, 0);
    mouse(doc, InjectedMouse::Pressed, x, y, button, clicks);
    mouse(doc, InjectedMouse::Released, x, y, button, clicks);
}

fn move_to(doc: &Document, element_id: &str) {
    let r = rect_of(doc, element_id);
    mouse(
        doc,
        InjectedMouse::Moved,
        r.x + r.width / 2.0,
        r.y + r.height / 2.0,
        MouseButton::None,
        0,
    );
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

// ---- Shared state (listeners write, checks and buttons read) ------------------------------

#[derive(Default, Clone, Copy)]
struct MouseRecord {
    seen: u32,
    button: Option<MouseButton>,
    tag: Option<EventTypeTag>,
    client: Point,
    offset: Point,
    page: Point,
    bubbles: bool,
    cancelable: bool,
    prevented: bool,
}

#[derive(Default)]
struct ProbeResult {
    event_type: String,
    tag: Option<EventTypeTag>,
    bubbles: bool,
    cancelable: bool,
    prevented: bool,
    related: Option<bool>, // Some(has a related target) when the call worked
    mouse_xy: i32,
    mouse_button: i32,
    key: i32,
    code: i32,
    modifiers: i32,
    drag_types: i32,
    drag_data: i32,
    drag_files: i32,
}

#[derive(Default)]
struct KeyRecord {
    seen: bool,
    key: String,
    code: String,
    modifiers: Modifiers,
    tag: Option<EventTypeTag>,
    mouse: i32,
}

#[derive(Default)]
struct DragRecord {
    seq: Vec<String>, // event types, repeats folded
    start_types: Vec<String>,
    start_read: String,
    start_allowed: Option<EffectAllowed>,
    over_effect: Option<DropEffect>,
    drops: u32,
    drop_plain: String,
    drop_custom: String,
    junk_found: bool,
    drop_types: usize,
    drop_files: Option<Vec<String>>,
    drop_allowed: Option<EffectAllowed>,
    drop_effect: Option<DropEffect>,
    drop_tag: Option<EventTypeTag>,
    drop_at: Point,
    cleared_types: usize,
    end_effect: Option<DropEffect>,
}

#[derive(Default)]
struct State {
    trace: Vec<String>,   // current targets, in call order
    targets: Vec<String>, // targets, in call order
    last_flags: Option<(bool, bool, bool)>,
    tick_trace: Vec<String>,
    tick_ids: Vec<ListenerId>,
    tick_serial: u32,
    probe: ProbeResult,
    click: MouseRecord,
    down: MouseRecord,
    dbl: MouseRecord,
    ctx_menu: MouseRecord,
    enter: String,
    leave: String,
    enter_bubbles: Option<bool>,
    key: KeyRecord,
    blur_line: String,
    focus_line: String,
    focus_tag: Option<EventTypeTag>,
    drag: DragRecord,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|state| f(&mut state.borrow_mut()))
}

// ---- Propagation: one listener per layer records where the event is ----------------------

const LAYERS: [&str; 3] = ["ev-outer", "ev-middle", "ev-inner"];

fn on_layer(doc: &Document, event: &Event<'_>) {
    let current = id_of(event.current_target(doc));
    let target = id_of(event.target(doc));
    let flags = event
        .flags()
        .ok()
        .map(|f| (f.bubbles, f.cancelable, f.default_prevented));
    with_state(|s| {
        s.trace.push(current.trim_start_matches("ev-").to_string());
        s.targets.push(target.trim_start_matches("ev-").to_string());
        s.last_flags = flags;
    });
    if let Ok(Some(element)) = event.current_target(doc) {
        let _ = element.class_list_add("hit");
    }
}

/// Fires an event at a layer; returns not_canceled as 0 / 1 (-1 when the dispatch failed).
fn fire_layers(
    doc: &Document,
    element_id: &str,
    name: &str,
    bubbles: bool,
    cancelable: bool,
) -> i32 {
    with_state(|s| {
        s.trace.clear();
        s.targets.clear();
        s.last_flags = None;
    });
    for layer in LAYERS {
        if let Some(e) = by_id(doc, layer) {
            let _ = e.class_list_remove("hit");
        }
    }
    dispatch(doc, element_id, name, bubbles, cancelable).map_or(-1, |v| v as i32)
}

fn bind_layers(doc: &Document) -> Result<(), Box<dyn Error>> {
    for layer in LAYERS {
        for name in ["ping", "halt", "veto"] {
            listen(doc, layer, name, on_layer)?;
        }
    }
    // "halt": the middle layer stops the event after its own listeners.
    listen(doc, "ev-middle", "halt", |_, event| {
        let _ = event.stop_propagation();
    })?;
    // "veto": the inner layer cancels the default action.
    listen(doc, "ev-inner", "veto", |_, event| {
        let _ = event.prevent_default();
    })?;
    Ok(())
}

fn trace() -> (String, String, Option<(bool, bool, bool)>) {
    with_state(|s| (s.trace.join(","), s.targets.join(","), s.last_flags))
}

fn flag(flags: Option<(bool, bool, bool)>, index: usize) -> i32 {
    flags.map_or(-1, |f| [f.0, f.1, f.2][index] as i32)
}

fn check_propagation(t: &Tour, doc: &Document) {
    fire_layers(doc, "ev-inner", "ping", true, false);
    let (order, targets, _) = trace();
    t.check(
        "prop.bubble_order",
        order == "inner,middle,outer",
        format!("current targets '{order}'"),
    );
    t.check(
        "prop.target",
        targets == "inner,inner,inner",
        format!("targets '{targets}'"),
    );

    fire_layers(doc, "ev-inner", "ping", false, false);
    let (order, _, flags) = trace();
    t.check(
        "prop.non_bubbling",
        order == "inner" && flag(flags, 0) == 0,
        format!("current targets '{order}', bubbles {}", flag(flags, 0)),
    );

    fire_layers(doc, "ev-middle", "ping", true, false);
    let (order, targets, _) = trace();
    t.check(
        "prop.from_middle",
        order == "middle,outer" && targets == "middle,middle",
        format!("current targets '{order}', targets '{targets}'"),
    );

    fire_layers(doc, "ev-inner", "halt", true, false);
    let (order, _, _) = trace();
    t.check(
        "prop.stop_propagation",
        order == "inner,middle",
        format!("current targets '{order}'"),
    );

    let not_canceled = fire_layers(doc, "ev-inner", "veto", true, true);
    let (order, _, flags) = trace();
    t.check(
        "cancel.prevent_default",
        not_canceled == 0 && flags == Some((true, true, true)) && order == "inner,middle,outer",
        format!("not canceled {not_canceled}, flags at outer {flags:?}, trace '{order}'"),
    );

    // prevent_default on a non-cancelable event changes nothing.
    let not_canceled = fire_layers(doc, "ev-inner", "veto", true, false);
    let (_, _, flags) = trace();
    t.check(
        "cancel.not_cancelable",
        not_canceled == 1 && flag(flags, 2) == 0 && flag(flags, 1) == 0,
        format!("not canceled {not_canceled}, flags at outer {flags:?}"),
    );

    // A detached element cannot be dispatched at (kui_c.h).
    let detached = match doc.create_element("div") {
        Ok(loose) => status(&loose.dispatch_event("ping", true, false)),
        Err(error) => error.status(),
    };
    t.known_bug(
        "dispatch.detached",
        detached == kui::sys::KUI_ERR_INVALID_ARG,
        "dispatch_event on an element that is not in the document returns KUI_OK",
        format!("status {detached}"),
    );
}

// ---- Listener management: ids, removing one, removing all of one name -------------------

fn add_tick(doc: &Document) -> Option<ListenerId> {
    let name = with_state(|s| {
        s.tick_serial += 1;
        s.tick_serial.to_string()
    });
    listen(doc, "lm-target", "tick", move |_, _| {
        with_state(|s| s.tick_trace.push(name.clone()))
    })
    .ok()
}

fn fire_tick(doc: &Document, name: &str) -> String {
    with_state(|s| s.tick_trace.clear());
    dispatch(doc, "lm-target", name, false, false);
    with_state(|s| s.tick_trace.join(","))
}

fn check_listeners(t: &Tour, doc: &Document) -> Result<(), Box<dyn Error>> {
    let target = by_id(doc, "lm-target").ok_or("page has no #lm-target")?;
    let (first, second, third) = (add_tick(doc), add_tick(doc), add_tick(doc));
    let ran = fire_tick(doc, "tick");
    let ids_distinct =
        matches!((first, second, third), (Some(a), Some(b), Some(c)) if a != b && b != c);
    t.check(
        "listen.registration_order",
        ran == "1,2,3" && ids_distinct,
        format!("ran '{ran}'"),
    );

    let removed = second
        .ok_or("no listener id")
        .map(|id| target.remove_event_listener("tick", id))?;
    let ran = fire_tick(doc, "tick");
    t.check(
        "listen.remove_event_listener",
        removed.is_ok() && ran == "1,3",
        format!("result {removed:?}, ran '{ran}'"),
    );

    listen(doc, "lm-target", "tock", |_, _| {
        with_state(|s| s.tick_trace.push("tock".to_string()))
    })?;
    let removed = target.remove_event_listeners("tick");
    let ran = fire_tick(doc, "tick");
    t.check(
        "listen.remove_event_listeners",
        removed.is_ok() && ran.is_empty(),
        format!("result {removed:?}, ran '{ran}'"),
    );
    let ran = fire_tick(doc, "tock");
    t.check(
        "listen.remove_keeps_other_names",
        ran == "tock",
        format!("ran '{ran}'"),
    );
    let _ = target.remove_event_listeners("tock");
    with_state(|s| s.tick_serial = 0);
    Ok(())
}

// ---- Event object: a synthetic event and the accessors of the other kinds ---------------

fn on_probe(doc: &Document, event: &Event<'_>) {
    let flags = event.flags();
    let probe = ProbeResult {
        event_type: event.event_type().unwrap_or_default(),
        tag: event.type_tag().ok(),
        bubbles: flags.as_ref().is_ok_and(|f| f.bubbles),
        cancelable: flags.as_ref().is_ok_and(|f| f.cancelable),
        prevented: flags.as_ref().is_ok_and(|f| f.default_prevented),
        related: event.related_target(doc).ok().map(|r| r.is_some()),
        mouse_xy: status(&event.mouse_client()),
        mouse_button: status(&event.mouse_button()),
        key: status(&event.key()),
        code: status(&event.code()),
        modifiers: status(&event.modifiers()),
        drag_types: status(&event.drag_types()),
        drag_data: status(&event.drag_data("text/plain")),
        drag_files: status(&event.drag_files()),
    };
    with_state(|s| s.probe = probe);
}

fn check_probe(t: &Tour, doc: &Document) {
    dispatch(doc, "probe", "probe", true, false);
    with_state(|s| {
        let p = &s.probe;
        t.check(
            "event.type",
            p.event_type == "probe" && p.tag == Some(EventTypeTag::Base),
            format!("type '{}', tag {:?}", p.event_type, p.tag),
        );
        t.check(
            "event.flags",
            p.bubbles && !p.cancelable && !p.prevented,
            format!(
                "bubbles {} cancelable {} prevented {}",
                p.bubbles, p.cancelable, p.prevented
            ),
        );
        t.check(
            "event.no_related_target",
            p.related == Some(false),
            format!("related {:?}", p.related),
        );
        t.check(
            "event.mouse_not_supported",
            p.mouse_xy == NOT_SUPPORTED && p.mouse_button == NOT_SUPPORTED,
            format!("client {}, button {}", p.mouse_xy, p.mouse_button),
        );
        t.check(
            "event.key_not_supported",
            p.key == NOT_SUPPORTED && p.code == NOT_SUPPORTED && p.modifiers == NOT_SUPPORTED,
            format!("key {}, code {}, modifiers {}", p.key, p.code, p.modifiers),
        );
        t.check(
            "event.drag_not_supported",
            p.drag_types == NOT_SUPPORTED
                && p.drag_data == NOT_SUPPORTED
                && p.drag_files == NOT_SUPPORTED,
            format!(
                "types {}, data {}, files {}",
                p.drag_types, p.drag_data, p.drag_files
            ),
        );
    });
}

// ---- Mouse: button, three coordinate spaces, related targets ----------------------------

fn button_number(button: Option<MouseButton>) -> i32 {
    match button {
        Some(MouseButton::None) => 0,
        Some(MouseButton::Left) => 1,
        Some(MouseButton::Middle) => 2,
        Some(MouseButton::Right) => 3,
        Some(MouseButton::Other(value)) => value,
        None => -1,
    }
}

fn write_mouse_line(doc: &Document, event_type: &str, r: &MouseRecord) {
    let Some(out) = by_id(doc, "mouse-last") else {
        return;
    };
    let _ = out.set_text_content(&format!(
        "{event_type} button={} client={:.0},{:.0} offset={:.0},{:.0} page={:.0},{:.0}",
        button_number(r.button),
        r.client.x,
        r.client.y,
        r.offset.x,
        r.offset.y,
        r.page.x,
        r.page.y
    ));
    let _ = out.set_attribute("data-type", event_type);
    let _ = out.set_attribute("data-button", &button_number(r.button).to_string());
    for (name, value) in [
        ("data-cx", r.client.x),
        ("data-cy", r.client.y),
        ("data-ox", r.offset.x),
        ("data-oy", r.offset.y),
        ("data-px", r.page.x),
        ("data-py", r.page.y),
    ] {
        let _ = out.set_attribute(name, &format!("{value:.2}"));
    }
}

fn on_mouse(doc: &Document, event: &Event<'_>, pick: fn(&mut State) -> &mut MouseRecord) {
    let event_type = event.event_type().unwrap_or_default();
    // The pad keeps its own context menu: cancel the default one.
    if event_type == "contextmenu" {
        let _ = event.prevent_default();
    }
    let flags = event.flags();
    let record = with_state(|s| {
        let r = pick(s);
        r.seen += 1;
        r.button = event.mouse_button().ok();
        r.tag = event.type_tag().ok();
        r.client = event.mouse_client().unwrap_or_default();
        r.offset = event.mouse_offset().unwrap_or_default();
        r.page = event.mouse_page().unwrap_or_default();
        r.bubbles = flags.as_ref().is_ok_and(|f| f.bubbles);
        r.cancelable = flags.as_ref().is_ok_and(|f| f.cancelable);
        r.prevented = flags.as_ref().is_ok_and(|f| f.default_prevented);
        *r
    });
    if event_type != "mousedown" {
        write_mouse_line(doc, &event_type, &record);
    }
}

fn on_hover(doc: &Document, event: &Event<'_>) {
    let event_type = event.event_type().unwrap_or_default();
    let current = id_of(event.current_target(doc));
    let related = id_of(event.related_target(doc));
    let bubbles = event.flags().ok().map(|f| f.bubbles);
    if let Ok(Some(element)) = event.current_target(doc) {
        let _ = if event_type == "mouseenter" {
            element.class_list_add("in")
        } else {
            element.class_list_remove("in")
        };
    }
    let line = with_state(|s| {
        if event_type == "mouseenter" {
            s.enter = format!("#{current} from #{}", or_dash(&related));
            s.enter_bubbles = bubbles;
        } else {
            s.leave = format!("#{current} to #{}", or_dash(&related));
        }
        format!(
            "enter: {} | leave: {}",
            or_dash(&s.enter),
            or_dash(&s.leave)
        )
    });
    set_text(doc, "hover-out", &line);
}

fn bind_mouse(doc: &Document) -> Result<(), Box<dyn Error>> {
    listen(doc, "mouse-pad", "click", |d, e| {
        on_mouse(d, e, |s| &mut s.click)
    })?;
    listen(doc, "mouse-pad", "mousedown", |d, e| {
        on_mouse(d, e, |s| &mut s.down)
    })?;
    listen(doc, "mouse-pad", "dblclick", |d, e| {
        on_mouse(d, e, |s| &mut s.dbl)
    })?;
    listen(doc, "mouse-pad", "contextmenu", |d, e| {
        on_mouse(d, e, |s| &mut s.ctx_menu)
    })?;
    for element_id in ["hover-a", "hover-b"] {
        for name in ["mouseenter", "mouseleave"] {
            listen(doc, element_id, name, on_hover)?;
        }
    }
    Ok(())
}

// ---- Wheel: the scroll amount of an injected wheel event -----------------------------------------------

/// Capture phase: listeners registered with `add_capture_event_listener` run while the event travels down.
fn check_capture(t: &Tour, doc: &Document) {
    let (Ok(Some(outer)), Ok(Some(middle)), Ok(Some(inner))) = (
        doc.get_element_by_id("ev-outer"),
        doc.get_element_by_id("ev-middle"),
        doc.get_element_by_id("ev-inner"),
    ) else {
        t.check(
            "capture.setup",
            false,
            "missing the propagation layers".into(),
        );
        return;
    };
    let order = Rc::new(RefCell::new(String::new()));
    let tag = |name: &'static str| {
        let order = order.clone();
        move |_: &kui::Event<'_>| {
            let mut text = order.borrow_mut();
            text.push_str(name);
            text.push(' ');
        }
    };
    let outer_plain = outer.add_event_listener("capture-probe", tag("op"));
    let middle_plain = middle.add_event_listener("capture-probe", tag("mp"));
    let inner_plain = inner.add_event_listener("capture-probe", tag("ip"));
    let inner_capture = inner.add_capture_event_listener("capture-probe", tag("ic"));
    let outer_capture = outer.add_capture_event_listener("capture-probe", tag("oc"));
    let _ = inner.dispatch_event("capture-probe", true, false);
    t.check(
        "capture.order",
        *order.borrow() == "oc ic ip mp op ",
        format!("order '{}'", order.borrow()),
    );

    if let Ok(id) = outer_capture {
        let _ = outer.remove_event_listener("capture-probe", id);
    }
    order.borrow_mut().clear();
    let _ = inner.dispatch_event("capture-probe", true, false);
    t.check(
        "capture.remove",
        *order.borrow() == "ic ip mp op ",
        format!("order '{}'", order.borrow()),
    );

    if let Ok(id) = outer_plain {
        let _ = outer.remove_event_listener("capture-probe", id);
    }
    if let Ok(id) = middle_plain {
        let _ = middle.remove_event_listener("capture-probe", id);
    }
    if let Ok(id) = inner_plain {
        let _ = inner.remove_event_listener("capture-probe", id);
    }
    if let Ok(id) = inner_capture {
        let _ = inner.remove_event_listener("capture-probe", id);
    }
}

fn check_wheel(t: &Tour, doc: &Document) {
    let Ok(Some(pad)) = doc.get_element_by_id("mouse-pad") else {
        t.check("wheel.setup", false, "missing #mouse-pad".into());
        return;
    };
    let rect = rect_of(doc, "mouse-pad");
    let (x, y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let seen = Rc::new(Cell::new(0));
    let delta = Rc::new(Cell::new((-1.0f32, -1.0f32)));
    let wheel_error = Rc::new(RefCell::new(String::new()));
    let (seen_in, delta_in, error_in) = (seen.clone(), delta.clone(), wheel_error.clone());
    let wheel_id = pad.add_event_listener("wheel", move |event| {
        seen_in.set(seen_in.get() + 1);
        match event.wheel_delta() {
            Ok(point) => delta_in.set((point.x, point.y)),
            Err(error) => *error_in.borrow_mut() = error.to_string(),
        }
    });
    let _ = doc.dispatch_mouse_event(&MouseInjection {
        kind: InjectedMouse::Wheel,
        position: Point { x, y },
        button: MouseButton::None,
        modifiers: Modifiers::default(),
        click_count: 0,
        delta: Point { x: 3.0, y: -7.5 },
    });
    if let Ok(id) = wheel_id {
        let _ = pad.remove_event_listener("wheel", id);
    }
    let (dx, dy) = delta.get();
    t.check(
        "wheel.delta",
        seen.get() == 1 && wheel_error.borrow().is_empty() && near(dx, 3.0) && near(dy, -7.5),
        format!(
            "seen {}, delta {dx:.1},{dy:.1}, error '{}'",
            seen.get(),
            wheel_error.borrow()
        ),
    );

    // Only a wheel event has a scroll amount.
    let probe_status = Rc::new(Cell::new(0));
    let status_in = probe_status.clone();
    let probe_id = pad.add_event_listener("wheel-probe", move |event| {
        if let Err(error) = event.wheel_delta() {
            status_in.set(error.status() as i32);
        }
    });
    let _ = pad.dispatch_event("wheel-probe", false, false);
    if let Ok(id) = probe_id {
        let _ = pad.remove_event_listener("wheel-probe", id);
    }
    t.check(
        "wheel.delta_of_a_non_wheel_event",
        probe_status.get() == kui::sys::KUI_ERR_NOT_SUPPORTED as i32,
        format!("status {}", probe_status.get()),
    );
}

fn check_mouse(t: &Tour, doc: &Document) {
    let pad = rect_of(doc, "mouse-pad");
    let (cx, cy) = (pad.x + pad.width / 2.0, pad.y + pad.height / 2.0);
    let scroll_top = doc
        .root()
        .ok()
        .flatten()
        .and_then(|root| root.scroll_metrics().ok())
        .map_or(0.0, |s| s.top);

    click_at(doc, "mouse-pad", MouseButton::Left, 1);
    let c = with_state(|s| s.click);
    t.check(
        "mouse.click",
        c.seen == 1
            && c.button == Some(MouseButton::Left)
            && c.tag == Some(EventTypeTag::Mouse)
            && c.bubbles
            && c.cancelable,
        format!(
            "seen {}, button {:?}, tag {:?}, bubbles {} cancelable {}",
            c.seen, c.button, c.tag, c.bubbles, c.cancelable
        ),
    );
    t.check(
        "mouse.client_xy",
        near(c.client.x, cx) && near(c.client.y, cy),
        format!(
            "client {:.1},{:.1}, pad centre {cx:.1},{cy:.1}",
            c.client.x, c.client.y
        ),
    );
    // Offset coordinates are relative to the padding box: inside the 4px border.
    t.known_bug(
        "mouse.offset_xy",
        near(c.offset.x, cx - pad.x - 4.0) && near(c.offset.y, cy - pad.y - 4.0),
        "mouse events never get offset coordinates (always 0,0)",
        format!(
            "offset {:.1},{:.1}, want {:.1},{:.1}",
            c.offset.x,
            c.offset.y,
            cx - pad.x - 4.0,
            cy - pad.y - 4.0
        ),
    );
    t.known_bug(
        "mouse.page_xy",
        near(c.page.x, c.client.x) && near(c.page.y, c.client.y + scroll_top),
        "mouse events never get page coordinates (always 0,0)",
        format!(
            "page {:.1},{:.1}, client {:.1},{:.1}, document scroll {scroll_top:.1}",
            c.page.x, c.page.y, c.client.x, c.client.y
        ),
    );

    click_at(doc, "mouse-pad", MouseButton::Right, 1);
    let (down, ctx_menu) = with_state(|s| (s.down, s.ctx_menu));
    t.check(
        "mouse.right_button",
        down.button == Some(MouseButton::Right)
            && ctx_menu.seen == 1
            && ctx_menu.button == Some(MouseButton::Right)
            && ctx_menu.prevented,
        format!(
            "mousedown button {:?}, contextmenu seen {} button {:?} prevented {}",
            down.button, ctx_menu.seen, ctx_menu.button, ctx_menu.prevented
        ),
    );

    // kui_c.h: click count 2 injects the second click of a double-click.
    click_at(doc, "mouse-pad", MouseButton::Left, 2);
    let seen = with_state(|s| s.dbl.seen);
    t.known_bug(
        "mouse.injected_click_count",
        seen == 1,
        "kui_devtools_dispatch_mouse_event ignores click_count (counts clicks by timing)",
        format!("dblclick seen {seen}"),
    );
    // Two clicks in a row on the same spot make a double-click.
    with_state(|s| s.dbl.seen = 0);
    click_at(doc, "mouse-pad", MouseButton::Left, 1);
    click_at(doc, "mouse-pad", MouseButton::Left, 2);
    let dbl = with_state(|s| s.dbl);
    t.check(
        "mouse.dblclick",
        dbl.seen == 1 && dbl.button == Some(MouseButton::Left),
        format!("dblclick seen {} button {:?}", dbl.seen, dbl.button),
    );

    move_to(doc, "hover-a");
    move_to(doc, "hover-b");
    let (enter, leave, bubbles) =
        with_state(|s| (s.enter.clone(), s.leave.clone(), s.enter_bubbles));
    t.check(
        "mouse.related_target",
        leave == "#hover-a to #hover-b" && enter == "#hover-b from #hover-a",
        format!("leave '{leave}', enter '{enter}'"),
    );
    // UI Events: mouseenter / mouseleave do not bubble.
    t.known_bug(
        "mouse.enter_does_not_bubble",
        bubbles == Some(false),
        "mouseenter / mouseleave are dispatched as bubbling events",
        format!("bubbles {bubbles:?}"),
    );
    move_to(doc, "probe");
}

// ---- Keyboard and focus ------------------------------------------------------------------

fn on_key(doc: &Document, event: &Event<'_>) {
    let record = KeyRecord {
        seen: true,
        key: event.key().unwrap_or_default(),
        code: event.code().unwrap_or_default(),
        modifiers: event.modifiers().unwrap_or_default(),
        tag: event.type_tag().ok(),
        mouse: status(&event.mouse_client()),
    };
    let m = record.modifiers;
    let mods: Vec<&str> = [
        (m.ctrl, "ctrl"),
        (m.shift, "shift"),
        (m.alt, "alt"),
        (m.meta, "meta"),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|(_, name)| *name)
    .collect();
    let mods = if mods.is_empty() {
        "none".to_string()
    } else {
        mods.join(",")
    };
    set_text(
        doc,
        "key-out",
        &format!(
            "keydown key={} code={} mods={mods}",
            record.key, record.code
        ),
    );
    with_state(|s| s.key = record);
}

fn on_focus_change(doc: &Document, event: &Event<'_>) {
    let event_type = event.event_type().unwrap_or_default();
    let current = id_of(event.current_target(doc));
    let related = id_of(event.related_target(doc));
    let tag = event.type_tag().ok();
    let line = with_state(|s| {
        s.focus_tag = tag;
        if event_type == "blur" {
            s.blur_line = format!("blur #{current} to #{}", or_dash(&related));
            s.blur_line.clone()
        } else {
            s.focus_line = format!("focus #{current} from #{}", or_dash(&related));
            s.focus_line.clone()
        }
    });
    set_text(doc, "focus-out", &line);
}

fn bind_keyboard(doc: &Document) -> Result<(), Box<dyn Error>> {
    listen(doc, "key-pad", "keydown", on_key)?;
    listen(doc, "key-pad", "focus", on_focus_change)?;
    listen(doc, "key-pad", "blur", on_focus_change)?;
    listen(doc, "key-other", "focus", on_focus_change)?;
    Ok(())
}

fn check_keyboard(t: &Tour, doc: &Document) {
    focus_id(doc, "key-pad");
    let ctrl_shift = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::default()
    };
    let _ = doc.dispatch_key_event(InjectedKey::Down, "a", "KeyA", ctrl_shift, "", false);
    let _ = doc.dispatch_key_event(InjectedKey::Up, "a", "KeyA", ctrl_shift, "", false);
    with_state(|s| {
        let k = &s.key;
        t.check(
            "key.key_code",
            k.seen && k.key == "a" && k.code == "KeyA",
            format!("seen {} key '{}' code '{}'", k.seen, k.key, k.code),
        );
        t.check(
            "key.modifiers",
            k.modifiers == ctrl_shift,
            format!("{:?}", k.modifiers),
        );
        t.check(
            "key.type_tag",
            k.tag == Some(EventTypeTag::Keyboard) && k.mouse == NOT_SUPPORTED,
            format!("tag {:?}, mouse accessor status {}", k.tag, k.mouse),
        );
    });

    focus_id(doc, "key-other");
    let (blur_line, focus_line, tag) =
        with_state(|s| (s.blur_line.clone(), s.focus_line.clone(), s.focus_tag));
    t.check(
        "focus.blur_related_target",
        blur_line == "blur #key-pad to #key-other",
        format!("'{blur_line}'"),
    );
    t.check(
        "focus.focus_related_target",
        focus_line == "focus #key-other from #key-pad" && tag == Some(EventTypeTag::Focus),
        format!("'{focus_line}', tag {tag:?}"),
    );
    blur_id(doc, "key-other");
}

// ---- Drag and drop: the data API from dragstart to dragend ------------------------------

fn effect_name(effect: Option<DropEffect>) -> &'static str {
    match effect {
        Some(DropEffect::Copy) => "copy",
        Some(DropEffect::Move) => "move",
        Some(DropEffect::Link) => "link",
        _ => "none",
    }
}

fn allowed_name(effect: Option<EffectAllowed>) -> &'static str {
    match effect {
        Some(EffectAllowed::None) => "none",
        Some(EffectAllowed::Copy) => "copy",
        Some(EffectAllowed::Move) => "move",
        Some(EffectAllowed::Link) => "link",
        Some(EffectAllowed::CopyMove) => "copyMove",
        Some(EffectAllowed::CopyLink) => "copyLink",
        Some(EffectAllowed::MoveLink) => "moveLink",
        Some(EffectAllowed::All) => "all",
        _ => "uninitialized",
    }
}

fn note_drag(doc: &Document, event_type: &str) {
    let seq = with_state(|s| {
        if s.drag.seq.last().map(String::as_str) == Some(event_type) {
            return None;
        }
        s.drag.seq.push(event_type.to_string());
        Some(s.drag.seq.join(" > "))
    });
    if let Some(seq) = seq {
        set_text(doc, "drag-seq", &seq);
    }
}

fn on_drag_source(doc: &Document, event: &Event<'_>) {
    let event_type = event.event_type().unwrap_or_default();
    if event_type == "dragstart" {
        with_state(|s| s.drag = DragRecord::default());
        set_class(doc, "drop-zone", "");
        set_text(doc, "drop-out", "dragging...");
        let _ = event.set_effect_allowed(EffectAllowed::CopyMove);
        let _ = event.set_drag_data("text/plain", "kui-chip");
        let _ = event.set_drag_data("text/x-kui", "42");
        let _ = event.set_drag_data("text/x-junk", "scratch");
        let _ = event.clear_drag_data(Some("text/x-junk"));
        let types = event.drag_types().unwrap_or_default();
        let read = event
            .drag_data("text/plain")
            .ok()
            .flatten()
            .unwrap_or_default();
        let allowed = event.effect_allowed().ok();
        with_state(|s| {
            s.drag.start_types = types;
            s.drag.start_read = read;
            s.drag.start_allowed = allowed;
        });
    } else if event_type == "dragend" {
        let effect = event.drop_effect().ok();
        with_state(|s| s.drag.end_effect = effect);
    }
    note_drag(doc, &event_type);
}

fn on_drop_zone(doc: &Document, event: &Event<'_>) {
    let event_type = event.event_type().unwrap_or_default();
    note_drag(doc, &event_type);
    match event_type.as_str() {
        "dragenter" | "dragover" => {
            // Accept the drop and ask for a copy.
            let _ = event.prevent_default();
            let _ = event.set_drop_effect(DropEffect::Copy);
            let effect = event.drop_effect().ok();
            with_state(|s| s.drag.over_effect = effect);
            set_class(doc, "drop-zone", "over");
            return;
        }
        "dragleave" => {
            if with_state(|s| s.drag.drops) == 0 {
                set_class(doc, "drop-zone", "");
            }
            return;
        }
        _ => {}
    }
    // drop
    let _ = event.prevent_default();
    let plain = event
        .drag_data("text/plain")
        .ok()
        .flatten()
        .unwrap_or_default();
    let custom = event
        .drag_data("text/x-kui")
        .ok()
        .flatten()
        .unwrap_or_default();
    let junk_found = matches!(event.drag_data("text/x-junk"), Ok(Some(_)));
    let types = event.drag_types().map_or(0, |t| t.len());
    let files = event.drag_files().ok();
    let allowed = event.effect_allowed().ok();
    let effect = event.drop_effect().ok();
    let tag = event.type_tag().ok();
    let at = event.mouse_client().unwrap_or_default();
    // Done with the data: clear every format.
    let _ = event.clear_drag_data(None);
    let cleared = event.drag_types().map_or(usize::MAX, |t| t.len());

    let file_list = files
        .as_ref()
        .filter(|f| !f.is_empty())
        .map(|f| format!(": {}", f.join(",")))
        .unwrap_or_default();
    let line = format!(
        "dropped '{plain}' + '{custom}' | types {types} | files {}{file_list} | effect {} / allowed {}",
        files.as_ref().map_or(0, |f| f.len()),
        effect_name(effect),
        allowed_name(allowed)
    );
    with_state(|s| {
        let d = &mut s.drag;
        d.drops += 1;
        d.drop_plain = plain;
        d.drop_custom = custom;
        d.junk_found = junk_found;
        d.drop_types = types;
        d.drop_files = files;
        d.drop_allowed = allowed;
        d.drop_effect = effect;
        d.drop_tag = tag;
        d.drop_at = at;
        d.cleared_types = cleared;
    });
    set_text(doc, "drop-out", &line);
    set_class(doc, "drop-zone", "dropped");
}

fn bind_drag(doc: &Document) -> Result<(), Box<dyn Error>> {
    listen(doc, "drag-src", "dragstart", on_drag_source)?;
    listen(doc, "drag-src", "dragend", on_drag_source)?;
    for name in ["dragenter", "dragover", "dragleave", "drop"] {
        listen(doc, "drop-zone", name, on_drop_zone)?;
    }
    Ok(())
}

/// Presses on the chip, moves past the drag threshold, moves onto the zone and releases.
fn simulate_drag(doc: &Document) {
    let (src, zone) = (rect_of(doc, "drag-src"), rect_of(doc, "drop-zone"));
    let (sx, sy) = (src.x + src.width / 2.0, src.y + src.height / 2.0);
    let (zx, zy) = (zone.x + zone.width / 2.0, zone.y + zone.height / 2.0);
    mouse(doc, InjectedMouse::Moved, sx, sy, MouseButton::None, 0);
    mouse(doc, InjectedMouse::Pressed, sx, sy, MouseButton::Left, 1);
    mouse(
        doc,
        InjectedMouse::Moved,
        sx + 12.0,
        sy,
        MouseButton::Left,
        0,
    );
    mouse(doc, InjectedMouse::Moved, zx, zy, MouseButton::Left, 0);
    mouse(doc, InjectedMouse::Released, zx, zy, MouseButton::Left, 1);
}

fn check_drag(t: &Tour, doc: &Document) {
    let zone = rect_of(doc, "drop-zone");
    simulate_drag(doc);
    with_state(|s| {
        let d = &s.drag;
        let seq = d.seq.join(" > ");
        t.check(
            "drag.sequence",
            seq.starts_with("dragstart")
                && seq.contains("dragenter > dragover")
                && seq.contains("drop")
                && seq.contains("dragend"),
            format!("sequence '{seq}'"),
        );
        let types = d.start_types.join(",");
        t.check(
            "drag.set_clear_types",
            types == "text/plain,text/x-kui" && d.start_read == "kui-chip",
            format!("types '{types}', read back '{}'", d.start_read),
        );
        t.check(
            "drag.effect_allowed",
            d.start_allowed == Some(EffectAllowed::CopyMove)
                && d.drop_allowed == Some(EffectAllowed::CopyMove),
            format!("dragstart {:?}, drop {:?}", d.start_allowed, d.drop_allowed),
        );
        t.check(
            "drag.drop_effect",
            d.over_effect == Some(DropEffect::Copy) && d.drop_effect == Some(DropEffect::Copy),
            format!("dragover {:?}, drop {:?}", d.over_effect, d.drop_effect),
        );
        t.check(
            "drag.get_data",
            d.drops == 1
                && d.drop_plain == "kui-chip"
                && d.drop_custom == "42"
                && !d.junk_found
                && d.drop_types == 2,
            format!(
                "drops {}, plain '{}', custom '{}', cleared format found {}, types {}",
                d.drops, d.drop_plain, d.drop_custom, d.junk_found, d.drop_types
            ),
        );
        // An in-page drag carries no files.
        t.check(
            "drag.files",
            d.drop_files.as_ref().is_some_and(|f| f.is_empty()),
            format!("files {:?}", d.drop_files),
        );
        t.check(
            "drag.clear_all",
            d.cleared_types == 0,
            format!("types after clear {}", d.cleared_types),
        );
        t.check(
            "drag.drop_event",
            d.drop_tag == Some(EventTypeTag::Drag)
                && near(d.drop_at.x, zone.x + zone.width / 2.0)
                && near(d.drop_at.y, zone.y + zone.height / 2.0),
            format!(
                "tag {:?}, client {:.1},{:.1}",
                d.drop_tag, d.drop_at.x, d.drop_at.y
            ),
        );
        // HTML DnD: dragend's dropEffect is the operation the drop target chose (copy).
        t.known_bug(
            "drag.dragend_effect",
            d.end_effect == Some(DropEffect::Copy),
            "dragend reports dropEffect 'move' whatever the target chose",
            format!("dragend effect {:?}", d.end_effect),
        );
    });
    move_to(doc, "probe");
}

// ---- Panel buttons ----------------------------------------------------------------------

fn show_prop(doc: &Document, label: &str, not_canceled: i32) {
    let (order, _, flags) = trace();
    set_text(
        doc,
        "prop-out",
        &format!(
            "{label}: {} | bubbles={} cancelable={} prevented={} not_canceled={not_canceled}",
            or_dash(&order),
            flag(flags, 0),
            flag(flags, 1),
            flag(flags, 2)
        ),
    );
}

fn show_listeners(doc: &Document, note: &str) {
    let count = with_state(|s| s.tick_ids.len());
    let text = if note.is_empty() {
        format!("listeners={count}")
    } else {
        format!("listeners={count} | {note}")
    };
    set_text(doc, "lm-out", &text);
}

fn on<F>(ctx: &Context<'_>, element_id: &str, handler: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(&Document) + 'static,
{
    listen(ctx.document, element_id, "click", move |doc, _| {
        handler(doc)
    })
    .map(|_| ())
}

fn bind_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-prop-bubble", |doc| {
        show_prop(
            doc,
            "ping",
            fire_layers(doc, "ev-inner", "ping", true, false),
        )
    })?;
    on(ctx, "btn-prop-flat", |doc| {
        show_prop(
            doc,
            "ping (no bubbles)",
            fire_layers(doc, "ev-inner", "ping", false, false),
        )
    })?;
    on(ctx, "btn-prop-stop", |doc| {
        show_prop(
            doc,
            "halt",
            fire_layers(doc, "ev-inner", "halt", true, false),
        )
    })?;
    on(ctx, "btn-prop-cancel", |doc| {
        show_prop(
            doc,
            "veto",
            fire_layers(doc, "ev-inner", "veto", true, true),
        )
    })?;
    on(ctx, "btn-lm-add", |doc| {
        if let Some(id) = add_tick(doc) {
            with_state(|s| s.tick_ids.push(id));
        }
        show_listeners(doc, "");
    })?;
    on(ctx, "btn-lm-remove", |doc| {
        if let (Some(id), Some(target)) =
            (with_state(|s| s.tick_ids.pop()), by_id(doc, "lm-target"))
        {
            let _ = target.remove_event_listener("tick", id);
        }
        show_listeners(doc, "");
    })?;
    on(ctx, "btn-lm-clear", |doc| {
        if let Some(target) = by_id(doc, "lm-target") {
            let _ = target.remove_event_listeners("tick");
        }
        with_state(|s| {
            s.tick_ids.clear();
            s.tick_serial = 0;
        });
        show_listeners(doc, "");
    })?;
    on(ctx, "btn-lm-fire", |doc| {
        let ran = fire_tick(doc, "tick");
        set_class(doc, "lm-target", if ran.is_empty() { "" } else { "fired" });
        show_listeners(
            doc,
            &format!("ran: {}", if ran.is_empty() { "none" } else { &ran }),
        );
    })?;
    on(ctx, "btn-probe", |doc| {
        dispatch(doc, "probe", "probe", true, false);
        let line =
            with_state(|s| {
                let p = &s.probe;
                let tag = match p.tag {
                    Some(EventTypeTag::Base) => "base",
                    Some(EventTypeTag::Mouse) => "mouse",
                    Some(EventTypeTag::Keyboard) => "keyboard",
                    Some(EventTypeTag::Focus) => "focus",
                    Some(EventTypeTag::Drag) => "drag",
                    _ => "?",
                };
                let na = |status: i32| if status == NOT_SUPPORTED { "n/a" } else { "?" };
                format!(
                "type={} tag={tag} bubbles={} cancelable={} | related={} mouse={} key={} drag={}",
                p.event_type,
                p.bubbles as i32,
                p.cancelable as i32,
                if p.related == Some(false) { "none" } else { "?" },
                na(p.mouse_xy),
                na(p.key),
                na(p.drag_types)
            )
            });
        set_text(doc, "probe-out", &line);
    })?;
    // The injected drag must not run inside this click's own dispatch: start it from a timer.
    let platform = Rc::clone(ctx.platform);
    on(ctx, "btn-dnd-simulate", move |doc| {
        set_text(doc, "drag-seq", "-");
        set_text(doc, "drop-out", "-");
        let owner = doc.clone();
        let _ = platform.set_timeout(10, move || simulate_drag(&owner));
    })
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document;
    bind_layers(doc)?;
    listen(doc, "probe", "probe", on_probe)?;
    bind_mouse(doc)?;
    bind_keyboard(doc)?;
    bind_drag(doc)?;

    let t = Tour::new(doc);
    check_propagation(&t, doc);
    if let Err(error) = check_listeners(&t, doc) {
        t.check("listen.setup", false, error.to_string());
    }
    check_probe(&t, doc);
    check_mouse(&t, doc);
    check_keyboard(&t, doc);
    check_drag(&t, doc);
    check_wheel(&t, doc);
    check_capture(&t, doc);
    t.finish();
    bind_buttons(ctx)
}
