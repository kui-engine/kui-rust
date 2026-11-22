// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_animation_02_transition_timing.rs; edit that file, not this copy.
//! Host logic of examples/native/animation/02_transition_timing/main.cpp: #goBtn flips the six
//! dots #d1..#d6 between left: calc(100% - 20px) and left: 0, so each runs its own transition.

use crate::canonical_runner::Context;
use std::cell::Cell;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let button = ctx
        .document
        .get_element_by_id("goBtn")?
        .ok_or("page has no #goBtn")?;
    let doc = ctx.document.clone();
    let at_end = Cell::new(false);
    button.add_event_listener("click", move |_event| {
        at_end.set(!at_end.get());
        let left = if at_end.get() {
            "calc(100% - 20px)"
        } else {
            "0"
        };
        for i in 1..=6 {
            if let Ok(Some(dot)) = doc.get_element_by_id(&format!("d{i}")) {
                let _ = dot.set_style_property("left", left, false);
            }
        }
    })?;
    Ok(())
}
