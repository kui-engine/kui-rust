// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_03_class_toggling.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/03_class_toggling/main.cpp: three buttons toggle the
//! expanded / highlight / danger classes on the panel and echo its class list.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for (id, class_name) in [
        ("toggleExpand", "expanded"),
        ("toggleHighlight", "highlight"),
        ("toggleDanger", "danger"),
    ] {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let doc = ctx.document.clone();
        button.add_event_listener("click", move |_event| {
            let Ok(Some(panel)) = doc.get_element_by_id("panel") else {
                return;
            };
            let _ = panel.class_list_toggle(class_name);
            let current = panel.class_name().unwrap_or_default();
            if let Ok(Some(classes)) = doc.get_element_by_id("classes") {
                let _ = classes.set_text_content(&format!("Classes: {current}"));
            }
        })?;
    }
    Ok(())
}
