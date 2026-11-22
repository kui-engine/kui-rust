// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_showcase_04_settings_page.rs; edit that file, not this copy.
//! Host logic of examples/native/showcase/04_settings_page/main.cpp: clicking a switch #t0..#t2
//! flips its class between "toggle on" and "toggle".

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for id in ["t0", "t1", "t2"] {
        let toggle = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let doc = ctx.document.clone();
        toggle.add_event_listener("click", move |_event| {
            if let Ok(Some(el)) = doc.get_element_by_id(id) {
                // Same test as the original: className().find("on").
                let on = el.class_name().unwrap_or_default().contains("on");
                let _ = el.set_class_name(if on { "toggle" } else { "toggle on" });
            }
        })?;
    }
    Ok(())
}
