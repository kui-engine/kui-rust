// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_html_09_forms.rs; edit that file, not this copy.
//! Host logic of examples/native/html/09_forms/main.cpp: the submit button reads the name input
//! and writes a greeting into the output box.
//!
//! The C++ original reads the input with getAttribute("value"), which is the content attribute and
//! stays empty while the user types, so it always greets "(empty)". This port reads the live value
//! (`Element::value`) so the example does what it means to.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let submit = ctx
        .document
        .get_element_by_id("submitBtn")?
        .ok_or("page has no #submitBtn")?;
    let doc = ctx.document.clone(); // the listener must be 'static, so it keeps its own reference

    submit.add_event_listener("click", move |_event| {
        let typed = match doc.get_element_by_id("nameInput") {
            Ok(Some(input)) => input.value().unwrap_or_default(),
            _ => String::new(),
        };
        let name = if typed.is_empty() {
            "(empty)".to_string()
        } else {
            typed
        };
        if let Ok(Some(output)) = doc.get_element_by_id("output") {
            let _ = output.set_text_content(&format!("Hello, {name}! Form submitted."));
            let _ = output.set_style_property("background", "#dcfce7", false);
            let _ = output.set_style_property("color", "#166534", false);
        }
    })?;
    Ok(())
}
