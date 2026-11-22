// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_html_21_html_loader.rs; edit that file, not this copy.
//! Host logic of examples/native/html/21_html_loader/main.cpp: once the inline page is loaded,
//! #status reports it.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    ctx.document
        .get_element_by_id("status")?
        .ok_or("page has no #status")?
        .set_text_content("Inline HTML loaded successfully.")?;
    Ok(())
}
