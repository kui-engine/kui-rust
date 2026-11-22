// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_html_16_css_variables.rs; edit that file, not this copy.
//! Host logic of examples/native/html/16_css_variables/main.cpp: #themeBtn toggles the theme-dark
//! class on <body>, which redefines the CSS variables.

use crate::canonical_runner::Context;
use std::cell::Cell;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let button = ctx
        .document
        .get_element_by_id("themeBtn")?
        .ok_or("page has no #themeBtn")?;
    let doc = ctx.document.clone();
    let dark = Cell::new(false);
    button.add_event_listener("click", move |_event| {
        dark.set(!dark.get());
        if let Ok(Some(body)) = doc.body() {
            let _ = if dark.get() {
                body.class_list_add("theme-dark")
            } else {
                body.class_list_remove("theme-dark")
            };
        }
    })?;
    Ok(())
}
