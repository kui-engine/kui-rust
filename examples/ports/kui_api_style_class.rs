// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_style_class.rs; edit that file, not this copy.
//! API tour: classes, inline style, computed style, stylesheets and animation state
//! (examples/api_tours/style_class). On start the host calls every method once, checks the result
//! against what the C ABI documents and logs one row per check into #tour-log. Animation state is
//! only observable once frames run, so those checks finish from a 400 ms timer and #tour-result
//! becomes "PASS n/n" then. The panel buttons repeat the calls by hand.

use crate::canonical_runner::Context;
use kui::{Document, Element, StyleProfile};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check: a documented behaviour; a failure makes the tour FAIL. (No known engine bugs in this
// tour; see kui_api_dom_tree.rs for the known_bug variant that logs a reported bug without failing.)

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
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- Small readers -----------------------------------------------------------------------

fn by_id<'d>(doc: &'d Document, element_id: &str) -> Option<Element<'d>> {
    doc.get_element_by_id(element_id).ok().flatten()
}

fn set_text(doc: &Document, element_id: &str, value: &str) {
    if let Some(element) = by_id(doc, element_id) {
        let _ = element.set_text_content(value);
    }
}

fn is_not_found<T>(result: &kui::Result<T>) -> bool {
    matches!(result, Err(e) if e.status() == kui::sys::KUI_ERR_NOT_FOUND)
}

fn class_of(e: &Element<'_>) -> String {
    e.class_name().unwrap_or_default()
}

fn has(e: &Element<'_>, token: &str) -> bool {
    e.class_list_contains(token).unwrap_or(false)
}

fn computed(e: &Element<'_>, property: &str) -> String {
    e.computed_style_value(property).unwrap_or_default()
}

fn computed_by_id(doc: &Document, element_id: &str, property: &str) -> String {
    by_id(doc, element_id)
        .map(|e| computed(&e, property))
        .unwrap_or_default()
}

fn property_names(e: &Element<'_>) -> Vec<String> {
    e.style_property_names().unwrap_or_default()
}

/// "count: name=value[ !important], ..." for every inline property.
fn describe_inline(e: &Element<'_>) -> String {
    let names = property_names(e);
    let parts: Vec<String> = names
        .iter()
        .map(|name| {
            let value = e.style_property(name).unwrap_or_default();
            let important = if e.style_property_is_important(name).unwrap_or(false) {
                " !important"
            } else {
                ""
            };
            format!("{name}={value}{important}")
        })
        .collect();
    format!("{}: {}", names.len(), parts.join(", "))
}

/// The page's own <style> text, read from <head>.
fn page_css(doc: &Document) -> String {
    doc.head()
        .ok()
        .flatten()
        .and_then(|head| head.query_selector("style").ok().flatten())
        .and_then(|style| style.text_content().ok())
        .unwrap_or_default()
}

// ---- Class list: add / remove / toggle / contains, class name and id --------------------

fn check_class_list(t: &Tour) {
    let Some(el) = by_id(&t.doc, "sc-cls") else {
        t.check("class.page", false, "missing #sc-cls".to_string());
        return;
    };
    let id = el.id().unwrap_or_default();
    t.check("el.get_id", id == "sc-cls", format!("id '{id}'"));
    t.check(
        "el.get_class_name",
        class_of(&el) == "a b",
        format!("className '{}'", class_of(&el)),
    );

    let _ = el.class_list_add("c");
    t.check(
        "el.class_list_add",
        class_of(&el) == "a b c" && has(&el, "c"),
        format!("className '{}'", class_of(&el)),
    );
    let _ = el.class_list_add("a");
    t.check(
        "el.class_list_add-duplicate",
        class_of(&el) == "a b c",
        format!("className '{}'", class_of(&el)),
    );

    let _ = el.class_list_remove("b");
    t.check(
        "el.class_list_remove",
        class_of(&el) == "a c" && !has(&el, "b"),
        format!(
            "className '{}', contains(b) {}",
            class_of(&el),
            has(&el, "b")
        ),
    );
    t.check(
        "el.class_list_contains",
        has(&el, "a") && !has(&el, "zz"),
        format!(
            "contains(a) {}, contains(zz) {}",
            has(&el, "a"),
            has(&el, "zz")
        ),
    );

    let _ = el.class_list_toggle("d");
    let on = has(&el, "d");
    let _ = el.class_list_toggle("d");
    let off = has(&el, "d");
    t.check(
        "el.class_list_toggle",
        on && !off,
        format!("after toggles {on} then {off}"),
    );

    let _ = el.set_class_name("tour-wide");
    t.check(
        "el.set_class_name",
        class_of(&el) == "tour-wide" && !has(&el, "a"),
        format!("className '{}'", class_of(&el)),
    );
    // The page stylesheet gives #sc-cls.tour-wide a width of 77px.
    let width = computed(&el, "width");
    t.check(
        "el.class_drives_style",
        width == "77px",
        format!("computed width '{width}'"),
    );
}

