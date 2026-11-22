//! 页内拖放数据（C ABI 第 15 节）。单独一个进程（要 init 平台，并创建一个真实窗口），
//! 用注入的鼠标输入走一遍完整拖放。

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use kui::{
    DropEffect, EffectAllowed, InjectedMouse, Modifiers, MouseButton, MouseInjection, Platform,
    Point, Window, WindowProps,
};

#[derive(Default)]
struct DropResult {
    data: Option<String>,
    missing: Option<String>,
    types: Vec<String>,
    allowed: Option<EffectAllowed>,
    effect: Option<DropEffect>,
    after_one: Vec<String>,
    after_all: Vec<String>,
}

fn mouse(kind: InjectedMouse, x: f32, click_count: i32) -> MouseInjection {
    MouseInjection {
        kind,
        position: Point { x, y: 50.0 },
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count,
        delta: Point::default(),
    }
}

fn mouse_at(kind: InjectedMouse, x: f32, y: f32, button: MouseButton) -> MouseInjection {
    MouseInjection {
        kind,
        position: Point { x, y },
        button,
        modifiers: Modifiers::default(),
        click_count: 1,
        delta: Point::default(),
    }
}

/// `<kui-context-menu>`：注入右键打开声明式菜单，menu-open 的 related_target 是被右键的元素，按在外面关闭。
fn declarative_context_menu_related_target() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-context-menu".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    document.load_html(
        r#"<html><body style="margin:0"><div id="row" style="width:200px;height:60px">row</div><kui-context-menu id="menu" for="row"><kui-menu-item text="Open"></kui-menu-item></kui-context-menu></body></html>"#,
    )?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }
    let menu = document.get_element_by_id("menu")?.expect("menu");
    let related = Rc::new(RefCell::new(String::new()));
    let sink = Rc::clone(&related);
    let owner = document.clone();
    menu.add_event_listener("menu-open", move |event| {
        if let Ok(Some(element)) = event.related_target(&owner) {
            *sink.borrow_mut() = element.id().unwrap_or_default();
        }
    })?;
    document.dispatch_mouse_event(&mouse_at(
        InjectedMouse::Pressed,
        20.0,
        20.0,
        MouseButton::Right,
    ))?;
    document.dispatch_mouse_event(&mouse_at(
        InjectedMouse::Released,
        20.0,
        20.0,
        MouseButton::Right,
    ))?;
    assert!(menu.has_attribute("open")?, "右键应打开菜单");
    assert_eq!(
        related.borrow().as_str(),
        "row",
        "related_target 是被右键的元素"
    );
    document.dispatch_mouse_event(&mouse_at(
        InjectedMouse::Pressed,
        300.0,
        300.0,
        MouseButton::Left,
    ))?;
    document.dispatch_mouse_event(&mouse_at(
        InjectedMouse::Released,
        300.0,
        300.0,
        MouseButton::Left,
    ))?;
    assert!(!menu.has_attribute("open")?, "按在外面应关闭菜单");
    Ok(())
}

