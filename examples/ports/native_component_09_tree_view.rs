// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_09_tree_view.rs; edit that file, not this copy.
//! Host logic of examples/native/component/09_tree_view/main.cpp: clicking folder label fN toggles
//! "open" on its children list cN; clicking a file (i0..i5) writes "Selected: <file>".

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for f in 0..3 {
        let label = ctx
            .document
            .get_element_by_id(&format!("f{f}"))?
            .ok_or("page is missing a folder label")?;
        let doc = ctx.document.clone();
        label.add_event_listener("click", move |_event| {
            if let Ok(Some(children)) = doc.get_element_by_id(&format!("c{f}")) {
                let open = children
                    .class_name()
                    .map(|c| c.contains("open"))
                    .unwrap_or(false);
                let _ = children.set_class_name(if open { "children" } else { "children open" });
            }
        })?;
    }
    for i in 0..6 {
        // The original skips missing files.
        let Some(file) = ctx.document.get_element_by_id(&format!("i{i}"))? else {
            continue;
        };
        let doc = ctx.document.clone();
        file.add_event_listener("click", move |event| {
            let text = match event.target(&doc) {
                Ok(Some(target)) => target.text_content().unwrap_or_default(),
                _ => return,
            };
            if let Ok(Some(status)) = doc.get_element_by_id("status") {
                let _ = status.set_text_content(&format!("Selected: {text}"));
            }
        })?;
    }
    Ok(())
}
