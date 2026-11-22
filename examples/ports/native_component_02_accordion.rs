// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_02_accordion.rs; edit that file, not this copy.
//! Host logic of examples/native/component/02_accordion/main.cpp: clicking header N closes every
//! section and reopens section N unless it was already open.

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

fn set_class(doc: &Document, id: &str, class: &str) {
    if let Ok(Some(el)) = doc.get_element_by_id(id) {
        let _ = el.set_class_name(class);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for i in 0..4 {
        let header = ctx
            .document
            .get_element_by_id(&format!("h{i}"))?
            .ok_or("page has no accordion header")?;
        let doc = ctx.document.clone();
        header.add_event_listener("click", move |_event| {
            let was_open = match doc.get_element_by_id(&format!("h{i}")) {
                Ok(Some(el)) => el.class_name().map(|c| c.contains("open")).unwrap_or(false),
                _ => false,
            };
            for j in 0..4 {
                set_class(&doc, &format!("h{j}"), "ah");
                set_class(&doc, &format!("b{j}"), "ab");
            }
            if !was_open {
                set_class(&doc, &format!("h{i}"), "ah open");
                set_class(&doc, &format!("b{i}"), "ab open");
            }
        })?;
    }
    Ok(())
}
