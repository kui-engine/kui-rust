//! 应用框架（C ABI 第 11 节）：路由、数据绑定、动画状态、Observer。单独一个进程（要 init 平台，
//! 并创建一个真实窗口：路由视图与尺寸 / 相交检查都需要渲染帧）。

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use kui::{
    IntersectionOptions, MutationOptions, MutationType, Observable, Platform, RouteGuardKind,
    Window, WindowProps,
};

fn pump(platform: &Platform, rounds: usize, done: impl Fn() -> bool) -> kui::Result<()> {
    for _ in 0..rounds {
        if done() {
            break;
        }
        platform.wait_events(0.01)?;
    }
    Ok(())
}

fn router_binding_animation_and_observers() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-framework".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;

    // --- 路由 ---
    document
        .load_html(r#"<html><body><main id="view"></main><p id="label">old</p></body></html>"#)?;
    pump(&platform, 5, || false)?;
    {
        let router = document.router();
        let view = document.get_element_by_id("view")?.expect("#view");
        router.register_view(&view)?;
        let built = Rc::new(Cell::new(0));
        let counter = Rc::clone(&built);
        router.add_route("/users/:id", "user", move |doc, route| {
            counter.set(counter.get() + 1);
            let element = doc.create_element("p").ok()?;
            element.set_attribute("id", "routed").ok()?;
            let id = route.params.get("id").cloned().unwrap_or_default();
            element
                .set_text_content(&format!("view:{} id={id}", route.pattern))
                .ok()?;
            Some(element)
        })?;
        router.add_route("/admin", "", |_, _| None)?;
        assert!(router.add_route("/admin", "", |_, _| None).is_err());
        assert_eq!(router.route_count()?, 2);
        assert!(router.has_route("/admin")?);
        let guard = router.add_guard(RouteGuardKind::BeforeEnter, |_, to| {
            !to.path.starts_with("/admin")
        })?;
        let tab = Rc::new(RefCell::new(String::new()));
        let seen = Rc::clone(&tab);
        let navigated = router.on_navigated(move |route| {
            *seen.borrow_mut() = route.query_params.get("tab").cloned().unwrap_or_default();
        })?;

        assert!(router.push("/users/42?tab=info")?);
        assert_eq!(*tab.borrow(), "info");
        assert_eq!(router.param("id")?.as_deref(), Some("42"));
        assert_eq!(router.param("missing")?, None);
        let current = router.current()?;
        assert_eq!(current.pattern, "/users/:id");
        assert_eq!(current.query, "tab=info");
        assert_eq!(router.current_path()?, "/users/42?tab=info");
        let routed = document.get_element_by_id("routed")?.expect("#routed");
        assert_eq!(routed.text_content()?, "view:/users/:id id=42");
        assert_eq!(built.get(), 1);

        assert!(!router.push("/admin")?, "守卫应取消 /admin");
        router.remove_guard(guard)?;
        assert!(router.push("/admin")?);
        assert!(router.back()?);
        assert_eq!(router.history()?, (0, 2));
        assert!(!router.go(-5)?);
        router.remove_navigated(navigated)?;
        assert!(router.remove_route("/missing").is_err());
        router.unregister_view(&view)?;
        assert!(router.unregister_view(&view).is_err());
        router.clear()?;
        assert_eq!(router.route_count()?, 0);
    }

    // --- 数据绑定 ---
    {
        let label = document.get_element_by_id("label")?.expect("#label");
        let observable = Observable::new("hello")?;
        let changes = Rc::new(Cell::new(0));
        let counter = Rc::clone(&changes);
        let subscription = observable.subscribe(move || counter.set(counter.get() + 1))?;
        let binding = observable.bind(&label, "textContent", false)?;
        assert_eq!(label.text_content()?, "hello", "绑定应立即写入");
        observable.set("world")?;
        observable.set("world")?;
        assert_eq!(changes.get(), 1);
        document.flush_bindings()?;
        assert_eq!(label.text_content()?, "world");
        assert_eq!(observable.get()?, "world");
        observable.unsubscribe(subscription)?;
        assert!(observable.unsubscribe(subscription).is_err());
        observable.unbind(binding)?;
        assert!(observable.unbind(binding).is_err());
        assert!(observable.bind(&label, "", false).is_err());
        observable.unbind_all()?;
        // clone 再取一个引用：drop 原句柄后值仍在，drop 最后一个才释放。
        let shared = observable.clone();
        assert_eq!(shared.as_raw(), observable.as_raw());
        drop(observable);
        shared.set("shared")?;
        assert_eq!(shared.get()?, "shared");
        assert!(!document.as_raw().is_null());
    }

    // --- 动画状态与 Observer ---
    document.load_html(
        r#"<html><head><style>@keyframes pulse{from{opacity:0}to{opacity:1}}#box{animation:pulse 1s infinite}</style></head>
           <body style="margin:0"><div id="box" title="a" style="width:120px;height:40px"></div></body></html>"#,
    )?;
    let box_el = document.get_element_by_id("box")?.expect("#box");
    let animating = Cell::new(false);
    pump(&platform, 100, || {
        animating.set(box_el.animation_state().map(|(a, _)| a).unwrap_or(false));
        animating.get()
    })?;
    assert!(animating.get(), "应报告 CSS 动画");
    assert!(document.animation_counts()?.0 >= 1);

    let old_title = Rc::new(RefCell::new(String::new()));
    let seen = Rc::clone(&old_title);
    let mutation = document.create_mutation_observer(move |records| {
        for record in records {
            if record.kind == MutationType::Attributes && record.attribute_name == "title" {
                assert!(record.target.is_some());
                *seen.borrow_mut() = record.old_value.clone();
            }
        }
    })?;
    mutation.observe_mutations(
        &box_el,
        &MutationOptions {
            attributes: true,
            attribute_old_value: true,
            attribute_filter: vec!["title".into()],
            ..MutationOptions::default()
        },
    )?;
    assert!(
        mutation.observe(&box_el).is_err(),
        "MutationObserver 要用 observe_mutations"
    );
    box_el.set_attribute("title", "b")?;
    assert_eq!(*old_title.borrow(), "a");
    // clone 再取一个引用：drop 原句柄后观察仍有效，drop 最后一个才断开。
    let shared_mutation = mutation.clone();
    assert_eq!(shared_mutation.as_raw(), mutation.as_raw());
    drop(mutation);
    box_el.set_attribute("title", "c")?;
    assert_eq!(*old_title.borrow(), "b", "克隆句柄应继续观察");
    drop(shared_mutation);

    let width = Rc::new(Cell::new(0.0f32));
    let w = Rc::clone(&width);
    let resize = document.create_resize_observer(move |entries| {
        if let Some(last) = entries.last() {
            w.set(last.content_width);
        }
    })?;
    resize.observe(&box_el)?;
    pump(&platform, 100, || width.get() != 0.0)?;
    assert_eq!(width.get(), 120.0, "首次观察报告尺寸");
    drop(resize);

    let state = Rc::new(Cell::new(-1));
    let s = Rc::clone(&state);
    let intersection = document.create_intersection_observer(
        &IntersectionOptions {
            thresholds: vec![0.0, 1.0],
            ..IntersectionOptions::default()
        },
        move |entries| {
            if let Some(last) = entries.last() {
                s.set(last.is_intersecting as i32);
            }
        },
    )?;
    intersection.observe(&box_el)?;
    pump(&platform, 100, || state.get() != -1)?;
    assert_eq!(state.get(), 1);
    drop(intersection);
    assert!(document
        .create_intersection_observer(
            &IntersectionOptions {
                thresholds: vec![2.0],
                ..IntersectionOptions::default()
            },
            |_| {},
        )
        .is_err());
    Ok(())
}

fn main() {
    common::run(&[(
        "router_binding_animation_and_observers",
        router_binding_animation_and_observers,
    )]);
}
