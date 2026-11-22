// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_network_devtools.rs; edit that file, not this copy.
//! API tour: HTTP client and in-process DevTools (examples/api_tours/network_devtools). The host
//! fetches from the window's own DevTools server (a loopback HTTP server: /json/version) with
//! the sync, worker-thread and main-thread request methods, provokes transport errors (closed
//! port, malformed URL, cancel), runs raw CDP commands, injects mouse / key / IME input and reads
//! box models. No internet access is needed. One #tour-log row per check; #tour-result ends as
//! "PASS n/n" once every asynchronous answer is in. The panel buttons repeat the calls.

use crate::canonical_runner::Context;
use kui::{
    Document, HTTPClient, HTTPResponse, ImeEventKind, InjectedKey, InjectedMouse, Modifiers,
    MouseButton, MouseInjection, Point, Rect, Window,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

const CLOSED_URL: &str = "http://127.0.0.1:1/"; // nothing listens on port 1

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug).

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    known: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
    pending: Cell<u32>,     // asynchronous checks still waiting for their callback
    ready_done: Cell<bool>, // the synchronous part has run
    finished: Cell<bool>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            known: Cell::new(0),
            failed_names: RefCell::new(Vec::new()),
            pending: Cell::new(0),
            ready_done: Cell::new(false),
            finished: Cell::new(false),
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
        if self.finished.replace(true) {
            return;
        }
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

    /// One asynchronous check answered; the verdict follows the last one.
    fn settle(&self) {
        self.pending.set(self.pending.get().saturating_sub(1));
        if self.pending.get() == 0 && self.ready_done.get() {
            self.finish();
        }
    }
}

// ---- Host state ---------------------------------------------------------------------------

struct Host {
    doc: Document,
    client: HTTPClient,
    cancel_client: HTTPClient,
    version_url: String,
    port: u16,
    clicks: Cell<u32>,
    wheels: Cell<u32>,
    moved: Cell<(f32, f32)>,
    first_key: RefCell<(String, String)>,
    shift_seen: Cell<bool>,
}

// Worker-thread callbacks must be `Send`, so they cannot carry the tour; the main-thread task
// they post finds it here (only ever touched on the UI thread).
thread_local! {
    static STATE: RefCell<Option<(Rc<Tour>, Rc<Host>)>> = const { RefCell::new(None) };
}

fn with_state(f: impl FnOnce(&Tour, &Host)) {
    STATE.with(|state| {
        if let Some((tour, host)) = state.borrow().as_ref() {
            f(tour, host);
        }
    });
}

fn is_json_version(reply: &kui::Result<HTTPResponse>) -> bool {
    matches!(reply, Ok(r) if r.status == 200 && r.content_type.contains("application/json")
        && r.body.starts_with(b"{") && String::from_utf8_lossy(&r.body).contains("\"Browser\":\"KUI/")
        && r.error.is_empty())
}

fn summary(reply: &kui::Result<HTTPResponse>) -> String {
    match reply {
        Ok(r) => format!(
            "http {} type {:?} error {:?}",
            r.status, r.content_type, r.error
        ),
        Err(error) => format!("err {error}"),
    }
}

/// A transport failure: status 0 and an error text.
fn is_transport_error(reply: &kui::Result<HTTPResponse>) -> bool {
    matches!(reply, Ok(r) if r.status == 0 && !r.error.is_empty())
}

