// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_04_custom_element.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/04_custom_element/main.cpp: defines kui-card and
//! kui-badge with lifecycle counters before the page loads (before_load), so parsing the page
//! runs the callbacks as in the original; once loaded, marks the first badge with
//! data-live="registered" and writes the counters into #lifecycle.

use crate::canonical_runner::Context;
use kui::CustomElementDefinition;
use std::cell::Cell;
use std::error::Error;

thread_local! {
    static CONNECTED: Cell<u32> = const { Cell::new(0) };
    static DISCONNECTED: Cell<u32> = const { Cell::new(0) };
    static ATTRIBUTES: Cell<u32> = const { Cell::new(0) };
}

fn bump(counter: &'static std::thread::LocalKey<Cell<u32>>) {
    counter.with(|c| c.set(c.get() + 1));
}

pub fn before_load(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let mut doc = ctx.document.clone();
    for tag in ["kui-card", "kui-badge"] {
        doc.define_custom_element(
            tag,
            CustomElementDefinition {
                connected: Some(Box::new(|_| bump(&CONNECTED))),
                disconnected: Some(Box::new(|_| bump(&DISCONNECTED))),
                attribute_changed: Some(Box::new(|_, _, _, _| bump(&ATTRIBUTES))),
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document;
    if let Some(badge) = doc.query_selector("kui-badge")? {
        badge.set_attribute("data-live", "registered")?;
    }
    doc.get_element_by_id("lifecycle")?
        .ok_or("page has no #lifecycle")?
        .set_text_content(&format!(
            "Registry definitions: kui-card, kui-badge · connected={} · disconnected={} · attribute callbacks={}",
            CONNECTED.with(Cell::get),
            DISCONNECTED.with(Cell::get),
            ATTRIBUTES.with(Cell::get)
        ))?;
    Ok(())
}
