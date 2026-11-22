//! 安全接口第二批：DOM 结构与查询、classList、innerHTML、样式文本、几何、自定义元素、媒体 / Lottie、
//! 显示器、DevTools 注入与盒模型、窗口状态与外观。单独一个进程（要 init 平台，并创建一个真实窗口）。

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use kui::{
    Cursor, CustomElementDefinition, Document, FileDialogOptions, FrameRadii, InjectedKey,
    InjectedMouse, Menu, MenuItem, Modifiers, MouseButton, MouseInjection, Platform, Point,
    PositioningOptions, ToastLevel, ToastOptions, TrayEvent, Window, WindowProps,
};

fn dom(document: &mut Document) -> kui::Result<()> {
    document.load_html(
        r#"<html><head><title>t</title></head><body>
           <ul id="list" class="a b"><li id="one">1</li><li id="two">2</li><li id="three">3</li></ul>
           <div id="box" style="color: red; width: 10px !important">box</div>
           </body></html>"#,
    )?;
    assert_eq!(
        document.root()?.expect("html").tag_name()?.to_lowercase(),
        "html"
    );
    assert_eq!(
        document.head()?.expect("head").tag_name()?.to_lowercase(),
        "head"
    );
    let body = document.body()?.expect("body");
    assert_eq!(document.query_selector_all("li")?.len(), 3);
    assert!(document.query_selector_all(".missing")?.is_empty());

    // 树遍历与子树查询。
    let list = document.get_element_by_id("list")?.expect("#list");
    assert_eq!(list.child_count()?, 3);
    assert_eq!(list.first_child()?.expect("first").id()?, "one");
    assert_eq!(list.last_child()?.expect("last").id()?, "three");
    let two = list.query_selector("#two")?.expect("#two");
    assert_eq!(two.previous_sibling()?.expect("prev").id()?, "one");
    assert_eq!(two.next_sibling()?.expect("next").id()?, "three");
    assert_eq!(two.parent()?.expect("parent").id()?, "list");
    assert!(list.last_child()?.unwrap().next_sibling()?.is_none());
    assert_eq!(list.query_selector_all("li")?.len(), 3);

    // id / class / classList / 属性。
    assert_eq!(list.class_name()?, "a b");
    list.class_list_add("c")?;
    list.class_list_remove("a")?;
    list.class_list_toggle("d")?;
    assert!(list.class_list_contains("c")? && list.class_list_contains("d")?);
    assert!(!list.class_list_contains("a")?);
    list.set_class_name("x")?;
    assert_eq!(list.class_name()?, "x");
    assert!(list.has_attribute("id")?);
    list.set_attribute("data-k", "v")?;
    list.remove_attribute("data-k")?;
    assert!(!list.has_attribute("data-k")?);

    // innerHTML 与删除。
    let clone = two.clone(); // retain：同一个元素的第二个句柄
    list.remove_child(&clone)?;
    assert_eq!(list.child_count()?, 2);
    assert!(list.remove_child(&two).is_err(), "已不是子元素");
    list.set_inner_html("<li id=\"n\">new</li>")?;
    assert!(list.inner_html()?.contains("id=\"n\""));
    list.first_child()?.unwrap().remove()?;
    assert_eq!(list.child_count()?, 0);

    // 内联样式文本、属性名、!important、删除。
    let boxed = document.get_element_by_id("box")?.expect("#box");
    assert!(boxed.style_property_is_important("width")?);
    assert!(!boxed.style_property_is_important("color")?);
    assert_eq!(boxed.style_property_names()?, vec!["color", "width"]);
    boxed.remove_style_property("color")?;
    assert_eq!(boxed.style_property_names()?, vec!["width"]);
    boxed.set_style_text("height: 5px")?;
    assert!(boxed.style_text()?.contains("height"));

    // 几何、滚动、焦点、锚定定位：独立文档也能调用。
    boxed.client_rect()?;
    boxed.offset_rect()?;
    boxed.scroll_metrics()?;
    boxed.set_scroll_position(0.0, 0.0)?;
    boxed.scroll_by(0.0, 0.0)?;
    boxed.focus()?;
    boxed.blur()?;
    let tip = document.create_element("div")?;
    body.append_child(&tip)?;
    tip.position_relative_to(&boxed, "bottom-start", &PositioningOptions::default())?;

    // 自定义元素：接入 / 观察的属性变化 / 离开。
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (a, b, c) = (Rc::clone(&log), Rc::clone(&log), Rc::clone(&log));
    document.define_custom_element(
        "x-rust-card",
        CustomElementDefinition {
            observed_attributes: vec!["watched".into()],
            connected: Some(Box::new(move |element| {
                a.borrow_mut()
                    .push(format!("connected:{}", element.tag_name().unwrap()))
            })),
            disconnected: Some(Box::new(move |_| {
                b.borrow_mut().push("disconnected".into())
            })),
            attribute_changed: Some(Box::new(move |_, name, old, new| {
                c.borrow_mut().push(format!("{name}:{old}->{new}"))
            })),
        },
    )?;
    assert!(document.custom_element_is_defined("x-rust-card")?);
    assert!(!document.custom_element_is_defined("x-unknown")?);
    let card = document.create_element("x-rust-card")?;
    body.append_child(&card)?;
    card.set_attribute("watched", "1")?;
    card.set_attribute("ignored", "1")?;
    card.remove()?;
    assert_eq!(
        *log.borrow(),
        vec![
            "connected:x-rust-card".to_string(),
            "watched:->1".to_string(),
            "disconnected".to_string()
        ]
    );

    // 扩展状态：没有 init_extensions 时扩展管理器不存在；用空目录初始化后可以查询。
    assert_eq!(
        document
            .extension_is_loaded("x-missing")
            .unwrap_err()
            .status(),
        kui::sys::KUI_ERR_NOT_SUPPORTED
    );
    let empty = std::env::temp_dir().join(format!("kui-rust-no-ext-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("temp dir");
    assert_eq!(document.init_extensions(empty.to_str().unwrap())?, 0);
    assert!(!document.extension_is_loaded("x-missing")?);
    document.reset_extension_perf_stats()?;
    let _ = std::fs::remove_dir(&empty);

    // 媒体与 Lottie 元素的读写（测试进程没有音频设备，也能读写属性）。
    let audio = document.create_element("audio")?;
    body.append_child(&audio)?;
    audio.set_volume(0.25)?;
    assert!((audio.volume()? - 0.25).abs() < 1e-6);
    audio.set_muted(true)?;
    assert!(audio.muted()?);
    assert!(audio.paused()?);
    assert!((0..=4).contains(&audio.ready_state()?));
    assert!((0..=3).contains(&audio.network_state()?));
    audio.set_controls(true)?;
    audio.media_is_available()?;
    audio.current_time()?;
    audio.duration()?;
    let lottie = document.create_element("lottie-player")?;
    body.append_child(&lottie)?;
    assert!(!lottie.lottie_complete()?);
    lottie.lottie_natural_size()?;
    lottie.pause_lottie()?;

    // 文档句柄 clone 后仍指向同一份文档。
    let again = document.clone();
    assert!(again.get_element_by_id("box")?.is_some());
    Ok(())
}

/// C ABI 第 9 节的 DOM 部分；单独一个函数，因为要对文档重新 load_html（前面的元素句柄还借用着文档）。
fn dom_editing(document: &mut Document) -> kui::Result<()> {
    // 插入 / 替换 / cloneNode / matches / closest / outerHTML / 相邻插入 / 文本 / 指针捕获。
    document.load_html(
        r#"<html><body><div id="list" class="box"><p id="a">A</p><p id="b">B</p></div><span id="free">f</span></body></html>"#,
    )?;
    let list = document.get_element_by_id("list")?.expect("#list");
    let a = document.get_element_by_id("a")?.expect("#a");
    let b = document.get_element_by_id("b")?.expect("#b");
    let free = document.get_element_by_id("free")?.expect("#free");
    let ids = |parent: &kui::Element<'_>| -> String {
        let mut out = Vec::new();
        let mut child = parent.first_child().unwrap();
        while let Some(current) = child {
            out.push(current.id().unwrap());
            child = current.next_sibling().unwrap();
        }
        out.join(",")
    };
    list.insert_before(&free, Some(&b))?;
    assert_eq!(ids(&list), "a,free,b");
    list.insert_before(&a, None)?;
    assert_eq!(ids(&list), "free,b,a");
    assert_eq!(
        a.insert_before(&list, None).unwrap_err().status(),
        kui::sys::KUI_ERR_INVALID_ARG,
        "插入祖先应被拒绝"
    );
    let em = document.create_element("em")?;
    list.replace_child(&em, &b)?;
    assert_eq!(list.child_count()?, 3);
    let copy = list.clone_node(true)?;
    assert_eq!(copy.child_count()?, 3);
    assert!(copy.parent()?.is_none(), "cloneNode 的结果脱离文档");
    assert!(list.matches("div.box")? && !list.matches("span")?);
    assert_eq!(a.closest(".box")?.expect("closest").id()?, "list");
    assert!(a.closest(".missing")?.is_none());
    assert!(a.outer_html()?.contains("id=\"a\""));
    assert_eq!(list.computed_style_value("display")?, "block");
    assert_eq!(list.computed_style_value("no-such-property")?, "");
    list.insert_adjacent_html("beforeend", "<i id=\"tail\"></i>")?;
    assert_eq!(list.last_child()?.expect("tail").id()?, "tail");
    a.append_text(" more")?;
    assert_eq!(a.text_content()?, "A more");
    a.set_pointer_capture(1)?;
    assert!(a.has_pointer_capture(1)?);
    a.release_pointer_capture(1)?;
    assert!(!a.has_pointer_capture(1)?);
    assert!(!a.has_focus()?);
    assert_eq!(document.get_elements_by_tag_name("p")?.len(), 1);
    assert_eq!(document.get_elements_by_class_name("box")?.len(), 1);
    Ok(())
}

fn platform_and_devtools(platform: &Platform, document: &Document) -> kui::Result<()> {
    let count = platform.monitor_count()?;
    assert!(count >= 1);
    assert_eq!(platform.monitors()?.len(), count);
    let primary = platform.primary_monitor()?;
    assert!(primary.primary && primary.scale > 0.0 && primary.bounds.width > 0.0);
    let physical = platform.primary_monitor_physical_size()?;
    assert!(physical.width > 0.0 && physical.height > 0.0);
    platform.has_clipboard_text()?;
    platform.set_f12_devtools_enabled(false)?;
    let _ = FileDialogOptions::default(); // 对话框要人工交互，只检查类型可用
    let before = platform.now()?;
    platform.wait_events(0.01)?;
    assert!(platform.now()? >= before && before > 0.0);
    assert!(
        platform
            .monitor_from_point(primary.bounds.x + 1.0, primary.bounds.y + 1.0)?
            .primary
    );

    let model = document.box_model("#box")?.expect("#box 的盒模型");
    assert!(model.width >= 0.0);
    assert!(document.box_model("#missing")?.is_none());
    document.dispatch_mouse_event(&MouseInjection {
        kind: InjectedMouse::Moved,
        position: Point { x: 1.0, y: 1.0 },
        button: MouseButton::None,
        modifiers: Modifiers::default(),
        click_count: 0,
        delta: Point::default(),
    })?;
    document.dispatch_key_event(
        InjectedKey::Down,
        "a",
        "KeyA",
        Modifiers::default(),
        "",
        false,
    )?;
    document.dispatch_key_event(
        InjectedKey::Up,
        "a",
        "KeyA",
        Modifiers::default(),
        "",
        false,
    )?;
    Ok(())
}

fn window(platform: &Platform) -> kui::Result<()> {
    let window = Window::new(&WindowProps {
        title: "kui-rust-test".into(),
        width: 320,
        height: 240,
        min_width: 200,
        min_height: 150,
        ..WindowProps::default()
    })?;
    assert!(window.is_visible()?);
    assert!(!window.native_handle()?.is_null());
    assert!(window.device_pixel_ratio()? > 0.0);
    assert!(!window.should_close()?);

    let resized = Rc::new(RefCell::new(None));
    let r = Rc::clone(&resized);
    window.on_resize(move |w, h| *r.borrow_mut() = Some((w, h)))?;
    window.resize(360, 260)?;
    // X11 的尺寸变化经 ConfigureNotify 异步送达（Windows 是 SetWindowPos 里同步发 WM_SIZE），有上限地等回调。
    for _ in 0..150 {
        if resized.borrow().is_some() {
            break;
        }
        platform.wait_events(0.02)?;
    }
    let (w, h) = window.size()?;
    assert!(w >= 200 && h >= 150);
    assert!(resized.borrow().is_some(), "尺寸变化回调应被调用");
    window.clear_on_resize()?;

    window.set_position(40, 40)?;
    window.position()?;
    window.set_opacity(0.9)?;
    window.set_always_on_top(false)?;
    window.set_frameless_sizeable(false)?;
    window.set_frame_radius(6.0)?;
    window.clear_frame_radius()?;
    window.minimize()?;
    platform.poll_events()?;
    window.restore()?;
    platform.poll_events()?;
    window.maximize()?;
    for _ in 0..50 {
        if window.is_maximized()? {
            break;
        }
        platform.wait_events(0.02)?;
    }
    // X11（EWMH）的最大化由窗口管理器执行、异步生效，is_maximized 读的是它写回的 _NET_WM_STATE；
    // 没有窗口管理器的 Xvfb 里不会发生。Linux 上只要求请求本身不出错。
    if !cfg!(target_os = "linux") {
        assert!(window.is_maximized()?);
    }
    window.restore()?;
    window.is_minimized()?;
    assert_eq!(window.devtools_port()?, None, "没有启动 DevTools");
    window.devtools_url()?;

    // C ABI 第 9 节：焦点、光标、全屏、无框开关、四角圆角、输入法候选框、按窗口找显示器。
    window.focus()?;
    window.has_focus()?;
    window.set_cursor(Cursor::Pointer)?;
    window.set_cursor_visible(true)?;
    window.set_fullscreen(true)?;
    assert!(window.is_fullscreen()?);
    window.set_fullscreen(false)?;
    assert!(!window.is_fullscreen()?);
    window.set_frameless(true)?;
    window.set_frameless_shadow(true)?;
    let state = window.frame_state()?;
    // Linux / macOS 的内核还没有无边框窗口（setter 是空函数，T-DEBT-POSIX-FRAMELESS-RUNTIME-NOOP）：在那里断言
    // 「仍为 false」当绊线，内核实现后这里会失败，届时同一提交改回与 Windows 相同的断言。
    if cfg!(windows) {
        assert!(state.frameless && state.shadow);
    } else {
        assert!(!state.frameless && !state.shadow);
    }
    let radii = FrameRadii {
        top_left: 4.0,
        top_right: 8.0,
        bottom_right: 12.0,
        bottom_left: 16.0,
    };
    window.set_frame_radii(radii)?;
    assert_eq!(window.frame_radii()?, Some(radii));
    window.clear_frame_radius()?;
    assert_eq!(window.frame_radii()?, None);
    window.set_frameless(false)?;
    window.set_ime_candidate_pos(10.0, 20.0)?;
    assert!(platform.monitor_from_window(&window)?.bounds.width > 0.0);

    // C ABI 第 10 节：菜单栏、托盘、快捷键、Toast、截图。
    let menus = vec![Menu {
        text: "&File".into(),
        items: vec![
            MenuItem {
                text: "&Open".into(),
                shortcut: "Ctrl+O".into(),
                id: 1,
                ..Default::default()
            },
            MenuItem {
                separator: true,
                ..Default::default()
            },
            MenuItem {
                text: "Recent".into(),
                children: vec![MenuItem {
                    text: "a.txt".into(),
                    id: 31,
                    ..Default::default()
                }],
                ..Default::default()
            },
            MenuItem {
                text: "Wrap".into(),
                id: 2,
                checkable: true,
                ..Default::default()
            },
        ],
    }];
    window.set_menu_bar(&menus, Some(|_id: u32| {}))?;
    window.set_menu_item_checked(2, true)?;
    window.set_menu_item_enabled(31, false)?;
    assert_eq!(
        window
            .set_menu_item_checked(999, true)
            .unwrap_err()
            .status(),
        kui::sys::KUI_ERR_NOT_FOUND
    );
    window.clear_menu_bar()?;

    if window.create_tray("", "kui rust tray", true).is_ok() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&events);
        window.on_tray_event(move |event| seen.borrow_mut().push(event))?;
        window.simulate_tray_event(TrayEvent::Click)?;
        window.simulate_tray_event(TrayEvent::BalloonClick)?;
        assert_eq!(
            *events.borrow(),
            vec![TrayEvent::Click, TrayEvent::BalloonClick]
        );
        window.set_tray_context_menu(
            &[MenuItem {
                text: "Quit".into(),
                id: 9,
                ..Default::default()
            }],
            Some(|_id: u32| {}),
        )?;
        window.destroy_tray()?;
        assert!(!window.tray_active()?);
    }

    let first = window.bind_shortcut("Ctrl+S", || {})?;
    assert_ne!(first, 0);
    assert_eq!(
        window.bind_shortcut("Ctrl+S", || {})?,
        first,
        "同一组合再绑返回同一 id"
    );
    assert!(window.bind_shortcut("", || {}).is_err());
    window.unbind_all_shortcuts()?;

    let mut desktop_doc = window.document()?;
    desktop_doc.load_html("<html><body><p>desktop</p></body></html>")?;
    for _ in 0..10 {
        platform.poll_events()?;
    }
    let toast = desktop_doc.show_toast(&ToastOptions {
        message: "Saved".into(),
        level: ToastLevel::Success,
        duration_ms: 0,
        ..Default::default()
    })?;
    assert!(toast != 0 && desktop_doc.toast_count()? == 1);
    desktop_doc.dismiss_toast(toast)?;
    let shot = std::env::temp_dir().join(format!("kui-rust-shot-{}.png", std::process::id()));
    assert!(window.save_screenshot(shot.to_str().unwrap())?);
    let _ = std::fs::remove_file(&shot);

    let close_listener = window.add_event_listener("close", true, |event| {
        let _ = event.prevent_default();
    })?;
    window.remove_event_listener("close", close_listener)?;
    window.add_event_listener("focus", false, |_| {})?;
    window.remove_event_listeners("focus")?;
    Ok(())
}

fn dom_platform_devtools_and_window() -> kui::Result<()> {
    let platform = Platform::init()?;
    let mut document = Document::new()?;
    dom(&mut document)?;
    platform_and_devtools(&platform, &document)?;
    dom_editing(&mut document)?;
    window(&platform)?;
    Ok(())
}

fn main() {
    common::run(&[(
        "dom_platform_devtools_and_window",
        dom_platform_devtools_and_window,
    )]);
}
