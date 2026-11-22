// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_popups_input.rs; edit that file, not this copy.
//! API tour: popups, toasts and input pickers (examples/api_tours/popups_input). At start-up the
//! host shows and dismisses toasts, tooltips and context menus, reads the popup layer's state
//! back after each step, configures the tooltip theme / delay and the default context menu, and
//! drives the <input> picker panel (open, page, time, colour, commit, close, enable) checking the
//! picker state and the input's value (`Element::value`). #tour-result ends as
//! "PASS n/n"; the panel buttons repeat the calls with a visible effect.
//!
//! Binding note: PopupPosition / PopupType are Rust enums, so the C/Go checks that pass an
//! out-of-range position or type (KUI_ERR_INVALID_ARG) cannot be expressed here.

use crate::canonical_runner::Context;
use kui::sys::{KuiStatus, KUI_ERR_INVALID_ARG, KUI_ERR_NOT_FOUND, KUI_ERR_NOT_SUPPORTED};
use kui::{
    Document, InputPickerState, MenuItem, PickerMode, Platform, PopupPosition, PopupType, Rect,
    ToastLevel, ToastOptions, TooltipTheme,
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

// ---- Small helpers ----------------------------------------------------------------------

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// The KuiStatus of a result (0 for Ok).
fn status<T>(result: &kui::Result<T>) -> KuiStatus {
    result.as_ref().err().map_or(0, |e| e.status())
}

fn status_name(status: KuiStatus) -> &'static str {
    match status {
        0 => "OK",
        KUI_ERR_INVALID_ARG => "INVALID_ARG",
        KUI_ERR_NOT_FOUND => "NOT_FOUND",
        KUI_ERR_NOT_SUPPORTED => "NOT_SUPPORTED",
        _ => "ERROR",
    }
}

fn mode_name(mode: PickerMode) -> &'static str {
    match mode {
        PickerMode::Date => "date",
        PickerMode::Time => "time",
        PickerMode::DateTimeLocal => "datetime-local",
        PickerMode::Color => "color",
        PickerMode::None => "none",
    }
}

const PURPLE: TooltipTheme = TooltipTheme {
    background: 0x7c3a_edff,
    foreground: 0xffff_ffff,
    border: 0x5b21_b6ff,
    radius: 8.0,
};

fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem {
            text: "Red".into(),
            id: 1,
            ..Default::default()
        },
        MenuItem {
            text: "Green".into(),
            id: 2,
            ..Default::default()
        },
        MenuItem {
            separator: true,
            ..Default::default()
        },
        MenuItem {
            text: "Disabled".into(),
            id: 3,
            disabled: true,
            ..Default::default()
        },
    ]
}

struct Host {
    doc: Document,
    platform: Rc<Platform>,
    t: Tour,
    toast_base: Cell<i32>,
    last_toast: Cell<u32>,
    toast_serial: Cell<i32>,
    default_theme: Cell<TooltipTheme>,
    purple_on: Cell<bool>,
    delay_step: Cell<usize>,
    tip_id: Cell<u32>,
    menu_id: Cell<u32>,
    menu_released: Rc<Cell<u32>>,
    context_own: Cell<bool>,
    time_changes: Cell<u32>,
    time_inputs: Cell<u32>,
}

