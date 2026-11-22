// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_data_observers.rs; edit that file, not this copy.
//! API tour: observable values and bindings, Mutation / Resize / Intersection observers, the theme
//! manager and the document router (examples/api_tours/data_observers). On start the host calls
//! every method once, checks the results against what the C ABI documents and logs one row per
//! check into #tour-log. Resize and intersection entries only arrive after rendered frames, so
//! those checks run from a chain of timers and #tour-result becomes "PASS n/n" at the end (about
//! 1 s). The panel buttons repeat the calls on the visible elements.
//!
//! Gaps in the Rust binding (reported): no safe wrapper for kui_observable_retain,
//! kui_observer_retain or kui_document_router_get_current_path, and no way to reach the raw
//! handle of a safe Observable / Observer / Document. `check_raw_gaps` calls them through the
//! public `kui::sys` on objects it creates with `kui::sys` itself.

use crate::canonical_runner::Context;
use kui::sys;
use kui::{
    Document, Element, IntersectionEntry, IntersectionOptions, MutationOptions, MutationRecord,
    MutationType, Observable, Observer, Platform, Rect, ResizeEntry, RouteGuardKind, RouteInfo,
    ThemeManager,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::c_void;
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

    fn input_value(&self, element_id: &str) -> String {
        match self.doc.get_element_by_id(element_id) {
            Ok(Some(element)) => element.value().unwrap_or_default(),
            _ => String::new(),
        }
    }
}

// ---- Small readers -----------------------------------------------------------------------

fn by_id<'d>(doc: &'d Document, element_id: &str) -> Option<Element<'d>> {
    doc.get_element_by_id(element_id).ok().flatten()
}

fn status<T>(result: &kui::Result<T>) -> sys::KuiStatus {
    match result {
        Ok(_) => sys::KUI_OK,
        Err(error) => error.status(),
    }
}

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn id_of(element: &Option<Element<'_>>) -> String {
    element
        .as_ref()
        .and_then(|e| e.id().ok())
        .unwrap_or_default()
}

fn text_of(doc: &Document, element_id: &str) -> String {
    by_id(doc, element_id)
        .and_then(|e| e.text_content().ok())
        .unwrap_or_default()
}

fn attr_of(doc: &Document, element_id: &str, name: &str) -> String {
    by_id(doc, element_id)
        .and_then(|e| e.attribute(name).ok())
        .unwrap_or_default()
}

fn computed(doc: &Document, element_id: &str, property: &str) -> String {
    by_id(doc, element_id)
        .and_then(|e| e.computed_style_value(property).ok())
        .unwrap_or_default()
}

fn rect_of(doc: &Document, element_id: &str) -> Rect {
    by_id(doc, element_id)
        .and_then(|e| e.bounding_rect().ok())
        .unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        })
}

fn set_text(doc: &Document, element_id: &str, text: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_text_content(text);
    }
}

fn set_attr(doc: &Document, element_id: &str, name: &str, value: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_attribute(name, value);
    }
}

fn set_style(doc: &Document, element_id: &str, property: &str, value: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_style_property(property, value, false);
    }
}

fn set_class(doc: &Document, element_id: &str, name: &str, on: bool) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = if on {
            e.class_list_add(name)
        } else {
            e.class_list_remove(name)
        };
    }
}

fn scroll_to(doc: &Document, element_id: &str, top: f32) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = e.set_scroll_position(0.0, top);
    }
}

/// A CSS colour as "#rrggbb" or "rgb(r, g, b)" / "rgba(r, g, b, a)".
fn colour_is(value: &str, r: i64, g: i64, b: i64) -> bool {
    let got: Option<[i64; 3]> = if let Some(hex) = value.strip_prefix('#').filter(|h| h.len() == 6)
    {
        i64::from_str_radix(hex, 16)
            .ok()
            .map(|v| [(v >> 16) & 255, (v >> 8) & 255, v & 255])
    } else if value.starts_with("rgb") {
        let inner = value
            .split('(')
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .unwrap_or("");
        let parts: Vec<i64> = inner
            .split(',')
            .take(3)
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        (parts.len() == 3).then(|| [parts[0], parts[1], parts[2]])
    } else {
        None
    };
    got == Some([r, g, b])
}

/// Binds `obs` to a property of the element with that id; the error is the raw status.
fn bind_id(
    doc: &Document,
    obs: &Observable,
    element_id: &str,
    property: &str,
    two_way: bool,
) -> Result<u32, sys::KuiStatus> {
    match by_id(doc, element_id) {
        Some(e) => obs
            .bind(&e, property, two_way)
            .map_err(|error| error.status()),
        None => Err(sys::KUI_ERR_NOT_FOUND),
    }
}

// ---- Observable: values, subscriptions, one- and two-way bindings, flush ----------------

