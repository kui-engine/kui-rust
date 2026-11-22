// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_10_list_view.rs; edit that file, not this copy.
//! Host logic of examples/native/component/10_list_view/main.cpp: clicking row N selects it (only
//! that row gets "li sel") and writes "Selected row N+1" into the status bar.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for i in 0..5 {
        let row = ctx
            .document
            .get_element_by_id(&format!("r{i}"))?
            .ok_or("page is missing a list row")?;
        let doc = ctx.document.clone();
        row.add_event_listener("click", move |_event| {
            for j in 0..5 {
                if let Ok(Some(el)) = doc.get_element_by_id(&format!("r{j}")) {
                    let _ = el.set_class_name("li");
                }
            }
            if let Ok(Some(el)) = doc.get_element_by_id(&format!("r{i}")) {
                let _ = el.set_class_name("li sel");
            }
            if let Ok(Some(status)) = doc.get_element_by_id("status") {
                let _ = status.set_text_content(&format!("Selected row {}", i + 1));
            }
        })?;
    }
    Ok(())
}
