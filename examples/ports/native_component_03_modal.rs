// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_03_modal.rs; edit that file, not this copy.
//! Host logic of examples/native/component/03_modal/main.cpp: the two open buttons fill and show
//! the modal; Cancel / Confirm hide it and report the choice in the status line.

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(el)) = doc.get_element_by_id(id) {
        let _ = el.set_text_content(text);
    }
}

fn set_class(doc: &Document, id: &str, class: &str) {
    if let Ok(Some(el)) = doc.get_element_by_id(id) {
        let _ = el.set_class_name(class);
    }
}

fn on_click(
    ctx: &Context<'_>,
    id: &str,
    handler: impl Fn(&Document) + 'static,
) -> Result<(), Box<dyn Error>> {
    let el = ctx
        .document
        .get_element_by_id(id)?
        .ok_or("page is missing a modal button")?;
    let doc = ctx.document.clone();
    el.add_event_listener("click", move |_event| handler(&doc))?;
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on_click(ctx, "openC", |doc| {
        set_text(doc, "mT", "Confirm Action");
        set_text(doc, "mM", "Are you sure you want to proceed?");
        set_class(doc, "okBtn", "btn bp");
        set_class(doc, "ov", "overlay show");
    })?;
    on_click(ctx, "openD", |doc| {
        set_text(doc, "mT", "Delete Item");
        set_text(doc, "mM", "This cannot be undone. Delete permanently?");
        set_class(doc, "okBtn", "btn bd");
        set_class(doc, "ov", "overlay show");
    })?;
    on_click(ctx, "canBtn", |doc| {
        set_class(doc, "ov", "overlay");
        set_text(doc, "status", "Cancelled.");
    })?;
    on_click(ctx, "okBtn", |doc| {
        set_class(doc, "ov", "overlay");
        set_text(doc, "status", "Confirmed!");
    })?;
    Ok(())
}