fn check_observable(t: &Tour) {
    let doc = &t.doc;
    let obs = match Observable::new("0") {
        Ok(obs) => obs,
        Err(error) => {
            t.check("ob.create_get", false, format!("Observable::new: {error}"));
            return;
        }
    };
    let value = obs.get().unwrap_or_default();
    t.check("ob.create_get", value == "0", format!("value '{value}'"));

    let empty = Observable::new("").and_then(|e| e.get());
    t.check(
        "ob.create_empty",
        matches!(&empty, Ok(v) if v.is_empty()),
        format!("{empty:?}"),
    );

    let _ = obs.set("1");
    let value = obs.get().unwrap_or_default();
    t.check("ob.set_get", value == "1", format!("value '{value}'"));

    // Subscribers run synchronously, and only when the value actually changed.
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let sub = obs
        .subscribe(move || counter.set(counter.get() + 1))
        .unwrap_or(0);
    let _ = obs.set("2");
    t.check(
        "ob.subscribe",
        calls.get() == 1,
        format!("calls after one change {}", calls.get()),
    );
    let _ = obs.set("2");
    t.check(
        "ob.set_unchanged",
        calls.get() == 1,
        format!("calls after setting the same value {}", calls.get()),
    );
    let first = obs.unsubscribe(sub);
    let _ = obs.set("3");
    let again = obs.unsubscribe(sub);
    t.check(
        "ob.unsubscribe",
        first.is_ok() && calls.get() == 1 && status(&again) == sys::KUI_ERR_NOT_FOUND,
        format!("{first:?}, calls {}, second {again:?}", calls.get()),
    );

    // Binding writes the current value right away.
    let text_binding = bind_id(doc, &obs, "s-text", "textContent", false);
    let text = text_of(doc, "s-text");
    t.check(
        "ob.bind_text",
        matches!(text_binding, Ok(id) if id != 0) && text == "3",
        format!("{text_binding:?}, text '{text}'"),
    );
    let text_binding = text_binding.unwrap_or(0);

    // Later changes are batched until the next frame or an explicit flush.
    let _ = obs.set("4");
    let text = text_of(doc, "s-text");
    t.check(
        "ob.batched",
        text == "3",
        format!("text right after set '{text}' (want the old '3')"),
    );
    let flushed = doc.flush_bindings();
    let text = text_of(doc, "s-text");
    t.check(
        "ob.flush_bindings",
        flushed.is_ok() && text == "4",
        format!("{flushed:?}, text '{text}'"),
    );

    // Any other property name is an attribute.
    let attr_binding = bind_id(doc, &obs, "s-attr", "data-level", false);
    let attr = attr_of(doc, "s-attr", "data-level");
    t.check(
        "ob.bind_attr",
        attr_binding.is_ok() && attr == "4",
        format!("{attr_binding:?}, data-level '{attr}'"),
    );
    let attr_binding = attr_binding.unwrap_or(0);

    // "style" is an attribute too: the value is the element's inline declaration.
    if let Ok(style) = Observable::new("background-color: #ff0000") {
        let bound = bind_id(doc, &style, "s-style", "style", false);
        let red = computed(doc, "s-style", "background-color");
        let _ = style.set("background-color: #00ff00");
        let _ = doc.flush_bindings();
        let green = computed(doc, "s-style", "background-color");
        t.check(
            "ob.bind_style",
            bound.is_ok() && colour_is(&red, 255, 0, 0) && colour_is(&green, 0, 255, 0),
            format!("{bound:?}, computed '{red}' then '{green}'"),
        );
    }

    // Two-way: "value" on an <input> (user input writes back; typed by the test case).
    if let Ok(input) = Observable::new("seed") {
        let bound = bind_id(doc, &input, "s-input", "value", true);
        let seeded = t.input_value("s-input");
        let _ = input.set("next");
        let _ = doc.flush_bindings();
        let after = t.input_value("s-input");
        t.check(
            "ob.bind_two_way",
            bound.is_ok() && seeded == "seed" && after == "next",
            format!("{bound:?}, input value '{seeded}' then '{after}'"),
        );
    }

    let invalid = bind_id(doc, &obs, "s-text", "", false);
    t.check(
        "ob.bind_invalid",
        invalid == Err(sys::KUI_ERR_INVALID_ARG),
        format!("empty property -> {invalid:?}"),
    );

    // Unbinding one binding leaves the others.
    let unbound = obs.unbind(text_binding);
    let _ = obs.set("5");
    let _ = doc.flush_bindings();
    let (text, attr) = (text_of(doc, "s-text"), attr_of(doc, "s-attr", "data-level"));
    let again = obs.unbind(text_binding);
    t.check(
        "ob.unbind",
        unbound.is_ok() && text == "4" && attr == "5" && status(&again) == sys::KUI_ERR_NOT_FOUND,
        format!("{unbound:?}, text '{text}' (want 4), data-level '{attr}' (want 5)"),
    );

    let all = obs.unbind_all();
    let _ = obs.set("6");
    let _ = doc.flush_bindings();
    let attr = attr_of(doc, "s-attr", "data-level");
    let again = obs.unbind(attr_binding);
    t.check(
        "ob.unbind_all",
        all.is_ok() && attr == "5" && status(&again) == sys::KUI_ERR_NOT_FOUND,
        format!("{all:?}, data-level '{attr}' (want 5)"),
    );

    // Dropping the Observable releases the last reference (kui_observable_release), which also
    // releases its subscriptions: the closure's captured counter is freed.
    let counter = calls.clone();
    let _ = obs.subscribe(move || counter.set(counter.get() + 1));
    let held = Rc::strong_count(&calls);
    drop(obs);
    t.check(
        "ob.release",
        held == 2 && Rc::strong_count(&calls) == 1,
        format!(
            "closure references before / after drop {held} / {}",
            Rc::strong_count(&calls)
        ),
    );
}

// ---- Gaps: retain and get_current_path through kui::sys (no safe wrapper) ----------------

unsafe extern "C" fn ignore_mutations(
    _records: *const sys::KuiCMutationRecord,
    _count: usize,
    _user: *mut c_void,
) {
}

fn check_raw_gaps(t: &Tour) {
    unsafe {
        // Observable: one retain, then two releases; the value survives the first.
        let mut obs: *mut sys::KuiObservable = std::ptr::null_mut();
        let created = sys::kui_observable_create(b"kept".as_ptr().cast(), 4, &mut obs);
        let retained = if obs.is_null() {
            sys::KUI_ERR_INTERNAL
        } else {
            sys::kui_observable_retain(obs)
        };
        let mut buf = [0u8; 16];
        let mut len = 0usize;
        if !obs.is_null() {
            sys::kui_observable_release(obs);
            sys::kui_observable_get(obs, buf.as_mut_ptr().cast(), buf.len(), &mut len);
            sys::kui_observable_release(obs);
        }
        let value = String::from_utf8_lossy(&buf[..len.min(buf.len())]).to_string();
        t.check(
            "ob.retain_release",
            created == sys::KUI_OK && retained == sys::KUI_OK && value == "kept",
            format!("create {created}, retain {retained}, value after one release '{value}'"),
        );

        // Observer: the same with a mutation observer.
        let mut observer: *mut sys::KuiObserver = std::ptr::null_mut();
        let created = sys::kui_mutation_observer_create(
            Some(ignore_mutations),
            std::ptr::null_mut(),
            None,
            &mut observer,
        );
        let (retained, first, last) = if observer.is_null() {
            (
                sys::KUI_ERR_INTERNAL,
                sys::KUI_ERR_INTERNAL,
                sys::KUI_ERR_INTERNAL,
            )
        } else {
            (
                sys::kui_observer_retain(observer),
                sys::kui_observer_release(observer),
                sys::kui_observer_release(observer),
            )
        };
        t.check(
            "mut.retain_release",
            created == sys::KUI_OK
                && retained == sys::KUI_OK
                && first == sys::KUI_OK
                && last == sys::KUI_OK,
            format!("create {created}, retain {retained}, release {first} / {last}"),
        );

        // Router current path on a document of our own.
        let mut doc: *mut sys::KuiDocument = std::ptr::null_mut();
        sys::kui_document_create(&mut doc);
        let mut path = String::new();
        if !doc.is_null() {
            let mut navigated = 0;
            sys::kui_document_router_push(doc, b"/raw/1".as_ptr().cast(), 6, &mut navigated);
            let mut buf = [0u8; 32];
            let mut len = 0usize;
            sys::kui_document_router_get_current_path(
                doc,
                buf.as_mut_ptr().cast(),
                buf.len(),
                &mut len,
            );
            path = String::from_utf8_lossy(&buf[..len.min(buf.len())]).to_string();
            sys::kui_document_release(doc);
        }
        t.check(
            "rt.get_current_path",
            path == "/raw/1",
            format!("current path '{path}'"),
        );
    }
}

// ---- MutationObserver: child list, attributes, character data, filter, disconnect -------

#[derive(Default)]
struct MutationLog {
    batches: usize,
    records: usize,
    kind: Option<MutationType>,
    target: String,
    target_type: i32,
    added: String,
    added_type: i32,
    removed: String,
    attribute: String,
    old_value: String,
}

