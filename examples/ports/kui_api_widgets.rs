// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_widgets.rs; edit that file, not this copy.
//! API tour: native widgets (examples/api_tours/widgets). The host mounts two ListViews (single
//! and multi selection), a multi-select TreeView and a VirtualList into the page, calls every
//! selection / expand / scroll / data method once and checks the state read back (selected set,
//! focus, node and visible row counts, rendered range); #tour-result ends as "PASS n/n". The
//! panel buttons repeat the calls and print the state into each panel's readout.

use crate::canonical_runner::Context;
use kui::{
    Document, ListView, ListViewConfig, ListViewItem, SelectionMode, TreeView, TreeViewConfig,
    VirtualList, VirtualListConfig, VirtualListState,
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

    // ---- Small readers ----

    fn text(&self, id: &str) -> String {
        match self.doc.get_element_by_id(id) {
            Ok(Some(el)) => el.text_content().unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn has(&self, id: &str, needle: &str) -> bool {
        self.text(id).contains(needle)
    }

    fn set(&self, id: &str, text: &str) {
        if let Ok(Some(el)) = self.doc.get_element_by_id(id) {
            let _ = el.set_text_content(text);
        }
    }
}

fn indices(values: &[u32]) -> String {
    let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(","))
}

// ---- Shared state: the widgets (set right after creation) and what their callbacks saw ----

struct App {
    t: Tour,
    single: RefCell<Option<Rc<ListView>>>,
    multi: RefCell<Option<Rc<ListView>>>,
    tree: RefCell<Option<Rc<TreeView>>>,
    scratch: RefCell<Option<Rc<TreeView>>>,
    vlist: RefCell<Option<Rc<VirtualList>>>,
    // ListView
    lv_generation: Cell<i32>, // refresh: appended to every row label once > 0
    lv_provided: Cell<i32>,   // item_provider calls
    lv_count: Cell<u32>,
    // TreeView
    tree_selection: RefCell<String>, // last on_selection_changed
    tree_last_select: Cell<i64>,
    tree_last_toggle: Cell<i64>,
    tree_last_expanded: Cell<i32>,
    readme_long: Cell<bool>,
    // VirtualList
    vl_items: RefCell<Vec<u32>>, // the data model: one id per row
    vl_next_new: Cell<u32>,
    vl_generation: Cell<i32>,
    vl_rendered: Cell<i32>, // render_item calls
    vl_focus: Cell<i32>,    // last on_focus_change
    vl_focus_old: Cell<i32>,
    vl_total: Cell<u32>, // set_total_items button
}

impl App {
    fn single(&self) -> Rc<ListView> {
        self.single.borrow().clone().expect("single list")
    }
    fn multi(&self) -> Rc<ListView> {
        self.multi.borrow().clone().expect("multi list")
    }
    fn tree(&self) -> Rc<TreeView> {
        self.tree.borrow().clone().expect("tree")
    }
    fn scratch(&self) -> Rc<TreeView> {
        self.scratch.borrow().clone().expect("scratch tree")
    }
    fn vlist(&self) -> Rc<VirtualList> {
        self.vlist.borrow().clone().expect("virtual list")
    }
}

// ==========================================================================================
// ---- ListView: selection modes, range selection, item count, scrolling -------------------

const LV_COUNT: u32 = 200;
const LV_ROW: f32 = 30.0;
const LV_OVERSCAN: u32 = 2;

fn lv_selected(list: &ListView) -> String {
    indices(&list.selected().unwrap_or_default())
}

fn lv_is(list: &ListView, index: u32) -> bool {
    list.is_selected(index).unwrap_or(false)
}

fn lv_focus(list: &ListView) -> i32 {
    list.focused_index().unwrap_or(-2)
}

fn lv_state(list: &ListView) -> Option<VirtualListState> {
    list.state().ok()
}

fn show_list(app: &App) {
    let (single, multi) = (app.single(), app.multi());
    app.t.set("lv-single-sel", &lv_selected(&single));
    app.t.set("lv-multi-sel", &lv_selected(&multi));
    app.t.set("lv-focus", &lv_focus(&multi).to_string());
    if let Some(state) = lv_state(&multi) {
        app.t.set("lv-start", &state.visible_start.to_string());
        app.t.set("lv-end", &state.visible_end.to_string());
        app.t.set("lv-total", &state.total_items.to_string());
    }
}

fn create_list(
    app: &Rc<App>,
    id: &str,
    mode: SelectionMode,
    name: &'static str,
) -> Result<ListView, Box<dyn Error>> {
    let container = app
        .t
        .doc
        .get_element_by_id(id)?
        .ok_or_else(|| format!("page has no #{id}"))?;
    let (provider, select, activate) = (app.clone(), app.clone(), app.clone());
    Ok(ListView::new(
        &container,
        ListViewConfig {
            item_count: LV_COUNT,
            item_height: LV_ROW,
            overscan: LV_OVERSCAN,
            selection_mode: mode,
            item_bg: "#ffffff".into(),
            item_hover_bg: "#f1f5f9".into(),
            item_selected_bg: "#2563eb".into(),
            item_text_color: "#0f172a".into(),
            item_sub_color: "#64748b".into(),
            item_provider: Some(Box::new(move |index| {
                provider.lv_provided.set(provider.lv_provided.get() + 1);
                let generation = provider.lv_generation.get();
                let text = if generation > 0 {
                    format!("Row {index} r{generation}")
                } else {
                    format!("Row {index}")
                };
                ListViewItem {
                    text,
                    ..Default::default()
                }
            })),
            // Clicks and keyboard selection on either list.
            on_select: Some(Box::new(move |index| {
                select
                    .t
                    .set("lv-event", &format!("on_select({index}) {name}"));
                show_list(&select);
            })),
            on_activate: Some(Box::new(move |index| {
                activate
                    .t
                    .set("lv-event", &format!("on_activate({index}) {name}"));
            })),
            ..Default::default()
        },
    )?)
}

fn check_list_view(app: &App) {
    let t = &app.t;
    let (single, multi) = (app.single(), app.multi());
    let state = lv_state(&multi);
    t.check(
        "list.create",
        state.is_some_and(|s| {
            s.total_items == LV_COUNT
                && s.item_height == LV_ROW
                && s.total_height == 6000.0
                && s.visible_start == 0
                && s.visible_end > 0
                && s.visible_end <= LV_COUNT
        }) && app.lv_provided.get() > 0
            && t.has("lv-multi", "Row 0"),
        format!("state {state:?} provided {}", app.lv_provided.get()),
    );

    // Single selection: a second select replaces the first.
    let _ = single.select(2);
    let _ = single.select(4);
    let got = lv_selected(&single);
    t.check(
        "list.select-single",
        got == "[4]" && !lv_is(&single, 2) && lv_is(&single, 4) && lv_focus(&single) == 4,
        format!("selected {got} focus {}", lv_focus(&single)),
    );

    // Multi selection: selections accumulate.
    let _ = multi.select(2);
    let _ = multi.select(4);
    let got = lv_selected(&multi);
    t.check(
        "list.select-multi",
        got == "[2,4]" && lv_is(&multi, 2) && lv_is(&multi, 4) && lv_focus(&multi) == 4,
        format!("selected {got} focus {}", lv_focus(&multi)),
    );

    // kui_c.h: select_range adds rows [start, end).
    let _ = multi.select_range(6, 9);
    let got = lv_selected(&multi);
    t.check(
        "list.select_range",
        got == "[2,4,6,7,8]" && !lv_is(&multi, 9),
        format!("selected {got}"),
    );

    let _ = multi.deselect(4);
    let got = lv_selected(&multi);
    t.check(
        "list.deselect",
        got == "[2,6,7,8]" && !lv_is(&multi, 4),
        format!("selected {got}"),
    );

    let values = multi.selected();
    t.check(
        "list.get_selected-count",
        values.as_ref().is_ok_and(|v| v.len() == 4),
        format!("{values:?}"),
    );

    let _ = single.clear_selection();
    let _ = multi.clear_selection();
    let values = multi.selected().unwrap_or_default();
    t.check(
        "list.clear_selection",
        values.is_empty() && !lv_is(&multi, 2) && !lv_is(&single, 4),
        format!("selected {values:?}"),
    );

    let range =
        |list: &ListView| lv_state(list).map_or((0, 0), |s| (s.visible_start, s.visible_end));
    let _ = multi.scroll_to_index(100);
    let (start, end) = range(&multi);
    t.check(
        "list.scroll_to_index",
        start <= 100 && end > 100 && start > 0,
        format!("rendered {start}-{end}"),
    );

    let _ = multi.scroll_to_bottom();
    let (start, end) = range(&multi);
    t.check(
        "list.scroll_to_bottom",
        end == LV_COUNT && start > 100,
        format!("rendered {start}-{end}"),
    );

    let _ = multi.scroll_to_top();
    let (start, end) = range(&multi);
    t.check(
        "list.scroll_to_top",
        start == 0,
        format!("rendered {start}-{end}"),
    );

    // Shrinking the list drops selected rows that no longer exist.
    let _ = multi.select(10);
    let _ = multi.select(150);
    let _ = multi.set_item_count(100);
    let state = lv_state(&multi);
    let got = lv_selected(&multi);
    t.check(
        "list.set_item_count",
        state.is_some_and(|s| s.total_items == 100 && s.total_height == 3000.0) && got == "[10]",
        format!("state {state:?} selected {got}"),
    );
    let _ = multi.set_item_count(LV_COUNT);
    let _ = multi.clear_selection();

    // refresh re-asks the provider for the rows in view.
    let before = app.lv_provided.get();
    app.lv_generation.set(1);
    let _ = multi.refresh();
    let _ = single.refresh();
    t.check(
        "list.refresh",
        app.lv_provided.get() > before
            && t.has("lv-multi", "Row 0 r1")
            && t.has("lv-single", "Row 0 r1"),
        format!("provider calls {before} -> {}", app.lv_provided.get()),
    );
}

type Handler = Box<dyn Fn(&App)>;

// Panel buttons: the same calls on the visible lists.
fn list_buttons() -> Vec<(&'static str, Handler)> {
    fn both(f: fn(&ListView)) -> Handler {
        Box::new(move |app: &App| {
            f(&app.single());
            f(&app.multi());
            show_list(app);
        })
    }
    vec![
        (
            "btn-lv-sel2",
            both(|v| {
                let _ = v.select(2);
            }),
        ),
        (
            "btn-lv-sel4",
            both(|v| {
                let _ = v.select(4);
            }),
        ),
        (
            "btn-lv-range",
            Box::new(|app: &App| {
                let _ = app.multi().select_range(6, 9);
                show_list(app);
            }),
        ),
        (
            "btn-lv-desel",
            both(|v| {
                let _ = v.deselect(2);
            }),
        ),
        (
            "btn-lv-clear",
            both(|v| {
                let _ = v.clear_selection();
            }),
        ),
        (
            "btn-lv-goto",
            both(|v| {
                let _ = v.scroll_to_index(100);
            }),
        ),
        (
            "btn-lv-top",
            both(|v| {
                let _ = v.scroll_to_top();
            }),
        ),
        (
            "btn-lv-bottom",
            both(|v| {
                let _ = v.scroll_to_bottom();
            }),
        ),
        (
            "btn-lv-count",
            Box::new(|app: &App| {
                let count = if app.lv_count.get() == LV_COUNT {
                    50
                } else {
                    LV_COUNT
                };
                app.lv_count.set(count);
                let _ = app.single().set_item_count(count);
                let _ = app.multi().set_item_count(count);
                show_list(app);
            }),
        ),
        (
            "btn-lv-refresh",
            Box::new(|app: &App| {
                app.lv_generation.set(app.lv_generation.get() + 1);
                let _ = app.single().refresh();
                let _ = app.multi().refresh();
                app.t
                    .set("lv-event", &format!("refresh r{}", app.lv_generation.get()));
            }),
        ),
    ]
}

// ==========================================================================================
// ---- TreeView: nodes, expand / collapse, multi selection, scrolling -----------------------

// Node indices of the visible tree (added in this order):
//   0 src { 1 core { 2 types.h, 3 string.h }, 4 ui { 5 button.c, 6 list.c }, 7 main.c }
//   8 docs { 9 guide.md, 10 api.md }   11 README
const N_SRC: u32 = 0;
const N_CORE: u32 = 1;
const N_TYPES: u32 = 2;
const N_STRING: u32 = 3;
const N_UI: u32 = 4;
const N_BUTTON: u32 = 5;
const N_MAIN: u32 = 7;
const N_DOCS: u32 = 8;
const N_API: u32 = 10;
const N_README: u32 = 11;
const N_NODES: u32 = 12;

fn tree_selected(tree: &TreeView) -> String {
    indices(&tree.selected().unwrap_or_default())
}

fn tree_is(tree: &TreeView, index: u32) -> bool {
    tree.is_selected(index).unwrap_or(false)
}

fn tree_index(tree: &TreeView) -> i32 {
    tree.selected_index().unwrap_or(-2)
}

fn tree_rows(tree: &TreeView) -> u32 {
    tree.visible_row_count().unwrap_or(0)
}

fn tree_expanded(tree: &TreeView, index: u32) -> bool {
    tree.node(index).map(|n| n.expanded).unwrap_or(false)
}

fn tree_scroll_top(app: &App) -> f32 {
    match app.t.doc.get_element_by_id("tree-box") {
        Ok(Some(el)) => el.scroll_metrics().map(|m| m.top).unwrap_or(-1.0),
        _ => -1.0,
    }
}

fn show_tree(app: &App) {
    let tree = app.tree();
    app.t.set("tree-sel", &tree_selected(&tree));
    app.t.set("tree-idx", &tree_index(&tree).to_string());
    app.t.set("tree-rows", &tree_rows(&tree).to_string());
    app.t
        .set("tree-nodes", &tree.node_count().unwrap_or(0).to_string());
    if let Ok(node) = tree.node(N_README) {
        app.t.set("tree-readme", &node.text);
    }
    app.t
        .set("tree-scroll", &format!("{:.0}", tree_scroll_top(app)));
}

fn create_tree(app: &Rc<App>, id: &str, multi: bool) -> Result<TreeView, Box<dyn Error>> {
    let container = app
        .t
        .doc
        .get_element_by_id(id)?
        .ok_or_else(|| format!("page has no #{id}"))?;
    let tree = TreeView::new()?;
    let mut config = TreeViewConfig {
        item_height: 26.0,
        indent_width: 16.0,
        overscan: 2,
        item_bg: "#ffffff".into(),
        item_hover_bg: "#f1f5f9".into(),
        item_selected_bg: "#16a34a".into(),
        item_text_color: "#0f172a".into(),
        multi_select: multi,
        ..Default::default()
    };
    if multi {
        tree.add_node("src", "")?;
        tree.add_child(N_SRC, "core", "")?;
        tree.add_child(N_CORE, "types.h", "")?;
        tree.add_child(N_CORE, "string.h", "")?;
        tree.add_child(N_SRC, "ui", "")?;
        tree.add_child(N_UI, "button.c", "")?;
        tree.add_child(N_UI, "list.c", "")?;
        tree.add_child(N_SRC, "main.c", "")?;
        tree.add_node("docs", "")?;
        tree.add_child(N_DOCS, "guide.md", "")?;
        tree.add_child(N_DOCS, "api.md", "")?;
        tree.add_node("README", "")?;
        let (select, toggle, selection) = (app.clone(), app.clone(), app.clone());
        config.on_select = Some(Box::new(move |index| {
            select.tree_last_select.set(i64::from(index));
            select.t.set("tree-event", &format!("on_select({index})"));
            show_tree(&select);
        }));
        config.on_toggle = Some(Box::new(move |index, expanded| {
            toggle.tree_last_toggle.set(i64::from(index));
            toggle.tree_last_expanded.set(i32::from(expanded));
            let state = if expanded { "expanded" } else { "collapsed" };
            toggle
                .t
                .set("tree-event", &format!("on_toggle({index}, {state})"));
            show_tree(&toggle);
        }));
        config.on_selection_changed = Some(Box::new(move |selected| {
            *selection.tree_selection.borrow_mut() = indices(selected);
        }));
    } else {
        // Scratch tree: 0 A { 2 C }, 1 B.
        tree.add_node("A", "*")?;
        tree.add_node("B", "*")?;
        tree.add_child(0, "C", "*")?;
    }
    tree.setup(&container, config)?;
    Ok(tree)
}

fn check_tree_nodes(app: &App) {
    let (t, tree) = (&app.t, app.tree());
    let count = tree.node_count().unwrap_or(0);
    t.check(
        "tree.get_node_count",
        count == N_NODES,
        format!("count {count}"),
    );

    let root = tree.node(N_SRC);
    let core = tree.node(N_CORE);
    t.check(
        "tree.get_node",
        root.as_ref()
            .is_ok_and(|r| r.parent == -1 && r.children.len() == 3)
            && core.as_ref().is_ok_and(|c| {
                c.parent == N_SRC as i32 && !c.expanded && !c.disabled && c.text == "core"
            }),
        format!("root {root:?} core {core:?}"),
    );

    // node() reads the children through get_node_child.
    t.check(
        "tree.get_node_child",
        core.as_ref()
            .is_ok_and(|c| c.children == [N_TYPES, N_STRING])
            && root
                .as_ref()
                .is_ok_and(|r| r.children.get(2) == Some(&N_MAIN)),
        format!("core {core:?}"),
    );

    let missing = tree.node(N_NODES);
    t.check(
        "tree.get_node-missing",
        matches!(&missing, Err(e) if e.status() == kui::sys::KUI_ERR_NOT_FOUND),
        format!("{missing:?}"),
    );
}

fn check_tree_expand(app: &App) {
    let (t, tree) = (&app.t, app.tree());
    let rows = tree_rows(&tree);
    t.check("tree.visible-collapsed", rows == 3, format!("rows {rows}"));

    let last = || (app.tree_last_toggle.get(), app.tree_last_expanded.get());
    let _ = tree.expand(N_SRC);
    let rows = tree_rows(&tree);
    t.check(
        "tree.expand",
        rows == 6 && tree_expanded(&tree, N_SRC) && last() == (i64::from(N_SRC), 1),
        format!("rows {rows} on_toggle {:?}", last()),
    );

    let _ = tree.expand_all();
    let rows = tree_rows(&tree);
    t.check(
        "tree.expand_all",
        rows == N_NODES && tree_expanded(&tree, N_UI),
        format!("rows {rows}"),
    );

    let _ = tree.collapse(N_CORE);
    let rows = tree_rows(&tree);
    t.check(
        "tree.collapse",
        rows == 10 && !tree_expanded(&tree, N_CORE) && last() == (i64::from(N_CORE), 0),
        format!("rows {rows} on_toggle {:?}", last()),
    );

    let _ = tree.toggle(N_CORE);
    let rows = tree_rows(&tree);
    let _ = tree.toggle(N_CORE);
    t.check(
        "tree.toggle",
        rows == N_NODES && tree_rows(&tree) == 10 && !tree_expanded(&tree, N_CORE),
        format!("rows after toggles {rows}, {}", tree_rows(&tree)),
    );

    let _ = tree.collapse_all();
    let rows = tree_rows(&tree);
    t.check(
        "tree.collapse_all",
        rows == 3
            && !tree_expanded(&tree, N_SRC)
            && !tree_expanded(&tree, N_UI)
            && !tree_expanded(&tree, N_DOCS),
        format!("rows {rows}"),
    );
}

fn check_tree_selection(app: &App) {
    let (t, tree) = (&app.t, app.tree());
    let changed = || app.tree_selection.borrow().clone();
    let _ = tree.expand_all();
    let _ = tree.select(N_DOCS);
    let got = tree_selected(&tree);
    t.check(
        "tree.select",
        got == "[8]"
            && tree_index(&tree) == N_DOCS as i32
            && tree_is(&tree, N_DOCS)
            && app.tree_last_select.get() == i64::from(N_DOCS)
            && changed() == "[8]",
        format!(
            "selected {got} index {} on_select {} on_selection_changed {}",
            tree_index(&tree),
            app.tree_last_select.get(),
            changed()
        ),
    );

    let _ = tree.toggle_select(N_README);
    let got = tree_selected(&tree);
    t.check(
        "tree.toggle_select-add",
        got == "[8,11]" && changed() == "[8,11]",
        format!("selected {got}"),
    );
    let _ = tree.toggle_select(N_DOCS);
    let got = tree_selected(&tree);
    t.check(
        "tree.toggle_select-remove",
        got == "[11]" && !tree_is(&tree, N_DOCS),
        format!("selected {got}"),
    );

    // select sets the anchor; select_range selects the visible rows from it to the target.
    let _ = tree.select(N_TYPES);
    let _ = tree.select_range(N_BUTTON);
    let got = tree_selected(&tree);
    t.check(
        "tree.select_range",
        got == "[2,3,4,5]" && tree_index(&tree) == N_BUTTON as i32,
        format!("selected {got} index {}", tree_index(&tree)),
    );

    let _ = tree.select_all();
    let count = tree.selected().map(|v| v.len()).unwrap_or(0);
    t.check(
        "tree.select_all",
        count == N_NODES as usize && tree_is(&tree, N_API),
        format!("count {count}"),
    );

    let _ = tree.clear_selection();
    let count = tree.selected().map(|v| v.len()).unwrap_or(99);
    t.check(
        "tree.clear_selection",
        count == 0 && tree_index(&tree) == -1 && !tree_is(&tree, N_TYPES) && changed() == "[]",
        format!("count {count} index {}", tree_index(&tree)),
    );
}

// The scratch tree (single selection): replacing selection, set_node, nodes added after setup.
fn check_tree_scratch(app: &App) {
    let (t, tree) = (&app.t, app.scratch());
    let _ = tree.select(0);
    let _ = tree.select(1);
    let got = tree_selected(&tree);
    t.check(
        "tree.select-single",
        got == "[1]" && !tree_is(&tree, 0) && tree_index(&tree) == 1,
        format!("selected {got} index {}", tree_index(&tree)),
    );

    // The Rust wrapper always passes text and icon (an empty string is a value, not "keep").
    let _ = tree.set_node(2, "C2", "*", true);
    let _ = tree.set_node(1, "B", "#", false);
    let (c, b) = (tree.node(2), tree.node(1));
    t.check(
        "tree.set_node",
        c.as_ref()
            .is_ok_and(|c| c.text == "C2" && c.icon == "*" && c.disabled)
            && b.as_ref()
                .is_ok_and(|b| b.text == "B" && b.icon == "#" && !b.disabled),
        format!("node 2 {c:?} node 1 {b:?}"),
    );

    // kui_c.h: nodes added after setup appear after refresh.
    let _ = tree.expand(0);
    let before = tree_rows(&tree);
    let _ = tree.add_child(0, "D", "");
    let count = tree.node_count().unwrap_or(0);
    let added = tree_rows(&tree);
    let _ = tree.refresh();
    t.check(
        "tree.refresh",
        before == 3
            && added == 3
            && count == 4
            && tree_rows(&tree) == 4
            && t.has("scratch-tree", "C2")
            && t.has("scratch-tree", "D"),
        format!(
            "rows {before} -> {added} -> {}, nodes {count}",
            tree_rows(&tree)
        ),
    );
}

fn tree_buttons() -> Vec<(&'static str, Handler)> {
    fn on(f: fn(&App, &TreeView)) -> Handler {
        Box::new(move |app: &App| {
            f(app, &app.tree());
            show_tree(app);
        })
    }
    vec![
        (
            "btn-tree-expand-all",
            on(|_, t| {
                let _ = t.expand_all();
            }),
        ),
        (
            "btn-tree-collapse-all",
            on(|_, t| {
                let _ = t.collapse_all();
            }),
        ),
        (
            "btn-tree-toggle",
            on(|_, t| {
                let _ = t.toggle(N_SRC);
            }),
        ),
        (
            "btn-tree-collapse",
            on(|_, t| {
                let _ = t.collapse(N_CORE);
            }),
        ),
        (
            "btn-tree-select",
            on(|_, t| {
                let _ = t.select(N_CORE);
            }),
        ),
        (
            "btn-tree-toggle-sel",
            on(|_, t| {
                let _ = t.toggle_select(N_DOCS);
            }),
        ),
        (
            "btn-tree-range",
            on(|_, t| {
                let _ = t.select_range(N_MAIN);
            }),
        ),
        (
            "btn-tree-all",
            on(|_, t| {
                let _ = t.select_all();
            }),
        ),
        (
            "btn-tree-clear",
            on(|_, t| {
                let _ = t.clear_selection();
            }),
        ),
        (
            "btn-tree-scroll",
            on(|_, t| {
                let _ = t.scroll_to_node(N_README);
            }),
        ),
        (
            "btn-tree-rename",
            on(|app, t| {
                app.readme_long.set(!app.readme_long.get());
                let name = if app.readme_long.get() {
                    "README.md"
                } else {
                    "README"
                };
                let _ = t.set_node(N_README, name, "", false);
                let _ = t.refresh();
            }),
        ),
    ]
}

// ==========================================================================================
// ---- VirtualList: focus, insert / remove, total count, scrolling --------------------------

const VL_COUNT: u32 = 1000;
const VL_ROW: f32 = 24.0;
const VL_OVERSCAN: u32 = 3;
const VL_NEW_BASE: u32 = 1_000_000; // ids at or above are inserted rows ("New n")

fn vl_reset(app: &App) {
    *app.vl_items.borrow_mut() = (0..VL_COUNT).collect();
    app.vl_next_new.set(1);
}

fn vl_label(app: &App, index: u32) -> String {
    let id = app
        .vl_items
        .borrow()
        .get(index as usize)
        .copied()
        .unwrap_or(index);
    let mut text = if id >= VL_NEW_BASE {
        format!("New {}", id - VL_NEW_BASE)
    } else {
        format!("Item {id}")
    };
    if app.vl_generation.get() > 0 {
        text.push_str(&format!(" r{}", app.vl_generation.get()));
    }
    text
}

fn vl_model_insert(app: &App, index: usize, count: u32) {
    let start = app.vl_next_new.get();
    app.vl_next_new.set(start + count);
    let mut items = app.vl_items.borrow_mut();
    let at = index.min(items.len());
    items.splice(at..at, (start..start + count).map(|n| VL_NEW_BASE + n));
}

fn vl_model_remove(app: &App, index: usize, count: usize) {
    let mut items = app.vl_items.borrow_mut();
    if index + count <= items.len() {
        items.drain(index..index + count);
    }
}

fn vl_state(app: &App) -> Option<VirtualListState> {
    app.vlist().state().ok()
}

fn show_vlist(app: &App) {
    if let Some(state) = vl_state(app) {
        app.t.set("vl-total", &state.total_items.to_string());
        app.t.set("vl-focus", &state.focused_index.to_string());
        app.t.set("vl-start", &state.visible_start.to_string());
        app.t.set("vl-end", &state.visible_end.to_string());
    }
    app.t.set("vl-first", &vl_label(app, 0));
}

fn create_vlist(app: &Rc<App>) -> Result<VirtualList, Box<dyn Error>> {
    let container = app
        .t
        .doc
        .get_element_by_id("vl-box")?
        .ok_or("page has no #vl-box")?;
    vl_reset(app);
    let (render, focus, activate) = (app.clone(), app.clone(), app.clone());
    Ok(VirtualList::new(
        &container,
        VirtualListConfig {
            total_items: VL_COUNT,
            item_height: VL_ROW,
            overscan: VL_OVERSCAN,
            enable_keyboard: true,
            render_item: Some(Box::new(move |doc, index| {
                render.vl_rendered.set(render.vl_rendered.get() + 1);
                let row = doc.create_element("div").ok()?;
                let background = if index as i32 == render.vl_focus.get() {
                    "#f59e0b"
                } else {
                    "#ffffff"
                };
                for (name, value) in [
                    ("display", "flex"),
                    ("align-items", "center"),
                    ("padding", "0 10px"),
                    ("font-size", "12px"),
                    ("color", "#0f172a"),
                    ("background", background),
                ] {
                    let _ = row.set_style_property(name, value, false);
                }
                let _ = row.set_text_content(&vl_label(&render, index));
                Some(row) // KUI takes it
            })),
            on_focus_change: Some(Box::new(move |old_index, new_index| {
                focus.vl_focus_old.set(old_index);
                focus.vl_focus.set(new_index);
                focus
                    .t
                    .set("vl-event", &format!("focus {old_index} -> {new_index}"));
            })),
            on_activate: Some(Box::new(move |index| {
                activate.t.set("vl-event", &format!("on_activate({index})"));
            })),
            ..Default::default()
        },
    )?)
}

fn check_vlist(app: &App) {
    let (t, list) = (&app.t, app.vlist());
    let state = vl_state(app);
    t.check(
        "vlist.get_state",
        state.is_some_and(|s| {
            s.total_items == VL_COUNT
                && s.item_height == VL_ROW
                && s.total_height == 24000.0
                && s.focused_index == -1
                && s.visible_start == 0
                && s.visible_end > 0
                && s.visible_end <= VL_COUNT
        }) && app.vl_rendered.get() > 0
            && t.has("vl-box", "Item 0"),
        format!("state {state:?}"),
    );

    let range = || vl_state(app).map_or((0, 0), |s| (s.visible_start, s.visible_end));
    let _ = list.scroll_to_index(500);
    let (start, end) = range();
    t.check(
        "vlist.scroll_to_index",
        start <= 500 && end > 500 && t.has("vl-box", "Item 500"),
        format!("rendered {start}-{end}"),
    );

    let _ = list.scroll_to_bottom();
    let (start, end) = range();
    t.check(
        "vlist.scroll_to_bottom",
        end == VL_COUNT && t.has("vl-box", "Item 999"),
        format!("rendered {start}-{end}"),
    );

    let _ = list.scroll_to_top();
    let (start, end) = range();
    t.check(
        "vlist.scroll_to_top",
        start == 0 && t.has("vl-box", "Item 0"),
        format!("rendered {start}-{end}"),
    );

    let focused = || vl_state(app).map_or(-2, |s| s.focused_index);
    // kui_c.h: set_focused_index calls on_focus_change when it changes.
    let _ = list.set_focused_index(10);
    let (start, end) = range();
    t.check(
        "vlist.set_focused_index",
        focused() == 10
            && app.vl_focus_old.get() == -1
            && app.vl_focus.get() == 10
            && start <= 10
            && end > 10,
        format!(
            "focus {} event {}->{} rendered {start}-{end}",
            focused(),
            app.vl_focus_old.get(),
            app.vl_focus.get()
        ),
    );

    // kui_c.h: clamped to the rows; -1 clears.
    let _ = list.set_focused_index(5000);
    let (clamped, event_old) = (focused(), app.vl_focus_old.get());
    let _ = list.set_focused_index(-1);
    t.check(
        "vlist.set_focused_index-clamp",
        clamped == 999
            && event_old == 10
            && focused() == -1
            && app.vl_focus_old.get() == 999
            && app.vl_focus.get() == -1,
        format!(
            "5000 -> {clamped}, -1 -> {} (event {}->{})",
            focused(),
            app.vl_focus_old.get(),
            app.vl_focus.get()
        ),
    );

    let _ = list.set_focused_index(10);
    let _ = list.move_focus(5);
    let moved = focused();
    let _ = list.move_focus(-100);
    t.check(
        "vlist.move_focus",
        moved == 15 && focused() == 0 && app.vl_focus.get() == 0,
        format!("10+5 -> {moved}, -100 -> {}", focused()),
    );
    let _ = list.set_focused_index(-1);

    // Insert three rows at the top, then remove them again.
    let _ = list.scroll_to_top();
    vl_model_insert(app, 0, 3);
    let _ = list.insert_items(0, 3);
    let _ = list.scroll_to_top();
    let state = vl_state(app);
    t.check(
        "vlist.insert_items",
        state.is_some_and(|s| s.total_items == VL_COUNT + 3 && s.total_height == 24072.0)
            && t.has("vl-box", "New 1"),
        format!("state {state:?}"),
    );

    vl_model_remove(app, 0, 3);
    let _ = list.remove_items(0, 3);
    let state = vl_state(app);
    t.check(
        "vlist.remove_items",
        state.is_some_and(|s| s.total_items == VL_COUNT)
            && !t.has("vl-box", "New 1")
            && t.has("vl-box", "Item 0"),
        format!("state {state:?}"),
    );

    let _ = list.set_total_items(50);
    let state = vl_state(app);
    t.check(
        "vlist.set_total_items",
        state.is_some_and(|s| {
            s.total_items == 50 && s.total_height == 1200.0 && s.visible_end <= 50
        }),
        format!("state {state:?}"),
    );
    let _ = list.set_total_items(VL_COUNT);

    let before = app.vl_rendered.get();
    app.vl_generation.set(1);
    let _ = list.refresh();
    t.check(
        "vlist.refresh",
        app.vl_rendered.get() > before && t.has("vl-box", "Item 0 r1"),
        format!("render calls {before} -> {}", app.vl_rendered.get()),
    );

    // Removing the rows around the focused row: the focus must stay within the rows.
    let _ = list.set_focused_index(999);
    vl_model_remove(app, 990, 10);
    let _ = list.remove_items(990, 10);
    let state = vl_state(app);
    t.known_bug(
        "vlist.remove_items-focus",
        state.is_some_and(|s| s.total_items == 990 && s.focused_index < 990),
        "remove_items leaves focused_index past the last row",
        format!("state {state:?}"),
    );

    // Back to the starting state.
    vl_reset(app);
    let _ = list.set_total_items(VL_COUNT);
    let _ = list.set_focused_index(-1);
    let _ = list.scroll_to_top();
    let _ = list.refresh();
}

fn vlist_buttons() -> Vec<(&'static str, Handler)> {
    fn after_focus(f: fn(&VirtualList)) -> Handler {
        Box::new(move |app: &App| {
            let list = app.vlist();
            f(&list);
            let _ = list.refresh(); // repaint the amber focus row
            show_vlist(app);
        })
    }
    fn on(f: fn(&App, &VirtualList)) -> Handler {
        Box::new(move |app: &App| {
            f(app, &app.vlist());
            show_vlist(app);
        })
    }
    vec![
        (
            "btn-vl-focus",
            after_focus(|l| {
                let _ = l.set_focused_index(10);
            }),
        ),
        (
            "btn-vl-prev",
            after_focus(|l| {
                let _ = l.move_focus(-1);
            }),
        ),
        (
            "btn-vl-next",
            after_focus(|l| {
                let _ = l.move_focus(1);
            }),
        ),
        (
            "btn-vl-insert",
            on(|app, l| {
                vl_model_insert(app, 0, 3);
                let _ = l.insert_items(0, 3);
            }),
        ),
        (
            "btn-vl-remove",
            on(|app, l| {
                if app.vl_items.borrow().len() < 3 {
                    return;
                }
                vl_model_remove(app, 0, 3);
                let _ = l.remove_items(0, 3);
            }),
        ),
        (
            "btn-vl-goto",
            on(|_, l| {
                let _ = l.scroll_to_index(500);
            }),
        ),
        (
            "btn-vl-top",
            on(|_, l| {
                let _ = l.scroll_to_top();
            }),
        ),
        (
            "btn-vl-bottom",
            on(|_, l| {
                let _ = l.scroll_to_bottom();
            }),
        ),
        (
            "btn-vl-total",
            on(|app, l| {
                let total = if app.vl_total.get() == 50 {
                    app.vl_items.borrow().len() as u32
                } else {
                    50
                };
                app.vl_total.set(total);
                let _ = l.set_total_items(total);
            }),
        ),
        (
            "btn-vl-refresh",
            on(|app, l| {
                app.vl_generation.set(app.vl_generation.get() + 1);
                let _ = l.refresh();
                app.t
                    .set("vl-event", &format!("refresh r{}", app.vl_generation.get()));
            }),
        ),
    ]
}

// ==========================================================================================
// ---- Deferred phase: scrolling the tree needs a laid-out frame, then the verdict ---------

fn phase_tree_scrolled(app: &App) {
    let tree = app.tree();
    let scrolled = tree_scroll_top(app);
    let _ = tree.scroll_to_node(N_SRC);
    let back = tree_scroll_top(app);
    app.t.check(
        "tree.scroll_to_node",
        scrolled > 0.0 && back == 0.0,
        format!("README -> {scrolled:.0}, src -> {back:.0}"),
    );
    // Back to the starting state.
    let _ = tree.collapse_all();
    let _ = tree.clear_selection();
    show_tree(app);
    show_list(app);
    show_vlist(app);
    for id in ["lv-event", "tree-event", "vl-event"] {
        app.t.set(id, "-");
    }
    app.t.finish();
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let app = Rc::new(App {
        t: Tour::new(ctx.document),
        single: RefCell::new(None),
        multi: RefCell::new(None),
        tree: RefCell::new(None),
        scratch: RefCell::new(None),
        vlist: RefCell::new(None),
        lv_generation: Cell::new(0),
        lv_provided: Cell::new(0),
        lv_count: Cell::new(LV_COUNT),
        tree_selection: RefCell::new(String::new()),
        tree_last_select: Cell::new(-1),
        tree_last_toggle: Cell::new(-1),
        tree_last_expanded: Cell::new(-1),
        readme_long: Cell::new(false),
        vl_items: RefCell::new(Vec::new()),
        vl_next_new: Cell::new(1),
        vl_generation: Cell::new(0),
        vl_rendered: Cell::new(0),
        vl_focus: Cell::new(-1),
        vl_focus_old: Cell::new(-2),
        vl_total: Cell::new(VL_COUNT),
    });
    // The widgets live as long as the app (the callbacks hold it): the example runs until exit.
    let single = create_list(&app, "lv-single", SelectionMode::Single, "single")?;
    app.single.replace(Some(Rc::new(single)));
    let multi = create_list(&app, "lv-multi", SelectionMode::Multi, "multi")?;
    app.multi.replace(Some(Rc::new(multi)));
    let tree = create_tree(&app, "tree-box", true)?;
    app.tree.replace(Some(Rc::new(tree)));
    let scratch = create_tree(&app, "scratch-tree", false)?;
    app.scratch.replace(Some(Rc::new(scratch)));
    let vlist = create_vlist(&app)?;
    app.vlist.replace(Some(Rc::new(vlist)));

    check_list_view(&app);
    check_tree_nodes(&app);
    check_tree_expand(&app);
    check_tree_selection(&app);
    check_tree_scratch(&app);
    check_vlist(&app);

    let (deferred, platform) = (app.clone(), ctx.platform.clone());
    ctx.platform.set_timeout(150, move || {
        let _ = deferred.tree().scroll_to_node(N_README); // the tree is fully expanded here
        let _ = platform.set_timeout(150, move || phase_tree_scrolled(&deferred));
    })?;

    for (id, handler) in list_buttons()
        .into_iter()
        .chain(tree_buttons())
        .chain(vlist_buttons())
    {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let app = app.clone();
        button.add_event_listener("click", move |_event| handler(&app))?;
    }
    Ok(())
}