// ---- Inline style: properties, !important, enumeration, whole-declaration text ----------

fn check_inline_style(t: &Tour) {
    let Some(el) = by_id(&t.doc, "sc-style") else {
        t.check("inline.page", false, "missing #sc-style".to_string());
        return;
    };
    let missing = el.style_property("color");
    t.check(
        "el.get_style_property-missing",
        is_not_found(&missing),
        format!("result {missing:?}"),
    );

    // kui_c.h: named colours come back serialized as hex, lengths keep their unit.
    let _ = el.set_style_property("color", "red", false);
    let color = el.style_property("color");
    t.check(
        "el.set_get_style_property",
        matches!(&color, Ok(value) if value == "#ff0000"),
        format!("color {color:?}"),
    );
    let _ = el.set_style_property("width", "100px", false);
    let width = el.style_property("width").unwrap_or_default();
    t.check(
        "el.get_style_property-length",
        width == "100px",
        format!("width '{width}'"),
    );

    let before = el.style_property_is_important("color").unwrap_or(true);
    let _ = el.set_style_property("background-color", "#00ff00", true);
    let after = el
        .style_property_is_important("background-color")
        .unwrap_or(false);
    t.check(
        "el.style_property_is_important",
        !before && after,
        format!("color {before}, background-color {after}"),
    );

    let names = property_names(&el);
    let joined = names.join(",");
    t.check(
        "el.get_style_property_count_name",
        names.len() == 3
            && ["color", "width", "background-color"]
                .iter()
                .all(|n| names.iter().any(|m| m == n)),
        format!("count {}, names '{joined}'", names.len()),
    );

    let text = el.style_text().unwrap_or_default();
    t.check(
        "el.get_style_text",
        text.contains("width")
            && text.contains("100px")
            && text.contains("background-color")
            && text.contains("!important"),
        format!("cssText '{text}'"),
    );

    // set_style_text replaces the whole declaration.
    let _ = el.set_style_text("margin-left: 4px; padding-top: 2px");
    let margin = el.style_property("margin-left").unwrap_or_default();
    let color = el.style_property("color");
    t.check(
        "el.set_style_text",
        property_names(&el).len() == 2 && margin == "4px" && is_not_found(&color),
        format!(
            "count {}, margin-left '{margin}', color {color:?}",
            property_names(&el).len()
        ),
    );

    let _ = el.remove_style_property("margin-left");
    let removed = el.style_property("margin-left");
    t.check(
        "el.remove_style_property",
        is_not_found(&removed) && property_names(&el).len() == 1,
        format!("result {removed:?}, count {}", property_names(&el).len()),
    );
}

// ---- Computed style: the cascade's resolved values --------------------------------------

fn check_computed(t: &Tour) {
    let Some(el) = by_id(&t.doc, "cs-target") else {
        t.check("computed.page", false, "missing #cs-target".to_string());
        return;
    };
    let (display, width) = (computed(&el, "display"), computed(&el, "width"));
    let (font, padding) = (computed(&el, "font-size"), computed(&el, "padding-left"));
    t.check(
        "el.get_computed_style_value",
        display == "flex" && width == "240px" && font == "15px" && padding == "8px",
        format!(
            "display '{display}', width '{width}', font-size '{font}', padding-left '{padding}'"
        ),
    );
    let unknown = el.computed_style_value("no-such-property");
    t.check(
        "el.get_computed_style_value-unknown",
        matches!(&unknown, Ok(value) if value.is_empty()),
        format!("result {unknown:?}"),
    );
    // Inline style feeds the cascade too.
    let inline = computed_by_id(&t.doc, "sc-style", "padding-top");
    t.check(
        "el.get_computed_style_value-inline",
        inline == "2px",
        format!("padding-top '{inline}'"),
    );
}

// ---- Stylesheets & profile: add, clear every author sheet, restore the page's own --------