fn remember(log: &RefCell<MutationLog>, records: &[MutationRecord<'_>]) {
    let Some(last) = records.last() else { return };
    let mut log = log.borrow_mut();
    log.batches += 1;
    log.records += records.len();
    log.kind = Some(last.kind);
    log.target = id_of(&last.target);
    log.target_type = last.target_node_type;
    log.added = id_of(&last.added);
    log.added_type = last.added_node_type;
    log.removed = id_of(&last.removed);
    log.attribute = last.attribute_name.clone();
    log.old_value = last.old_value.clone();
}

fn check_mutation(t: &Tour) {
    let doc = &t.doc;
    let (Some(root), Some(list)) = (by_id(doc, "s-mut"), by_id(doc, "s-list")) else {
        t.check("mut.page", false, "missing #s-mut / #s-list".into());
        return;
    };
    let log = Rc::new(RefCell::new(MutationLog::default()));
    let sink = log.clone();
    let obs = match doc.create_mutation_observer(move |records| remember(&sink, records)) {
        Ok(obs) => obs,
        Err(error) => {
            t.check("mut.create", false, format!("{error}"));
            return;
        }
    };
    t.check("mut.create", true, String::new());

    let empty = obs.observe_mutations(&root, &MutationOptions::default());
    t.check(
        "mut.observe_needs_kind",
        status(&empty) == sys::KUI_ERR_INVALID_ARG,
        format!("no kind -> {empty:?}"),
    );
    let all = MutationOptions {
        child_list: true,
        attributes: true,
        character_data: true,
        subtree: true,
        attribute_old_value: true,
        character_data_old_value: true,
        attribute_filter: Vec::new(),
    };
    let observed = obs.observe_mutations(&root, &all);
    t.check(
        "mut.observe",
        observed.is_ok() && log.borrow().batches == 0,
        format!("{observed:?}, batches {}", log.borrow().batches),
    );

    // A new child in the subtree: one childList record, delivered synchronously.
    let item = doc.create_element("li").ok();
    if let Some(item) = &item {
        let _ = item.set_text_content("one");
        let _ = item.set_attribute("id", "s-li-1");
        let _ = list.append_child(item);
    }
    {
        let l = log.borrow();
        t.check(
            "mut.child_list_add",
            l.records == 1
                && l.kind == Some(MutationType::ChildList)
                && l.target == "s-list"
                && l.added == "s-li-1"
                && l.added_type == 1,
            format!(
                "records {}, type {:?}, target '{}', added '{}' (type {})",
                l.records, l.kind, l.target, l.added, l.added_type
            ),
        );
    }

    if let Some(item) = &item {
        let _ = item.remove();
    }
    {
        let l = log.borrow();
        t.check(
            "mut.child_list_remove",
            l.records == 2
                && l.kind == Some(MutationType::ChildList)
                && l.removed == "s-li-1"
                && l.target == "s-list",
            format!(
                "records {}, type {:?}, removed '{}', target '{}'",
                l.records, l.kind, l.removed, l.target
            ),
        );
    }

    set_attr(doc, "s-mitem", "data-state", "on");
    {
        let l = log.borrow();
        t.check(
            "mut.attributes",
            l.records == 3
                && l.kind == Some(MutationType::Attributes)
                && l.target == "s-mitem"
                && l.attribute == "data-state"
                && l.old_value == "off",
            format!(
                "type {:?}, target '{}', name '{}', old '{}'",
                l.kind, l.target, l.attribute, l.old_value
            ),
        );
    }

    // Rewriting the lone text child changes its data in place: a characterData record whose
    // target is the parent element, with the text node's type.
    set_text(doc, "s-mtext", "world");
    {
        let l = log.borrow();
        t.check(
            "mut.character_data",
            l.kind == Some(MutationType::CharacterData)
                && l.target == "s-mtext"
                && l.target_type == 3
                && l.old_value == "hello",
            format!(
                "type {:?}, target '{}' (node type {}), old '{}'",
                l.kind, l.target, l.target_type, l.old_value
            ),
        );
    }

    // Observing again replaces the options: attributes only, filtered to data-state.
    let filter = MutationOptions {
        attributes: true,
        subtree: true,
        attribute_filter: vec!["data-state".to_string()],
        ..MutationOptions::default()
    };
    let _ = obs.observe_mutations(&root, &filter);
    let before = log.borrow().records;
    set_attr(doc, "s-mitem", "title", "ignored");
    set_text(doc, "s-mtext", "again");
    let filtered = log.borrow().records == before;
    set_attr(doc, "s-mitem", "data-state", "off");
    {
        let l = log.borrow();
        t.check(
            "mut.attribute_filter",
            filtered
                && l.records == before + 1
                && l.attribute == "data-state"
                && l.old_value.is_empty(),
            format!(
                "filtered {filtered}, records {before} -> {}, name '{}', old '{}' (not requested)",
                l.records, l.attribute, l.old_value
            ),
        );
    }

    let (observe, unobserve) = (obs.observe(&root), obs.unobserve(&root));
    t.check(
        "mut.generic_observe",
        status(&observe) == sys::KUI_ERR_INVALID_ARG
            && status(&unobserve) == sys::KUI_ERR_NOT_SUPPORTED,
        format!(
            "observe {observe:?} (want INVALID_ARG), unobserve {unobserve:?} (want NOT_SUPPORTED)"
        ),
    );

    let disconnected = obs.disconnect();
    let before = log.borrow().records;
    set_attr(doc, "s-mitem", "data-state", "on");
    t.check(
        "mut.disconnect",
        disconnected.is_ok() && log.borrow().records == before,
        format!(
            "{disconnected:?}, records {before} -> {}",
            log.borrow().records
        ),
    );

    // Dropping the observer releases it (kui_observer_release) and frees the callback.
    let held = Rc::strong_count(&log);
    drop(obs);
    t.check(
        "mut.release",
        held == 2 && Rc::strong_count(&log) == 1,
        format!(
            "callback references before / after drop {held} / {}",
            Rc::strong_count(&log)
        ),
    );
}

// ---- Theme manager: built-ins, register / replace, apply, enumerate ---------------------

fn vars(bg: &str, fg: &str, accent: &str) -> HashMap<String, String> {
    HashMap::from([
        ("--tour-bg".to_string(), bg.to_string()),
        ("--tour-fg".to_string(), fg.to_string()),
        ("--tour-accent".to_string(), accent.to_string()),
    ])
}

fn html_custom_property(doc: &Document, name: &str) -> String {
    doc.query_selector("html")
        .ok()
        .flatten()
        .and_then(|html| html.style_property(name).ok())
        .unwrap_or_default()
}

fn check_theme(t: &Tour) -> Option<ThemeManager> {
    let doc = &t.doc;
    let themes = match ThemeManager::new() {
        Ok(themes) => themes,
        Err(error) => {
            t.check("th.builtin", false, format!("ThemeManager::new: {error}"));
            return None;
        }
    };
    let names = themes.theme_names().unwrap_or_default();
    let has = |names: &[String], name: &str| names.iter().any(|n| n == name);
    t.check(
        "th.builtin",
        names.len() == 2 && has(&names, "light") && has(&names, "dark"),
        format!("names {names:?}"),
    );

    let dark = themes.theme("dark").unwrap_or_default();
    let all_custom = dark.keys().all(|k| k.starts_with("--"));
    t.check(
        "th.builtin_variables",
        !dark.is_empty() && all_custom,
        format!(
            "dark has {} variables, all custom properties {all_custom}",
            dark.len()
        ),
    );

    let active = themes.active_theme().unwrap_or_default();
    t.check(
        "th.active_empty",
        active.is_empty(),
        format!("active before any apply '{active}'"),
    );

    let _ = themes.register_theme("tour-light", &vars("#f0fdf4", "#14532d", "#22c55e"));
    let registered = themes.register_theme("tour-dark", &vars("#0f172a", "#e2e8f0", "#38bdf8"));
    let names = themes.theme_names().unwrap_or_default();
    let light = themes.theme("tour-light").unwrap_or_default();
    t.check(
        "th.register",
        registered.is_ok() && names.len() == 4 && has(&names, "tour-dark") && light.len() == 3,
        format!(
            "{registered:?}, names {names:?}, tour-light variables {}",
            light.len()
        ),
    );
    let fg = light.get("--tour-fg").cloned().unwrap_or_default();
    t.check(
        "th.get_variable",
        fg == "#14532d",
        format!("tour-light --tour-fg '{fg}'"),
    );

    // Registering an existing name replaces it.
    let _ = themes.register_theme("tour-dark", &vars("#0f172a", "#e2e8f0", "#f59e0b"));
    let names = themes.theme_names().unwrap_or_default();
    let accent = themes
        .theme("tour-dark")
        .ok()
        .and_then(|v| v.get("--tour-accent").cloned())
        .unwrap_or_default();
    t.check(
        "th.register_replace",
        names.len() == 4 && accent == "#f59e0b",
        format!("count {}, --tour-accent '{accent}'", names.len()),
    );

    let unknown = themes.theme("nope");
    t.check(
        "th.not_found",
        status(&unknown) == sys::KUI_ERR_NOT_FOUND,
        format!("unknown theme -> {:?}", unknown.err()),
    );

    // Applying sets the variables inline on <html>; the card picks them up through var().
    let applied = themes.apply("tour-dark", doc);
    let active = themes.active_theme().unwrap_or_default();
    let bg = html_custom_property(doc, "--tour-bg");
    t.check(
        "th.apply",
        applied.is_ok() && active == "tour-dark" && bg == "#0f172a",
        format!("{applied:?}, active '{active}', <html> --tour-bg '{bg}'"),
    );
    let css = computed(doc, "theme-card", "background-color");
    t.check(
        "th.apply_computed",
        colour_is(&css, 0x0f, 0x17, 0x2a),
        format!("card background-color '{css}'"),
    );

    let unknown = themes.apply("nope", doc);
    let active = themes.active_theme().unwrap_or_default();
    t.check(
        "th.apply_unknown",
        status(&unknown) == sys::KUI_ERR_NOT_FOUND && active == "tour-dark",
        format!("{unknown:?}, active '{active}'"),
    );

    let _ = themes.apply("tour-light", doc);
    let css = computed(doc, "theme-card", "background-color");
    t.check(
        "th.switch",
        colour_is(&css, 0xf0, 0xfd, 0xf4),
        format!("card background-color after tour-light '{css}'"),
    );

    // Dropping a manager releases it (kui_theme_manager_release).
    let other = ThemeManager::new();
    t.check(
        "th.release",
        other.is_ok(),
        format!("{:?}", other.as_ref().err()),
    );
    drop(other);
    Some(themes)
}

// ---- Router: routes, views, navigation, history, params, guards, callbacks --------------

#[derive(Default)]
struct RouterState {
    nav_calls: Cell<u32>,
    nav_path: RefCell<String>,
    guard_seen: RefCell<String>,
    block_leave: Cell<bool>,
    admin_locked: Cell<bool>,
    navigated_id: Cell<u32>,
}

/// Builds the view element for a route; the returned element is handed over to KUI.
fn add_view(
    doc: &Document,
    pattern: &str,
    name: &str,
    element_id: &'static str,
    title: &'static str,
) -> kui::Result<()> {
    doc.router().add_route(pattern, name, move |doc, route| {
        let view = doc.create_element("div").ok()?;
        let _ = view.set_attribute("id", element_id);
        let _ = view.set_attribute("class", "view");
        let text = if element_id == "view-user" {
            format!(
                "User {}",
                route.params.get("id").cloned().unwrap_or_default()
            )
        } else {
            title.to_string()
        };
        let _ = view.set_text_content(&text);
        Some(view)
    })
}

fn add_routes(doc: &Document) -> kui::Result<()> {
    add_view(doc, "/", "home", "view-home", "Home")?;
    add_view(doc, "/user/:id", "user", "view-user", "User")?;
    add_view(doc, "/about", "about", "view-about", "About")?;
    add_view(doc, "/admin", "admin", "view-admin", "Admin area")
}

fn on_navigated(doc: &Document, state: &Rc<RouterState>) -> u32 {
    let (doc, state) = (doc.clone(), state.clone());
    doc.clone()
        .router()
        .on_navigated(move |route: &RouteInfo| {
            state.nav_calls.set(state.nav_calls.get() + 1);
            *state.nav_path.borrow_mut() = route.path.clone();
            set_text(
                &doc,
                "route-nav",
                &format!("navigated: {} (pattern {})", route.path, route.pattern),
            );
        })
        .unwrap_or(0)
}

fn view_id(doc: &Document) -> String {
    by_id(doc, "route-view")
        .and_then(|view| view.first_child().ok().flatten())
        .and_then(|child| child.id().ok())
        .unwrap_or_default()
}

fn view_children(doc: &Document) -> usize {
    by_id(doc, "route-view")
        .and_then(|v| v.child_count().ok())
        .unwrap_or(99)
}

fn check_router(t: &Tour) -> Rc<RouterState> {
    let doc = &t.doc;
    let router = doc.router();
    let state = Rc::new(RouterState::default());
    state.admin_locked.set(true);
    let Some(view) = by_id(doc, "route-view") else {
        t.check("rt.page", false, "missing #route-view".into());
        return state;
    };
    let path = || router.current().map(|r| r.path).unwrap_or_default();

    let (index, size) = router.history().unwrap_or((-2, -2));
    t.check(
        "rt.history_initial",
        index == -1 && size == 0,
        format!("index {index}, size {size} (want -1, 0)"),
    );

    let added = add_routes(doc);
    let count = router.route_count().unwrap_or(99);
    let has = router.has_route("/user/:id").unwrap_or(false);
    t.check(
        "rt.add_route",
        added.is_ok() && count == 4 && has,
        format!("{added:?}, count {count}, has /user/:id {has}"),
    );
    let duplicate = add_view(doc, "/about", "again", "view-about", "About");
    let has = router.has_route("/nope").unwrap_or(true);
    t.check(
        "rt.add_route_duplicate",
        status(&duplicate) == sys::KUI_ERR_INVALID_ARG && !has,
        format!("duplicate -> {duplicate:?}, has /nope {has}"),
    );

    let _ = add_view(doc, "/tmp", "tmp", "view-home", "Home");
    let removed = router.remove_route("/tmp");
    let count = router.route_count().unwrap_or(99);
    let again = router.remove_route("/tmp");
    t.check(
        "rt.remove_route",
        removed.is_ok() && count == 4 && status(&again) == sys::KUI_ERR_NOT_FOUND,
        format!("{removed:?}, count {count}, again {again:?}"),
    );

    let registered = router.register_view(&view);
    state.navigated_id.set(on_navigated(doc, &state));
    t.check(
        "rt.register_view",
        registered.is_ok() && view_children(doc) == 0,
        format!("{registered:?}, children {}", view_children(doc)),
    );

    let navigated = router.push("/").unwrap_or(false);
    t.check(
        "rt.push",
        navigated && path() == "/" && view_id(doc) == "view-home",
        format!(
            "navigated {navigated}, path '{}', view '{}'",
            path(),
            view_id(doc)
        ),
    );
    t.check(
        "rt.navigated_callback",
        state.nav_calls.get() == 1 && *state.nav_path.borrow() == "/",
        format!(
            "calls {}, last path '{}'",
            state.nav_calls.get(),
            state.nav_path.borrow()
        ),
    );

    let navigated = router.push("/user/42?tab=2").unwrap_or(false);
    let param = router.param("id").ok().flatten().unwrap_or_default();
    let text = text_of(doc, "view-user");
    t.check(
        "rt.push_param",
        navigated && param == "42" && view_id(doc) == "view-user" && text == "User 42",
        format!(
            "navigated {navigated}, param id '{param}', view '{}' '{text}'",
            view_id(doc)
        ),
    );
    let route = router.current().unwrap_or_default();
    let tab = route.query_params.get("tab").cloned().unwrap_or_default();
    t.check(
        "rt.get_current",
        route.path == "/user/42?tab=2"
            && route.pattern == "/user/:id"
            && route.query == "tab=2"
            && tab == "2"
            && route.params.len() == 1,
        format!(
            "path '{}', pattern '{}', query '{}', tab '{tab}', params {}",
            route.path,
            route.pattern,
            route.query,
            route.params.len()
        ),
    );
    let missing = router.param("nope");
    t.check(
        "rt.get_param_missing",
        matches!(missing, Ok(None)),
        format!("param(nope) {missing:?}"),
    );
    let (index, size) = router.history().unwrap_or((-2, -2));
    t.check(
        "rt.history",
        index == 1 && size == 2,
        format!("index {index}, size {size} (want 1, 2)"),
    );

    let navigated = router.go(-1).unwrap_or(false);
    let (index, size) = router.history().unwrap_or((-2, -2));
    t.check(
        "rt.go_back",
        navigated && path() == "/" && index == 0 && size == 2,
        format!(
            "navigated {navigated}, path '{}', index {index}, size {size}",
            path()
        ),
    );
    let navigated = router.go(1).unwrap_or(false);
    t.check(
        "rt.go_forward",
        navigated && path() == "/user/42?tab=2",
        format!("navigated {navigated}, path '{}'", path()),
    );
    let navigated = router.go(5).unwrap_or(true);
    t.check(
        "rt.go_out_of_range",
        !navigated,
        format!("go(5) navigated {navigated}"),
    );

    let navigated = router.replace("/about").unwrap_or(false);
    let (index, size) = router.history().unwrap_or((-2, -2));
    t.check(
        "rt.replace",
        navigated && path() == "/about" && index == 1 && size == 2 && view_id(doc) == "view-about",
        format!(
            "navigated {navigated}, path '{}', index {index}, size {size}, view '{}'",
            path(),
            view_id(doc)
        ),
    );

    // Guards: before-enter refuses /admin; before-leave refuses /about -> /.
    let seen = state.clone();
    let enter = router
        .add_guard(RouteGuardKind::BeforeEnter, move |from, to| {
            *seen.guard_seen.borrow_mut() = format!("{} -> {}", from.path, to.path);
            to.path != "/admin"
        })
        .unwrap_or(0);
    let calls = state.nav_calls.get();
    let navigated = router.push("/admin").unwrap_or(true);
    t.check(
        "rt.guard_enter_blocks",
        !navigated
            && path() == "/about"
            && state.nav_calls.get() == calls
            && *state.guard_seen.borrow() == "/about -> /admin",
        format!(
            "navigated {navigated}, path '{}', callbacks {calls} -> {}, guard saw '{}'",
            path(),
            state.nav_calls.get(),
            state.guard_seen.borrow()
        ),
    );

    let flag = state.clone();
    let leave = router
        .add_guard(RouteGuardKind::BeforeLeave, move |from, to| {
            !(flag.block_leave.get() && from.path == "/about" && to.path == "/")
        })
        .unwrap_or(0);
    state.block_leave.set(true);
    let navigated = router.push("/").unwrap_or(true);
    t.check(
        "rt.guard_leave_blocks",
        !navigated && path() == "/about",
        format!("navigated {navigated}, path '{}'", path()),
    );
    let _ = router.remove_guard(leave);
    let navigated = router.push("/").unwrap_or(false);
    t.check(
        "rt.remove_guard",
        navigated && path() == "/",
        format!("navigated {navigated}, path '{}'", path()),
    );
    let _ = router.remove_guard(enter);
    let navigated = router.push("/admin").unwrap_or(false);
    t.check(
        "rt.guard_removed_allows",
        navigated && view_id(doc) == "view-admin",
        format!("navigated {navigated}, view '{}'", view_id(doc)),
    );

    // A path no route matches still navigates and empties the view.
    let navigated = router.push("/missing").unwrap_or(false);
    let route = router.current().unwrap_or_default();
    t.check(
        "rt.unmatched",
        navigated && route.pattern.is_empty() && view_children(doc) == 0,
        format!(
            "navigated {navigated}, pattern '{}', children {}",
            route.pattern,
            view_children(doc)
        ),
    );

    let calls = state.nav_calls.get();
    let removed = router.remove_navigated(state.navigated_id.get());
    let _ = router.push("/about");
    t.check(
        "rt.remove_navigated_callback",
        removed.is_ok() && state.nav_calls.get() == calls,
        format!(
            "{removed:?}, callbacks {calls} -> {}",
            state.nav_calls.get()
        ),
    );

    let unregistered = router.unregister_view(&view);
    let _ = router.push("/");
    let children = view_children(doc);
    let again = router.unregister_view(&view);
    t.check(
        "rt.unregister_view",
        unregistered.is_ok() && children == 1 && status(&again) == sys::KUI_ERR_NOT_FOUND,
        format!("{unregistered:?}, children after navigating unregistered {children} (want the old 1), again {again:?}"),
    );
    // Registering fills the view right away with the current route.
    let _ = router.register_view(&view);
    t.check(
        "rt.register_fills",
        view_id(doc) == "view-home",
        format!("view after register '{}'", view_id(doc)),
    );

    let cleared = router.clear();
    let count = router.route_count().unwrap_or(99);
    let has = router.has_route("/").unwrap_or(true);
    t.check(
        "rt.clear_routes",
        cleared.is_ok() && count == 0 && !has,
        format!("{cleared:?}, count {count}, has / {has}"),
    );

    // Set up again for the panel: routes, the /admin lock and the navigated callback.
    let _ = add_routes(doc);
    let lock = state.clone();
    let _ = router.add_guard(RouteGuardKind::BeforeEnter, move |_from, to| {
        !(lock.admin_locked.get() && to.path == "/admin")
    });
    state.navigated_id.set(on_navigated(doc, &state));
    let _ = router.push("/");
    state
}

// ---- Resize & intersection observers: entries after rendered frames (timer chain) -------

#[derive(Default, Clone, Copy)]
struct SizeLog {
    reports: u32,
    content_w: f32,
    content_h: f32,
    border_w: f32,
    border_h: f32,
    content_x: f32,
    content_y: f32,
}

#[derive(Default, Clone, Copy)]
struct SeenLog {
    reports: u32,
    intersecting: bool,
    ratio: f32,
    root_w: f32,
    root_h: f32,
}

#[derive(Default)]
struct FrameLogs {
    rs_box: SizeLog,
    fluid: SizeLog,
    a: SeenLog,
    b: SeenLog,
    panel: SeenLog,
}

struct Frames {
    t: Rc<Tour>,
    platform: Rc<Platform>,
    logs: Rc<RefCell<FrameLogs>>,
    resize: Option<Observer>,
    inter: Option<Observer>,
    viewport: RefCell<Option<Observer>>,
    fluid_frozen: Cell<u32>,
    fluid_before: Cell<f32>,
}

fn show_resize(doc: &Document, logs: &FrameLogs) {
    let (b, f) = (&logs.rs_box, &logs.fluid);
    set_text(
        doc,
        "rs-out",
        &format!(
            "box {:.0}x{:.0} (border {:.0}x{:.0}) | fluid {:.0}",
            b.content_w, b.content_h, b.border_w, b.border_h, f.content_w
        ),
    );
}

fn show_intersection(doc: &Document, logs: &FrameLogs) {
    let io = |log: &SeenLog| if log.intersecting { "in" } else { "out" };
    set_text(
        doc,
        "io-out",
        &format!(
            "A {} {:.2} | B {} {:.2}",
            io(&logs.a),
            logs.a.ratio,
            io(&logs.b),
            logs.b.ratio
        ),
    );
}

fn observe_id(doc: &Document, obs: &Observer, element_id: &str) {
    if let Some(e) = by_id(doc, element_id) {
        let _ = obs.observe(&e);
    }
}

fn start_frame_checks(t: &Rc<Tour>, platform: &Rc<Platform>) -> Rc<Frames> {
    let doc = &t.doc;
    let logs = Rc::new(RefCell::new(FrameLogs::default()));

    let (sink, page) = (logs.clone(), doc.clone());
    let resize = doc.create_resize_observer(move |entries: &[ResizeEntry<'_>]| {
        let mut logs = sink.borrow_mut();
        for entry in entries {
            let log = match id_of(&entry.target).as_str() {
                "rs-box" => &mut logs.rs_box,
                "rs-fluid" => &mut logs.fluid,
                _ => continue,
            };
            log.reports += 1;
            log.content_w = entry.content_width;
            log.content_h = entry.content_height;
            log.border_w = entry.border_width;
            log.border_h = entry.border_height;
            log.content_x = entry.content_x;
            log.content_y = entry.content_y;
        }
        show_resize(&page, &logs);
    });

    let (sink, page) = (logs.clone(), doc.clone());
    let root = by_id(doc, "io-scroller");
    let inter = root.as_ref().map(|root| {
        let options = IntersectionOptions {
            root: Some(root),
            thresholds: vec![0.0, 0.5, 1.0],
            ..Default::default()
        };
        doc.create_intersection_observer(&options, move |entries: &[IntersectionEntry<'_>]| {
            let mut logs = sink.borrow_mut();
            for entry in entries {
                let log = match id_of(&entry.target).as_str() {
                    "io-a" => &mut logs.a,
                    "io-b" => &mut logs.b,
                    _ => continue,
                };
                log.reports += 1;
                log.intersecting = entry.is_intersecting;
                log.ratio = entry.intersection_ratio;
                log.root_w = entry.root_bounds.width;
                log.root_h = entry.root_bounds.height;
            }
            show_intersection(&page, &logs);
        })
    });

    let sink = logs.clone();
    let viewport = doc.create_intersection_observer(
        &IntersectionOptions::default(),
        move |entries: &[IntersectionEntry<'_>]| {
            let mut logs = sink.borrow_mut();
            for entry in entries {
                logs.panel.reports += 1;
                logs.panel.intersecting = entry.is_intersecting;
            }
        },
    );

    let ok = resize.is_ok() && matches!(inter, Some(Ok(_))) && viewport.is_ok();
    t.check(
        "frames.create",
        ok,
        format!(
            "resize {:?}, intersection {:?}, viewport {:?}",
            resize.as_ref().err(),
            inter.as_ref().map(|r| r.is_ok()),
            viewport.as_ref().err()
        ),
    );
    let frames = Rc::new(Frames {
        t: t.clone(),
        platform: platform.clone(),
        logs,
        resize: resize.ok(),
        inter: inter.and_then(|r| r.ok()),
        viewport: RefCell::new(viewport.ok()),
        fluid_frozen: Cell::new(0),
        fluid_before: Cell::new(0.0),
    });
    if !ok {
        t.finish();
        return frames;
    }
    if let (Some(resize), Some(inter)) = (&frames.resize, &frames.inter) {
        observe_id(doc, resize, "rs-box");
        observe_id(doc, resize, "rs-fluid");
        observe_id(doc, inter, "io-a");
        observe_id(doc, inter, "io-b");
    }
    if let Some(viewport) = frames.viewport.borrow().as_ref() {
        observe_id(doc, viewport, "p-io");
    }
    next(&frames, phase1);
    frames
}

fn next(frames: &Rc<Frames>, step: fn(&Rc<Frames>)) {
    let later = frames.clone();
    if let Err(error) = frames.platform.set_timeout(350, move || step(&later)) {
        frames
            .t
            .check("frames.timer", false, format!("set_timeout: {error}"));
        frames.t.finish();
    }
}

fn phase1(f: &Rc<Frames>) {
    let (t, doc) = (&f.t, &f.t.doc);
    let stage = rect_of(doc, "rs-stage");
    let client = by_id(doc, "io-scroller")
        .and_then(|e| e.client_rect().ok())
        .unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });
    let logs = f.logs.borrow();
    let (b, fl, a, bb, panel) = (logs.rs_box, logs.fluid, logs.a, logs.b, logs.panel);
    drop(logs);

    // content-box 160x40, padding 8, border 2: border box 180x60, content at 10,10.
    t.check(
        "rs.first_report",
        b.reports >= 1
            && near(b.content_w, 160.0, 0.5)
            && near(b.content_h, 40.0, 0.5)
            && near(b.border_w, 180.0, 0.5)
            && near(b.border_h, 60.0, 0.5),
        format!(
            "reports {}, content {:.1}x{:.1}, border {:.1}x{:.1}",
            b.reports, b.content_w, b.content_h, b.border_w, b.border_h
        ),
    );
    t.known_bug(
        "rs.content_offset",
        near(b.content_x, 10.0, 0.5) && near(b.content_y, 10.0, 0.5),
        "content_x / content_y give the padding offset (contentRect.x), not the documented offset inside the border box (padding + border)",
        format!("offset {:.1},{:.1} (want 10,10)", b.content_x, b.content_y),
    );
    // 50% of the stage's content box (border 1 + padding 4 on each side).
    t.check(
        "rs.percentage",
        fl.reports >= 1 && near(fl.content_w, (stage.width - 10.0) / 2.0, 0.6),
        format!(
            "reports {}, content {:.1}, stage {:.1}",
            fl.reports, fl.content_w, stage.width
        ),
    );
    t.check(
        "io.first_report",
        a.reports >= 1 && bb.reports >= 1,
        format!("reports A {}, B {}", a.reports, bb.reports),
    );
    t.check(
        "io.visible",
        a.intersecting && near(a.ratio, 1.0, 0.01),
        format!("A intersecting {}, ratio {:.2}", a.intersecting, a.ratio),
    );
    t.check(
        "io.hidden",
        !bb.intersecting && near(bb.ratio, 0.0, 0.01),
        format!("B intersecting {}, ratio {:.2}", bb.intersecting, bb.ratio),
    );
    // W3C IntersectionObserver: an element root that clips its content intersects through its
    // padding area, i.e. the client rect.
    t.known_bug(
        "io.root_bounds",
        near(a.root_h, client.height, 1.0) && near(a.root_w, client.width, 1.0),
        "root_bounds is the root's border box (border and scrollbar included), not its padding area",
        format!("root {:.1}x{:.1}, scroller client {:.1}x{:.1}", a.root_w, a.root_h, client.width, client.height),
    );
    t.check(
        "io.viewport_root",
        panel.reports >= 1 && panel.intersecting,
        format!(
            "panel against the viewport: reports {}, intersecting {}",
            panel.reports, panel.intersecting
        ),
    );

    f.fluid_before.set(fl.content_w);
    set_style(doc, "rs-box", "width", "200px");
    set_class(doc, "rs-stage", "narrow", true);
    scroll_to(doc, "io-scroller", 10000.0);
    next(f, phase2);
}

