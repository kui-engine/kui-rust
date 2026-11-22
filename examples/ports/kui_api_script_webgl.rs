// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_script_webgl.rs; edit that file, not this copy.
//! API tour: script engine control and a host-driven WebGL page (examples/api_tours/script_webgl).
//! On start the host exchanges every kind of script value with the page's Lua, calls Lua and lets
//! Lua call the host, exercises limits, print, eval_file and document re-binding, then watches the
//! WebGL scene advance, pause and resume. One row per check goes into #tour-log; #tour-result ends
//! as "PASS n/n". The panel buttons repeat the calls by hand.
//!
//! Binding notes: `ScriptOutput::error` is the engine's last error (kui_script_engine_get_last_error),
//! and every function value that comes out of Lua (`global`, call results) is a retained handle
//! (kui_script_function_retain); dropping the last clone releases it (kui_script_function_release).

use crate::canonical_runner::Context;
use kui::{Document, Platform, ScriptEngine, ScriptFunction, ScriptOutput, ScriptValue};
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

// ---- Small helpers ------------------------------------------------------------------------

fn kind(value: &ScriptValue) -> &'static str {
    match value {
        ScriptValue::Nil => "nil",
        ScriptValue::Bool(_) => "bool",
        ScriptValue::Int(_) => "int",
        ScriptValue::Float(_) => "float",
        ScriptValue::String(_) => "string",
        ScriptValue::Function(_) => "function",
        ScriptValue::UserData(_) => "userdata",
    }
}

/// A value of either Lua number subtype.
fn number(value: &ScriptValue) -> f64 {
    match value {
        ScriptValue::Int(v) => *v as f64,
        ScriptValue::Float(v) => *v,
        _ => 0.0,
    }
}

fn int(value: &ScriptValue) -> i64 {
    if let ScriptValue::Int(v) = value {
        *v
    } else {
        0
    }
}

fn string(value: &ScriptValue) -> String {
    if let ScriptValue::String(v) = value {
        v.clone()
    } else {
        String::new()
    }
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(element)) = doc.get_element_by_id(id) {
        let _ = element.set_text_content(text);
    }
}

fn get_text(doc: &Document, id: &str) -> String {
    match doc.get_element_by_id(id) {
        Ok(Some(element)) => element.text_content().unwrap_or_default(),
        _ => String::new(),
    }
}

fn failed_output(error: impl ToString) -> ScriptOutput {
    ScriptOutput {
        success: false,
        error: error.to_string(),
        value: ScriptValue::Nil,
    }
}

struct Host {
    doc: Document,
    script: Rc<ScriptEngine>,
    platform: Rc<Platform>,
    tour: Tour,
    reports: Rc<Cell<u32>>,
    prints: Rc<Cell<u32>>,
    double: RefCell<Option<ScriptFunction>>, // host_double, also the Lua global host_double
    counter: RefCell<Option<ScriptFunction>>, // a Lua closure the "Lua counter++" button calls
    mark: Cell<i64>,
}

impl Host {
    fn eval(&self, code: &str) -> ScriptOutput {
        self.script
            .eval_value(code, "tour")
            .unwrap_or_else(failed_output)
    }

    fn eval_bool(&self, code: &str) -> bool {
        let out = self.eval(code);
        out.success && out.value == ScriptValue::Bool(true)
    }

    fn call(&self, name: &str, args: &[ScriptValue]) -> ScriptOutput {
        self.script
            .call_function(name, args)
            .unwrap_or_else(failed_output)
    }

    fn frames(&self) -> i64 {
        let out = self.call("getFrameCount", &[]);
        if out.success {
            int(&out.value)
        } else {
            -1
        }
    }

    fn global(&self, name: &str) -> ScriptValue {
        self.script.global(name).unwrap_or(ScriptValue::Nil)
    }

    // ---- Host functions Lua can call (register_function / create_function) --------------

