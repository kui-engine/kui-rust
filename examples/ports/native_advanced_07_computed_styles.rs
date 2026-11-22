// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_07_computed_styles.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/07_computed_styles/main.cpp: the Run Demo button writes
//! a success message into the output box and turns it green.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let action = ctx
        .document
        .get_element_by_id("actionBtn")?
        .ok_or("page has no #actionBtn")?;
    let doc = ctx.document.clone();

    action.add_event_listener("click", move |_event| {
        if let Ok(Some(output)) = doc.get_element_by_id("output") {
            let _ = output.set_text_content("Reading Computed Styles demo executed successfully!");
            let _ = output.set_style_property("background", "#dcfce7", false);
            let _ = output.set_style_property("color", "#166534", false);
        }
    })?;
    Ok(())
}