fn check_stylesheets(t: &Tour) {
    let mut doc = t.doc.clone(); // add / clear take &mut Document
    let profile = doc.style_profile();
    t.check(
        "doc.get_style_profile",
        matches!(profile, Ok(StyleProfile::NativeGui)),
        format!("profile {profile:?}"),
    );

    let added = doc.add_stylesheet("#sc-sheet { width: 123px; }");
    let width = computed_by_id(&doc, "sc-sheet", "width");
    t.check(
        "doc.add_stylesheet",
        added.is_ok() && width == "123px",
        format!("result {added:?}, width '{width}'"),
    );

    let css = page_css(&doc);
    t.check(
        "head.page_css",
        css.contains("#cs-target"),
        format!("page <style> text has {} bytes", css.len()),
    );

    // Clearing drops the page sheet as well as ours: #cs-target falls back to display:block.
    let cleared = doc.clear_stylesheets();
    let display = computed_by_id(&doc, "cs-target", "display");
    let width = computed_by_id(&doc, "sc-sheet", "width");
    t.check(
        "doc.clear_stylesheets",
        cleared.is_ok() && display == "block" && width != "123px",
        format!("result {cleared:?}, display '{display}', width '{width}'"),
    );

    let restored = doc.add_stylesheet(&css);
    let display = computed_by_id(&doc, "cs-target", "display");
    let width = computed_by_id(&doc, "sc-sheet", "width");
    t.check(
        "doc.add_stylesheet-restore",
        restored.is_ok() && display == "flex" && width != "123px",
        format!("result {restored:?}, display '{display}', width '{width}'"),
    );
}

// ---- Animation state: needs running frames, so it is checked from a timer ---------------

fn check_animation(t: &Tour) {
    let state = |element_id: &str| {
        by_id(&t.doc, element_id)
            .and_then(|e| e.animation_state().ok())
            .unwrap_or((false, false))
    };
    let (animating, transitioning) = state("anim-box");
    t.check(
        "el.get_animation_state-animation",
        animating,
        format!("animating {animating}, transitioning {transitioning}"),
    );
    let (animating, transitioning) = state("sc-trans");
    t.check(
        "el.get_animation_state-transition",
        transitioning,
        format!("animating {animating}, transitioning {transitioning}"),
    );
    let (animating, transitioning) = state("cls-target");
    t.check(
        "el.get_animation_state-idle",
        !animating && !transitioning,
        format!("animating {animating}, transitioning {transitioning}"),
    );
    let (animations, transitions) = t.doc.animation_counts().unwrap_or((0, 0));
    t.check(
        "doc.get_animation_counts",
        animations >= 1 && transitions >= 1,
        format!("animations {animations}, transitions {transitions}"),
    );
    t.finish();
}

/// Starts the #anim-box animation (class .spinning) and a 2 s width transition on #sc-trans (inline
/// style beats the 40px sheet value), then checks the state 400 ms later. It runs after the
/// stylesheet checks only so that those checks see a quiet page; clearing the sheet that holds a
/// running animation's @keyframes is safe (the engine cancels that animation:
/// test_keyframes_stylesheet_removal).
fn start_animation_checks(ctx: &Context<'_>, t: Rc<Tour>) {
    if let Some(box_el) = by_id(&t.doc, "anim-box") {
        let _ = box_el.class_list_add("spinning");
    }
    let _ = computed_by_id(&t.doc, "sc-trans", "width"); // settle the old value first
    if let Some(bar) = by_id(&t.doc, "sc-trans") {
        let _ = bar.set_style_property("width", "200px", false);
    }
    let later = t.clone();
    if let Err(error) = ctx
        .platform
        .set_timeout(400, move || check_animation(&later))
    {
        t.check("animation.timer", false, format!("set_timeout: {error}"));
        t.finish();
    }
}

// ---- Panel buttons -----------------------------------------------------------------------