fn phase2(f: &Rc<Frames>) {
    let (t, doc) = (&f.t, &f.t.doc);
    let fluid = rect_of(doc, "rs-fluid");
    let logs = f.logs.borrow();
    let (b, fl, a, bb) = (logs.rs_box, logs.fluid, logs.a, logs.b);
    drop(logs);
    t.check(
        "rs.resize_box",
        b.reports >= 2 && near(b.content_w, 200.0, 0.5) && near(b.border_w, 220.0, 0.5),
        format!(
            "reports {}, content {:.1}, border {:.1} (want 200, 220)",
            b.reports, b.content_w, b.border_w
        ),
    );
    t.check(
        "rs.resize_layout",
        fl.reports >= 2
            && near(fl.content_w, fluid.width, 0.6)
            && fl.content_w < f.fluid_before.get() - 10.0,
        format!(
            "reports {}, content {:.1} (laid out {:.1}, before {:.1})",
            fl.reports,
            fl.content_w,
            fluid.width,
            f.fluid_before.get()
        ),
    );
    t.check(
        "io.scroll_in",
        bb.intersecting && near(bb.ratio, 1.0, 0.01),
        format!("B intersecting {}, ratio {:.2}", bb.intersecting, bb.ratio),
    );
    t.check(
        "io.scroll_out",
        !a.intersecting && near(a.ratio, 0.0, 0.01),
        format!("A intersecting {}, ratio {:.2}", a.intersecting, a.ratio),
    );

    // Stop watching the fluid box, then change everything back.
    if let (Some(resize), Some(e)) = (&f.resize, by_id(doc, "rs-fluid")) {
        let _ = resize.unobserve(&e);
    }
    f.fluid_frozen.set(fl.reports);
    set_class(doc, "rs-stage", "narrow", false);
    set_style(doc, "rs-box", "width", "160px");
    scroll_to(doc, "io-scroller", 0.0);
    next(f, phase3);
}

