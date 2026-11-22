// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_05_virtual_list.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/05_virtual_list/main.cpp: a 10,000-row VirtualList in
//! #listContainer (42px rows, overscan 5), Top / Item 5,000 / Bottom buttons, and a 250 ms
//! interval that reports the live DOM range in #stats.

use crate::canonical_runner::Context;
use kui::{Document, VirtualList, VirtualListConfig};
use std::error::Error;
use std::rc::Rc;

fn update_stats(doc: &Document, list: &VirtualList) {
    let Ok(state) = list.state() else { return };
    if let Ok(Some(stats)) = doc.get_element_by_id("stats") {
        let _ = stats.set_text_content(&format!(
            "10,000 logical items · live DOM range [{}, {}) · {} rows rendered",
            state.visible_start,
            state.visible_end,
            state.visible_end - state.visible_start
        ));
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let container = ctx
        .document
        .get_element_by_id("listContainer")?
        .ok_or("page has no #listContainer")?;
    let list = VirtualList::new(
        &container,
        VirtualListConfig {
            total_items: 10000,
            item_height: 42.0,
            overscan: 5,
            render_item: Some(Box::new(|item_doc: &Document, index| {
                let row = item_doc.create_element("div").ok()?;
                let _ = row.set_class_name("row");
                let _ = row.set_style_property("height", "42px", false);
                let _ = row.set_inner_html(&format!(
                    "<span class=\"idx\">#{}</span>Virtualized item {}",
                    index + 1,
                    index + 1
                ));
                Some(row)
            })),
            ..Default::default()
        },
    )?;
    let list = Rc::new(list);

    let actions: [(&str, fn(&VirtualList) -> kui::Result<()>); 3] = [
        ("topBtn", |list| list.scroll_to_top()),
        ("middleBtn", |list| list.scroll_to_index(4999)),
        ("bottomBtn", |list| list.scroll_to_bottom()),
    ];
    for (id, action) in actions {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let (doc, list) = (ctx.document.clone(), list.clone());
        button.add_event_listener("click", move |_event| {
            let _ = action(&list);
            update_stats(&doc, &list);
        })?;
    }
    // Like the original, the interval runs until the example exits.
    let doc = ctx.document.clone();
    ctx.platform
        .set_interval(250, move || update_stats(&doc, &list))?;
    Ok(())
}