fn in_page_drag_carries_data() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-drag".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    document.load_html(
        r#"<html><body style="margin:0"><div id="src" draggable="true" style="position:absolute;left:10px;top:10px;width:80px;height:80px">s</div><div id="dst" style="position:absolute;left:200px;top:10px;width:80px;height:80px">d</div></body></html>"#,
    )?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }
    let source = document.get_element_by_id("src")?.expect("src");
    let target = document.get_element_by_id("dst")?.expect("dst");

    let started = Rc::new(Cell::new(0));
    let counter = Rc::clone(&started);
    source.add_event_listener("dragstart", move |event| {
        counter.set(counter.get() + 1);
        let _ = event.set_drag_data("text/plain", "hello drag");
        let _ = event.set_drag_data("application/x-kui", "42");
        let _ = event.set_effect_allowed(EffectAllowed::Copy);
    })?;
    target.add_event_listener("dragover", |event| {
        let _ = event.prevent_default();
        let _ = event.set_drop_effect(DropEffect::Copy);
    })?;
    let dropped: Rc<RefCell<Option<DropResult>>> = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&dropped);
    target.add_event_listener("drop", move |event| {
        let mut result = DropResult {
            data: event.drag_data("text/plain").ok().flatten(),
            missing: event.drag_data("missing/type").ok().flatten(),
            types: event.drag_types().unwrap_or_default(),
            allowed: event.effect_allowed().ok(),
            effect: event.drop_effect().ok(),
            ..DropResult::default()
        };
        let _ = event.clear_drag_data(Some("application/x-kui"));
        result.after_one = event.drag_types().unwrap_or_default();
        let _ = event.clear_drag_data(None);
        result.after_all = event.drag_types().unwrap_or_else(|_| vec!["error".into()]);
        *sink.borrow_mut() = Some(result);
    })?;
    let plain_status = Rc::new(Cell::new(None));
    let status_sink = Rc::clone(&plain_status);
    target.add_event_listener("mousedown", move |event| {
        status_sink.set(event.drag_types().err().map(|error| error.status()));
    })?;

    document.dispatch_mouse_event(&mouse(InjectedMouse::Pressed, 50.0, 1))?;
    for x in (60..=240).step_by(30) {
        document.dispatch_mouse_event(&mouse(InjectedMouse::Moved, x as f32, 0))?;
    }
    document.dispatch_mouse_event(&mouse(InjectedMouse::Released, 240.0, 1))?;

    assert_eq!(started.get(), 1, "dragstart");
    let result = dropped.borrow_mut().take().expect("drop 没有触发");
    assert_eq!(result.data.as_deref(), Some("hello drag"));
    assert_eq!(result.missing, None);
    assert_eq!(result.types, ["text/plain", "application/x-kui"]);
    assert_eq!(result.allowed, Some(EffectAllowed::Copy));
    assert_eq!(result.effect, Some(DropEffect::Copy));
    assert_eq!(result.after_one, ["text/plain"]);
    assert!(result.after_all.is_empty());

    document.dispatch_mouse_event(&mouse(InjectedMouse::Pressed, 240.0, 1))?;
    document.dispatch_mouse_event(&mouse(InjectedMouse::Released, 240.0, 1))?;
    assert_eq!(plain_status.get(), Some(kui::sys::KUI_ERR_NOT_SUPPORTED));
    Ok(())
}

fn form_control_state() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-form-state".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    document.load_html(
        r#"<html><body><input id="name" type="text" value="initial"><select id="size"><option value="s">S</option><option value="m" selected>M</option><option value="l">L</option></select><input id="agree" type="checkbox"><progress id="prog" value="3" max="10"></progress><div id="plain" value="attr"></div></body></html>"#,
    )?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }
    let get = |id: &str| document.get_element_by_id(id).map(|e| e.expect(id));
    let (name, size, agree, prog, plain) = (
        get("name")?,
        get("size")?,
        get("agree")?,
        get("prog")?,
        get("plain")?,
    );

    assert_eq!(name.value()?, "initial");
    name.set_value("Ada")?;
    assert_eq!(name.value()?, "Ada");
    // 内容属性仍是初值：当前值与属性是两回事
    assert_eq!(name.attribute("value")?, "initial");
    assert_eq!(size.selected_index()?, 1);
    size.set_selected_index(2)?;
    assert_eq!(size.value()?, "l");
    agree.set_checked(true)?;
    assert!(agree.checked()?);
    prog.set_value("7.5")?;
    assert_eq!(prog.value()?, "7.5");
    assert!(
        prog.set_value("abc").is_err(),
        "non-numeric progress value must be rejected"
    );
    assert!(plain.value().is_err(), "a div has no value state");
    assert!(
        name.selected_index().is_err(),
        "a text input has no selected index"
    );
    Ok(())
}

