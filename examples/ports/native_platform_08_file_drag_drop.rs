// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_08_file_drag_drop.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/08_file_drag_drop/main.cpp: dragenter / dragover on the
//! root element highlight #zone (class "over"), dragleave removes it, and a drop of OS files
//! lists their full UTF-8 paths in #list.

use crate::canonical_runner::Context;
use kui::{Document, Event, EventTypeTag};
use std::error::Error;

fn zone_over(doc: &Document, on: bool) {
    if let Ok(Some(zone)) = doc.get_element_by_id("zone") {
        let _ = zone.class_list_remove("over");
        if on {
            let _ = zone.class_list_add("over");
        }
    }
}

fn highlight(doc: &Document, event: &Event<'_>) {
    let _ = event.prevent_default(); // allows the drop
    zone_over(doc, true);
}

fn leave(doc: &Document, _event: &Event<'_>) {
    zone_over(doc, false);
}

fn drop_files(doc: &Document, event: &Event<'_>) {
    zone_over(doc, false);
    if !matches!(event.type_tag(), Ok(EventTypeTag::Drag)) {
        return;
    }
    let files = match event.drag_files() {
        Ok(files) if !files.is_empty() => files,
        _ => return,
    };
    let _ = event.prevent_default();
    let mut html = format!(
        "<div class=\"row count\">收到 {} 个项目:</div>",
        files.len()
    );
    for path in &files {
        html += &format!("<div class=\"row\">{path}</div>");
    }
    if let Ok(Some(list)) = doc.get_element_by_id("list") {
        let _ = list.set_inner_html(&html);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let root = ctx.document.root()?.ok_or("page has no root element")?; // whole-window target
    let handlers: [(&str, fn(&Document, &Event<'_>)); 4] = [
        ("dragover", highlight),
        ("dragenter", highlight),
        ("dragleave", leave),
        ("drop", drop_files),
    ];
    for (name, handler) in handlers {
        let doc = ctx.document.clone();
        root.add_event_listener(name, move |event| handler(&doc, event))?;
    }
    Ok(())
}