    fn register_host_functions(&self) {
        // host_report(frames, fps): Lua's render loop reports progress; the host shows it.
        let (doc, reports) = (self.doc.clone(), self.reports.clone());
        let _ = self.script.register_function("host_report", move |args| {
            reports.set(reports.get() + 1);
            let frames = args.first().map_or(0.0, number);
            let fps = args.get(1).map_or(0.0, number);
            set_text(
                &doc,
                "gl-report",
                &format!("frames={frames:.0} | fps={fps:.0}"),
            );
            Ok(ScriptValue::Nil)
        });
        // host_kind(v): names the kind of value the host received.
        let _ = self.script.register_function("host_kind", |args| {
            Ok(ScriptValue::String(
                kind(args.first().unwrap_or(&ScriptValue::Nil)).to_string(),
            ))
        });
        // host_fail(): a host error becomes a Lua error with this message.
        let _ = self
            .script
            .register_function("host_fail", |_| Err("host says no".to_string()));
    }

    // ---- Globals: every value kind through set_global / global ------------------------------

    fn check_globals(&self) {
        let t = &self.tour;
        let _ = self.script.set_global("t_nil", &ScriptValue::Nil);
        let (value, missing) = (self.global("t_nil"), self.global("no_such_global"));
        t.check(
            "global.nil",
            value == ScriptValue::Nil && missing == ScriptValue::Nil,
            format!("values {value:?} / {missing:?}"),
        );

        let _ = self.script.set_global("t_bool", &ScriptValue::Bool(true));
        let value = self.global("t_bool");
        t.check(
            "global.bool",
            value == ScriptValue::Bool(true) && self.eval_bool("return t_bool == true"),
            format!("value {value:?}"),
        );

        let _ = self.script.set_global("t_int", &ScriptValue::Int(42));
        let value = self.global("t_int");
        t.check(
            "global.int",
            value == ScriptValue::Int(42) && self.eval_bool("return math.type(t_int) == 'integer'"),
            format!("value {value:?}"),
        );

        let _ = self.script.set_global("t_float", &ScriptValue::Float(2.5));
        let value = self.global("t_float");
        t.check(
            "global.float",
            value == ScriptValue::Float(2.5)
                && self.eval_bool("return math.type(t_float) == 'float'"),
            format!("value {value:?}"),
        );

        let _ = self
            .script
            .set_global("t_str", &ScriptValue::String("héllo, KUI".into()));
        let value = self.global("t_str");
        t.check(
            "global.string",
            value == ScriptValue::String("héllo, KUI".into())
                && self.eval_bool("return #t_str == 11"),
            format!("value {value:?}"),
        );

        // The page's own globals are what the scene reads every frame.
        let value = self.global("gl_ready");
        t.check(
            "webgl.page_ready",
            value == ScriptValue::Bool(true),
            format!(
                "gl_ready {value:?}, #gl-status '{}'",
                get_text(&self.doc, "gl-status")
            ),
        );
    }

    // ---- Calls: host -> Lua (call_function) and Lua -> host (register_function) ------------

    fn check_calls(&self) {
        let t = &self.tour;
        let out = self.call("setSpeed", &[ScriptValue::Float(1.5)]);
        let speed = self.global("speed");
        t.check(
            "call_function.setSpeed",
            out.success && number(&out.value) == 1.5 && number(&speed) == 1.5,
            format!("result {out:?}, global speed {speed:?}"),
        );
        self.call("setSpeed", &[ScriptValue::Float(1.0)]);

        let (first, second) = (self.call("togglePause", &[]), self.call("togglePause", &[]));
        t.check(
            "call_function.togglePause",
            first.value == ScriptValue::Bool(true) && second.value == ScriptValue::Bool(false),
            format!("first {:?} second {:?}", first.value, second.value),
        );

        let out = self.call("noSuchFunction", &[]);
        t.check(
            "get_last_error.missing_function",
            !out.success && out.error.contains("noSuchFunction"),
            format!("result {out:?}"),
        );
        let out = self.call("getFrameCount", &[]);
        t.check(
            "get_last_error.cleared",
            out.success && matches!(out.value, ScriptValue::Int(_)) && out.error.is_empty(),
            format!("result {out:?}"),
        );

        self.register_host_functions();
        let out = self.eval("return hostKinds()");
        t.check(
            "register_function.kinds",
            out.success && string(&out.value) == "nil,bool,int,float,string,function",
            format!("result {out:?}"),
        );

        let out = self
            .eval("local ok, e = pcall(host_fail) return (not ok) and tostring(e) or 'no error'");
        t.check(
            "register_function.error",
            out.success && string(&out.value).contains("host says no"),
            format!("result {out:?}"),
        );
    }