fn phase3(f: &Rc<Frames>) {
    let t = &f.t;
    let logs = f.logs.borrow();
    let (b, fl, a, bb) = (logs.rs_box, logs.fluid, logs.a, logs.b);
    drop(logs);
    t.check(
        "rs.unobserve",
        fl.reports == f.fluid_frozen.get(),
        format!(
            "unobserved fluid reports {} -> {}",
            f.fluid_frozen.get(),
            fl.reports
        ),
    );
    t.check(
        "rs.still_observing",
        near(b.content_w, 160.0, 0.5),
        format!(
            "box content width after the reset {:.1} (want 160)",
            b.content_w
        ),
    );
    t.check(
        "io.scroll_back",
        a.intersecting && near(a.ratio, 1.0, 0.01) && !bb.intersecting,
        format!("A {} {:.2}, B {}", a.intersecting, a.ratio, bb.intersecting),
    );
    let disconnected = f
        .viewport
        .borrow()
        .as_ref()
        .map(|v| v.disconnect().is_ok())
        .unwrap_or(false);
    drop(f.viewport.borrow_mut().take()); // kui_observer_release
    t.check(
        "io.disconnect_close",
        disconnected,
        "disconnect failed".into(),
    );
    if let Some(resize) = &f.resize {
        observe_id(&t.doc, resize, "rs-fluid"); // back on for the panel buttons
    }
    t.finish();
}

