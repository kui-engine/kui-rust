// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_02_event_handling.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/02_event_handling/main.cpp: click / mouseenter /
//! mouseleave on the target each append a log entry (click with its client coordinates) and
//! scroll the log to the bottom.

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

fn add_log(doc: &Document, kind: &str, detail: &str) {
    let Ok(Some(log)) = doc.get_element_by_id("log") else {
        return;
    };
    let Ok(entry) = doc.create_element("div") else {
        return;
    };
    let _ = entry.set_class_name("log-entry");
    let _ = entry.set_inner_html(&format!("<span class=\"type\">{kind}</span> {detail}"));
    let _ = log.append_child(&entry);
    if let Ok(metrics) = log.scroll_metrics() {
        let _ = log.set_scroll_position(metrics.left, metrics.height);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let target = ctx
        .document
        .get_element_by_id("target")?
        .ok_or("page has no #target")?;

    let doc = ctx.document.clone();
    target.add_event_listener("click", move |event| {
        let point = event
            .mouse_client()
            .unwrap_or(kui::Point { x: 0.0, y: 0.0 });
        add_log(
            &doc,
            "click",
            &format!("x={} y={}", point.x as i32, point.y as i32),
        );
    })?;
    let doc = ctx.document.clone();
    target.add_event_listener("mouseenter", move |_event| add_log(&doc, "mouseenter", ""))?;
    let doc = ctx.document.clone();
    target.add_event_listener("mouseleave", move |_event| add_log(&doc, "mouseleave", ""))?;
    Ok(())
}