    // ---- Function values: create, call, retain, release ------------------------------------

    fn check_functions(&self) {
        let t = &self.tour;
        let double = self.script.create_function(|args| match args.first() {
            Some(ScriptValue::Int(v)) => Ok(ScriptValue::Int(v * 2)),
            Some(other) => Ok(ScriptValue::Float(number(other) * 2.0)),
            None => Err("host_double needs a number".to_string()),
        });
        let out = match &double {
            Ok(f) => {
                let _ = self
                    .script
                    .set_global("host_double", &ScriptValue::Function(f.clone()));
                self.eval("return applyTwice(host_double, 5)")
            }
            Err(error) => failed_output(error),
        };
        t.check(
            "function_create.as_lua_value",
            out.success && out.value == ScriptValue::Int(20),
            format!("result {out:?}"),
        );

        let value = double
            .as_ref()
            .map_err(|e| e.to_string())
            .and_then(|f| f.call(&[ScriptValue::Int(21)]).map_err(|e| e.to_string()));
        t.check(
            "function_call.host",
            value == Ok(ScriptValue::Int(42)),
            format!("value {value:?}"),
        );

        // A function value read back from Lua is a separately retained handle; dropping it
        // releases only that handle, Lua's reference stays.
        let copy = self.global("host_double");
        let value = match &copy {
            ScriptValue::Function(f) => f.call(&[ScriptValue::Int(4)]).map_err(|e| e.to_string()),
            other => Err(format!("host_double is {other:?}")),
        };
        drop(copy);
        t.check(
            "function_retain_release",
            value == Ok(ScriptValue::Int(8)) && self.eval_bool("return host_double(3) == 6"),
            format!("value {value:?}"),
        );
        *self.double.borrow_mut() = double.ok();

        // global hands out a retained Lua function; calling it returns another one.
        let mut got = Vec::new();
        if let ScriptValue::Function(make) = self.global("makeCounter") {
            if let Ok(ScriptValue::Function(counter)) = make.call(&[]) {
                for _ in 0..3 {
                    got.push(counter.call(&[]).map(|v| int(&v)).unwrap_or(-1));
                }
                *self.counter.borrow_mut() = Some(counter);
            }
        }
        t.check(
            "get_global.function",
            self.counter.borrow().is_some() && got == [1, 2, 3],
            format!("values {got:?}"),
        );

        // A Lua error inside a Lua function yields nil (kui_c.h, as in native C++).
        let value = match self.global("luaFails") {
            ScriptValue::Function(f) => f.call(&[]).map_err(|e| e.to_string()),
            other => Err(format!("luaFails is {other:?}")),
        };
        t.check(
            "function_call.lua_error_is_nil",
            value == Ok(ScriptValue::Nil),
            format!("value {value:?}"),
        );
    }

    // ---- Timers: the engine runs due timers only when the host ticks it ---------------------

    fn check_timers(&self) {
        // A timer is due `delay` after the engine's clock when the script scheduled it; ticking the
        // current time first makes the test independent of any earlier tick.
        let now = self.platform.now().unwrap_or(0.0);
        let _ = self.script.tick_timers(now);
        let scheduled = self.eval(
            "tour_timer_fired = false; setTimeout(function() tour_timer_fired = true end, 50)",
        );
        let _ = self.script.tick_timers(now + 10.0);
        let before = self.eval_bool("return tour_timer_fired == false");
        let ticked = self.script.tick_timers(now + 100.0);
        let after = self.eval_bool("return tour_timer_fired == true");
        self.tour.check(
            "timers.tick_timers",
            scheduled.success && ticked.is_ok() && before && after,
            format!(
                "scheduled {}, not fired early {before}, fired after the tick {after}",
                scheduled.success
            ),
        );
    }

    // ---- Limits and resource usage ---------------------------------------------------------