// ---- Panel buttons -----------------------------------------------------------------------

struct Panel {
    doc: Document,
    count: Observable,
    level: Observable,
    badge: Observable,
    name: Rc<Observable>,
    text_binding: Cell<u32>,
    mut_observer: RefCell<Option<Observer>>,
    mut_total: Rc<Cell<usize>>,
    mut_next: Cell<u32>,
    rs_width: Cell<u32>,
    themes: Option<ThemeManager>,
    router: Rc<RouterState>,
    _frames: Rc<Frames>,
}

impl Panel {
    fn show_observable(&self) {
        let bound = if self.text_binding.get() != 0 {
            "yes"
        } else {
            "no"
        };
        set_text(
            &self.doc,
            "ob-out",
            &format!(
                "count={} text={} level={} bound={bound}",
                self.count.get().unwrap_or_default(),
                text_of(&self.doc, "ob-text"),
                self.level.get().unwrap_or_default()
            ),
        );
    }

    fn observe_mut(&self) {
        let options = MutationOptions {
            child_list: true,
            attributes: true,
            character_data: true,
            subtree: true,
            attribute_old_value: true,
            character_data_old_value: true,
            attribute_filter: Vec::new(),
        };
        if let (Some(obs), Some(boxed)) = (
            self.mut_observer.borrow().as_ref(),
            by_id(&self.doc, "mut-box"),
        ) {
            let _ = obs.observe_mutations(&boxed, &options);
        }
    }

