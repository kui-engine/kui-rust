// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_virtual_list_example.rs; edit that file, not this copy.
//! Host logic of examples/virtual_list_example/main.cpp: a 100,000-row VirtualList in
//! #vlist-container (32px rows, overscan 5) with insert / remove / scroll buttons, a 50,000-item
//! single-selection ListView in #listview-container, and the performance stats in #stats-content.

use crate::canonical_runner::Context;
use kui::{
    Document, Element, ListView, ListViewConfig, ListViewItem, SelectionMode, VirtualList,
    VirtualListConfig,
};
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct Stats {
    setup_ms: Cell<f64>,
    insert_ms: Cell<f64>,
    insert_ops: Cell<i32>,
    insert_count: Cell<u32>,
    rng: Cell<u64>,
}

/// The original's createElement + setTextContent + style()->setProperty.
fn make_el<'a>(
    doc: &'a Document,
    tag: &str,
    text: &str,
    styles: &[(&str, &str)],
) -> Option<Element<'a>> {
    let el = doc.create_element(tag).ok()?;
    if !text.is_empty() {
        let _ = el.set_text_content(text);
    }
    for (name, value) in styles {
        let _ = el.set_style_property(name, value, false);
    }
    Some(el)
}

fn append(parent: &Element<'_>, child: Option<Element<'_>>) {
    if let Some(child) = child {
        let _ = parent.append_child(&child);
    }
}

fn render_row(doc: &Document, index: u32) -> Option<Element<'_>> {
    let odd = index % 2 != 0;
    let row = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "flex"),
            ("align-items", "center"),
            ("padding", "0 12px"),
            ("font-size", "13px"),
            ("color", if odd { "#94a3b8" } else { "#cbd5e1" }),
            (
                "background",
                if odd {
                    "transparent"
                } else {
                    "rgba(255,255,255,0.02)"
                },
            ),
            ("font-family", "'Consolas', monospace"),
        ],
    )?;
    append(
        &row,
        make_el(
            doc,
            "span",
            &index.to_string(),
            &[
                ("color", "#475569"),
                ("width", "70px"),
                ("font-size", "11px"),
                ("flex-shrink", "0"),
            ],
        ),
    );
    let text = format!(
        "Item #{index}{}{}",
        if index % 1000 == 0 {
            "  ** milestone **"
        } else {
            ""
        },
        if index % 10000 == 0 {
            "  >>> MAJOR <<<"
        } else {
            ""
        }
    );
    append(&row, make_el(doc, "span", &text, &[("flex", "1")]));
    append(
        &row,
        make_el(
            doc,
            "span",
            if odd { "odd" } else { "even" },
            &[
                ("font-size", "10px"),
                ("color", "#475569"),
                ("flex-shrink", "0"),
            ],
        ),
    );
    Some(row)
}

fn update_stats_display(doc: &Document, list: &VirtualList, stats: &Stats) {
    let Ok(Some(el)) = doc.get_element_by_id("stats-content") else {
        return;
    };
    let _ = el.set_text_content(""); // clear existing content
    let add_line = |text: &str, color: &str| {
        append(
            &el,
            make_el(
                doc,
                "div",
                text,
                &[
                    ("color", color),
                    ("font-family", "'Consolas', monospace"),
                    ("font-size", "12px"),
                    ("margin-bottom", "2px"),
                ],
            ),
        );
    };
    let pick = |good: bool| if good { "#4ade80" } else { "#f87171" };
    let Ok(state) = list.state() else { return };
    add_line(&format!("Total items: {}", state.total_items), "#38bdf8");
    add_line(
        &format!(
            "Visible range: [{}, {}) ({} rendered)",
            state.visible_start,
            state.visible_end,
            state.visible_end - state.visible_start
        ),
        "#94a3b8",
    );
    add_line(
        &format!("Recycle pool: {} elements", state.recycle_pool_size),
        "#94a3b8",
    );
    add_line(
        &format!("Total height: {:.0} px", state.total_height),
        "#94a3b8",
    );
    let setup = stats.setup_ms.get();
    add_line(&format!("Setup time: {setup:.2} ms"), pick(setup < 16.0));
    if stats.insert_ops.get() > 0 {
        let insert = stats.insert_ms.get();
        add_line(
            &format!(
                "Last insert: {insert:.2} ms ({} ops, {} items added)",
                stats.insert_ops.get(),
                stats.insert_count.get()
            ),
            pick(insert < 50.0),
        );
    }
}