    fn check_limits(&self) {
        let t = &self.tour;
        let Ok(usage) = self.script.resource_usage() else {
            t.check(
                "get_resource_usage.defaults",
                false,
                "resource_usage failed".into(),
            );
            return;
        };
        t.check(
            "get_resource_usage.defaults",
            usage.memory_bytes > 0
                && usage.memory_limit == 256 * 1024 * 1024
                && usage.instruction_limit == 100_000_000,
            format!("usage {usage:?}"),
        );

        let _ = self.script.set_instruction_limit(10_000);
        let now = self
            .script
            .resource_usage()
            .map_or(0, |u| u.instruction_limit);
        let out = self.eval("while true do end");
        t.check(
            "set_instruction_limit.stops_loop",
            now == 10_000 && !out.success && out.error.contains("instruction limit"),
            format!("limit {now} result {out:?}"),
        );
        let _ = self.script.set_instruction_limit(usage.instruction_limit);
        let now = self
            .script
            .resource_usage()
            .map_or(0, |u| u.instruction_limit);
        let out = self.eval("local n = 0 for i = 1, 100000 do n = n + i end return n");
        t.check(
            "set_instruction_limit.restore",
            now == usage.instruction_limit
                && out.success
                && out.value == ScriptValue::Int(5_000_050_000),
            format!("limit {now} result {out:?}"),
        );

        let small = usage.memory_bytes + 1024 * 1024;
        let _ = self.script.set_memory_limit(small);
        let now = self.script.resource_usage().map_or(0, |u| u.memory_limit);
        let out = self.eval("local s = string.rep('x', 16 * 1024 * 1024) return #s");
        t.check(
            "set_memory_limit.refuses",
            now == small && !out.success && out.error.contains("memory"),
            format!("limit {now} result {out:?}"),
        );
        let _ = self.script.set_memory_limit(usage.memory_limit);
        let now = self.script.resource_usage().map_or(0, |u| u.memory_limit);
        let out = self.eval(
            "local s = string.rep('x', 16 * 1024 * 1024) local n = #s s = nil collectgarbage() return n",
        );
        t.check(
            "set_memory_limit.restore",
            now == usage.memory_limit
                && out.success
                && out.value == ScriptValue::Int(16 * 1024 * 1024),
            format!("limit {now} result {out:?}"),
        );
    }

    // ---- print, eval_file, document binding ------------------------------------------------

    fn check_print_file_binding(&self) {
        let t = &self.tour;
        let captured = Rc::new(RefCell::new(String::new()));
        let sink = captured.clone();
        let _ = self
            .script
            .set_print_handler(move |message| *sink.borrow_mut() = message.to_string());
        self.eval("print('hello', 42)");
        let text = captured.borrow().clone();
        t.check(
            "set_print_handler.captures",
            text == "hello\t42",
            format!("captured {text:?}"),
        );
        // The handler kept after the tour shows Lua's print() output in #print-out.
        let (doc, prints) = (self.doc.clone(), self.prints.clone());
        let _ = self.script.set_print_handler(move |message| {
            prints.set(prints.get() + 1);
            set_text(&doc, "print-out", &format!("#{} {message}", prints.get()));
        });

        let out = write_lua_file()
            .map_err(|e| e.to_string())
            .and_then(|path| {
                let out = self.script.eval_file(&path).map_err(|e| e.to_string());
                let _ = std::fs::remove_file(&path);
                out
            })
            .unwrap_or_else(failed_output);
        let loaded = self.global("from_file");
        t.check(
            "eval_file.runs",
            out.success
                && out.value == ScriptValue::Int(42)
                && loaded == ScriptValue::String("loaded".into()),
            format!("result {out:?} from_file {loaded:?}"),
        );
        let out = self
            .script
            .eval_file("no/such/file.lua")
            .unwrap_or_else(failed_output);
        t.check(
            "eval_file.missing",
            !out.success && !out.error.is_empty(),
            format!("result {out:?}"),
        );

        let _ = self.script.unbind_document();
        t.check(
            "unbind_document",
            self.eval_bool("return document == nil"),
            "document still bound".into(),
        );
        let _ = self.script.bind_document(&self.doc);
        t.check(
            "bind_document.again",
            self.eval_bool("return document:getElementById('gl-canvas') ~= nil"),
            "document not usable after re-binding".into(),
        );
    }
}

/// Puts a small Lua chunk into the temp directory.
fn write_lua_file() -> std::io::Result<String> {
    let path = std::env::temp_dir().join("kui_api_script_webgl_rust.lua");
    std::fs::write(&path, "from_file = 'loaded'\nreturn 6 * 7\n")?;
    Ok(path.to_string_lossy().into_owned())
}