    fn show_theme(&self) {
        let active = self
            .themes
            .as_ref()
            .and_then(|m| m.active_theme().ok())
            .unwrap_or_default();
        set_text(
            &self.doc,
            "theme-out",
            &format!(
                "active={active} | --tour-bg={}",
                html_custom_property(&self.doc, "--tour-bg")
            ),
        );
    }

    fn show_route(&self, note: &str) {
        let router = self.doc.router();
        let route = router.current().unwrap_or_default();
        let (index, size) = router.history().unwrap_or((-2, -2));
        let id = router
            .param("id")
            .ok()
            .flatten()
            .unwrap_or_else(|| "-".into());
        let locked = if self.router.admin_locked.get() {
            "yes"
        } else {
            "no"
        };
        set_text(
            &self.doc,
            "route-out",
            &format!(
                "{note}path={} pattern={} | {index}/{size} | id={id} | locked={locked}",
                route.path, route.pattern
            ),
        );
    }

    fn navigate(&self, path: &str, replace: bool) {
        let router = self.doc.router();
        let navigated = if replace {
            router.replace(path)
        } else {
            router.push(path)
        }
        .unwrap_or(false);
        self.show_route(&if navigated {
            String::new()
        } else {
            format!("blocked {path} | ")
        });
    }

    fn go_by(&self, delta: i32, fail_note: &str) {
        let navigated = self.doc.router().go(delta).unwrap_or(false);
        self.show_route(if navigated { "" } else { fail_note });
    }
}