fn host_integration() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-host".into(),
        width: 320,
        height: 200,
        frameless: true,
        sizeable: true,
        hidden: true,
        frame_radius: Some(8.0),
        ..WindowProps::default()
    })?;
    assert!(
        !window.is_visible()?,
        "a window created hidden must not be visible"
    );
    window.process_frame()?;
    window.show()?;
    assert!(window.is_visible()?, "visible after show");
    assert!(
        window.set_devtools_token_auth(false).is_err(),
        "no DevTools server yet"
    );

    let mut document = window.document()?;
    document.load_html(
        r#"<html><body style="margin:0"><div id="box" style="width:200px;height:100px;overflow:auto"><div style="height:1000px"></div><p id="end" style="margin:0;height:20px">end</p></div></body></html>"#,
    )?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }
    let boxed = document.get_element_by_id("box")?.expect("box");
    let end = document.get_element_by_id("end")?.expect("end");
    end.scroll_into_view(true)?;
    assert!(
        boxed.scroll_metrics()?.top > 500.0,
        "scroll_into_view scrolls the container down"
    );
    boxed.set_scroll_position(0.0, 0.0)?;
    boxed.start_momentum_scroll(0.0, 0.5)?;
    for _ in 0..20 {
        platform.wait_events(0.02)?;
    }
    assert!(
        boxed.scroll_metrics()?.top > 0.0,
        "momentum scroll moves the container"
    );

    // 捕获阶段监听：从根往下先执行，目标处捕获在前、普通在后，再冒泡。
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let tag = |name: &'static str| {
        let log = Rc::clone(&log);
        move |_: &kui::Event<'_>| log.borrow_mut().push(name)
    };
    let box_plain = boxed.add_event_listener("probe", tag("box"))?;
    let box_capture = boxed.add_capture_event_listener("probe", tag("box-capture"))?;
    let end_plain = end.add_event_listener("probe", tag("end"))?;
    let end_capture = end.add_capture_event_listener("probe", tag("end-capture"))?;
    end.dispatch_event("probe", true, true)?;
    assert_eq!(*log.borrow(), ["box-capture", "end-capture", "end", "box"]);
    boxed.remove_event_listener("probe", box_plain)?;
    boxed.remove_event_listener("probe", box_capture)?;
    end.remove_event_listener("probe", end_plain)?;
    end.remove_event_listener("probe", end_capture)?;

    document.set_base_path("assets/pages/")?;
    assert_eq!(document.base_path()?, "assets/pages/");
    document.set_render_device_pixel_ratio(window.device_pixel_ratio()?)?;

    let engine = kui::ScriptEngine::new()?;
    engine.bind_document(&document)?;
    let scheduled = engine.eval_value(
        "host_timer_fired = false; setTimeout(function() host_timer_fired = true end, 50)",
        "test",
    )?;
    assert!(scheduled.success);
    engine.tick_timers(0.0)?;
    assert_eq!(
        engine
            .eval_value("return tostring(host_timer_fired)", "test")?
            .value,
        kui::ScriptValue::String("false".into())
    );
    engine.tick_timers(100.0)?;
    assert_eq!(
        engine
            .eval_value("return tostring(host_timer_fired)", "test")?
            .value,
        kui::ScriptValue::String("true".into())
    );
    engine.unbind_document()?;

    kui::console_set_enabled(true)?;
    kui::console_set_stderr_output(false)?;
    kui::console_set_min_level(kui::LogLevel::Debug)?;
    kui::stack_trace_install(&kui::StackTraceConfig {
        output_mode: 2,
        ..Default::default()
    })?;
    kui::stack_trace_uninstall()?;
    kui::crash_reporter_install(&kui::CrashReporterConfig::default())?;
    kui::crash_reporter_uninstall()?;
    if let Err(error) = kui::disable_ime() {
        assert_eq!(
            error.status() as i32,
            kui::sys::KUI_ERR_NOT_SUPPORTED as i32
        );
    }
    Ok(())
}

fn main() {
    common::run(&[
        ("form_control_state", form_control_state),
        ("host_integration", host_integration),
        ("in_page_drag_carries_data", in_page_drag_carries_data),
        (
            "declarative_context_menu_related_target",
            declarative_context_menu_related_target,
        ),
    ]);
}
