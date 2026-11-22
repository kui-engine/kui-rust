// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_devtools_01_console_log.rs; edit that file, not this copy.
//! Host logic of examples/native/devtools/01_console_log/main.cpp: Log Info / Warn / Error append
//! a "log li|lw|le" row to #output (Info reads "User action #<rows added so far>"), Clear empties
//! it; F12 DevTools is enabled and the window title gets the live DevTools URL appended.

use crate::canonical_runner::Context;
use kui::Document;
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;

fn add_log(doc: &Document, count: &Cell<i32>, cls: &str, msg: &str) {
    if let Ok(Some(output)) = doc.get_element_by_id("output") {
        if let Ok(entry) = doc.create_element("div") {
            let _ = entry.set_class_name(&format!("log {cls}"));
            let _ = entry.set_text_content(msg);
            let _ = output.append_child(&entry);
        }
    }
    count.set(count.get() + 1);
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    ctx.platform.set_f12_devtools_enabled(true)?;
    // devtools_runtime.h startLiveDevTools: the runner already started DevTools; show its URL.
    let title = ctx.window.title()?;
    match ctx.window.devtools_url()? {
        Some(url) if !url.is_empty() => ctx.window.set_title(&format!("{title} — {url}"))?,
        _ => ctx
            .window
            .set_title(&format!("{title} — DevTools failed to start"))?,
    }

    let count = Rc::new(Cell::new(0));
    let handlers: [(&str, fn(&Document, &Cell<i32>)); 4] = [
        ("btnI", |doc, count| {
            add_log(doc, count, "li", &format!("User action #{}", count.get()))
        }),
        ("btnW", |doc, count| {
            add_log(doc, count, "lw", "Warning: potential issue")
        }),
        ("btnE", |doc, count| {
            add_log(doc, count, "le", "Error: operation failed")
        }),
        ("btnC", |doc, _count| {
            if let Ok(Some(output)) = doc.get_element_by_id("output") {
                let _ = output.set_inner_html("");
            }
        }),
    ];
    for (id, handler) in handlers {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let (doc, count) = (ctx.document.clone(), count.clone());
        button.add_event_listener("click", move |_event| handler(&doc, &count))?;
    }
    Ok(())
}
