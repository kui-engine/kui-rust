// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_01_tabs.rs; edit that file, not this copy.
//! Host logic of examples/native/component/01_tabs/main.cpp: clicking tab N marks tab N and
//! panel N active and every other tab / panel inactive.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for i in 0..4 {
        let tab = ctx
            .document
            .get_element_by_id(&format!("tab{i}"))?
            .ok_or("page has no tab")?;
        let doc = ctx.document.clone();
        tab.add_event_listener("click", move |_event| {
            for j in 0..4 {
                let (tab_class, panel_class) = if j == i {
                    ("tab active", "panel active")
                } else {
                    ("tab", "panel")
                };
                if let Ok(Some(el)) = doc.get_element_by_id(&format!("tab{j}")) {
                    let _ = el.set_class_name(tab_class);
                }
                if let Ok(Some(el)) = doc.get_element_by_id(&format!("p{j}")) {
                    let _ = el.set_class_name(panel_class);
                }
            }
        })?;
    }
    Ok(())
}
