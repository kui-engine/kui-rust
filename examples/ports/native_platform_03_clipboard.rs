// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_03_clipboard.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/03_clipboard/main.cpp: Copy puts #copyInput's text on
//! the system clipboard; Paste shows the clipboard text in #pasteResult ("(empty)" when none).
//!
//! The original reads getAttribute("value"), which never sees what the user typed; this port
//! copies the live value (`Element::value`), which is what the original means to do.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let copy = ctx
        .document
        .get_element_by_id("copyBtn")?
        .ok_or("page has no #copyBtn")?;
    let paste = ctx
        .document
        .get_element_by_id("pasteBtn")?
        .ok_or("page has no #pasteBtn")?;

    let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
    copy.add_event_listener("click", move |_event| {
        let value = match doc.get_element_by_id("copyInput") {
            Ok(Some(input)) => input.value().unwrap_or_default(),
            _ => String::new(),
        };
        let _ = platform.set_clipboard_text(&value);
    })?;

    let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
    paste.add_event_listener("click", move |_event| {
        let text = platform.clipboard_text().unwrap_or_default();
        if let Ok(Some(result)) = doc.get_element_by_id("pasteResult") {
            let _ = result.set_text_content(if text.is_empty() { "(empty)" } else { &text });
        }
    })?;
    Ok(())
}