impl Host {
    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(element)) = self.doc.get_element_by_id(id) {
            let _ = element.set_text_content(text);
        }
    }

    fn show_reply(&self, prefix: &str, reply: &kui::Result<HTTPResponse>) {
        let (text, body) = match reply {
            Ok(r) => {
                let content_type = if r.content_type.is_empty() {
                    "-"
                } else {
                    r.content_type.as_str()
                };
                let mut text = format!(
                    "{prefix} status={} type={content_type} bytes={}",
                    r.status,
                    r.body.len()
                );
                if !r.error.is_empty() {
                    text.push_str(&format!(" error={}", r.error));
                }
                (text, String::from_utf8_lossy(&r.body).into_owned())
            }
            Err(error) => (format!("{prefix} error={error}"), String::new()),
        };
        self.set_text("http-out", &text);
        self.set_text("http-body", if body.is_empty() { "-" } else { &body });
    }

    fn center(&self, id: &str) -> (f32, f32) {
        let rect = self.rect(id);
        (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn rect(&self, id: &str) -> Rect {
        self.doc
            .get_element_by_id(id)
            .ok()
            .flatten()
            .and_then(|e| e.bounding_rect().ok())
            .unwrap_or_default()
    }

    fn focus(&self, id: &str) {
        if let Ok(Some(element)) = self.doc.get_element_by_id(id) {
            let _ = element.focus();
        }
    }

    fn input_value(&self, id: &str) -> String {
        match self.doc.get_element_by_id(id) {
            Ok(Some(element)) => element.value().unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn mouse(
        &self,
        kind: InjectedMouse,
        x: f32,
        y: f32,
        button: MouseButton,
        click_count: i32,
        dy: f32,
    ) -> kui::Result<()> {
        self.doc.dispatch_mouse_event(&MouseInjection {
            kind,
            position: Point { x, y },
            button,
            modifiers: Modifiers::default(),
            click_count,
            delta: Point { x: 0.0, y: dy },
        })
    }

    fn inject_click(&self, id: &str) {
        let (x, y) = self.center(id);
        let _ = self.mouse(InjectedMouse::Pressed, x, y, MouseButton::Left, 1, 0.0);
        let _ = self.mouse(InjectedMouse::Released, x, y, MouseButton::Left, 1, 0.0);
    }

    fn inject_wheel(&self) {
        let (x, y) = self.center("wheel-box");
        let _ = self.mouse(InjectedMouse::Wheel, x, y, MouseButton::None, 0, 120.0);
    }

    /// Key down / char / key up for each letter, as a keyboard would.
    fn inject_text(&self, letters: &str) {
        let none = Modifiers::default();
        for letter in letters.chars() {
            let (key, code) = (
                letter.to_string(),
                format!("Key{}", letter.to_ascii_uppercase()),
            );
            let _ = self
                .doc
                .dispatch_key_event(InjectedKey::Down, &key, &code, none, "", false);
            let _ = self
                .doc
                .dispatch_key_event(InjectedKey::Char, &key, &code, none, &key, false);
            let _ = self
                .doc
                .dispatch_key_event(InjectedKey::Up, &key, &code, none, "", false);
        }
    }

    /// Start, an intermediate composition, then the committed text ("ni" -> U+4F60).
    fn inject_ime(&self) {
        let _ = self.doc.dispatch_ime_event(ImeEventKind::Start, "");
        let _ = self.doc.dispatch_ime_event(ImeEventKind::Update, "ni");
        let _ = self
            .doc
            .dispatch_ime_event(ImeEventKind::Commit, "\u{4f60}");
    }

    fn cdp_text(&self, command: &str) -> String {
        match self.doc.dispatch_devtools_command(command) {
            Ok(response) => format!("ok {}", response.chars().take(300).collect::<String>()),
            Err(error) => format!("error {error}"),
        }
    }
}

/// The DevTools port: started by the runner when KUI_EXAMPLE_DEVTOOLS_PORT is set, otherwise
/// started here (port 0 picks a free one).
fn devtools_port(window: &Window) -> u16 {
    if let Ok(Some(port)) = window.devtools_port() {
        if port != 0 {
            return port;
        }
    }
    let _ = window.start_devtools(0);
    window.devtools_port().ok().flatten().unwrap_or(0)
}

// ---- HTTP against the loopback DevTools server ---------------------------------------------

fn check_http(t: &Rc<Tour>, h: &Rc<Host>) {
    t.check("http.client_create", true, String::new()); // HTTPClient::new succeeded in ready()
    let agent = h.client.set_user_agent("kui-api-tour/1.0");
    t.check("http.set_user_agent", agent.is_ok(), format!("{agent:?}"));

    let reply = h.client.fetch(&h.version_url, 3.0);
    t.check("http.fetch_sync", is_json_version(&reply), summary(&reply));
    t.check(
        "http.response_get_content_type",
        matches!(&reply, Ok(r) if r.content_type == "application/json"),
        summary(&reply),
    );
    let reply = h.client.fetch("not a url", 3.0);
    t.check(
        "http.response_get_error-bad-url",
        is_transport_error(&reply),
        summary(&reply),
    );
    let body_len = reply.as_ref().map_or(0, |r| r.body.len());
    t.known_bug(
        "http.response_get_body-empty-on-error",
        body_len == 0,
        "Rust HTTPResponse.body holds one NUL byte when the response has no body          (take_http_response sizes the buffer body_size + 1 and only truncates when body_size != 0)",
        format!("body of {body_len} bytes"),
    );

    // Main-thread callbacks may touch the DOM directly.
    let (tour, host) = (t.clone(), h.clone());
    match h
        .client
        .fetch_async_on_main_thread(&h.version_url, 3.0, move |reply| {
            tour.check(
                "http.fetch_async_on_main_thread",
                tour.ready_done.get() && is_json_version(&reply),
                format!("after ready {}, {}", tour.ready_done.get(), summary(&reply)),
            );
            host.show_reply("GET /json/version (main thread):", &reply);
            tour.settle();
        }) {
        Ok(()) => t.pending.set(t.pending.get() + 1),
        Err(error) => t.check("http.fetch_async_on_main_thread", false, error.to_string()),
    }
    let tour = t.clone();
    match h
        .client
        .fetch_async_on_main_thread(CLOSED_URL, 3.0, move |reply| {
            tour.check(
                "http.response_get_error-closed-port",
                is_transport_error(&reply),
                summary(&reply),
            );
            tour.settle();
        }) {
        Ok(()) => t.pending.set(t.pending.get() + 1),
        Err(error) => t.check(
            "http.response_get_error-closed-port",
            false,
            error.to_string(),
        ),
    }
    // Worker-thread callbacks hand the result to the UI thread with post_to_main_thread.
    match h.client.fetch_async(&h.version_url, 3.0, |reply| {
        let _ = kui::post_to_main_thread(move || {
            with_state(|t, _| {
                t.check("http.fetch_async", is_json_version(&reply), summary(&reply));
                t.settle();
            })
        });
    }) {
        Ok(()) => t.pending.set(t.pending.get() + 1),
        Err(error) => t.check("http.fetch_async", false, error.to_string()),
    }
    // A request that is cancelled right away still answers once, with the error set.
    match h.cancel_client.fetch_async(CLOSED_URL, 3.0, |reply| {
        let _ = kui::post_to_main_thread(move || {
            with_state(|t, _| {
                t.check(
                    "http.cancel_all",
                    matches!(&reply, Ok(r) if r.status == 0 && !r.error.is_empty()),
                    summary(&reply),
                );
                t.settle();
            })
        });
    }) {
        Ok(()) => {
            t.pending.set(t.pending.get() + 1);
            if let Err(error) = h.cancel_client.cancel_all() {
                t.check("http.cancel_all-status", false, error.to_string());
            }
        }
        Err(error) => t.check("http.cancel_all", false, error.to_string()),
    }
}

// ---- DevTools: port, raw CDP commands --------------------------------------------------------

// ---- DevTools token check: off for tools that cannot pass the token, then on again --------------------

fn check_token_auth(t: &Tour, window: &Window) {
    let off = window.set_devtools_token_auth(false);
    let on = window.set_devtools_token_auth(true);
    t.check(
        "dt.set_devtools_token_auth",
        off.is_ok() && on.is_ok(),
        format!("off {off:?}, on {on:?}"),
    );
}

fn check_devtools_commands(t: &Tour, h: &Host) {
    let env_port = std::env::var("KUI_EXAMPLE_DEVTOOLS_PORT").unwrap_or_default();
    t.check(
        "dt.window_get_devtools_port",
        h.port != 0 && (env_port.is_empty() || env_port == h.port.to_string()),
        format!("port {} (KUI_EXAMPLE_DEVTOOLS_PORT={env_port})", h.port),
    );
    let response = h.doc.dispatch_devtools_command(
        r##"{"id":1,"method":"DOM.setAttribute","params":{"selector":"#cdp-target","name":"data-cdp","value":"via-command"}}"##,
    );
    let attr = h
        .doc
        .get_element_by_id("cdp-target")
        .ok()
        .flatten()
        .and_then(|e| e.attribute("data-cdp").ok())
        .unwrap_or_default();
    t.check(
        "dt.dispatch_command",
        matches!(&response, Ok(r) if r.contains("\"id\":1") && !r.contains("\"error\""))
            && attr == "via-command",
        format!("attr {attr:?} response {response:?}"),
    );
    let response = h
        .doc
        .dispatch_devtools_command(r#"{"id":2,"method":"Nope.nothing"}"#);
    t.check(
        "dt.dispatch_command-unknown-method",
        matches!(&response, Ok(r) if r.contains("\"error\"")),
        format!("{response:?}"),
    );
}

// ---- Input injection: listeners record what arrives ----------------------------------------

fn listen(ctx: &Context<'_>, host: &Rc<Host>) -> Result<(), Box<dyn Error>> {
    let element = |id: &str| -> Result<kui::Element<'_>, Box<dyn Error>> {
        Ok(ctx
            .document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?)
    };
    let h = host.clone();
    element("inj-target")?.add_event_listener("click", move |_event| {
        h.clicks.set(h.clicks.get() + 1);
        h.set_text("inj-target", &format!("clicks={}", h.clicks.get()));
    })?;
    let h = host.clone();
    element("wheel-box")?.add_event_listener("wheel", move |_event| {
        h.wheels.set(h.wheels.get() + 1);
        h.set_text("wheel-box", &format!("wheel={}", h.wheels.get()));
    })?;
    let h = host.clone();
    element("wheel-box")?.add_event_listener("mousemove", move |event| {
        if let Ok(point) = event.mouse_client() {
            h.moved.set((point.x, point.y));
        }
    })?;
    let h = host.clone();
    element("inj-input")?.add_event_listener("keydown", move |event| {
        if h.first_key.borrow().0.is_empty() {
            *h.first_key.borrow_mut() = (
                event.key().unwrap_or_default(),
                event.code().unwrap_or_default(),
            );
        }
        if event.modifiers().map_or(false, |m| m.shift) {
            h.shift_seen.set(true);
        }
    })?;
    Ok(())
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn check_injection(t: &Tour, h: &Host) {
    h.inject_click("inj-target");
    t.check(
        "dt.dispatch_mouse_event-click",
        h.clicks.get() == 1,
        format!("clicks {}", h.clicks.get()),
    );

    let (x, y) = h.center("wheel-box");
    let moved = h.mouse(
        InjectedMouse::Moved,
        x + 3.0,
        y + 2.0,
        MouseButton::None,
        0,
        0.0,
    );
    let (mx, my) = h.moved.get();
    t.check(
        "dt.dispatch_mouse_event-move",
        moved.is_ok() && near(mx, x + 3.0) && near(my, y + 2.0),
        format!(
            "{moved:?} moved to ({:.1}, {:.1}), listener saw ({mx:.1}, {my:.1})",
            x + 3.0,
            y + 2.0
        ),
    );
    h.inject_wheel();
    t.check(
        "dt.dispatch_mouse_event-wheel",
        h.wheels.get() == 1,
        format!("wheel events {}", h.wheels.get()),
    );
    let invalid = h.mouse(
        InjectedMouse::Moved,
        0.0,
        0.0,
        MouseButton::Other(9),
        0,
        0.0,
    );
    t.check(
        "dt.dispatch_mouse_event-invalid-button",
        matches!(&invalid, Err(e) if e.status() == kui::sys::KUI_ERR_INVALID_ARG),
        format!("{invalid:?}"),
    );

    h.focus("inj-input");
    h.inject_text("kui");
    let value = h.input_value("inj-input");
    let (key, code) = h.first_key.borrow().clone();
    t.check(
        "dt.dispatch_key_event",
        value == "kui" && key == "k" && code == "KeyK",
        format!("value {value:?}, first keydown key {key:?} code {code:?}"),
    );
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    let _ = h
        .doc
        .dispatch_key_event(InjectedKey::Down, "Shift", "ShiftLeft", shift, "", false);
    let _ = h
        .doc
        .dispatch_key_event(InjectedKey::Up, "Shift", "ShiftLeft", shift, "", false);
    t.check(
        "dt.dispatch_key_event-modifiers",
        h.shift_seen.get(),
        "keydown listener saw no Shift".into(),
    );

    h.focus("ime-input");
    h.inject_ime();
    let value = h.input_value("ime-input");
    t.check(
        "dt.dispatch_ime_event",
        value == "\u{4f60}",
        format!("value {value:?}"),
    );
}

// ---- Box model: four quads vs the bounding rect -----------------------------------------------

fn size(quad: &[Point; 4]) -> (f32, f32) {
    (quad[2].x - quad[0].x, quad[2].y - quad[0].y)
}

fn close_to(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

fn check_box_model(t: &Tour, h: &Host) {
    let model = h.doc.box_model("#box-target");
    let rect = h.rect("box-target");
    match &model {
        Ok(Some(b)) => {
            let (pw, _) = size(&b.padding);
            let (bw, bh) = size(&b.border);
            let (mw, _) = size(&b.margin);
            t.check(
                "dt.get_box_model",
                close_to(b.width, 120.0)
                    && close_to(b.height, 40.0)
                    && close_to(pw, 140.0)
                    && close_to(bw, 148.0)
                    && close_to(bh, 68.0)
                    && close_to(mw, 160.0)
                    && close_to(b.content[0].x - b.border[0].x, 14.0),
                format!(
                    "content {}x{} padding {pw} border {bw}x{bh} margin {mw}",
                    b.width, b.height
                ),
            );
            t.check(
                "dt.get_box_model-vs-bounding-rect",
                close_to(b.border[0].x, rect.x)
                    && close_to(b.border[0].y, rect.y)
                    && close_to(bw, rect.width)
                    && close_to(bh, rect.height),
                format!("border quad {:?} vs rect {rect:?}", b.border[0]),
            );
        }
        other => t.check("dt.get_box_model", false, format!("{other:?}")),
    }
    let missing = h.doc.box_model("#no-such-box");
    t.check(
        "dt.get_box_model-missing",
        matches!(missing, Ok(None)),
        format!("{missing:?}"),
    );
}

// ---- Panel buttons ----------------------------------------------------------------------------

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
    on(ctx, host, "btn-http-main", |h| {
        let shown = h.clone();
        let _ = h
            .client
            .fetch_async_on_main_thread(&h.version_url, 3.0, move |reply| {
                shown.show_reply("GET (main thread):", &reply)
            });
    })?;
    on(ctx, host, "btn-http-sync", |h| {
        h.show_reply("GET (sync):", &h.client.fetch(&h.version_url, 3.0))
    })?;
    on(ctx, host, "btn-http-closed", |h| {
        let shown = h.clone();
        let _ = h
            .client
            .fetch_async_on_main_thread(CLOSED_URL, 3.0, move |reply| {
                shown.show_reply("closed port:", &reply)
            });
    })?;
    on(ctx, host, "btn-http-bad", |h| {
        h.show_reply("bad URL:", &h.client.fetch("not a url", 3.0))
    })?;
    on(ctx, host, "btn-http-cancel", |h| {
        let started = h.cancel_client.fetch_async(CLOSED_URL, 3.0, |reply| {
            let _ = kui::post_to_main_thread(move || {
                with_state(|_, h| h.show_reply("worker + cancel_all:", &reply))
            });
        });
        if started.is_ok() {
            let _ = h.cancel_client.cancel_all();
        }
    })?;
    on(ctx, host, "btn-cdp-attr", |h| {
        let text = h.cdp_text(
            r##"{"id":11,"method":"DOM.setAttribute","params":{"selector":"#cdp-target","name":"data-cdp","value":"button"}}"##,
        );
        h.set_text("cdp-out", &text);
    })?;
    on(ctx, host, "btn-cdp-metrics", |h| {
        let text = h.cdp_text(r#"{"id":12,"method":"Page.getLayoutMetrics"}"#);
        h.set_text("cdp-out", &text);
    })?;
    on(ctx, host, "btn-cdp-unknown", |h| {
        let text = h.cdp_text(r#"{"id":13,"method":"Nope.nothing"}"#);
        h.set_text("cdp-out", &text);
    })?;
    on(ctx, host, "btn-inj-click", |h| {
        h.inject_click("inj-target");
        h.set_text(
            "inj-out",
            &format!("injected click -> clicks={}", h.clicks.get()),
        );
    })?;
    on(ctx, host, "btn-inj-type", |h| {
        h.focus("inj-input");
        h.inject_text("kui");
        h.set_text(
            "inj-out",
            &format!("typed -> value={}", h.input_value("inj-input")),
        );
    })?;
    on(ctx, host, "btn-inj-ime", |h| {
        h.focus("ime-input");
        h.inject_ime();
        h.set_text(
            "inj-out",
            &format!("IME -> value={}", h.input_value("ime-input")),
        );
    })?;
    on(ctx, host, "btn-inj-wheel", |h| {
        h.inject_wheel();
        h.set_text(
            "inj-out",
            &format!("injected wheel -> wheel={}", h.wheels.get()),
        );
    })?;
    on(ctx, host, "btn-box", |h| {
        let rect = h.rect("box-target");
        let text = match h.doc.box_model("#box-target") {
            Ok(Some(b)) => {
                let ((pw, ph), (bw, bh), (mw, mh)) =
                    (size(&b.padding), size(&b.border), size(&b.margin));
                format!(
                    "content={:.0}x{:.0} padding={pw:.0}x{ph:.0} border={bw:.0}x{bh:.0} margin={mw:.0}x{mh:.0} | rect={:.0}x{:.0}",
                    b.width, b.height, rect.width, rect.height
                )
            }
            other => format!("{other:?}"),
        };
        h.set_text("box-out", &text);
    })?;
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let port = devtools_port(ctx.window);
    let host = Rc::new(Host {
        doc: ctx.document.clone(),
        client: HTTPClient::new()?,
        cancel_client: HTTPClient::new()?,
        version_url: format!("http://127.0.0.1:{port}/json/version"),
        port,
        clicks: Cell::new(0),
        wheels: Cell::new(0),
        moved: Cell::new((0.0, 0.0)),
        first_key: RefCell::new((String::new(), String::new())),
        shift_seen: Cell::new(false),
    });
    let tour = Rc::new(Tour::new(ctx.document));
    STATE.with(|state| *state.borrow_mut() = Some((tour.clone(), host.clone())));

    listen(ctx, &host)?;
    check_http(&tour, &host);
    check_devtools_commands(&tour, &host);
    check_token_auth(&tour, ctx.window);
    check_injection(&tour, &host);
    check_box_model(&tour, &host);
    tour.ready_done.set(true);
    if tour.pending.get() == 0 {
        tour.finish();
    }
    // Fails whatever asynchronous check has not answered within 2.6 s, then publishes.
    let t = tour.clone();
    ctx.platform.set_timeout(2600, move || {
        if !t.finished.get() && t.pending.get() > 0 {
            t.check(
                "http.async-answers",
                false,
                format!("{} request(s) never answered", t.pending.get()),
            );
        }
        t.finish();
    })?;
    bind_buttons(ctx, &host)
}
