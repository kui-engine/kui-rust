// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_hello_world.rs; edit that file, not this copy.
//! Host logic of examples/hello_world/main.cpp: #clickBtn increments #counter; the counter turns
//! #f59e0b from 5 clicks and #10b981 from 10.

use crate::canonical_runner::Context;
use std::cell::Cell;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let btn = ctx
        .document
        .get_element_by_id("clickBtn")?
        .ok_or("page has no #clickBtn")?;
    let doc = ctx.document.clone();
    let count = Cell::new(0);
    btn.add_event_listener("click", move |_event| {
        count.set(count.get() + 1);
        let Ok(Some(counter)) = doc.get_element_by_id("counter") else {
            return;
        };
        let _ = counter.set_text_content(&count.get().to_string());
        if count.get() >= 10 {
            let _ = counter.set_style_property("color", "#10b981", false);
        } else if count.get() >= 5 {
            let _ = counter.set_style_property("color", "#f59e0b", false);
        }
    })?;
    Ok(())
}