fn mutation_summary(total: usize, records: &[MutationRecord<'_>]) -> Option<String> {
    let last = records.last()?;
    let kind = match last.kind {
        MutationType::ChildList => "childList",
        MutationType::Attributes => "attributes",
        MutationType::CharacterData => "characterData",
    };
    let mut out = format!("records={total} | {kind} #{}", id_of(&last.target));
    let (added, removed) = (id_of(&last.added), id_of(&last.removed));
    if !added.is_empty() {
        out.push_str(&format!(" +#{added}"));
    }
    if !removed.is_empty() {
        out.push_str(&format!(" -#{removed}"));
    }
    if last.kind == MutationType::Attributes {
        out.push_str(&format!(" {}", last.attribute_name));
    }
    if last.kind != MutationType::ChildList {
        out.push_str(&format!(" old={}", last.old_value));
    }
    Some(out)
}

fn setup_panel(
    ctx: &Context<'_>,
    themes: Option<ThemeManager>,
    router: Rc<RouterState>,
    frames: Rc<Frames>,
) -> Result<Rc<Panel>, Box<dyn Error>> {
    let doc = ctx.document;
    let panel = Rc::new(Panel {
        doc: doc.clone(),
        count: Observable::new("0")?,
        level: Observable::new("cool")?,
        badge: Observable::new("background-color: #dbeafe")?,
        name: Rc::new(Observable::new("KUI")?),
        text_binding: Cell::new(0),
        mut_observer: RefCell::new(None),
        mut_total: Rc::new(Cell::new(0)),
        mut_next: Cell::new(1),
        rs_width: Cell::new(160),
        themes,
        router,
        _frames: frames,
    });
    panel
        .text_binding
        .set(bind_id(doc, &panel.count, "ob-text", "textContent", false).unwrap_or(0));
    let _ = bind_id(doc, &panel.level, "ob-badge", "data-level", false);
    let _ = bind_id(doc, &panel.badge, "ob-badge", "style", false);
    let _ = bind_id(doc, &panel.name, "ob-input", "value", true);
    let (page, name) = (doc.clone(), Rc::downgrade(&panel.name));
    let echo = move || {
        if let Some(name) = name.upgrade() {
            set_text(
                &page,
                "ob-echo",
                &format!("echo: {}", name.get().unwrap_or_default()),
            );
        }
    };
    echo();
    panel.name.subscribe(echo)?;
    panel.show_observable();

    let (page, total) = (doc.clone(), panel.mut_total.clone());
    let observer = doc.create_mutation_observer(move |records| {
        total.set(total.get() + records.len());
        if let Some(out) = mutation_summary(total.get(), records) {
            set_text(&page, "mut-out", &out);
        }
    })?;
    *panel.mut_observer.borrow_mut() = Some(observer);
    panel.observe_mut();
    panel.show_theme();
    panel.show_route("");
    Ok(panel)
}

/// Registers a click handler that gets the shared panel.
fn on<F>(
    ctx: &Context<'_>,
    panel: &Rc<Panel>,
    element_id: &str,
    handler: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&Panel) + 'static,
{
    let panel = panel.clone();
    let element = ctx
        .document
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener("click", move |_event| handler(&panel))?;
    Ok(())
}

fn bind_buttons(ctx: &Context<'_>, panel: &Rc<Panel>) -> Result<(), Box<dyn Error>> {
    on(ctx, panel, "btn-ob-inc", |p| {
        let value: i64 = p.count.get().unwrap_or_default().parse().unwrap_or(0);
        let _ = p.count.set(&(value + 1).to_string());
        let _ = p.doc.flush_bindings();
        p.show_observable();
    })?;
    on(ctx, panel, "btn-ob-color", |p| {
        let hot = p.level.get().unwrap_or_default() != "hot";
        let _ = p.level.set(if hot { "hot" } else { "cool" });
        let _ = p.badge.set(if hot {
            "background-color: #fee2e2"
        } else {
            "background-color: #dbeafe"
        });
        let _ = p.doc.flush_bindings();
        p.show_observable();
    })?;
    on(ctx, panel, "btn-ob-unbind", |p| {
        if p.text_binding.get() != 0 {
            let _ = p.count.unbind(p.text_binding.get());
        }
        p.text_binding.set(0);
        p.show_observable();
    })?;
    on(ctx, panel, "btn-ob-rebind", |p| {
        if p.text_binding.get() == 0 {
            p.text_binding
                .set(bind_id(&p.doc, &p.count, "ob-text", "textContent", false).unwrap_or(0));
        }
        p.show_observable();
    })?;
    on(ctx, panel, "btn-mut-add", |p| {
        let n = p.mut_next.get();
        p.mut_next.set(n + 1);
        if let (Ok(item), Some(list)) = (p.doc.create_element("li"), by_id(&p.doc, "mut-list")) {
            let _ = item.set_text_content(&format!("item {n}"));
            let _ = item.set_attribute("id", &format!("mut-li-{n}"));
            let _ = list.append_child(&item);
        }
    })?;
    on(ctx, panel, "btn-mut-remove", |p| {
        if let Some(list) = by_id(&p.doc, "mut-list") {
            if let Ok(Some(last)) = list.last_child() {
                let _ = list.remove_child(&last);
            }
        }
    })?;
    on(ctx, panel, "btn-mut-attr", |p| {
        let next = if attr_of(&p.doc, "mut-item", "data-state") == "on" {
            "off"
        } else {
            "on"
        };
        set_attr(&p.doc, "mut-item", "data-state", next);
    })?;
    on(ctx, panel, "btn-mut-text", |p| {
        let next = if text_of(&p.doc, "mut-text") == "hello" {
            "world"
        } else {
            "hello"
        };
        set_text(&p.doc, "mut-text", next);
    })?;
    on(ctx, panel, "btn-mut-stop", |p| {
        if let Some(obs) = p.mut_observer.borrow().as_ref() {
            let _ = obs.disconnect();
        }
        set_text(
            &p.doc,
            "mut-out",
            &format!("disconnected (records={})", p.mut_total.get()),
        );
    })?;
    on(ctx, panel, "btn-mut-start", |p| {
        p.observe_mut();
        set_text(
            &p.doc,
            "mut-out",
            &format!("observing (records={})", p.mut_total.get()),
        );
    })?;
    on(ctx, panel, "btn-rs-grow", |p| {
        p.rs_width.set(p.rs_width.get() + 40);
        set_style(
            &p.doc,
            "rs-box",
            "width",
            &format!("{}px", p.rs_width.get()),
        );
    })?;
    on(ctx, panel, "btn-rs-reset", |p| {
        p.rs_width.set(160);
        set_style(&p.doc, "rs-box", "width", "160px");
        set_class(&p.doc, "rs-stage", "narrow", false);
    })?;
    on(ctx, panel, "btn-rs-stage", |p| {
        if let Some(stage) = by_id(&p.doc, "rs-stage") {
            let narrow = stage.class_list_contains("narrow").unwrap_or(false);
            let _ = if narrow {
                stage.class_list_remove("narrow")
            } else {
                stage.class_list_add("narrow")
            };
        }
    })?;
    on(ctx, panel, "btn-io-bottom", |p| {
        scroll_to(&p.doc, "io-scroller", 10000.0)
    })?;
    on(ctx, panel, "btn-io-top", |p| {
        scroll_to(&p.doc, "io-scroller", 0.0)
    })?;
    on(ctx, panel, "btn-th-light", |p| {
        if let Some(m) = &p.themes {
            let _ = m.apply("tour-light", &p.doc);
        }
        p.show_theme();
    })?;
    on(ctx, panel, "btn-th-dark", |p| {
        if let Some(m) = &p.themes {
            let _ = m.apply("tour-dark", &p.doc);
        }
        p.show_theme();
    })?;
    on(ctx, panel, "btn-th-list", |p| {
        let Some(m) = &p.themes else { return };
        let names = m.theme_names().unwrap_or_default();
        let mut out = format!("{} themes:", names.len());
        for name in &names {
            out.push_str(&format!(
                " {name}({})",
                m.theme(name).map(|v| v.len()).unwrap_or(0)
            ));
        }
        set_text(&p.doc, "theme-out", &out);
    })?;
    on(ctx, panel, "btn-rt-home", |p| p.navigate("/", false))?;
    on(ctx, panel, "btn-rt-user", |p| p.navigate("/user/7", false))?;
    on(ctx, panel, "btn-rt-about", |p| p.navigate("/about", true))?;
    on(ctx, panel, "btn-rt-admin", |p| p.navigate("/admin", false))?;
    on(ctx, panel, "btn-rt-lock", |p| {
        p.router.admin_locked.set(!p.router.admin_locked.get());
        p.show_route("");
    })?;
    on(ctx, panel, "btn-rt-back", |p| p.go_by(-1, "no back | "))?;
    on(ctx, panel, "btn-rt-fwd", |p| p.go_by(1, "no forward | "))?;
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let t = Rc::new(Tour {
        doc: ctx.document.clone(),
        total: Cell::new(0),
        failed: Cell::new(0),
        known: Cell::new(0),
        failed_names: RefCell::new(Vec::new()),
    });
    check_observable(&t);
    check_raw_gaps(&t);
    check_mutation(&t);
    let themes = check_theme(&t);
    let router = check_router(&t);
    let frames = start_frame_checks(&t, ctx.platform); // finishes the verdict from its timers

    let panel = setup_panel(ctx, themes, router, frames)?;
    bind_buttons(ctx, &panel)
}
