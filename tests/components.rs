//! 组件（C ABI 第 18 节）：内置元素、主题、VirtualList、ListView、TreeView。单独一个进程（要 init 平台，
//! 并创建一个真实窗口）。

mod common;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use kui::{
    InjectedMouse, ListView, ListViewConfig, ListViewItem, Modifiers, MouseButton, MouseInjection,
    Platform, Point, SelectionMode, ThemeManager, TreeNodeUpdate, TreeView, TreeViewConfig,
    VirtualList, VirtualListConfig, Window, WindowProps,
};

fn click(kind: InjectedMouse) -> MouseInjection {
    MouseInjection {
        kind,
        position: Point { x: 350.0, y: 70.0 },
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        delta: Point::default(),
    }
}

fn components() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-components".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    assert_eq!(document.style_profile()?, kui::StyleProfile::NativeGui);
    document.set_style_profile(kui::StyleProfile::BrowserCompat)?;
    assert_eq!(document.style_profile()?, kui::StyleProfile::BrowserCompat);
    document.set_style_profile(kui::StyleProfile::NativeGui)?;
    document.register_builtin_components()?;
    assert!(document.custom_element_is_defined("kui-dialog")?);

    let themes = ThemeManager::new()?;
    assert_eq!(themes.theme_names()?.len(), 2);
    let mut brand = HashMap::new();
    brand.insert("--brand-bg".to_string(), "#123456".to_string());
    themes.register_theme("brand", &brand)?;
    assert_eq!(themes.theme("brand")?, brand);
    assert_eq!(
        themes.theme("missing").unwrap_err().status(),
        kui::sys::KUI_ERR_NOT_FOUND
    );
    document.load_html(
        r#"<html><body style="margin:0"><div id="vl" style="width:200px;height:100px"></div><div style="position:absolute;left:300px;top:0"><div id="lv" style="width:200px;height:120px"></div></div><div style="position:absolute;left:0;top:200px"><div id="tv" style="width:200px;height:120px"></div></div></body></html>"#,
    )?;
    themes.apply("brand", &document)?;
    assert_eq!(themes.active_theme()?, "brand");
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }

    // VirtualList
    let rendered = Rc::new(Cell::new(0));
    let focus_changes = Rc::new(Cell::new(0));
    let (r, f) = (Rc::clone(&rendered), Rc::clone(&focus_changes));
    let vl_container = document.get_element_by_id("vl")?.expect("vl");
    let vlist = VirtualList::new(
        &vl_container,
        VirtualListConfig {
            total_items: 1000,
            item_height: 20.0,
            render_item: Some(Box::new(move |doc, index| {
                r.set(r.get() + 1);
                let row = doc.create_element("div").ok()?;
                row.set_text_content(&format!("row {index}")).ok()?;
                Some(row)
            })),
            on_focus_change: Some(Box::new(move |_, _| f.set(f.get() + 1))),
            ..VirtualListConfig::default()
        },
    )?;
    let state = vlist.state()?;
    assert!(rendered.get() > 0 && state.visible_end - state.visible_start < 50);
    assert_eq!(state.total_height, 20000.0);
    vlist.scroll_to_index(500)?;
    let state = vlist.state()?;
    assert!(
        state.visible_start <= 500 && 500 < state.visible_end,
        "{state:?}"
    );
    vlist.set_focused_index(3)?;
    vlist.move_focus(2)?;
    assert_eq!((vlist.state()?.focused_index, focus_changes.get()), (5, 2));
    assert!(vlist.remove_items(990, 20).is_err());
    drop(vlist);

    // ListView
    let selected = Rc::new(Cell::new(-1i64));
    let s = Rc::clone(&selected);
    let lv_container = document.get_element_by_id("lv")?.expect("lv");
    let list = ListView::new(
        &lv_container,
        ListViewConfig {
            item_count: 100,
            item_height: 30.0,
            selection_mode: SelectionMode::Multi,
            item_selected_bg: "#ff0000".into(),
            item_provider: Some(Box::new(|index| ListViewItem {
                text: format!("Item {index}"),
                ..ListViewItem::default()
            })),
            on_select: Some(Box::new(move |index| s.set(i64::from(index)))),
            ..ListViewConfig::default()
        },
    )?;
    list.select(5)?;
    list.select_range(10, 13)?;
    assert_eq!(list.selected()?, [5, 10, 11, 12]);
    list.clear_selection()?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }
    document.dispatch_mouse_event(&click(InjectedMouse::Pressed))?;
    document.dispatch_mouse_event(&click(InjectedMouse::Released))?;
    assert_eq!(selected.get(), 2);
    assert!(list.is_selected(2)?);

    // TreeView
    let toggles = Rc::new(Cell::new(0));
    let selection_size = Rc::new(Cell::new(0usize));
    let (t, z) = (Rc::clone(&toggles), Rc::clone(&selection_size));
    let tree = TreeView::new()?;
    let a = tree.add_node("A", "+")?;
    let a1 = tree.add_child(a, "A1", "")?;
    tree.add_child(a, "A2", "")?;
    let b = tree.add_node("B", "")?;
    let node = tree.node(a)?;
    assert_eq!(
        (node.children.len(), node.icon.as_str(), node.parent),
        (2, "+", -1)
    );
    tree.set_node(b, "B2", "", true)?;
    assert!(tree.node(b)?.disabled);
    // update_node：没设的字段保持原值。
    tree.update_node(
        b,
        &TreeNodeUpdate {
            text: Some("B3"),
            ..TreeNodeUpdate::default()
        },
    )?;
    let updated = tree.node(b)?;
    assert_eq!(
        (updated.text.as_str(), updated.disabled),
        ("B3", true),
        "只改文字，禁用状态保持"
    );
    tree.update_node(
        a,
        &TreeNodeUpdate {
            disabled: Some(false),
            ..TreeNodeUpdate::default()
        },
    )?;
    let kept = tree.node(a)?;
    assert_eq!(
        (kept.text.as_str(), kept.icon.as_str(), kept.disabled),
        ("A", "+", false)
    );
    tree.set_node(b, "B2", "", true)?;
    let tv_container = document.get_element_by_id("tv")?.expect("tv");
    tree.setup(
        &tv_container,
        TreeViewConfig {
            multi_select: true,
            on_toggle: Some(Box::new(move |_, _| t.set(t.get() + 1))),
            on_selection_changed: Some(Box::new(move |values| z.set(values.len()))),
            ..TreeViewConfig::default()
        },
    )?;
    assert_eq!(tree.visible_row_count()?, 2);
    tree.expand(a)?;
    assert_eq!((tree.visible_row_count()?, toggles.get()), (4, 1));
    tree.select(a1)?;
    tree.toggle_select(b)?;
    assert_eq!(tree.selected()?, [a1, b]);
    assert_eq!(selection_size.get(), 2);
    assert_eq!(
        tree.expand(42).unwrap_err().status(),
        kui::sys::KUI_ERR_NOT_FOUND
    );

    drop(tree);
    drop(list);
    drop(tv_container);
    drop(lv_container);
    drop(vl_container);
    drop(document);
    Ok(())
}

fn main() {
    common::run(&[("components", components)]);
}