impl Host {
    fn rect_of(&self, id: &str) -> Rect {
        self.doc
            .get_element_by_id(id)
            .ok()
            .flatten()
            .and_then(|e| e.bounding_rect().ok())
            .unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            })
    }

    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(e)) = self.doc.get_element_by_id(id) {
            let _ = e.set_text_content(text);
        }
    }

    fn has_type(&self, kind: PopupType) -> i32 {
        self.doc.popups().has(kind).map_or(-1, i32::from)
    }

    fn any_popup(&self) -> i32 {
        self.doc.popups().any().map_or(-1, i32::from)
    }

    fn toast_count(&self) -> i32 {
        self.doc.toast_count().unwrap_or(-1)
    }

    /// The input's current value (what the user typed): `Element::value`.
    fn input_value(&self, id: &str) -> String {
        match self.doc.get_element_by_id(id) {
            Ok(Some(element)) => element.value().unwrap_or_default(),
            _ => String::new(),
        }
    }

    // ---- Toasts: show two, dismiss one by id, then all; the count is read back ----------

    fn show_toast(&self, message: &str, level: ToastLevel) -> u32 {
        let options = ToastOptions {
            message: message.into(),
            title: "KUI API tour".into(),
            level,
            duration_ms: 0,
            closable: true,
            ..Default::default()
        };
        self.doc.show_toast(&options).unwrap_or(0)
    }

    fn check_toasts(&self) {
        let base = self.toast_count();
        self.toast_base.set(base);
        let first = self.show_toast("First toast (stays until dismissed)", ToastLevel::Info);
        let second = self.show_toast("Second toast", ToastLevel::Success);
        let after = self.toast_count();
        self.t.check(
            "toast.get_toast_count",
            base >= 0 && after == base + 2 && first != 0 && second != 0 && first != second,
            format!("count {base} -> {after}, ids {first} {second}"),
        );
        let dismissed = self.doc.dismiss_toast(first);
        self.t.check(
            "toast.dismiss_toast",
            dismissed.is_ok(),
            format!("{dismissed:?}"),
        );
        let all = self.doc.dismiss_all_toasts();
        self.t
            .check("toast.dismiss_all_toasts", all.is_ok(), format!("{all:?}"));
        // Dismissed toasts play an exit animation and still count; the final phase checks 0.
    }

    // ---- Tooltip: delay and theme read back, a tooltip shown at an element's rect -------

    fn check_tooltip(&self) {
        let pop = self.doc.popups();
        let delay = pop.tooltip_delay().unwrap_or(-1.0);
        let _ = pop.set_tooltip_delay(150.0);
        let after = pop.tooltip_delay().unwrap_or(-1.0);
        let bad = pop.set_tooltip_delay(-5.0);
        self.t.check(
            "tooltip.delay",
            near(delay, 500.0, 0.01)
                && near(after, 150.0, 0.01)
                && status(&bad) == KUI_ERR_INVALID_ARG,
            format!("default {delay:.1}, after set(150) {after:.1}, set(-5) {bad:?}"),
        );
        let _ = pop.set_tooltip_delay(500.0);

        let default_theme = pop.tooltip_theme().unwrap_or(PURPLE);
        self.default_theme.set(default_theme);
        let _ = pop.set_tooltip_theme(PURPLE);
        let theme = pop.tooltip_theme();
        let bad = pop.set_tooltip_theme(TooltipTheme {
            radius: -1.0,
            ..PURPLE
        });
        self.t.check(
            "tooltip.theme",
            theme.as_ref().ok() == Some(&PURPLE) && status(&bad) == KUI_ERR_INVALID_ARG,
            format!("read back {theme:?}, radius -1 {bad:?}"),
        );
        let _ = pop.set_tooltip_theme(default_theme);

        let anchor = self.rect_of("tip-anchor");
        let shown_id = pop.show_tooltip("Tooltip from the host", anchor, PopupPosition::Bottom);
        let shown = self.has_type(PopupType::Tooltip);
        self.t.check(
            "tooltip.show_tooltip",
            matches!(shown_id, Ok(id) if id != 0) && shown == 1 && anchor.width > 0.0,
            format!(
                "{shown_id:?} has tooltip {shown} anchor {:.0}x{:.0}",
                anchor.width, anchor.height
            ),
        );
        let _ = pop.dismiss(*shown_id.as_ref().unwrap_or(&0));
        let shown = self.has_type(PopupType::Tooltip);
        let unknown = pop.dismiss(987_654);
        self.t.check(
            "popup.dismiss_popup",
            shown == 0 && unknown.is_ok(),
            format!("has tooltip after dismiss {shown}, unknown id {unknown:?}"),
        );
    }

    // ---- Context menu: default menu switch, a host menu shown and dismissed by type -----

    /// Opens the host menu at #menu-spot's top-left corner; choosing an item writes #menu-out,
    /// dropping the closure (when the menu is gone) counts as the release.
    fn show_menu(&self) -> kui::Result<u32> {
        let spot = self.rect_of("menu-spot");
        let doc = self.doc.clone();
        let release = ReleaseCounter(self.menu_released.clone());
        self.doc
            .popups()
            .show_context_menu(spot.x, spot.y, &menu_items(), move |id| {
                let _keep = &release;
                let name = match id {
                    1 => "Red",
                    2 => "Green",
                    _ => "?",
                };
                if let Ok(Some(e)) = doc.get_element_by_id("menu-out") {
                    let _ = e.set_text_content(&format!("chose item {id} ({name})"));
                }
            })
    }

    fn check_menu(&self) {
        let pop = self.doc.popups();
        let initial = pop.default_context_menu_enabled();
        let _ = pop.set_default_context_menu_enabled(true);
        let on = pop.default_context_menu_enabled();
        let _ = pop.set_default_context_menu_enabled(false);
        let off = pop.default_context_menu_enabled();
        self.t.check(
            "menu.default_enabled",
            matches!((&initial, &on, &off), (Ok(false), Ok(true), Ok(false))),
            format!("initial {initial:?}, after on {on:?}, after off {off:?}"),
        );

        let released_before = self.menu_released.get();
        let id = self.show_menu();
        let open = self.has_type(PopupType::ContextMenu);
        self.t.check(
            "menu.show_context_menu",
            matches!(id, Ok(id) if id != 0)
                && open == 1
                && self.menu_released.get() == released_before,
            format!(
                "{id:?} has menu {open} released {}",
                self.menu_released.get()
            ),
        );
        let bad = pop.show_context_menu(10.0, 10.0, &[], |_| {});
        self.t.check(
            "menu.no_items",
            status(&bad) == KUI_ERR_INVALID_ARG,
            format!("{bad:?}"),
        );
        let _ = pop.dismiss_type(PopupType::ContextMenu);
        let open = self.has_type(PopupType::ContextMenu);
        let released = self.menu_released.get();
        self.t.check(
            "menu.release_fn",
            open == 0 && released == released_before + 1,
            format!("has menu {open}, release calls {released_before} -> {released}"),
        );
    }

    // ---- Popup layer: both kinds at once, removed by type, by id and all at once --------

    fn show_both(&self) {
        let anchor = self.rect_of("tip-anchor");
        let tip =
            self.doc
                .popups()
                .show_tooltip("Tooltip from the host", anchor, PopupPosition::Bottom);
        self.tip_id.set(tip.unwrap_or(0));
        self.menu_id.set(self.show_menu().unwrap_or(0));
    }

    fn check_popup_layer(&self) {
        let pop = self.doc.popups();
        self.show_both();
        let (any, tip, menu, custom) = (
            self.any_popup(),
            self.has_type(PopupType::Tooltip),
            self.has_type(PopupType::ContextMenu),
            self.has_type(PopupType::Custom),
        );
        self.t.check(
            "popup.has_popups",
            any == 1 && tip == 1 && menu == 1 && custom == 0,
            format!("any {any} tooltip {tip} menu {menu} custom {custom}"),
        );
        let _ = pop.dismiss_type(PopupType::Tooltip);
        let (tip, menu) = (
            self.has_type(PopupType::Tooltip),
            self.has_type(PopupType::ContextMenu),
        );
        self.t.check(
            "popup.dismiss_popups_of_type",
            tip == 0 && menu == 1,
            format!("tooltip {tip} menu {menu}"),
        );
        let _ = pop.dismiss(self.menu_id.get());
        let any = self.any_popup();
        self.t
            .check("popup.dismiss_popup-menu", any == 0, format!("any {any}"));
        self.show_both();
        let _ = pop.dismiss_all();
        let any = self.any_popup();
        self.t
            .check("popup.dismiss_all_popups", any == 0, format!("any {any}"));
    }

    // ---- Input pickers: open per input type, page / time / colour, commit and close -----

    fn picker_state(&self) -> InputPickerState {
        self.doc.input_picker().state().unwrap_or(InputPickerState {
            open: false,
            enabled: false,
            mode: PickerMode::None,
            visible_year: 0,
            visible_month: 0,
            color_h: 0.0,
            color_s: 0.0,
            color_v: 0.0,
            hour: 0,
            minute: 0,
        })
    }

    /// The id of the picker's target input ("" when it is closed).
    fn picker_target(&self) -> kui::Result<String> {
        Ok(match self.doc.input_picker().target()? {
            Some(target) => target.attribute("id").unwrap_or_default(),
            None => String::new(),
        })
    }

    /// Status of opening the picker for the input with that id (0 = opened).
    fn open_picker(&self, id: &str, mode: PickerMode) -> KuiStatus {
        match self.doc.get_element_by_id(id) {
            Ok(Some(element)) => status(&element.open_input_picker(mode)),
            _ => KUI_ERR_NOT_FOUND,
        }
    }

    fn check_picker_date(&self) {
        let picker = self.doc.input_picker();
        let state = self.picker_state();
        let target = self.picker_target();
        self.t.check(
            "picker.initial_state",
            !state.open && state.enabled && target.as_deref() == Ok(""),
            format!(
                "open {} enabled {} target {target:?}",
                state.open, state.enabled
            ),
        );

        let opened = self.open_picker("in-date", PickerMode::None);
        let state = self.picker_state();
        let target = self.picker_target();
        let panel = matches!(self.doc.query_selector("kui-picker"), Ok(Some(_)));
        self.t.check(
            "picker.open_date",
            opened == 0
                && state.open
                && state.mode == PickerMode::Date
                && state.visible_year == 2025
                && state.visible_month == 3
                && panel,
            format!(
                "status {opened} open {} mode {} page {}-{} panel {panel}",
                state.open,
                mode_name(state.mode),
                state.visible_year,
                state.visible_month
            ),
        );
        self.t.check(
            "picker.get_target",
            target.as_deref() == Ok("in-date"),
            format!("target {target:?}"),
        );

        let _ = picker.set_month(2026, 7);
        let state = self.picker_state();
        let bad = picker.set_month(2026, 13);
        self.t.check(
            "picker.set_month",
            state.visible_year == 2026
                && state.visible_month == 7
                && status(&bad) == KUI_ERR_INVALID_ARG,
            format!(
                "page {}-{}, month 13 {bad:?}",
                state.visible_year, state.visible_month
            ),
        );

        let _ = picker.close();
        let state = self.picker_state();
        let target = self.picker_target();
        let value = self.input_value("in-date");
        self.t.check(
            "picker.close",
            !state.open && target.as_deref() == Ok("") && value == "2025-03-21",
            format!("open {} target {target:?} value {value:?}", state.open),
        );
    }

    fn check_picker_time(&self) {
        let picker = self.doc.input_picker();
        let opened = self.open_picker("in-time", PickerMode::Time);
        let state = self.picker_state();
        self.t.check(
            "picker.open_time",
            opened == 0
                && state.open
                && state.mode == PickerMode::Time
                && state.hour == 14
                && state.minute == 30,
            format!(
                "status {opened} open {} mode {} time {}:{}",
                state.open,
                mode_name(state.mode),
                state.hour,
                state.minute
            ),
        );
        let _ = picker.set_time(9, 5);
        let after = self.picker_state();
        let bad = picker.set_time(24, 0);
        self.t.check(
            "picker.set_time",
            after.hour == 9 && after.minute == 5 && status(&bad) == KUI_ERR_INVALID_ARG,
            format!("time {}:{}, 24:00 {bad:?}", after.hour, after.minute),
        );
        let _ = picker.commit();
        let state = self.picker_state();
        let value = self.input_value("in-time");
        self.t.check(
            "picker.commit_time",
            !state.open && value == "09:05",
            format!("open {} value {value:?}", state.open),
        );
        self.t.check(
            "picker.commit_time-events",
            self.time_inputs.get() > 0 && self.time_changes.get() > 0,
            format!(
                "input {} change {}",
                self.time_inputs.get(),
                self.time_changes.get()
            ),
        );
    }

    fn check_picker_datetime(&self) {
        let picker = self.doc.input_picker();
        let opened = self.open_picker("in-dtl", PickerMode::None);
        let state = self.picker_state();
        self.t.check(
            "picker.open_datetime_local",
            opened == 0
                && state.mode == PickerMode::DateTimeLocal
                && state.visible_year == 2025
                && state.visible_month == 3,
            format!(
                "status {opened} mode {} page {}-{}",
                mode_name(state.mode),
                state.visible_year,
                state.visible_month
            ),
        );
        self.t.known_bug(
            "picker.open_datetime_local-time",
            state.hour == 14 && state.minute == 30,
            "the datetime-local panel ignores the input's time and opens at the current time",
            format!(
                "time {}:{}, value 2025-03-21T14:30",
                state.hour, state.minute
            ),
        );
        let _ = picker.set_month(2026, 7);
        let _ = picker.set_time(8, 15);
        let _ = picker.commit();
        let value = self.input_value("in-dtl");
        self.t.check(
            "picker.commit_datetime_local",
            value == "2026-07-21T08:15",
            format!("value {value:?}"),
        );
    }

    fn check_picker_color(&self) {
        let picker = self.doc.input_picker();
        let opened = self.open_picker("in-color", PickerMode::Color);
        let s = self.picker_state();
        // #3b82f6 = rgb(59,130,246): h 217.2, s 0.760, v 0.965.
        self.t.check(
            "picker.open_color",
            opened == 0
                && s.mode == PickerMode::Color
                && near(s.color_h.into(), 217.2, 1.0)
                && near(s.color_s.into(), 0.760, 0.01)
                && near(s.color_v.into(), 0.965, 0.01),
            format!(
                "status {opened} mode {} hsv {:.1} {:.3} {:.3}",
                mode_name(s.mode),
                s.color_h,
                s.color_s,
                s.color_v
            ),
        );
        let _ = picker.set_color_hsv(0.0, 1.0, 1.0);
        let a = self.picker_state();
        let bad = picker.set_color_hsv(400.0, 1.0, 1.0);
        self.t.check(
            "picker.set_color_hsv",
            near(a.color_h.into(), 0.0, 0.01)
                && near(a.color_s.into(), 1.0, 0.01)
                && near(a.color_v.into(), 1.0, 0.01)
                && status(&bad) == KUI_ERR_INVALID_ARG,
            format!(
                "hsv {:.1} {:.2} {:.2}, h 400 {bad:?}",
                a.color_h, a.color_s, a.color_v
            ),
        );
        let _ = picker.commit();
        let value = self.input_value("in-color");
        self.t.check(
            "picker.commit_color",
            value == "#ff0000",
            format!("value {value:?}"),
        );
    }

    fn check_picker_refusals(&self) {
        let picker = self.doc.input_picker();
        let text = self.open_picker("in-text", PickerMode::None);
        let div = self.open_picker("not-input", PickerMode::None);
        self.t.check(
            "picker.no_panel",
            text == KUI_ERR_NOT_SUPPORTED && div == KUI_ERR_INVALID_ARG,
            format!("text input status {text}, div status {div}"),
        );
        let _ = picker.set_enabled(false);
        let off = self.picker_state();
        let disabled = self.open_picker("in-date", PickerMode::None);
        let _ = picker.set_enabled(true);
        let on = self.picker_state();
        self.t.check(
            "picker.set_enabled",
            !off.enabled && disabled == KUI_ERR_NOT_SUPPORTED && !off.open && on.enabled,
            format!(
                "disabled {} open {} (status {disabled}), re-enabled {}",
                !off.enabled, off.open, on.enabled
            ),
        );
    }

    // ---- Panel buttons ------------------------------------------------------------------

    fn report_toasts(&self, what: &str) {
        self.set_text("toast-out", &format!("{what} count={}", self.toast_count()));
    }

    fn report_tooltip(&self) {
        let pop = self.doc.popups();
        let bg = pop.tooltip_theme().map_or(0, |t| t.background);
        let delay = pop.tooltip_delay().unwrap_or(0.0);
        let name = if self.purple_on.get() {
            "purple"
        } else {
            "default"
        };
        self.set_text(
            "tip-out",
            &format!(
                "theme={name} bg=#{:06x} delay={delay:.0} tooltip={}",
                bg >> 8,
                self.has_type(PopupType::Tooltip)
            ),
        );
    }

    fn show_tip(&self) {
        let anchor = self.rect_of("tip-anchor");
        let tip =
            self.doc
                .popups()
                .show_tooltip("Tooltip from the host", anchor, PopupPosition::Bottom);
        self.tip_id.set(tip.unwrap_or(0));
        self.report_tooltip();
    }

    /// A click anywhere dismisses open popups first, so each popup-layer button shows both
    /// kinds again and then applies its own removal; the report is "before -> after".
    fn popup_counts(&self) -> String {
        format!(
            "popups={} tooltip={} menu={}",
            self.any_popup(),
            self.has_type(PopupType::Tooltip),
            self.has_type(PopupType::ContextMenu)
        )
    }

    fn popup_step(&self, remove: impl FnOnce(&Host)) {
        self.show_both();
        let before = self.popup_counts();
        remove(self);
        self.set_text("pop-out", &format!("{before} -> {}", self.popup_counts()));
    }

    fn report_picker(&self, what: &str) {
        let s = self.picker_state();
        let target = self.picker_target().unwrap_or_default();
        let target = if target.is_empty() {
            "-".to_string()
        } else {
            target
        };
        self.set_text(
            "pick-out",
            &format!(
                "{what}open={} enabled={} mode={} page={:04}-{:02} target={target}",
                i32::from(s.open),
                i32::from(s.enabled),
                mode_name(s.mode),
                s.visible_year,
                s.visible_month
            ),
        );
    }

    fn report_values(&self) {
        let text = format!(
            "time={} dtl={} color={}",
            self.input_value("in-time"),
            self.input_value("in-dtl"),
            self.input_value("in-color")
        );
        self.set_text("pick-values", &text);
    }

    fn pick_and_commit(&self, id: &str, mode: PickerMode, set: impl FnOnce(&kui::InputPicker<'_>)) {
        if self.open_picker(id, mode) == 0 {
            let picker = self.doc.input_picker();
            set(&picker);
            let _ = picker.commit();
        }
        self.report_picker("");
        self.report_values();
    }

    fn button(self: &Rc<Self>, id: &str) {
        let pop = self.doc.popups();
        match id {
            "btn-toast-show" => {
                let serial = self.toast_serial.get() + 1;
                self.toast_serial.set(serial);
                let level = [
                    ToastLevel::Info,
                    ToastLevel::Success,
                    ToastLevel::Warning,
                    ToastLevel::Error,
                ][(serial % 4) as usize];
                self.last_toast
                    .set(self.show_toast(&format!("Toast #{serial} from the host"), level));
                self.report_toasts(&format!("shown #{serial}"));
            }
            "btn-toast-dismiss" => {
                let _ = self.doc.dismiss_toast(self.last_toast.get());
                self.report_toasts("dismissed last");
            }
            "btn-toast-clear" => {
                let _ = self.doc.dismiss_all_toasts();
                self.report_toasts("dismissed all");
            }
            "btn-toast-count" => self.report_toasts("now"),
            "btn-tip-show" => self.show_tip(),
            "btn-tip-hide" => {
                let _ = pop.dismiss(self.tip_id.get());
                self.report_tooltip();
            }
            "btn-tip-theme" => {
                self.purple_on.set(!self.purple_on.get());
                let theme = if self.purple_on.get() {
                    PURPLE
                } else {
                    self.default_theme.get()
                };
                let _ = pop.set_tooltip_theme(theme);
                self.show_tip(); // the click closed the tooltip: show it again in the new colours
            }
            "btn-tip-delay" => {
                let delays = [500.0, 150.0, 1000.0];
                self.delay_step.set((self.delay_step.get() + 1) % 3);
                let _ = pop.set_tooltip_delay(delays[self.delay_step.get()]);
                self.report_tooltip();
            }
            "btn-menu-show" => {
                self.menu_id.set(self.show_menu().unwrap_or(0));
                self.set_text(
                    "menu-out",
                    &format!("menu open={}", self.has_type(PopupType::ContextMenu)),
                );
            }
            "btn-menu-default" => {
                let enabled = pop.default_context_menu_enabled().unwrap_or(false);
                let _ = pop.set_default_context_menu_enabled(!enabled);
                let enabled = pop.default_context_menu_enabled().unwrap_or(false);
                self.set_text(
                    "menu-out",
                    if enabled { "default=on" } else { "default=off" },
                );
            }
            "btn-pop-both" => {
                self.show_both();
                self.set_text("pop-out", &self.popup_counts());
            }
            "btn-pop-tip-off" => self.popup_step(|h| {
                let _ = h.doc.popups().dismiss_type(PopupType::Tooltip);
            }),
            "btn-pop-menu-id" => self.popup_step(|h| {
                let _ = h.doc.popups().dismiss(h.menu_id.get());
            }),
            "btn-pop-clear" => self.popup_step(|h| {
                let _ = h.doc.popups().dismiss_all();
            }),
            "btn-pick-date" => {
                let opened = self.open_picker("in-date", PickerMode::None);
                self.report_picker(&format!("status={} ", status_name(opened)));
            }
            "btn-pick-close" => {
                let _ = self.doc.input_picker().close();
                self.report_picker("");
            }
            "btn-pick-time" => self.pick_and_commit("in-time", PickerMode::Time, |p| {
                let _ = p.set_time(18, 45);
            }),
            "btn-pick-dtl" => self.pick_and_commit("in-dtl", PickerMode::DateTimeLocal, |p| {
                let _ = p.set_month(2027, 1);
                let _ = p.set_time(22, 10);
            }),
            "btn-pick-color" => self.pick_and_commit("in-color", PickerMode::Color, |p| {
                let _ = p.set_color_hsv(120.0, 1.0, 1.0);
            }),
            "btn-pick-toggle" => {
                let picker = self.doc.input_picker();
                let _ = picker.set_enabled(!self.picker_state().enabled);
                let opened = self.open_picker("in-date", PickerMode::None);
                if opened == 0 {
                    let _ = picker.close();
                }
                self.report_picker(&format!("open(date) status={} ", status_name(opened)));
            }
            _ => {}
        }
    }

    /// Right-click: the built-in menu when it is enabled, the host's own menu otherwise.
    fn on_context_menu(self: &Rc<Self>, event: &kui::Event<'_>) {
        let enabled = self
            .doc
            .popups()
            .default_context_menu_enabled()
            .unwrap_or(false);
        self.context_own.set(!enabled);
        if !enabled {
            let _ = event.prevent_default();
            self.menu_id.set(self.show_menu().unwrap_or(0));
        }
        let h = self.clone();
        let _ = self.platform.set_timeout(100, move || {
            let kind = if h.context_own.get() {
                "custom"
            } else {
                "default"
            };
            h.set_text(
                "menu-out",
                &format!(
                    "contextmenu: {kind} menu open={}",
                    h.has_type(PopupType::ContextMenu)
                ),
            );
        });
    }
}

