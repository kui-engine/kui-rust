// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_12_tooltip_and_tabindex.rs; edit that file, not this copy.
//! Host logic of examples/native/component/12_tooltip_and_tabindex/main.cpp: logs focus / blur of
//! every .fcard and Tab / Shift+Tab keydowns at the root into #log (newest line first). The title
//! tooltips and the Tab focus walk are engine-native and need no host code.

use crate::canonical_runner::Context;
use kui::{Document, EventTypeTag};
use std::error::Error;

fn log_line(doc: &Document, line: &str) {
    if let Ok(Some(log)) = doc.get_element_by_id("log") {
        let old = log.text_content().unwrap_or_default();
        let _ = log.set_text_content(&format!("{line}\n{old}"));
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    for card in ctx.document.query_selector_all(".fcard")? {
        let name = card.attribute("data-name").unwrap_or_default();
        let (doc, focus_line) = (ctx.document.clone(), format!("[focus] {name}"));
        card.add_event_listener("focus", move |_event| log_line(&doc, &focus_line))?;
        let (doc, blur_line) = (ctx.document.clone(), format!("[blur]  {name}"));
        card.add_event_listener("blur", move |_event| log_line(&doc, &blur_line))?;
    }

    let root = ctx.document.root()?.ok_or("document has no root element")?;
    let doc = ctx.document.clone();
    root.add_event_listener("keydown", move |event| {
        if !matches!(event.type_tag(), Ok(EventTypeTag::Keyboard)) {
            return;
        }
        if event.key().map(|key| key != "Tab").unwrap_or(true) {
            return;
        }
        let shift = event.modifiers().map(|m| m.shift).unwrap_or(false);
        log_line(
            &doc,
            if shift {
                "[keydown] Shift+Tab -> focus moves back"
            } else {
                "[keydown] Tab -> focus advances"
            },
        );
    })?;
    Ok(())
}
