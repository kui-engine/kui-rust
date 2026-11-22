// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_05_breadcrumb.rs; edit that file, not this copy.
//! Host logic of examples/native/component/05_breadcrumb/main.cpp: clicking any breadcrumb link
//! (bc0..bc6) writes "Navigated to: <link text>" into the status line.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for i in 0..7 {
        // The original skips missing links.
        let Some(link) = ctx.document.get_element_by_id(&format!("bc{i}"))? else {
            continue;
        };
        let doc = ctx.document.clone();
        link.add_event_listener("click", move |event| {
            let text = match event.target(&doc) {
                Ok(Some(target)) => target.text_content().unwrap_or_default(),
                _ => return,
            };
            if let Ok(Some(status)) = doc.get_element_by_id("status") {
                let _ = status.set_text_content(&format!("Navigated to: {text}"));
            }
        })?;
    }
    Ok(())
}