/// Registers a click handler on the element with that id; the handler gets its own document.
fn on<F>(ctx: &Context<'_>, element_id: &str, mut handler: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&mut Document) + 'static,
{
    let mut doc = ctx.document.clone();
    let element = ctx
        .document
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener("click", move |_event| handler(&mut doc))?;
    Ok(())
}

fn bind_class_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    fn with_target(doc: &Document, change: impl Fn(&Element<'_>)) {
        let Some(el) = by_id(doc, "cls-target") else {
            return;
        };
        change(&el);
        let accent = if has(&el, "accent") { "yes" } else { "no" };
        let line = format!(
            "#{} class=\"{}\" | accent={accent}",
            el.id().unwrap_or_default(),
            class_of(&el)
        );
        set_text(doc, "cls-out", &line);
    }
    on(ctx, "btn-cls-toggle", |doc| {
        with_target(doc, |el| {
            let _ = el.class_list_toggle("accent");
        })
    })?;
    on(ctx, "btn-cls-add", |doc| {
        with_target(doc, |el| {
            let _ = el.class_list_add("big");
        })
    })?;
    on(ctx, "btn-cls-remove", |doc| {
        with_target(doc, |el| {
            let _ = el.class_list_remove("big");
        })
    })?;
    on(ctx, "btn-cls-reset", |doc| {
        with_target(doc, |el| {
            let _ = el.set_class_name("card");
        })
    })
}

fn bind_inline_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    fn with_target(doc: &Document, change: impl Fn(&Element<'_>) -> String) {
        let Some(el) = by_id(doc, "st-target") else {
            return;
        };
        let line = change(&el);
        set_text(doc, "st-out", &line);
    }
    fn show_text(el: &Element<'_>) -> String {
        let text = el.style_text().unwrap_or_default();
        if text.is_empty() {
            "(no inline style)".to_string()
        } else {
            text
        }
    }
    on(ctx, "btn-st-set", |doc| {
        with_target(doc, |el| {
            let _ = el.set_style_property("color", "#b91c1c", false);
            let _ = el.set_style_property("background-color", "#fee2e2", true);
            show_text(el)
        })
    })?;
    on(ctx, "btn-st-text", |doc| {
        with_target(doc, |el| {
            let _ = el.set_style_text("background-color: #fef3c7; padding: 12px");
            show_text(el)
        })
    })?;
    on(ctx, "btn-st-remove", |doc| {
        with_target(doc, |el| {
            let _ = el.remove_style_property("background-color");
            show_text(el)
        })
    })?;
    on(ctx, "btn-st-list", |doc| {
        with_target(doc, |el| describe_inline(el))
    })
}

fn bind_computed_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    fn read(doc: &Document, toggle_wide: bool) {
        let Some(el) = by_id(doc, "cs-target") else {
            return;
        };
        if toggle_wide {
            let _ = el.class_list_toggle("wide");
        }
        let line = format!(
            "display={} | width={} | font-size={} | color={}",
            computed(&el, "display"),
            computed(&el, "width"),
            computed(&el, "font-size"),
            computed(&el, "color")
        );
        set_text(doc, "cs-out", &line);
    }
    on(ctx, "btn-cs-read", |doc| read(doc, false))?;
    on(ctx, "btn-cs-wide", |doc| read(doc, true))
}

fn bind_sheet_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-sheet-add", |doc| {
        let _ = doc.add_stylesheet(
            "#sheet-probe { background-color: #dcfce7; border-left-color: #16a34a; }",
        );
        set_text(doc, "sheet-out", "host sheet added");
    })?;
    // Safe while #anim-box is animating: the animation whose @keyframes live in a cleared sheet is
    // cancelled, and the restored page sheet starts it again.
    on(ctx, "btn-sheet-clear", |doc| {
        let css = page_css(doc);
        let _ = doc.clear_stylesheets();
        if css.is_empty() {
            set_text(
                doc,
                "sheet-out",
                "cleared all sheets (page <style> not found)",
            );
            return;
        }
        let _ = doc.add_stylesheet(&css);
        set_text(doc, "sheet-out", "cleared all sheets, page sheet restored");
    })?;
    on(ctx, "btn-sheet-profile", |doc| {
        let name = match doc.style_profile() {
            Ok(StyleProfile::BrowserCompat) => "browser-compat",
            _ => "native-gui",
        };
        set_text(doc, "sheet-out", &format!("style profile = {name}"));
    })
}

fn bind_animation_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    fn toggle(doc: &Document, element_id: &str, class: &str) {
        if let Some(el) = by_id(doc, element_id) {
            let _ = el.class_list_toggle(class);
        }
    }
    fn yes_no(value: bool) -> &'static str {
        if value {
            "yes"
        } else {
            "no"
        }
    }
    on(ctx, "btn-anim-toggle", |doc| {
        toggle(doc, "anim-box", "spinning")
    })?;
    on(ctx, "btn-trans-go", |doc| toggle(doc, "trans-box", "wide"))?;
    on(ctx, "btn-anim-read", |doc| {
        let (animating, _) = by_id(doc, "anim-box")
            .and_then(|e| e.animation_state().ok())
            .unwrap_or_default();
        let (_, transitioning) = by_id(doc, "trans-box")
            .and_then(|e| e.animation_state().ok())
            .unwrap_or_default();
        let (animations, transitions) = doc.animation_counts().unwrap_or((0, 0));
        let line = format!(
            "box: animating={} | bar: transitioning={} | doc: animations={animations} transitions={transitions}",
            yes_no(animating),
            yes_no(transitioning)
        );
        set_text(doc, "anim-out", &line);
    })
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let t = Rc::new(Tour::new(ctx.document));
    check_class_list(&t);
    check_inline_style(&t);
    check_computed(&t);
    check_stylesheets(&t);
    start_animation_checks(ctx, t); // finishes the verdict from its timer

    bind_class_buttons(ctx)?;
    bind_inline_buttons(ctx)?;
    bind_computed_buttons(ctx)?;
    bind_sheet_buttons(ctx)?;
    bind_animation_buttons(ctx)?;
    Ok(())
}