// ---- Asynchronous part: the WebGL scene advances, pauses and resumes -------------------

fn phase_running(h: Rc<Host>) {
    let t = &h.tour;
    let now = h.frames();
    t.check(
        "webgl.frames_advance",
        now > h.mark.get(),
        format!("frames {} -> {now}", h.mark.get()),
    );
    let report = get_text(&h.doc, "gl-report");
    t.check(
        "register_function.host_report",
        h.reports.get() > 0 && report.starts_with("frames="),
        format!(
            "host_report calls {}, #gl-report '{report}'",
            h.reports.get()
        ),
    );
    h.call("togglePause", &[]);
    let paused = h.global("paused");
    t.check(
        "webgl.paused_global",
        paused == ScriptValue::Bool(true),
        format!("paused {paused:?}"),
    );
    // Let a frame already in flight land before taking the paused mark.
    let next = h.clone();
    let _ = h.platform.set_timeout(100, move || {
        next.mark.set(next.frames());
        let again = next.clone();
        let _ = next.platform.set_timeout(300, move || phase_paused(again));
    });
}

fn phase_paused(h: Rc<Host>) {
    let now = h.frames();
    h.tour.check(
        "webgl.pause_stops",
        now == h.mark.get(),
        format!("frames {} -> {now} while paused", h.mark.get()),
    );
    h.call("togglePause", &[]);
    let next = h.clone();
    let _ = h.platform.set_timeout(400, move || {
        let now = next.frames();
        next.tour.check(
            "webgl.resume",
            now > next.mark.get(),
            format!(
                "frames {now} after resume, {} while paused",
                next.mark.get()
            ),
        );
        next.tour.finish();
    });
}

// ---- Panel buttons -----------------------------------------------------------------------