/// Counts drops: the menu callback holds one, so the count rises when the menu releases it.
struct ReleaseCounter(Rc<Cell<u32>>);

impl Drop for ReleaseCounter {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

const BUTTONS: [&str; 21] = [
    "btn-toast-show",
    "btn-toast-dismiss",
    "btn-toast-clear",
    "btn-toast-count",
    "btn-tip-show",
    "btn-tip-hide",
    "btn-tip-theme",
    "btn-tip-delay",
    "btn-menu-show",
    "btn-menu-default",
    "btn-pop-both",
    "btn-pop-tip-off",
    "btn-pop-menu-id",
    "btn-pop-clear",
    "btn-pick-date",
    "btn-pick-close",
    "btn-pick-time",
    "btn-pick-dtl",
    "btn-pick-color",
    "btn-pick-toggle",
    "menu-spot",
];

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let h = Rc::new(Host {
        doc: ctx.document.clone(),
        platform: ctx.platform.clone(),
        t: Tour::new(ctx.document),
        toast_base: Cell::new(0),
        last_toast: Cell::new(0),
        toast_serial: Cell::new(0),
        default_theme: Cell::new(PURPLE),
        purple_on: Cell::new(false),
        delay_step: Cell::new(0),
        tip_id: Cell::new(0),
        menu_id: Cell::new(0),
        menu_released: Rc::new(Cell::new(0)),
        context_own: Cell::new(false),
        time_changes: Cell::new(0),
        time_inputs: Cell::new(0),
    });

