// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_08_tag_input.rs; edit that file, not this copy.
//! Host logic of examples/native/component/08_tag_input/main.cpp: clicking a suggestion (s0..s5)
//! appends a new <div class="tag"> with the suggestion's text to #tags.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for i in 0..6 {
        let suggestion = ctx
            .document
            .get_element_by_id(&format!("s{i}"))?
            .ok_or("page is missing a suggestion")?;
        let doc = ctx.document.clone();
        suggestion.add_event_listener("click", move |event| {
            let text = match event.target(&doc) {
                Ok(Some(target)) => target.text_content().unwrap_or_default(),
                _ => return,
            };
            let (Ok(Some(container)), Ok(tag)) =
                (doc.get_element_by_id("tags"), doc.create_element("div"))
            else {
                return;
            };
            let _ = tag.set_class_name("tag");
            let _ = tag.set_text_content(&text);
            let _ = container.append_child(&tag);
        })?;
    }
    Ok(())
}
