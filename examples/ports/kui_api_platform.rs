// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_platform.rs; edit that file, not this copy.
//! API tour: platform services (examples/api_tours/platform). The host reads the monotonic clock,
//! runs timeouts / intervals / idle callbacks / posted tasks (also from another thread), pumps the
//! event loop by hand (poll_events, wait_events), round-trips the clipboard (putting the original
//! text back), queries the monitors and toggles F12 DevTools. One #tour-log row per check;
//! #tour-result ends as "PASS n/n" once the timers have fired. Message box, file dialogs and
//! open_url are only behind (grey) buttons: they are modal or open the system browser.

use crate::canonical_runner::Context;
use kui::{
    Document, FileDialogOptions, MessageBoxButtons, MessageBoxResult, MessageBoxType, MonitorInfo,
    Platform, TimerId, Window,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check: a documented behaviour; a failure makes the tour FAIL.
// skip:  not possible in this environment; logged, not counted.

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
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
        if result == "fail" {
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

    fn skip(&self, name: &str, why: &str) {
        self.row(name, "skip", &format!("skipped: {why}"));
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

// ---- Host state -----------------------------------------------------------------------------

struct Host {
    doc: Document,
    win: Rc<Window>,
    platform: Rc<Platform>,
    timeout_elapsed: Cell<f64>,
    interval_ticks: Cell<u32>,
    interval_id: Cell<Option<TimerId>>,
    cancelled_fired: Cell<u32>,
    idle_ran: Cell<u32>,
    button_ticks: Cell<u32>,
    button_interval: Cell<Option<TimerId>>,
}

/// Posted tasks must be `Send`; they count into this shared counter.
static POSTED: AtomicU32 = AtomicU32::new(0);

impl Host {
    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(element)) = self.doc.get_element_by_id(id) {
            let _ = element.set_text_content(text);
        }
    }

    fn now(&self) -> f64 {
        self.platform.now().unwrap_or(0.0)
    }
}

fn post_count() -> u32 {
    POSTED.load(Ordering::SeqCst)
}

fn post_increment() -> kui::Result<()> {
    kui::post_to_main_thread(|| {
        POSTED.fetch_add(1, Ordering::SeqCst);
    })
}

// ---- Clock, timers, idle callbacks ------------------------------------------------------

fn check_clock_and_timers(t: &Tour, h: &Rc<Host>) {
    let first = h.platform.now();
    let second = h.now();
    t.check(
        "time.now",
        matches!(first, Ok(v) if v > 0.0 && second >= v),
        format!("{first:?} then {second:.3}"),
    );

    let started = h.now();
    let host = h.clone();
    let timer = h
        .platform
        .set_timeout(50, move || host.timeout_elapsed.set(host.now() - started));
    t.check(
        "time.set_timeout",
        matches!(timer, Ok(id) if id.raw() != 0),
        format!("{timer:?}"),
    );
    // Clears itself on the third tick.
    let host = h.clone();
    let interval = h.platform.set_interval(30, move || {
        host.interval_ticks.set(host.interval_ticks.get() + 1);
        if host.interval_ticks.get() == 3 {
            if let Some(id) = host.interval_id.get() {
                let _ = host.platform.clear_timer(id);
            }
        }
    });
    t.check(
        "time.set_interval",
        matches!((&interval, &timer), (Ok(i), Ok(o)) if i.raw() != 0 && i != o),
        format!("{interval:?}"),
    );
    h.interval_id.set(interval.ok());
    let host = h.clone();
    let cleared = match h.platform.set_timeout(40, move || {
        host.cancelled_fired.set(host.cancelled_fired.get() + 1)
    }) {
        Ok(id) => h.platform.clear_timer(id),
        Err(error) => Err(error),
    };
    t.check("time.clear_timer", cleared.is_ok(), format!("{cleared:?}"));
    let host = h.clone();
    let idle = h
        .platform
        .request_idle(move || host.idle_ran.set(host.idle_ran.get() + 1));
    t.check("time.request_idle", idle.is_ok(), format!("{idle:?}"));
}

// ---- Pumping the loop by hand: a posted task runs inside poll_events / wait_events -------

fn check_event_loop(t: &Tour, h: &Host) {
    let before = post_count();
    let posted = post_increment();
    t.check(
        "loop.post_to_main_thread-async",
        posted.is_ok() && post_count() == before,
        format!(
            "{posted:?}, ran {} time(s) before the loop",
            post_count() - before
        ),
    );
    let polled = h.platform.poll_events();
    t.check(
        "loop.poll_events-runs-posted-task",
        polled.is_ok() && post_count() == before + 1,
        format!(
            "{polled:?}, posted task ran {} time(s)",
            post_count() - before
        ),
    );

    let started = h.now();
    let waited = h.platform.wait_events(0.05);
    let elapsed = h.now() - started;
    t.check(
        "loop.wait_events-timeout",
        waited.is_ok() && elapsed < 1000.0,
        format!("{waited:?} returned after {elapsed:.1} ms"),
    );
    // A task posted from another thread wakes a waiting loop at once and runs on this thread.
    let before = post_count();
    let ui_thread = std::thread::current().id();
    let ran_on_ui = Arc::new(AtomicU32::new(0));
    let flag = ran_on_ui.clone();
    let _ = std::thread::spawn(move || {
        let _ = kui::post_to_main_thread(move || {
            if std::thread::current().id() == ui_thread {
                flag.store(1, Ordering::SeqCst);
            }
            POSTED.fetch_add(1, Ordering::SeqCst);
        });
    })
    .join();
    let started = h.now();
    let waited = h.platform.wait_events(2.0);
    let elapsed = h.now() - started;
    if post_count() == before {
        let _ = h.platform.poll_events();
    }
    t.check(
        "loop.wait_events-woken-by-post",
        waited.is_ok()
            && post_count() == before + 1
            && ran_on_ui.load(Ordering::SeqCst) == 1
            && elapsed < 1500.0,
        format!(
            "{waited:?}, ran {} time(s) (on the UI thread: {}), waited {elapsed:.1} ms of 2000",
            post_count() - before,
            ran_on_ui.load(Ordering::SeqCst)
        ),
    );
}

// ---- Clipboard round trip (original text restored) ---------------------------------------

const SAMPLE: &str = "KUI clipboard \u{4f60}\u{597d} 42";

/// Saves, writes the sample, reads it back and restores; `None` tour means a button press.
/// Returns the summary and whether the clipboard held text (otherwise it is left alone).
fn clipboard_round_trip(platform: &Platform, t: Option<&Tour>) -> (String, bool) {
    if !platform.has_clipboard_text().unwrap_or(false) {
        return ("clipboard holds no text: left alone".into(), false);
    }
    let original = match platform.clipboard_text() {
        Ok(text) => text,
        Err(error) => return (format!("could not read the clipboard: {error}"), true),
    };
    let set = platform.set_clipboard_text(SAMPLE);
    let has_after = platform.has_clipboard_text().unwrap_or(false);
    let read_back = platform.clipboard_text().unwrap_or_default();
    let ok = set.is_ok() && has_after && read_back == SAMPLE;
    if let Some(t) = t {
        t.check(
            "clip.set_clipboard_text",
            set.is_ok() && has_after,
            format!("{set:?} has {has_after}"),
        );
        t.check(
            "clip.get_clipboard_text",
            read_back == SAMPLE,
            format!("read back {read_back:?}"),
        );
    }
    let _ = platform.set_clipboard_text(&original);
    let restored = platform.clipboard_text().unwrap_or_default();
    if let Some(t) = t {
        t.check(
            "clip.restored",
            restored == original,
            format!(
                "original of {} bytes, now {} bytes",
                original.len(),
                restored.len()
            ),
        );
    }
    (
        format!(
            "round trip {} (sample of {} bytes), original text ({} bytes) restored={}",
            if ok { "ok" } else { "FAILED" },
            SAMPLE.len(),
            original.len(),
            restored == original
        ),
        true,
    )
}

fn check_clipboard(t: &Tour, h: &Host) {
    let has = h.platform.has_clipboard_text();
    t.check("clip.has_clipboard_text", has.is_ok(), format!("{has:?}"));
    if !clipboard_round_trip(&h.platform, Some(t)).1 {
        t.skip(
            "clip.round-trip",
            "the clipboard holds no text now; it is not overwritten",
        );
    }
}

// ---- Monitors ---------------------------------------------------------------------------------

fn contains(m: &MonitorInfo, x: f32, y: f32) -> bool {
    x >= m.bounds.x
        && y >= m.bounds.y
        && x < m.bounds.x + m.bounds.width
        && y < m.bounds.y + m.bounds.height
}

// ---- Diagnostics: console, stack trace writer, crash reporter, input method ---------------------------

fn check_diagnostics(t: &Tour) {
    let console = kui::console_set_enabled(true)
        .and_then(|_| kui::console_set_stderr_output(false))
        .and_then(|_| kui::console_set_min_level(kui::LogLevel::Debug));
    t.check(
        "diag.console_configure",
        console.is_ok(),
        format!("{console:?}"),
    );

    let stack_config = kui::StackTraceConfig {
        output_mode: 2,
        ..Default::default()
    };
    let installed = kui::stack_trace_install(&stack_config);
    let uninstalled = kui::stack_trace_uninstall();
    t.check(
        "diag.stack_trace_install_uninstall",
        installed.is_ok() && uninstalled.is_ok(),
        format!("install {installed:?}, uninstall {uninstalled:?}"),
    );

    // Without crashpad_handler next to the executable the call succeeds and reports false.
    let crash = kui::crash_reporter_install(&kui::CrashReporterConfig::default());
    let crash_uninstalled = kui::crash_reporter_uninstall();
    t.check(
        "diag.crash_reporter_install_uninstall",
        crash.is_ok() && crash_uninstalled.is_ok(),
        format!("install {crash:?}, uninstall {crash_uninstalled:?}"),
    );

    // Windows turns the input method off for this thread; other platforms report NOT_SUPPORTED.
    let ime = kui::disable_ime().map_or_else(|e| e.status() as i32, |_| 0);
    t.check(
        "diag.disable_ime",
        ime == 0 || ime == kui::sys::KUI_ERR_NOT_SUPPORTED as i32,
        format!("status {ime}"),
    );
}

fn check_monitors(t: &Tour, h: &Host) {
    let count = h.platform.monitor_count();
    t.check(
        "mon.get_monitor_count",
        matches!(count, Ok(n) if n >= 1),
        format!("{count:?}"),
    );
    let count = count.unwrap_or(0);
    let (mut primaries, mut valid, mut listed_primary) = (0, true, None);
    for index in 0..count {
        match h.platform.monitor(index) {
            Ok(m) => {
                if m.bounds.width <= 0.0
                    || m.bounds.height <= 0.0
                    || m.work_area.width <= 0.0
                    || m.work_area.width > m.bounds.width
                    || m.scale <= 0.0
                {
                    valid = false;
                }
                if m.primary {
                    primaries += 1;
                    listed_primary = Some(m.bounds);
                }
            }
            Err(_) => valid = false,
        }
    }
    t.check(
        "mon.get_monitor",
        valid && primaries == 1,
        format!("valid {valid}, {primaries} primary monitor(s)"),
    );
    let beyond = h.platform.monitor(count);
    t.check(
        "mon.get_monitor-out-of-range",
        beyond.is_err(),
        format!("index {count} -> {beyond:?}"),
    );

    let primary = h.platform.primary_monitor();
    t.check(
        "mon.get_primary_monitor",
        matches!(&primary, Ok(m) if m.primary && Some(m.bounds) == listed_primary),
        format!("{primary:?}"),
    );
    let primary = primary.map(|m| m.bounds).unwrap_or_default();
    let (cx, cy) = (
        primary.x + primary.width / 2.0,
        primary.y + primary.height / 2.0,
    );
    let from_point = h.platform.monitor_from_point(cx, cy);
    t.check(
        "mon.get_monitor_from_point",
        matches!(&from_point, Ok(m) if m.bounds == primary && m.primary),
        format!("{from_point:?}"),
    );

    let (wx, wy) = h.win.position().unwrap_or_default();
    let (ww, wh) = h.win.size().unwrap_or_default();
    let (centre_x, centre_y) = (wx as f32 + ww as f32 / 2.0, wy as f32 + wh as f32 / 2.0);
    let from_window = h.platform.monitor_from_window(&h.win);
    t.check(
        "mon.get_monitor_from_window",
        matches!(&from_window, Ok(m) if contains(m, centre_x, centre_y)),
        format!("window centre ({centre_x:.0}, {centre_y:.0}), {from_window:?}"),
    );
}

fn check_f12(t: &Tour, h: &Host) {
    let on = h.platform.set_f12_devtools_enabled(true);
    let off = h.platform.set_f12_devtools_enabled(false);
    t.check(
        "shell.set_f12_devtools_enabled",
        on.is_ok() && off.is_ok(),
        format!("on {on:?} off {off:?}"),
    );
}

// ---- Verdict once the timers have had their turn ----------------------------------------

fn finalize(t: &Tour, h: &Host) {
    let elapsed = h.timeout_elapsed.get();
    t.check(
        "time.set_timeout-fired",
        elapsed >= 45.0,
        format!("fired after {elapsed:.1} ms (-1 = never)"),
    );
    let ticks = h.interval_ticks.get();
    t.check(
        "time.set_interval-ticks",
        ticks == 3,
        format!("{ticks} tick(s), cleared on the third"),
    );
    let cancelled = h.cancelled_fired.get();
    t.check(
        "time.clear_timer-cancelled",
        cancelled == 0,
        format!("cleared timeout ran {cancelled} time(s)"),
    );
    let idle = h.idle_ran.get();
    t.check(
        "time.request_idle-ran",
        idle == 1,
        format!("idle callback ran {idle} time(s)"),
    );
    t.finish();
}

// ---- Panel buttons -----------------------------------------------------------------------

fn describe(m: &MonitorInfo) -> String {
    let mut text = format!(
        "{:.0}x{:.0} at ({:.0}, {:.0}) work {:.0}x{:.0} scale {:.2}",
        m.bounds.width,
        m.bounds.height,
        m.bounds.x,
        m.bounds.y,
        m.work_area.width,
        m.work_area.height,
        m.scale
    );
    if !m.name.is_empty() {
        text = format!("{}: {text}", m.name);
    }
    if m.primary {
        text.push_str(" primary");
    }
    text
}

fn dialog_options(title: &str, default_path: &str) -> FileDialogOptions {
    FileDialogOptions {
        title: title.into(),
        default_path: default_path.into(),
        filters: vec!["HTML pages|*.html;*.htm".into(), "All files|*.*".into()],
        allow_multiple: false,
    }
}

fn show_path(h: &Host, what: &str, result: kui::Result<Option<String>>) {
    let text = match result {
        Ok(Some(path)) => format!("{what}: status 0 {path}"),
        Ok(None) => format!("{what}: cancelled"),
        Err(error) => format!("{what}: {error}"),
    };
    h.set_text("shell-out", &text);
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
    on(ctx, host, "btn-timeout", |h| {
        let started = h.now();
        let later = h.clone();
        let _ = h.platform.set_timeout(100, move || {
            let sign = if later.now() - started >= 95.0 {
                ">= "
            } else {
                "< "
            };
            later.set_text("time-out", &format!("timeout fired after {sign}100 ms"));
        });
        h.set_text("time-out", "waiting 100 ms...");
    })?;
    on(ctx, host, "btn-interval", |h| {
        h.button_ticks.set(0);
        let later = h.clone();
        let id = h.platform.set_interval(50, move || {
            later.button_ticks.set(later.button_ticks.get() + 1);
            if later.button_ticks.get() == 3 {
                if let Some(id) = later.button_interval.get() {
                    let _ = later.platform.clear_timer(id);
                }
            }
            later.set_text(
                "time-out",
                &format!("interval tick {} of 3", later.button_ticks.get()),
            );
        });
        h.button_interval.set(id.ok());
    })?;
    on(ctx, host, "btn-cancel", |h| {
        let later = h.clone();
        let status = match h.platform.set_timeout(30, move || {
            later.set_text("time-out", "the cleared timer fired (bug)")
        }) {
            Ok(id) => h
                .platform
                .clear_timer(id)
                .map_or_else(|e| e.status() as i32, |_| 0),
            Err(error) => error.status() as i32,
        };
        h.set_text(
            "time-out",
            &format!("timer set and cleared: status {status}"),
        );
    })?;
    on(ctx, host, "btn-idle", |h| {
        let later = h.clone();
        let _ = h
            .platform
            .request_idle(move || later.set_text("time-out", "idle callback ran"));
    })?;
    on(ctx, host, "btn-post", |h| {
        // The task itself must be Send: it only bumps a counter; a timer then shows the result.
        let before = post_count();
        let _ = std::thread::spawn(|| post_increment()).join();
        let later = h.clone();
        let _ = h.platform.set_timeout(30, move || {
            if post_count() > before {
                later.set_text("time-out", "posted task ran on the UI thread");
            }
        });
    })?;
    on(ctx, host, "btn-poll", |h| {
        let before = post_count();
        let _ = post_increment();
        let _ = h.platform.poll_events();
        let answer = if post_count() == before + 1 {
            "yes"
        } else {
            "no"
        };
        h.set_text(
            "loop-out",
            &format!("posted task ran inside poll_events: {answer}"),
        );
    })?;
    on(ctx, host, "btn-wait", |h| {
        let started = h.now();
        let waited = h.platform.wait_events(0.05);
        let status = waited.map_or_else(|e| e.status() as i32, |_| 0);
        let answer = if h.now() - started < 1000.0 {
            "yes"
        } else {
            "no"
        };
        h.set_text(
            "loop-out",
            &format!("wait_events(0.05): status {status}, returned within 1 s: {answer}"),
        );
    })?;
    on(ctx, host, "btn-clip-read", |h| {
        let has = h.platform.has_clipboard_text().unwrap_or(false);
        let size = if has {
            h.platform.clipboard_text().map_or(0, |t| t.len())
        } else {
            0
        };
        h.set_text(
            "clip-out",
            &format!("has text={} ({size} bytes)", has as i32),
        );
    })?;
    on(ctx, host, "btn-clip-trip", |h| {
        let (summary, _) = clipboard_round_trip(&h.platform, None);
        h.set_text("clip-out", &summary);
    })?;
    on(ctx, host, "btn-mon-list", |h| {
        let monitors = h.platform.monitors().unwrap_or_default();
        let mut text = format!("{} monitor(s)", monitors.len());
        for m in &monitors {
            text.push_str(" | ");
            text.push_str(&describe(m));
        }
        h.set_text("mon-out", &text);
    })?;
    on(ctx, host, "btn-mon-window", |h| {
        match h.platform.monitor_from_window(&h.win) {
            Ok(m) => h.set_text("mon-out", &format!("this window: {}", describe(&m))),
            Err(_) => h.set_text("mon-out", "monitor lookup failed"),
        }
    })?;
    // Manual: modal.
    on(ctx, host, "btn-msg", |h| {
        let result = h.platform.message_box(
            "KUI API tour",
            "Do you like message boxes?",
            MessageBoxType::Question,
            MessageBoxButtons::YesNo,
        );
        let answer = if matches!(result, Ok(MessageBoxResult::Yes)) {
            "yes"
        } else {
            "no"
        };
        h.set_text("shell-out", &format!("message box answer: {answer}"));
    })?;
    // Manual: the three file dialogs are modal.
    on(ctx, host, "btn-open", |h| {
        show_path(
            h,
            "open",
            h.platform
                .open_file_dialog(&dialog_options("Open a page", "")),
        )
    })?;
    on(ctx, host, "btn-save", |h| {
        show_path(
            h,
            "save",
            h.platform
                .save_file_dialog(&dialog_options("Save the page as", "tour.html")),
        )
    })?;
    on(ctx, host, "btn-folder", |h| {
        show_path(
            h,
            "folder",
            h.platform.select_folder_dialog("Pick a folder"),
        )
    })?;
    // Manual: opens the system browser.
    on(ctx, host, "btn-url", |h| {
        let status = h
            .platform
            .open_url("https://example.com/")
            .map_or_else(|e| e.status() as i32, |_| 0);
        h.set_text("shell-out", &format!("open_url: status {status}"));
    })?;
    on(ctx, host, "btn-f12-on", |h| {
        if h.platform.set_f12_devtools_enabled(true).is_ok() {
            h.set_text(
                "shell-out",
                "F12 enabled: F12 opens DevTools in the browser",
            );
        }
    })?;
    on(ctx, host, "btn-f12-off", |h| {
        if h.platform.set_f12_devtools_enabled(false).is_ok() {
            h.set_text("shell-out", "F12 disabled");
        }
    })?;
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let host = Rc::new(Host {
        doc: ctx.document.clone(),
        win: ctx.window.clone(),
        platform: ctx.platform.clone(),
        timeout_elapsed: Cell::new(-1.0),
        interval_ticks: Cell::new(0),
        interval_id: Cell::new(None),
        cancelled_fired: Cell::new(0),
        idle_ran: Cell::new(0),
        button_ticks: Cell::new(0),
        button_interval: Cell::new(None),
    });
    let tour = Rc::new(Tour {
        doc: ctx.document.clone(),
        total: Cell::new(0),
        failed: Cell::new(0),
        failed_names: RefCell::new(Vec::new()),
    });
    check_clock_and_timers(&tour, &host);
    check_event_loop(&tour, &host);
    check_clipboard(&tour, &host);
    check_monitors(&tour, &host);
    check_f12(&tour, &host);
    check_diagnostics(&tour);
    let (t, h) = (tour.clone(), host.clone());
    ctx.platform.set_timeout(700, move || finalize(&t, &h))?;
    bind_buttons(ctx, &host)
}
