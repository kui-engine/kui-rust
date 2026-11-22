//! 安全接口的回调：事件监听、定时器、idle、脚本错误与工作线程版 HTTP。单独一个进程（要 init 平台）。

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use kui::{Document, EventTypeTag, ListenerId, Platform, ScriptEngine};

fn pump_until(platform: &Platform, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() && Instant::now() < deadline {
        platform.wait_events(0.01).expect("wait_events");
    }
}

/// 闭包被 drop 时计数，用来确认移除 / 清除后闭包确实被释放。
struct DropCounter(Rc<Cell<u32>>);
impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

fn listeners_timers_script_errors_and_http() -> kui::Result<()> {
    let platform = Platform::init()?;
    let mut loaded = Document::new()?;
    loaded.load_html(
        r#"<html><body><div id="outer"><button id="target">go</button></div></body></html>"#,
    )?;
    let document = Rc::new(loaded);
    let target = document.get_element_by_id("target")?.expect("#target");
    let outer = document.get_element_by_id("outer")?.expect("#outer");

    // 监听器拿到事件名、种类与目标；合成事件冒泡到外层；非鼠标事件读坐标报错。
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (doc, seen) = (Rc::clone(&document), Rc::clone(&log));
    let listener = target.add_event_listener("ping", move |event| {
        let hit = event.target(&doc).unwrap().expect("target is an element");
        seen.borrow_mut().push(format!(
            "{}:{}:{:?}",
            event.event_type().unwrap(),
            hit.attribute("id").unwrap(),
            event.type_tag().unwrap()
        ));
        assert!(event.mouse_client().is_err(), "合成事件不是鼠标事件");
        assert!(event.flags().unwrap().bubbles);
    })?;
    let bubbled = Rc::new(Cell::new(0));
    let counter = Rc::clone(&bubbled);
    outer.add_event_listener("ping", move |_| counter.set(counter.get() + 1))?;
    assert!(target.dispatch_event("ping", true, true)?);
    assert_eq!(
        *log.borrow(),
        vec![format!("ping:target:{:?}", EventTypeTag::Base)]
    );
    assert_eq!(bubbled.get(), 1, "冒泡到外层");

    // prevent_default 让 dispatch_event 返回 false。
    let cancel =
        target.add_event_listener("cancel-me", |event| event.prevent_default().unwrap())?;
    assert!(!target.dispatch_event("cancel-me", true, true)?);
    target.remove_event_listener("cancel-me", cancel)?;
    assert!(target.dispatch_event("cancel-me", true, true)?);

    // 移除后不再调用。
    target.remove_event_listener("ping", listener)?;
    target.dispatch_event("ping", true, true)?;
    assert_eq!(log.borrow().len(), 1);

    // 监听器在回调里移除自己：只调用一次，闭包随后释放。
    let once_calls = Rc::new(Cell::new(0));
    let once_drops = Rc::new(Cell::new(0));
    let slot: Rc<Cell<Option<ListenerId>>> = Rc::new(Cell::new(None));
    let (doc, calls, me, guard) = (
        Rc::clone(&document),
        Rc::clone(&once_calls),
        Rc::clone(&slot),
        DropCounter(Rc::clone(&once_drops)),
    );
    let id = target.add_event_listener("self-remove", move |_| {
        let _keep = &guard;
        calls.set(calls.get() + 1);
        let element = doc.get_element_by_id("target").unwrap().unwrap();
        element
            .remove_event_listener("self-remove", me.get().unwrap())
            .unwrap();
    })?;
    slot.set(Some(id));
    target.dispatch_event("self-remove", false, false)?;
    target.dispatch_event("self-remove", false, false)?;
    assert_eq!(once_calls.get(), 1);
    assert_eq!(once_drops.get(), 1, "移除自己的监听器在回调返回后释放");

    // 按事件名整组移除，闭包全部释放。
    let group_drops = Rc::new(Cell::new(0));
    for _ in 0..3 {
        let guard = DropCounter(Rc::clone(&group_drops));
        outer.add_event_listener("bulk", move |_| {
            let _keep = &guard;
        })?;
    }
    outer.remove_event_listeners("bulk")?;
    assert_eq!(group_drops.get(), 3);

    // 定时器：timeout 一次、interval 多次直到清除、清除未触发的 timeout 不执行且释放闭包、idle。
    let fired = Rc::new(Cell::new(0));
    let f = Rc::clone(&fired);
    platform.set_timeout(1, move || f.set(f.get() + 1))?;
    pump_until(&platform, || fired.get() == 1);
    assert_eq!(fired.get(), 1);

    let ticks = Rc::new(Cell::new(0));
    let t = Rc::clone(&ticks);
    let interval = platform.set_interval(1, move || t.set(t.get() + 1))?;
    pump_until(&platform, || ticks.get() >= 3);
    platform.clear_timer(interval)?;
    let after_clear = ticks.get();
    assert!(after_clear >= 3);
    for _ in 0..10 {
        platform.wait_events(0.01)?;
    }
    assert_eq!(ticks.get(), after_clear, "清除后不再触发");

    let never = Rc::new(Cell::new(0));
    let never_drops = Rc::new(Cell::new(0));
    let (n, guard) = (Rc::clone(&never), DropCounter(Rc::clone(&never_drops)));
    let pending = platform.set_timeout(60_000, move || {
        let _keep = &guard;
        n.set(1);
    })?;
    platform.clear_timer(pending)?;
    assert_eq!(never_drops.get(), 1, "清除未触发的定时器时释放闭包");
    assert_eq!(never.get(), 0);

    let idle = Rc::new(Cell::new(false));
    let i = Rc::clone(&idle);
    platform.request_idle(move || i.set(true))?;
    pump_until(&platform, || idle.get());
    assert!(idle.get());

    // 脚本错误回调。
    let engine = ScriptEngine::new()?;
    engine.bind_document(&document)?;
    let errors = Rc::new(RefCell::new(Vec::<(String, String)>::new()));
    let e = Rc::clone(&errors);
    engine.set_error_handler(move |message, source, _line| {
        e.borrow_mut().push((message.to_owned(), source.to_owned()))
    })?;
    let result = engine.eval("error('boom')", "events-test.lua")?;
    assert!(!result.success);
    assert!(
        errors
            .borrow()
            .iter()
            .any(|(message, _)| message.contains("boom")),
        "错误回调应收到 Lua 错误：{:?}",
        errors.borrow()
    );
    engine.clear_error_handler()?;
    engine.unbind_document()?;

    // 工作线程版 HTTP：闭包是 Send，在工作线程上执行。
    let client = kui::HTTPClient::new()?;
    let (sender, receiver) = mpsc::channel();
    let ui_thread = std::thread::current().id();
    client.fetch_async("not-a-url", 1.0, move |response| {
        let on_ui = std::thread::current().id() == ui_thread;
        let _ = sender.send((on_ui, response.map(|r| r.error.is_empty())));
    })?;
    let (on_ui, outcome) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("工作线程版 HTTP 应回调");
    assert!(!on_ui, "fetch_async 的回调在工作线程上");
    assert_eq!(outcome, Ok(false), "无效 URL 应带错误返回");

    drop(engine);
    drop(platform);
    Ok(())
}

fn main() {
    common::run(&[(
        "listeners_timers_script_errors_and_http",
        listeners_timers_script_errors_and_http,
    )]);
}
