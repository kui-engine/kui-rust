// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_window.rs; edit that file, not this copy.
//! API tour: the native window (examples/api_tours/window). The host changes title, size and
//! position, listens for resizes (callback and "resize" event listeners), cycles maximize and
//! fullscreen, shapes the frame, sets opacity / z-order / cursor / IME position, installs a
//! native menu bar, binds keyboard shortcuts, drives a hidden tray icon with simulated clicks,
//! reads the DevTools state and saves a screenshot -- then checks the window is back exactly as
//! it was. One #tour-log row per check; #tour-result ends as "PASS n/n". Minimize, stop DevTools
//! and the tray context menu are only behind (grey) buttons: they would stop rendering, cut the
//! inspector's connection or open a system menu.

use crate::canonical_runner::Context;
use kui::{
    Cursor, Document, FrameRadii, FrameState, InjectedKey, Menu, MenuItem, Modifiers, Platform,
    ScriptEngine, ScriptValue, TrayEvent, Window,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

const VK_F7: u32 = 0x76;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug).
// skip:      not available on this platform; logged, not counted.

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
        if result == "fail" || result == "known" {
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

    fn skip(&self, name: &str, why: &str) {
        self.row(name, "skip", &format!("skipped: {why}"));
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

// ---- Host state -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Geometry {
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

struct Host {
    win: Rc<Window>,
    doc: Document,
    script: Rc<ScriptEngine>,
    platform: Rc<Platform>,
    orig_title: String,
    orig: Geometry,
    orig_frame: FrameState,
    resize_calls: Cell<u32>,
    resized: Cell<(u32, u32)>,
    events_a: Cell<u32>,
    events_b: Cell<u32>,
    f7_first: Cell<u32>,
    f7_hits: Cell<u32>,
    ctrl_k_hits: Cell<u32>,
    button_hits: Cell<u32>,
    tray_events: RefCell<Vec<TrayEvent>>,
    menu_checked: Cell<bool>,
    menu_enabled: Cell<bool>,
    title_toggled: Cell<bool>,
}

fn flag(value: kui::Result<bool>) -> i32 {
    match value {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

fn status_of<T>(result: &kui::Result<T>) -> i32 {
    match result {
        Ok(_) => 0,
        Err(error) => error.status() as i32,
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SendMessageW(hwnd: *mut std::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
}

impl Host {
    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(element)) = self.doc.get_element_by_id(id) {
            let _ = element.set_text_content(text);
        }
    }

    fn geometry(&self) -> Geometry {
        let (width, height) = self.win.size().unwrap_or_default();
        let (x, y) = self.win.position().unwrap_or_default();
        Geometry {
            width,
            height,
            x,
            y,
        }
    }

    fn title(&self) -> String {
        self.win.title().unwrap_or_default()
    }

    fn lua_number(&self, code: &str) -> f64 {
        match self.script.eval_value(code, "tour") {
            Ok(out) if out.success => match out.value {
                ScriptValue::String(text) => text.parse().unwrap_or(-1.0),
                _ => -1.0,
            },
            _ => -1.0,
        }
    }

    /// A real key-down / key-up for the native window: the path a keyboard takes, where window
    /// shortcuts are matched. Windows only; false elsewhere.
    fn send_native_key(&self, virtual_key: u32) -> bool {
        #[cfg(windows)]
        {
            const WM_KEYDOWN: u32 = 0x0100;
            const WM_KEYUP: u32 = 0x0101;
            let Ok(handle) = self.win.native_handle() else {
                return false;
            };
            if handle.is_null() {
                return false;
            }
            unsafe {
                SendMessageW(handle, WM_KEYDOWN, virtual_key as usize, 1);
                SendMessageW(
                    handle,
                    WM_KEYUP,
                    virtual_key as usize,
                    0xC000_0001u32 as i32 as isize,
                );
            }
            true
        }
        #[cfg(not(windows))]
        {
            let _ = virtual_key;
            false
        }
    }

    fn ime_caret(&self) -> (f32, f32) {
        let rect = self
            .doc
            .get_element_by_id("ime-input")
            .ok()
            .flatten()
            .and_then(|e| e.bounding_rect().ok())
            .unwrap_or_default();
        (rect.x + 4.0, rect.y + rect.height)
    }

    fn show_resize(&self) {
        let (w, h) = self.resized.get();
        self.set_text(
            "resize-out",
            &format!("on_resize {w}x{h} (callbacks={})", self.resize_calls.get()),
        );
    }
}

// ---- Title, size, position, resize notifications ------------------------------------------

fn check_geometry(t: &Tour, h: &Rc<Host>) {
    const HOST_TITLE: &str = "kui_api_window: title set by the host";
    t.check(
        "win.get_title",
        h.orig_title == "KUI API \u{2014} Window",
        format!("title {:?}", h.orig_title),
    );
    let set = h.win.set_title(HOST_TITLE);
    t.check(
        "win.set_title",
        set.is_ok() && h.title() == HOST_TITLE,
        format!("{set:?} title {:?}", h.title()),
    );
    let _ = h.win.set_title(&h.orig_title);
    t.check(
        "win.set_title-restore",
        h.title() == h.orig_title,
        format!("title {:?}", h.title()),
    );

    let ratio = h.win.device_pixel_ratio();
    let lua_ratio = h.lua_number("return tostring(window.devicePixelRatio())");
    t.check(
        "win.get_device_pixel_ratio",
        matches!(ratio, Ok(r) if r > 0.0 && near(r, lua_ratio as f32)),
        format!("{ratio:?}, Lua window.devicePixelRatio() {lua_ratio:.3}"),
    );
    // Size is the client area in device pixels; the page sees it in CSS px.
    let ratio = ratio.unwrap_or(1.0) as f64;
    let inner = h.lua_number("return tostring(window.innerWidth())");
    t.check(
        "win.get_size",
        h.orig.width > 0 && h.orig.height > 0 && (inner * ratio - h.orig.width as f64).abs() < 1.5,
        format!(
            "size {}x{}, innerWidth {inner:.1} x ratio {ratio:.2}",
            h.orig.width, h.orig.height
        ),
    );
    let handle = h.win.native_handle();
    t.check(
        "win.get_native_handle",
        matches!(handle, Ok(p) if !p.is_null()),
        format!("{handle:?}"),
    );

    // Two "resize" listeners and the resize callback, then remove them one by one.
    let host = h.clone();
    let _ = h.win.on_resize(move |w, hgt| {
        host.resize_calls.set(host.resize_calls.get() + 1);
        host.resized.set((w, hgt));
        host.show_resize();
    });
    let host = h.clone();
    let listener_a = h.win.add_event_listener("resize", false, move |_| {
        host.events_a.set(host.events_a.get() + 1)
    });
    let host = h.clone();
    let _ = h.win.add_event_listener("resize", false, move |_| {
        host.events_b.set(host.events_b.get() + 1)
    });
    let (w0, h0) = (h.orig.width, h.orig.height);
    let resized = h.win.resize(w0 - 60, h0 - 40);
    let g = h.geometry();
    t.check(
        "win.resize",
        resized.is_ok() && g.width == w0 - 60 && g.height == h0 - 40,
        format!(
            "{resized:?} size {}x{}, wanted {}x{}",
            g.width,
            g.height,
            w0 - 60,
            h0 - 40
        ),
    );
    let (rw, rh) = h.resized.get();
    t.check(
        "win.on_resize",
        h.resize_calls.get() > 0 && rw as i32 == g.width && rh as i32 == g.height,
        format!("{} callback(s), last {rw}x{rh}", h.resize_calls.get()),
    );
    t.check(
        "win.add_event_listener-resize",
        listener_a.is_ok() && h.events_a.get() > 0 && h.events_b.get() > 0,
        format!("listener a {}, b {}", h.events_a.get(), h.events_b.get()),
    );

    let (a, b) = (h.events_a.get(), h.events_b.get());
    let removed = match &listener_a {
        Ok(id) => h.win.remove_event_listener("resize", *id),
        Err(_) => Ok(()),
    };
    let _ = h.win.resize(w0 - 30, h0 - 20);
    t.check(
        "win.remove_event_listener",
        removed.is_ok() && h.events_a.get() == a && h.events_b.get() > b,
        format!(
            "{removed:?}, removed listener {a} -> {}, kept listener {b} -> {}",
            h.events_a.get(),
            h.events_b.get()
        ),
    );
    let (a, b, calls) = (h.events_a.get(), h.events_b.get(), h.resize_calls.get());
    let removed = h.win.remove_event_listeners("resize");
    let _ = h.win.resize(w0, h0);
    let g = h.geometry();
    t.check(
        "win.remove_event_listeners",
        removed.is_ok()
            && h.events_a.get() == a
            && h.events_b.get() == b
            && h.resize_calls.get() > calls,
        format!(
            "{removed:?}, listeners {a}/{b} -> {}/{}, callback {calls} -> {}",
            h.events_a.get(),
            h.events_b.get(),
            h.resize_calls.get()
        ),
    );
    t.check(
        "win.resize-back",
        g.width == w0 && g.height == h0,
        format!("size {}x{}", g.width, g.height),
    );

    let moved = h.win.set_position(h.orig.x + 24, h.orig.y + 16);
    let g = h.geometry();
    t.check(
        "win.get_position",
        moved.is_ok() && g.x == h.orig.x + 24 && g.y == h.orig.y + 16,
        format!(
            "{moved:?} moved to ({}, {}), wanted ({}, {})",
            g.x,
            g.y,
            h.orig.x + 24,
            h.orig.y + 16
        ),
    );
    let _ = h.win.set_position(h.orig.x, h.orig.y);
}

// ---- Maximize / restore, fullscreen, focus, close flag ------------------------------------

fn check_state(t: &Tour, h: &Host) {
    let minimized = flag(h.win.is_minimized());
    t.check(
        "state.is_minimized",
        minimized == 0,
        format!("is_minimized {minimized}"),
    );
    let closing = flag(h.win.should_close());
    t.check(
        "state.should_close",
        closing == 0,
        format!("should_close {closing}"),
    );
    let focused = h.win.has_focus();
    t.check("state.has_focus", focused.is_ok(), format!("{focused:?}"));

    let maximized = flag(h.win.is_maximized());
    t.check(
        "state.is_maximized-initial",
        maximized == 0,
        format!("is_maximized {maximized}"),
    );
    let done = h.win.maximize();
    let (g, maximized) = (h.geometry(), flag(h.win.is_maximized()));
    t.check(
        "state.maximize",
        done.is_ok() && maximized == 1 && g.width >= h.orig.width,
        format!(
            "{done:?} is_maximized {maximized} size {}x{}",
            g.width, g.height
        ),
    );
    let done = h.win.restore();
    let (g, maximized) = (h.geometry(), flag(h.win.is_maximized()));
    t.check(
        "state.restore",
        done.is_ok() && maximized == 0 && g.width == h.orig.width && g.height == h.orig.height,
        format!(
            "{done:?} is_maximized {maximized} size {}x{}",
            g.width, g.height
        ),
    );

    let full = flag(h.win.is_fullscreen());
    t.check(
        "state.is_fullscreen-initial",
        full == 0,
        format!("is_fullscreen {full}"),
    );
    let done = h.win.set_fullscreen(true);
    let (g, full) = (h.geometry(), flag(h.win.is_fullscreen()));
    t.check(
        "state.set_fullscreen",
        done.is_ok() && full == 1 && g.width >= h.orig.width,
        format!(
            "{done:?} is_fullscreen {full} size {}x{}",
            g.width, g.height
        ),
    );
    let done = h.win.set_fullscreen(false);
    let (g, full) = (h.geometry(), flag(h.win.is_fullscreen()));
    t.check(
        "state.set_fullscreen-exit",
        done.is_ok() && full == 0 && g == h.orig,
        format!(
            "{done:?} is_fullscreen {full} geometry {g:?}, was {:?}",
            h.orig
        ),
    );
}

// ---- Frame shape: radii, frameless, shadow ----------------------------------------------

fn radii(tl: f32, tr: f32, br: f32, bl: f32) -> FrameRadii {
    FrameRadii {
        top_left: tl,
        top_right: tr,
        bottom_right: br,
        bottom_left: bl,
    }
}

// ---- Host integration: one frame now, enable / disable, render DPR, base path --------------------------

fn check_host(t: &Tour, h: &Host) {
    let frame = h.win.process_frame();
    t.check("host.process_frame", frame.is_ok(), format!("{frame:?}"));

    // Enabling / disabling input is Windows only; other platforms report NOT_SUPPORTED.
    let not_supported = kui::sys::KUI_ERR_NOT_SUPPORTED as i32;
    let was_enabled = h.win.is_enabled();
    if matches!(&was_enabled, Err(e) if e.status() as i32 == not_supported) {
        let set = status_of(&h.win.set_enabled(false));
        t.check(
            "host.set_enabled",
            set == not_supported,
            format!("status {set}"),
        );
    } else {
        let _ = h.win.set_enabled(false);
        let disabled = h.win.is_enabled().unwrap_or(true);
        let _ = h.win.set_enabled(true);
        let now = h.win.is_enabled().unwrap_or(false);
        t.check(
            "host.set_enabled",
            matches!(was_enabled, Ok(true)) && !disabled && now,
            format!("was {was_enabled:?}, disabled {disabled}, now {now}"),
        );
    }

    // A window created hidden (no flash), its first frame rendered while it is still hidden, then shown.
    let created = kui::Window::new(&kui::WindowProps {
        title: "kui-hidden".into(),
        width: 320,
        height: 200,
        frameless: true,
        sizeable: true,
        hidden: true,
        frame_radius: Some(8.0),
        ..kui::WindowProps::default()
    });
    let (mut hidden, mut shown) = (false, false);
    if let Ok(extra) = &created {
        hidden = !extra.is_visible().unwrap_or(true);
        let _ = extra.process_frame();
        let _ = extra.show();
        shown = extra.is_visible().unwrap_or(false);
    }
    t.check(
        "host.create_hidden_window",
        created.is_ok() && hidden && shown,
        format!(
            "created {}, hidden at creation {hidden}, visible after show {shown}",
            created.is_ok()
        ),
    );

    // Setting the render ratio to the value it already has leaves the page as it is.
    let ratio = h.win.device_pixel_ratio().unwrap_or(1.0);
    let set_ratio = h.doc.set_render_device_pixel_ratio(ratio);
    t.check(
        "host.set_render_device_pixel_ratio",
        set_ratio.is_ok(),
        format!("{set_ratio:?}, ratio {ratio:.3}"),
    );

    let original = h.doc.base_path().unwrap_or_default();
    let _ = h.doc.set_base_path("assets/");
    let changed = h.doc.base_path().unwrap_or_default();
    let _ = h.doc.set_base_path(&original);
    let restored = h.doc.base_path().unwrap_or_default();
    t.check(
        "host.set_get_base_path",
        changed == "assets/" && restored == original,
        format!("changed '{changed}', restored '{restored}'"),
    );
}

fn check_frame(t: &Tour, h: &Host) {
    let none = h.win.frame_radii();
    t.check(
        "frame.get_frame_radii-none",
        matches!(none, Ok(None)),
        format!("{none:?}"),
    );
    let _ = h.win.set_frame_radius(12.0);
    let r = h.win.frame_radii();
    t.check(
        "frame.set_frame_radius",
        matches!(r, Ok(Some(v)) if v == radii(12.0, 12.0, 12.0, 12.0)),
        format!("{r:?}"),
    );
    let want = radii(16.0, 4.0, 16.0, 4.0);
    let _ = h.win.set_frame_radii(want);
    let r = h.win.frame_radii();
    t.check(
        "frame.set_frame_radii",
        matches!(r, Ok(Some(v)) if v == want),
        format!("{r:?}"),
    );
    // A negative radius is not a valid shape: the call changes nothing.
    let _ = h.win.set_frame_radii(radii(-1.0, 4.0, 4.0, 4.0));
    let r = h.win.frame_radii();
    t.check(
        "frame.set_frame_radii-negative-ignored",
        matches!(r, Ok(Some(v)) if v == want),
        format!("{r:?}"),
    );
    let cleared = h.win.clear_frame_radius();
    let r = h.win.frame_radii();
    t.check(
        "frame.clear_frame_radius",
        cleared.is_ok() && matches!(r, Ok(None)),
        format!("{cleared:?} {r:?}"),
    );

    let done = h.win.set_frameless(true);
    let state = h.win.frame_state().unwrap_or_default();
    let g = h.geometry();
    t.check(
        "frame.set_frameless",
        done.is_ok() && state.frameless && g.width == h.orig.width && g.height == h.orig.height,
        format!("{done:?} state {state:?} client {}x{}", g.width, g.height),
    );
    let done = h.win.set_frameless_shadow(true);
    let state = h.win.frame_state().unwrap_or_default();
    t.check(
        "frame.set_frameless_shadow",
        done.is_ok() && state.shadow,
        format!("{done:?} state {state:?}"),
    );
    let _ = h.win.set_frameless_shadow(h.orig_frame.shadow);
    let _ = h.win.set_frameless(h.orig_frame.frameless);
    let state = h.win.frame_state().unwrap_or_default();
    let g = h.geometry();
    t.check(
        "frame.get_frame_state-restored",
        state == h.orig_frame && g.width == h.orig.width && g.height == h.orig.height,
        format!(
            "state {state:?} (was {:?}) client {}x{}",
            h.orig_frame, g.width, g.height
        ),
    );
}

// ---- Opacity, z-order, cursor, IME candidate position ----------------------------------

fn check_look(t: &Tour, h: &Host) {
    let set = h.win.set_opacity(0.85);
    let low = h.win.set_opacity(-0.1);
    let high = h.win.set_opacity(1.5);
    let invalid = kui::sys::KUI_ERR_INVALID_ARG as i32;
    t.check(
        "look.set_opacity",
        set.is_ok()
            && status_of(&low) == invalid
            && status_of(&high) == invalid
            && h.win.set_opacity(1.0).is_ok(),
        format!("0.85 -> {set:?}, -0.1 -> {low:?}, 1.5 -> {high:?}"),
    );
    let top = h.win.set_always_on_top(true);
    t.check(
        "look.set_always_on_top",
        top.is_ok() && h.win.set_always_on_top(false).is_ok(),
        format!("{top:?}"),
    );
    let cursor = h.win.set_cursor(Cursor::Crosshair);
    t.check(
        "look.set_cursor",
        cursor.is_ok() && h.win.set_cursor(Cursor::Default).is_ok(),
        format!("{cursor:?}"),
    );
    let hidden = h.win.set_cursor_visible(false);
    t.check(
        "look.set_cursor_visible",
        hidden.is_ok() && h.win.set_cursor_visible(true).is_ok(),
        format!("{hidden:?}"),
    );
    let (x, y) = h.ime_caret();
    let ime = h.win.set_ime_candidate_pos(x, y);
    t.check(
        "look.set_ime_candidate_pos",
        ime.is_ok(),
        format!("{ime:?} at ({x:.0}, {y:.0})"),
    );
}

// ---- Native menu bar -------------------------------------------------------------------------

const MENU_WRAP: u32 = 901;
const MENU_RELOAD: u32 = 902;
const MENU_ZOOM_IN: u32 = 903;
const MENU_ZOOM_OUT: u32 = 904;

fn item(text: &str, id: u32) -> MenuItem {
    MenuItem {
        text: text.into(),
        id,
        ..MenuItem::default()
    }
}

fn install_menu(h: &Rc<Host>) -> kui::Result<()> {
    h.menu_checked.set(true);
    h.menu_enabled.set(true);
    let menu = Menu {
        text: "&View".into(),
        items: vec![
            MenuItem {
                checkable: true,
                checked: true,
                ..item("&Word wrap", MENU_WRAP)
            },
            MenuItem {
                separator: true,
                ..MenuItem::default()
            },
            item("&Reload", MENU_RELOAD),
            MenuItem {
                children: vec![
                    MenuItem {
                        shortcut: "Ctrl++".into(),
                        ..item("Zoom &in", MENU_ZOOM_IN)
                    },
                    item("Zoom &out", MENU_ZOOM_OUT),
                ],
                ..item("&Zoom", 0)
            },
        ],
    };
    let host = h.clone();
    h.win.set_menu_bar(
        &[menu],
        Some(move |id: u32| host.set_text("menu-out", &format!("menu item {id} activated"))),
    )
}

fn check_menu(t: &Tour, h: &Rc<Host>) {
    let not_found = kui::sys::KUI_ERR_NOT_FOUND as i32;
    let early = h.win.set_menu_item_checked(MENU_WRAP, false);
    t.check(
        "menu.set_checked-no-menu",
        status_of(&early) == not_found,
        format!("{early:?}"),
    );
    let installed = install_menu(h);
    let g = h.geometry();
    t.check(
        "menu.set_menu_bar",
        installed.is_ok(),
        format!("{installed:?}"),
    );
    if cfg!(windows) {
        // A native menu bar takes its height from the client area.
        t.check(
            "menu.set_menu_bar-client-area",
            g.width == h.orig.width && g.height < h.orig.height,
            format!(
                "client {}x{}, was {}x{}",
                g.width, g.height, h.orig.width, h.orig.height
            ),
        );
    } else {
        t.skip(
            "menu.set_menu_bar-client-area",
            "the menu bar only takes client height on Windows",
        );
    }
    let top = h.win.set_menu_item_checked(MENU_WRAP, false);
    let nested = h.win.set_menu_item_checked(MENU_ZOOM_OUT, true);
    let missing = h.win.set_menu_item_checked(999, true);
    t.check(
        "menu.menu_set_checked",
        top.is_ok() && nested.is_ok() && status_of(&missing) == not_found,
        format!("top-level {top:?}, submenu {nested:?}, unknown id {missing:?}"),
    );
    let disabled = h.win.set_menu_item_enabled(MENU_RELOAD, false);
    let missing = h.win.set_menu_item_enabled(999, false);
    t.check(
        "menu.menu_set_enabled",
        disabled.is_ok()
            && status_of(&missing) == not_found
            && h.win.set_menu_item_enabled(MENU_RELOAD, true).is_ok(),
        format!("{disabled:?}, unknown id {missing:?}"),
    );
    let cleared = h.win.clear_menu_bar();
    let missing = h.win.set_menu_item_checked(MENU_WRAP, true);
    let g = h.geometry();
    t.check(
        "menu.clear_menu_bar",
        cleared.is_ok()
            && status_of(&missing) == not_found
            && g.width == h.orig.width
            && g.height == h.orig.height,
        format!(
            "{cleared:?}, set_checked afterwards {missing:?}, client {}x{}",
            g.width, g.height
        ),
    );
}

// ---- Keyboard shortcuts --------------------------------------------------------------------

fn check_shortcuts(t: &Tour, h: &Rc<Host>) {
    let host = h.clone();
    let f7 = h
        .win
        .bind_shortcut("F7", move || host.f7_first.set(host.f7_first.get() + 1));
    let host = h.clone();
    let ctrl_k = h.win.bind_shortcut("Ctrl+Shift+K", move || {
        host.ctrl_k_hits.set(host.ctrl_k_hits.get() + 1)
    });
    let f7_id = *f7.as_ref().unwrap_or(&0);
    t.check(
        "sc.bind_shortcut",
        f7_id != 0 && matches!(ctrl_k, Ok(id) if id != 0 && id != f7_id),
        format!("{f7:?} / {ctrl_k:?}"),
    );
    let host = h.clone();
    let again = h
        .win
        .bind_shortcut("F7", move || host.f7_hits.set(host.f7_hits.get() + 1));
    t.check(
        "sc.bind_shortcut-rebind-same-id",
        matches!(again, Ok(id) if id == f7_id),
        format!("{again:?}, first id {f7_id}"),
    );
    let invalid = h.win.bind_shortcut("Ctrl+", || {});
    let empty = h.win.bind_shortcut("", || {});
    let bad = kui::sys::KUI_ERR_INVALID_ARG as i32;
    t.check(
        "sc.bind_shortcut-invalid",
        status_of(&invalid) == bad && status_of(&empty) == bad,
        format!("'Ctrl+' -> {invalid:?}, '' -> {empty:?}"),
    );

    // Synthetic DevTools input goes to the page's event dispatcher.
    let none = Modifiers::default();
    let _ = h
        .doc
        .dispatch_key_event(InjectedKey::Down, "F7", "F7", none, "", false);
    let _ = h
        .doc
        .dispatch_key_event(InjectedKey::Up, "F7", "F7", none, "", false);
    t.known_bug(
        "sc.devtools-key-reaches-shortcut",
        h.f7_hits.get() == 1,
        "devtools_dispatch_key_event bypasses the window ShortcutManager (only the native key-down path calls tryDispatch), so injected keys never trigger bound shortcuts",
        format!("F7 callback ran {} time(s)", h.f7_hits.get()),
    );

    let mut hits = h.f7_hits.get();
    if h.send_native_key(VK_F7) {
        t.check(
            "sc.native-key-fires",
            h.f7_hits.get() == hits + 1 && h.f7_first.get() == 0,
            format!(
                "replacing callback {hits} -> {}, replaced callback {}",
                h.f7_hits.get(),
                h.f7_first.get()
            ),
        );
        hits = h.f7_hits.get();
        let off = h.win.set_shortcuts_enabled(false);
        h.send_native_key(VK_F7);
        t.check(
            "sc.set_shortcuts_enabled-off",
            off.is_ok() && h.f7_hits.get() == hits,
            format!("{off:?} hits {hits} -> {}", h.f7_hits.get()),
        );
        let on = h.win.set_shortcuts_enabled(true);
        h.send_native_key(VK_F7);
        t.check(
            "sc.set_shortcuts_enabled-on",
            on.is_ok() && h.f7_hits.get() == hits + 1,
            format!("{on:?} hits {hits} -> {}", h.f7_hits.get()),
        );
        hits = h.f7_hits.get();
        let unbound = h.win.unbind_shortcut(f7_id);
        h.send_native_key(VK_F7);
        t.check(
            "sc.unbind_shortcut",
            unbound.is_ok() && h.f7_hits.get() == hits,
            format!("{unbound:?} hits {hits} -> {}", h.f7_hits.get()),
        );
        let host = h.clone();
        let _ = h
            .win
            .bind_shortcut("F7", move || host.f7_hits.set(host.f7_hits.get() + 1));
        let all = h.win.unbind_all_shortcuts();
        h.send_native_key(VK_F7);
        t.check(
            "sc.unbind_all_shortcuts",
            all.is_ok() && h.f7_hits.get() == hits,
            format!("{all:?} hits {hits} -> {}", h.f7_hits.get()),
        );
    } else {
        t.skip(
            "sc.native-key-fires",
            "no native key injection on this platform",
        );
        let off = h.win.set_shortcuts_enabled(false);
        t.check(
            "sc.set_shortcuts_enabled",
            off.is_ok() && h.win.set_shortcuts_enabled(true).is_ok(),
            format!("{off:?}"),
        );
        let unbound = h.win.unbind_shortcut(f7_id);
        t.check(
            "sc.unbind_shortcut",
            unbound.is_ok(),
            format!("{unbound:?}"),
        );
        let all = h.win.unbind_all_shortcuts();
        t.check("sc.unbind_all_shortcuts", all.is_ok(), format!("{all:?}"));
    }
    t.check(
        "sc.ctrl-shift-k-untouched",
        h.ctrl_k_hits.get() == 0,
        format!("Ctrl+Shift+K ran {} time(s)", h.ctrl_k_hits.get()),
    );
}

// ---- Tray icon (created hidden; simulated clicks) -------------------------------------

fn tray_name(event: TrayEvent) -> &'static str {
    match event {
        TrayEvent::Click => "click",
        TrayEvent::DoubleClick => "double click",
        TrayEvent::RightClick => "right click",
        TrayEvent::BalloonClick => "balloon click",
    }
}

/// A tray icon with the application icon; false when the platform has none.
fn ensure_tray(h: &Rc<Host>, visible: bool) -> bool {
    if !h.win.tray_active().unwrap_or(false)
        && h.win.create_tray("", "KUI API tour", visible).is_err()
    {
        return false;
    }
    if visible {
        let _ = h.win.set_tray_visible(true);
    }
    let host = h.clone();
    let _ = h.win.on_tray_event(move |event| {
        host.tray_events.borrow_mut().push(event);
        let count = host.tray_events.borrow().len();
        host.set_text(
            "tray-out",
            &format!("tray callback: {} (events={count})", tray_name(event)),
        );
    });
    true
}

fn check_tray(t: &Tour, h: &Rc<Host>) {
    let (active, visible) = (h.win.tray_active(), h.win.tray_visible());
    t.check(
        "tray.is_active-before-create",
        matches!(active, Ok(false)) && matches!(visible, Ok(false)),
        format!("active {active:?} visible {visible:?}"),
    );
    if !ensure_tray(h, false) {
        t.skip("tray.simulate_event", "this platform has no tray icon");
        return;
    }
    let (active, visible) = (h.win.tray_active(), h.win.tray_visible());
    t.check(
        "tray.tray_is_active-hidden",
        matches!(active, Ok(true)) && matches!(visible, Ok(false)),
        format!("active {active:?} visible {visible:?}"),
    );
    h.tray_events.borrow_mut().clear();
    let _ = h.win.simulate_tray_event(TrayEvent::Click);
    let _ = h.win.simulate_tray_event(TrayEvent::DoubleClick);
    let _ = h.win.simulate_tray_event(TrayEvent::RightClick);
    let last = h.win.simulate_tray_event(TrayEvent::BalloonClick);
    let events = h.tray_events.borrow().clone();
    t.check(
        "tray.tray_simulate_event",
        last.is_ok()
            && events
                == [
                    TrayEvent::Click,
                    TrayEvent::DoubleClick,
                    TrayEvent::RightClick,
                    TrayEvent::BalloonClick,
                ],
        format!("{last:?} events {events:?}"),
    );
    let icon = h.win.set_tray_icon("");
    t.check(
        "tray.tray_set_icon",
        icon.is_ok() && h.win.tray_active().unwrap_or(false),
        format!("{icon:?}"),
    );
    let _ = h.win.destroy_tray();
    let active = h.win.tray_active();
    t.check(
        "tray.destroy",
        matches!(active, Ok(false)),
        format!("active {active:?}"),
    );
}

// ---- DevTools state and screenshot -----------------------------------------------------

/// Size of a PNG file, 0 when missing or not a PNG.
fn png_size(path: &std::path::Path) -> usize {
    match std::fs::read(path) {
        Ok(data) if data.starts_with(b"\x89PNG\r\n\x1a\n") => data.len(),
        _ => 0,
    }
}

fn check_devtools_screenshot(t: &Tour, h: &Host) {
    let port = h.win.devtools_port().ok().flatten().unwrap_or(0);
    let state = h.win.devtools_state();
    t.check(
        "dt.get_devtools_state",
        matches!(state, Ok((true, p, _)) if p == port && port != 0),
        format!("{state:?} (devtools_port {port})"),
    );
    let path = std::env::temp_dir().join("kui_api_window_shot_rust.png");
    let _ = std::fs::remove_file(&path);
    let saved = h.win.save_screenshot(&path.to_string_lossy());
    let size = png_size(&path);
    t.check(
        "dt.save_screenshot",
        matches!(saved, Ok(true)) && size > 100,
        format!("{saved:?}, {path:?} holds {size} PNG bytes"),
    );
    let _ = std::fs::remove_file(&path);
    let unwritable = std::env::temp_dir()
        .join("kui-api-window-no-such-dir")
        .join("shot.png");
    let saved = h.win.save_screenshot(&unwritable.to_string_lossy());
    t.check(
        "dt.save_screenshot-unwritable",
        matches!(saved, Ok(false)),
        format!("{saved:?}"),
    );
}

// ---- Everything back as it was ---------------------------------------------------------

fn check_restored(t: &Tour, h: &Host) {
    let g = h.geometry();
    let state = h.win.frame_state().unwrap_or_default();
    let radius = h.win.frame_radii();
    let (maximized, full) = (flag(h.win.is_maximized()), flag(h.win.is_fullscreen()));
    t.check(
        "win.state-restored",
        h.title() == h.orig_title && g == h.orig && maximized == 0 && full == 0 && state == h.orig_frame
            && matches!(radius, Ok(None)),
        format!(
            "title {:?} geometry {g:?} (was {:?}) maximized {maximized} fullscreen {full} frame {state:?} radius {radius:?}",
            h.title(),
            h.orig
        ),
    );
}

// ---- Panel buttons -----------------------------------------------------------------------

fn show_geometry(h: &Host) {
    let g = h.geometry();
    let ratio = h.win.device_pixel_ratio().unwrap_or(0.0);
    let handle = if h.win.native_handle().map_or(false, |p| !p.is_null()) {
        "yes"
    } else {
        "no"
    };
    h.set_text(
        "geom-out",
        &format!(
            "size={}x{} pos=({}, {}) dpr={ratio:.2} handle={handle} title={}",
            g.width,
            g.height,
            g.x,
            g.y,
            h.title()
        ),
    );
}

fn show_state(h: &Host, prefix: &str) {
    let focused = flag(h.win.has_focus());
    h.set_text(
        "state-out",
        &format!(
            "{prefix}maximized={} minimized={} fullscreen={} focused={focused} should_close={}",
            flag(h.win.is_maximized()),
            flag(h.win.is_minimized()),
            flag(h.win.is_fullscreen()),
            flag(h.win.should_close())
        ),
    );
}

fn show_frame(h: &Host) {
    let state = h.win.frame_state().unwrap_or_default();
    let radii = match h.win.frame_radii() {
        Ok(Some(r)) => format!(
            "{:.0}/{:.0}/{:.0}/{:.0}",
            r.top_left, r.top_right, r.bottom_right, r.bottom_left
        ),
        _ => "none".to_string(),
    };
    h.set_text(
        "frame-out",
        &format!(
            "radii={radii} frameless={} shadow={} sizeable={}",
            state.frameless as i32, state.shadow as i32, state.sizeable as i32
        ),
    );
}

fn report(h: &Host, id: &str, result: kui::Result<()>, success: &str, failure: &str) {
    match result {
        Ok(()) => h.set_text(id, success),
        Err(error) => h.set_text(id, &format!("{failure}: {error}")),
    }
}

fn on<F>(
    ctx: &Context<'_>,
    host: &Rc<Host>,
    element_id: &str,
    mut handler: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Rc<Host>) + 'static,
{
    let h = host.clone();
    let element = ctx
        .document
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener("click", move |_event| handler(&h))?;
    Ok(())
}

fn bind_buttons(ctx: &Context<'_>, host: &Rc<Host>) -> Result<(), Box<dyn Error>> {
    on(ctx, host, "btn-geom-read", |h| show_geometry(h))?;
    on(ctx, host, "btn-geom-shrink", |h| {
        let _ = h.win.resize(h.orig.width - 60, h.orig.height - 40);
        show_geometry(h);
    })?;
    on(ctx, host, "btn-geom-restore", |h| {
        let _ = h.win.resize(h.orig.width, h.orig.height);
        show_geometry(h);
    })?;
    on(ctx, host, "btn-title", |h| {
        h.title_toggled.set(!h.title_toggled.get());
        let title = if h.title_toggled.get() {
            "Title set from a button".to_string()
        } else {
            h.orig_title.clone()
        };
        let _ = h.win.set_title(&title);
        show_geometry(h);
    })?;
    on(ctx, host, "btn-state", |h| show_state(h, ""))?;
    on(ctx, host, "btn-max-cycle", |h| {
        let _ = h.win.maximize();
        let prefix = format!(
            "while maximized: {} | after restore: ",
            flag(h.win.is_maximized())
        );
        let _ = h.win.restore();
        show_state(h, &prefix);
    })?;
    on(ctx, host, "btn-fs-cycle", |h| {
        let _ = h.win.set_fullscreen(true);
        let prefix = format!(
            "while fullscreen: {} | after exit: ",
            flag(h.win.is_fullscreen())
        );
        let _ = h.win.set_fullscreen(false);
        show_state(h, &prefix);
    })?;
    // Manual: a minimized window stops rendering until it is restored from the taskbar.
    on(ctx, host, "btn-minimize", |h| {
        let _ = h.win.minimize();
    })?;
    on(ctx, host, "btn-radius", |h| {
        let _ = h.win.set_frame_radius(12.0);
        show_frame(h);
    })?;
    on(ctx, host, "btn-radii", |h| {
        let _ = h.win.set_frame_radii(radii(16.0, 4.0, 16.0, 4.0));
        show_frame(h);
    })?;
    on(ctx, host, "btn-radius-clear", |h| {
        let _ = h.win.clear_frame_radius();
        show_frame(h);
    })?;
    on(ctx, host, "btn-frameless", |h| {
        let frameless = h.win.frame_state().map_or(false, |s| s.frameless);
        let _ = h.win.set_frameless(!frameless);
        let _ = h.win.set_frameless_shadow(!frameless);
        show_frame(h);
    })?;
    on(ctx, host, "btn-opacity", |h| {
        report(
            h,
            "look-out",
            h.win.set_opacity(0.85),
            "opacity 0.85",
            "set_opacity failed",
        )
    })?;
    on(ctx, host, "btn-opaque", |h| {
        report(
            h,
            "look-out",
            h.win.set_opacity(1.0),
            "opacity 1.00",
            "set_opacity failed",
        )
    })?;
    on(ctx, host, "btn-top", |h| {
        report(
            h,
            "look-out",
            h.win.set_always_on_top(true),
            "always on top",
            "set_always_on_top failed",
        )
    })?;
    on(ctx, host, "btn-untop", |h| {
        report(
            h,
            "look-out",
            h.win.set_always_on_top(false),
            "normal z-order",
            "set_always_on_top failed",
        )
    })?;
    on(ctx, host, "btn-cursor", |h| {
        report(
            h,
            "look-out",
            h.win.set_cursor(Cursor::Wait),
            "cursor: wait (until the pointer moves to other content)",
            "set_cursor failed",
        )
    })?;
    on(ctx, host, "btn-cursor-hide", |h| {
        let _ = h.win.set_cursor_visible(false);
        h.set_text("look-out", "cursor hidden for 1 s");
        let later = h.clone();
        let _ = h.platform.set_timeout(1000, move || {
            let _ = later.win.set_cursor_visible(true);
            later.set_text("look-out", "cursor visible again");
        });
    })?;
    on(ctx, host, "btn-ime", |h| {
        if let Ok(Some(input)) = h.doc.get_element_by_id("ime-input") {
            let _ = input.focus();
        }
        let (x, y) = h.ime_caret();
        let result = h.win.set_ime_candidate_pos(x, y);
        h.set_text(
            "look-out",
            &format!("IME candidates at ({x:.0}, {y:.0}): {result:?}"),
        );
    })?;
    on(ctx, host, "btn-menu-set", |h| {
        report(
            h,
            "menu-out",
            install_menu(h),
            "menu installed: View > Word wrap (checked), Reload, Zoom",
            "set_menu_bar failed",
        )
    })?;
    on(ctx, host, "btn-menu-check", |h| {
        let result = h
            .win
            .set_menu_item_checked(MENU_WRAP, !h.menu_checked.get());
        if result.is_ok() {
            h.menu_checked.set(!h.menu_checked.get());
        }
        h.set_text(
            "menu-out",
            &format!(
                "Word wrap checked={} (status {})",
                h.menu_checked.get() as i32,
                status_of(&result)
            ),
        );
    })?;
    on(ctx, host, "btn-menu-enable", |h| {
        let result = h
            .win
            .set_menu_item_enabled(MENU_RELOAD, !h.menu_enabled.get());
        if result.is_ok() {
            h.menu_enabled.set(!h.menu_enabled.get());
        }
        h.set_text(
            "menu-out",
            &format!(
                "Reload enabled={} (status {})",
                h.menu_enabled.get() as i32,
                status_of(&result)
            ),
        );
    })?;
    on(ctx, host, "btn-menu-clear", |h| {
        report(
            h,
            "menu-out",
            h.win.clear_menu_bar(),
            "menu removed",
            "clear_menu_bar failed",
        )
    })?;
    on(ctx, host, "btn-sc-bind", |h| {
        h.button_hits.set(0);
        let host = h.clone();
        match h.win.bind_shortcut("F7", move || {
            host.button_hits.set(host.button_hits.get() + 1);
            host.set_text(
                "sc-out",
                &format!("F7 fired {} time(s)", host.button_hits.get()),
            );
        }) {
            Ok(id) => h.set_text("sc-out", &format!("F7 bound (id {id})")),
            Err(error) => h.set_text("sc-out", &format!("bind failed: {error}")),
        }
    })?;
    on(ctx, host, "btn-sc-press", |h| {
        let before = h.button_hits.get();
        if !h.send_native_key(VK_F7) {
            h.set_text(
                "sc-out",
                "no native key injection on this platform: press F7",
            );
        } else if h.button_hits.get() == before {
            h.set_text(
                "sc-out",
                &format!("F7 sent, no shortcut ran (fired {before} time(s))"),
            );
        }
    })?;
    on(ctx, host, "btn-sc-disable", |h| {
        let _ = h.win.set_shortcuts_enabled(false);
        h.set_text("sc-out", "shortcuts disabled");
    })?;
    on(ctx, host, "btn-sc-enable", |h| {
        let _ = h.win.set_shortcuts_enabled(true);
        h.set_text("sc-out", "shortcuts enabled");
    })?;
    on(ctx, host, "btn-sc-unbind", |h| {
        let _ = h.win.unbind_all_shortcuts();
        h.set_text("sc-out", "all shortcuts unbound");
    })?;
    on(ctx, host, "btn-tray-click", |h| {
        if !ensure_tray(h, false) {
            h.set_text("tray-out", "no tray on this platform");
            return;
        }
        let _ = h.win.simulate_tray_event(TrayEvent::Click);
    })?;
    on(ctx, host, "btn-tray-right", |h| {
        if !ensure_tray(h, false) {
            h.set_text("tray-out", "no tray on this platform");
            return;
        }
        let _ = h.win.simulate_tray_event(TrayEvent::RightClick);
    })?;
    // Manual: shows the tray icon and opens its context menu (a system popup).
    on(ctx, host, "btn-tray-popup", |h| {
        if !ensure_tray(h, true) {
            h.set_text("tray-out", "no tray on this platform");
            return;
        }
        let host = h.clone();
        let _ = h.win.set_tray_context_menu(
            &[item("Say hello", 1), item("Hide icon", 2)],
            Some(move |id: u32| host.set_text("tray-out", &format!("tray menu item {id}"))),
        );
        report(
            h,
            "tray-out",
            h.win.popup_tray_context_menu(),
            "context menu shown",
            "no context menu on this platform",
        );
    })?;
    on(ctx, host, "btn-shot", |h| {
        let path = std::env::temp_dir().join("kui_api_window_button_rust.png");
        let saved = h
            .win
            .save_screenshot(&path.to_string_lossy())
            .unwrap_or(false);
        let bytes = if png_size(&path) > 100 { ">100" } else { "0" };
        let _ = std::fs::remove_file(&path);
        h.set_text(
            "dt-out",
            &format!(
                "saved={} PNG bytes={bytes} ({}, removed again)",
                saved as i32,
                path.display()
            ),
        );
    })?;
    on(ctx, host, "btn-dt-state", |h| {
        let (running, port, clients) = h.win.devtools_state().unwrap_or((false, 0, 0));
        h.set_text(
            "dt-out",
            &format!(
                "devtools running={} port={port} clients={clients}",
                running as i32
            ),
        );
    })?;
    // Manual: stopping the server disconnects every inspector (and the automated check).
    on(ctx, host, "btn-dt-stop", |h| {
        let _ = h.win.stop_devtools();
        h.set_text("dt-out", "devtools stopped");
    })?;
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let win = ctx.window.clone();
    let (width, height) = win.size()?;
    let (x, y) = win.position()?;
    let host = Rc::new(Host {
        orig_title: win.title()?,
        orig: Geometry {
            width,
            height,
            x,
            y,
        },
        orig_frame: win.frame_state()?,
        win,
        doc: ctx.document.clone(),
        script: ctx.script.clone(),
        platform: ctx.platform.clone(),
        resize_calls: Cell::new(0),
        resized: Cell::new((0, 0)),
        events_a: Cell::new(0),
        events_b: Cell::new(0),
        f7_first: Cell::new(0),
        f7_hits: Cell::new(0),
        ctrl_k_hits: Cell::new(0),
        button_hits: Cell::new(0),
        tray_events: RefCell::new(Vec::new()),
        menu_checked: Cell::new(true),
        menu_enabled: Cell::new(true),
        title_toggled: Cell::new(false),
    });
    let tour = Tour::new(ctx.document);
    check_geometry(&tour, &host);
    check_state(&tour, &host);
    check_frame(&tour, &host);
    check_look(&tour, &host);
    check_menu(&tour, &host);
    check_shortcuts(&tour, &host);
    check_tray(&tour, &host);
    check_devtools_screenshot(&tour, &host);
    check_host(&tour, &host);
    check_restored(&tour, &host);
    tour.finish();
    bind_buttons(ctx, &host)
}