fn on<F>(ctx: &Context<'_>, h: &Rc<Host>, id: &str, mut handler: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Host) + 'static,
{
    let host = h.clone();
    let element = ctx
        .document
        .get_element_by_id(id)?
        .ok_or(format!("page has no #{id}"))?;
    element.add_event_listener("click", move |_event| handler(&host))?;
    Ok(())
}

fn bind_buttons(ctx: &Context<'_>, h: &Rc<Host>) -> Result<(), Box<dyn Error>> {
    on(ctx, h, "btn-gl-pause", |h| {
        let out = h.call("togglePause", &[]);
        let state = if out.value == ScriptValue::Bool(true) {
            "paused"
        } else {
            "running"
        };
        set_text(&h.doc, "gl-state", state);
    })?;
    on(ctx, h, "btn-gl-speed", |h| {
        let out = h.call("setSpeed", &[ScriptValue::Float(3.0)]);
        set_text(&h.doc, "gl-out", &format!("speed={}", number(&out.value)));
    })?;
    on(ctx, h, "btn-gl-tint", |h| {
        let tints = ["sky", "rose", "lime"];
        let current = string(&h.global("tint"));
        let index = tints
            .iter()
            .position(|t| *t == current)
            .unwrap_or(tints.len() - 1);
        let next = tints[(index + 1) % tints.len()];
        let _ = h
            .script
            .set_global("tint", &ScriptValue::String(next.into()));
        set_text(&h.doc, "gl-out", &format!("tint={next}"));
    })?;
    on(ctx, h, "btn-gl-scale", |h| {
        let next = if number(&h.global("scale")) < 0.65 {
            0.8
        } else {
            0.5
        };
        let _ = h.script.set_global("scale", &ScriptValue::Float(next));
        set_text(&h.doc, "gl-out", &format!("scale={next}"));
    })?;
    on(ctx, h, "btn-gl-frames", |h| {
        set_text(&h.doc, "gl-out", &format!("frames={}", h.frames()));
    })?;
    on(ctx, h, "btn-val-roundtrip", |h| {
        let kinds: Vec<&str> = [
            "t_nil",
            "t_bool",
            "t_int",
            "t_float",
            "t_str",
            "makeCounter",
        ]
        .iter()
        .map(|name| kind(&h.global(name)))
        .collect();
        set_text(&h.doc, "val-out", &kinds.join(","));
    })?;
    on(ctx, h, "btn-val-kinds", |h| {
        let out = h.eval("return hostKinds()");
        set_text(
            &h.doc,
            "val-out",
            &format!("host saw: {}", string(&out.value)),
        );
    })?;
    on(ctx, h, "btn-val-double", |h| {
        let value = h
            .double
            .borrow()
            .as_ref()
            .and_then(|f| f.call(&[ScriptValue::Int(21)]).ok());
        let shown = value.as_ref().map_or(0, int);
        set_text(&h.doc, "val-out", &format!("host_double(21) = {shown}"));
    })?;
    on(ctx, h, "btn-val-counter", |h| {
        let value = h.counter.borrow().as_ref().and_then(|f| f.call(&[]).ok());
        set_text(
            &h.doc,
            "val-out",
            &format!("counter={}", value.as_ref().map_or(0, int)),
        );
    })?;
    on(ctx, h, "btn-lim-loop", |h| {
        let saved = h
            .script
            .resource_usage()
            .map_or(100_000_000, |u| u.instruction_limit);
        let _ = h.script.set_instruction_limit(10_000);
        let out = h.eval("while true do end");
        let _ = h.script.set_instruction_limit(saved);
        set_text(&h.doc, "lim-out", &format!("stopped: {}", out.error));
    })?;
    on(ctx, h, "btn-lim-mem", |h| {
        let Ok(saved) = h.script.resource_usage() else {
            return;
        };
        let _ = h.script.set_memory_limit(saved.memory_bytes + 1024 * 1024);
        let out = h.eval("local s = string.rep('x', 16 * 1024 * 1024) return #s");
        let _ = h.script.set_memory_limit(saved.memory_limit);
        set_text(&h.doc, "lim-out", &format!("refused: {}", out.error));
    })?;
    on(ctx, h, "btn-lim-usage", |h| {
        let Ok(usage) = h.script.resource_usage() else {
            return;
        };
        set_text(
            &h.doc,
            "lim-out",
            &format!(
                "memory={} KiB | limit={} MiB | instructions={}",
                usage.memory_bytes / 1024,
                usage.memory_limit / (1024 * 1024),
                usage.instruction_limit
            ),
        );
    })?;
    on(ctx, h, "btn-lim-error", |h| {
        let out = h.call("noSuchFunction", &[]);
        set_text(&h.doc, "lim-out", &format!("last error: {}", out.error));
    })?;
    on(ctx, h, "btn-io-print", |h| {
        h.eval("print('hello from Lua', frames > 0)");
    })?;
    on(ctx, h, "btn-io-file", |h| {
        let mut value = -1;
        if let Ok(path) = write_lua_file() {
            if let Ok(out) = h.script.eval_file(&path) {
                if out.success {
                    value = int(&out.value);
                }
            }
            let _ = std::fs::remove_file(&path);
        }
        set_text(&h.doc, "io-out", &format!("eval_file -> {value}"));
    })?;
    on(ctx, h, "btn-io-rebind", |h| {
        let _ = h.script.unbind_document();
        let unbound = h.eval_bool("return document == nil");
        let _ = h.script.bind_document(&h.doc);
        let rebound = h.eval_bool("return document ~= nil");
        set_text(
            &h.doc,
            "io-out",
            &format!(
                "unbound: document={} | rebound: document={}",
                if unbound { "nil" } else { "set" },
                if rebound { "set" } else { "nil" }
            ),
        );
    })
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let h = Rc::new(Host {
        doc: ctx.document.clone(),
        script: ctx.script.clone(),
        platform: ctx.platform.clone(),
        tour: Tour::new(ctx.document),
        reports: Rc::new(Cell::new(0)),
        prints: Rc::new(Cell::new(0)),
        double: RefCell::new(None),
        counter: RefCell::new(None),
        mark: Cell::new(0),
    });
    h.check_globals();
    h.check_calls();
    h.check_functions();
    h.check_limits();
    h.check_timers();
    h.check_print_file_binding();
    // The rest needs frames: the scene must advance, stop while paused and resume.
    h.mark.set(h.frames());
    let next = h.clone();
    ctx.platform.set_timeout(600, move || phase_running(next))?;
    bind_buttons(ctx, &h)
}
