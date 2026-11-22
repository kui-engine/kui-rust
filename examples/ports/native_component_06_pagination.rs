// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_06_pagination.rs; edit that file, not this copy.
//! Host logic of examples/native/component/06_pagination/main.cpp: page buttons 1..5 and the
//! prev / next arrows move the current page and refresh the buttons, content and info line.

use crate::canonical_runner::Context;
use kui::Document;
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;

fn update(doc: &Document, page: i32) {
    let set_class = |id: &str, class: &str| {
        if let Ok(Some(el)) = doc.get_element_by_id(id) {
            let _ = el.set_class_name(class);
        }
    };
    for i in 1..=5 {
        set_class(
            &format!("pg{i}"),
            if i == page { "pg active" } else { "pg" },
        );
    }
    if let Ok(Some(el)) = doc.get_element_by_id("content") {
        let html = format!(
            "<p>Page {page}: Items {}-{} of 50.</p>",
            (page - 1) * 10 + 1,
            page * 10
        );
        let _ = el.set_inner_html(&html);
    }
    if let Ok(Some(el)) = doc.get_element_by_id("info") {
        let _ = el.set_text_content(&format!("Page {page} of 5"));
    }
    set_class("prev", if page <= 1 { "pg disabled" } else { "pg" });
    set_class("next", if page >= 5 { "pg disabled" } else { "pg" });
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let page = Rc::new(Cell::new(1));
    // Each button maps the current page to the new one (None = no change).
    let mut buttons: Vec<(String, Box<dyn Fn(i32) -> Option<i32>>)> = Vec::new();
    for i in 1..=5 {
        buttons.push((format!("pg{i}"), Box::new(move |_| Some(i))));
    }
    buttons.push(("prev".into(), Box::new(|p| (p > 1).then(|| p - 1))));
    buttons.push(("next".into(), Box::new(|p| (p < 5).then(|| p + 1))));
    for (id, step) in buttons {
        let el = ctx
            .document
            .get_element_by_id(&id)?
            .ok_or("page is missing a pager button")?;
        let doc = ctx.document.clone();
        let page = page.clone();
        el.add_event_listener("click", move |_event| {
            if let Some(next) = step(page.get()) {
                page.set(next);
                update(&doc, next);
            }
        })?;
    }
    Ok(())
}