fn total_items(list: &VirtualList) -> u32 {
    list.state().map(|s| s.total_items).unwrap_or(0)
}

fn timed_insert(doc: &Document, list: &VirtualList, stats: &Stats, index: u32, at: &str) {
    let t0 = Instant::now();
    let _ = list.insert_items(index, 1000);
    stats.insert_count.set(stats.insert_count.get() + 1000);
    stats.insert_ms.set(t0.elapsed().as_secs_f64() * 1000.0);
    stats.insert_ops.set(stats.insert_ops.get() + 1);
    update_stats_display(doc, list, stats);
    eprintln!(
        "[VList] Inserted 1000 items at {at}, took {:.2} ms",
        stats.insert_ms.get()
    );
}

type Handler = fn(&Document, &VirtualList, &Stats);

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document;

    // --- VirtualList: 100,000 rows ---
    let container = doc
        .get_element_by_id("vlist-container")?
        .ok_or("page has no #vlist-container")?;
    let t0 = Instant::now();
    let list = Rc::new(VirtualList::new(
        &container,
        VirtualListConfig {
            total_items: 100000,
            item_height: 32.0,
            overscan: 5,
            render_item: Some(Box::new(render_row)),
            ..Default::default()
        },
    )?);
    let stats = Rc::new(Stats::default());
    stats.setup_ms.set(t0.elapsed().as_secs_f64() * 1000.0);
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    stats.rng.set(seed | 1);
    eprintln!("[VList] Setup 100K items in {:.2} ms", stats.setup_ms.get());

    // --- Button handlers (wired before the lists are set up in the original; same effect) ---
    let handlers: [(&str, Handler); 6] = [
        ("btn-insert-mid", |d, l, s| {
            let mid = total_items(l) / 2;
            timed_insert(d, l, s, mid, &mid.to_string());
        }),
        ("btn-insert-begin", |d, l, s| timed_insert(d, l, s, 0, "0")),
        ("btn-insert-end", |d, l, s| {
            timed_insert(d, l, s, total_items(l), "end")
        }),
        ("btn-remove-mid", |d, l, s| {
            if total_items(l) < 1000 {
                return;
            }
            let t0 = Instant::now();
            let mid = total_items(l) / 2;
            let _ = l.remove_items(mid, 1000);
            s.insert_ms.set(t0.elapsed().as_secs_f64() * 1000.0);
            s.insert_ops.set(s.insert_ops.get() + 1);
            update_stats_display(d, l, s);
            eprintln!(
                "[VList] Removed 1000 items at {mid}, took {:.2} ms",
                s.insert_ms.get()
            );
        }),
        ("btn-scroll-random", |d, l, s| {
            let total = total_items(l);
            if total == 0 {
                return;
            }
            // xorshift64: std has no RNG; the original uses std::mt19937.
            let mut x = s.rng.get();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            s.rng.set(x);
            let idx = (x % u64::from(total)) as u32;
            let _ = l.scroll_to_index(idx);
            update_stats_display(d, l, s);
            eprintln!("[VList] Scrolled to index {idx}");
        }),
        ("btn-refresh-stats", update_stats_display),
    ];
    for (id, handler) in handlers {
        let button = doc
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let (d, l, s) = (doc.clone(), list.clone(), stats.clone());
        button.add_event_listener("click", move |_event| handler(&d, &l, &s))?;
    }

    // --- ListView: 50,000 items with selection ---
    const ICONS: [&str; 6] = [
        "\u{1F4C4}",
        "\u{1F4C1}",
        "\u{1F527}",
        "\u{1F4E6}",
        "\u{1F310}",
        "\u{26A1}",
    ];
    let lv_container = doc
        .get_element_by_id("listview-container")?
        .ok_or("page has no #listview-container")?;
    let list_view = ListView::new(
        &lv_container,
        ListViewConfig {
            item_count: 50000,
            item_height: 36.0,
            overscan: 5,
            selection_mode: SelectionMode::Single,
            item_provider: Some(Box::new(|index| ListViewItem {
                icon: ICONS[(index % 6) as usize].to_string(),
                text: format!("List item {index}"),
                subtitle: format!("Description for item {index}"),
                disabled: index % 50 == 49,
            })),
            on_select: Some(Box::new(|index| eprintln!("[ListView] Selected: {index}"))),
            ..Default::default()
        },
    )?;
    // The original keeps the ListView in a global for the whole run.
    std::mem::forget(list_view);

    update_stats_display(doc, &list, &stats); // initial stats display
    Ok(())
}
