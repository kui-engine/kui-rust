// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_01_window_resize.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/01_window_resize/main.cpp: #size shows the live client
//! size, once at start and again on every window resize.

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

fn update_size(doc: &Document, width: u32, height: u32) {
    if let Ok(Some(size)) = doc.get_element_by_id("size") {
        let _ = size.set_text_content(&format!("Live client size: {width} × {height} CSS px"));
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let (width, height) = ctx.window.size()?;
    update_size(ctx.document, width as u32, height as u32);
    let doc = ctx.document.clone();
    ctx.window
        .on_resize(move |width, height| update_size(&doc, width, height))?;
    Ok(())
}
