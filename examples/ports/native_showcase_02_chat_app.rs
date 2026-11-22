// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_showcase_02_chat_app.rs; edit that file, not this copy.
//! Host logic of examples/native/showcase/02_chat_app/main.cpp: Send appends the typed text to
//! #msgs as a "msg sent" bubble and clears #msgInput; an empty input sends nothing.
//!
//! The C++ original reads and clears the input through the "value" content attribute, which stays
//! empty while the user types, so it never sends anything. This port reads and clears the live
//! value (`Element::value` / `Element::set_value`).
//! ABI accessor.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let send = ctx
        .document
        .get_element_by_id("sendBtn")?
        .ok_or("page has no #sendBtn")?;
    let doc = ctx.document.clone();

    send.add_event_listener("click", move |_event| {
        let Ok(Some(input)) = doc.get_element_by_id("msgInput") else {
            return;
        };
        let text = input.value().unwrap_or_default();
        if text.is_empty() {
            return;
        }
        let Ok(msg) = doc.create_element("div") else {
            return;
        };
        let _ = msg.set_class_name("msg sent");
        let _ = msg.set_text_content(&text);
        if let Ok(Some(list)) = doc.get_element_by_id("msgs") {
            let _ = list.append_child(&msg);
        }
        let _ = input.set_value("");
    })?;
    Ok(())
}