    let time = ctx
        .document
        .get_element_by_id("in-time")?
        .ok_or("page has no #in-time")?;
    let counter = h.clone();
    time.add_event_listener("change", move |_| {
        counter.time_changes.set(counter.time_changes.get() + 1)
    })?;
    let counter = h.clone();
    time.add_event_listener("input", move |_| {
        counter.time_inputs.set(counter.time_inputs.get() + 1)
    })?;

    h.check_toasts();
    h.check_tooltip();
    h.check_menu();
    h.check_popup_layer();
    h.check_picker_date();
    h.check_picker_time();
    h.check_picker_datetime();
    h.check_picker_color();
    h.check_picker_refusals();
    h.report_values();
    // Toast exit animations take a few hundred milliseconds.
    let last = h.clone();
    ctx.platform.set_timeout(1200, move || {
        let count = last.toast_count();
        let base = last.toast_base.get();
        last.t.check(
            "toast.count_after_dismiss",
            count == base,
            format!("count {count} (base {base})"),
        );
        last.t.finish();
    })?;

    for id in BUTTONS {
        let element = ctx
            .document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?;
        let handler = h.clone();
        if id == "menu-spot" {
            element
                .add_event_listener("contextmenu", move |event| handler.on_context_menu(event))?;
        } else {
            element.add_event_listener("click", move |_| handler.button(id))?;
        }
    }
    Ok(())
}
