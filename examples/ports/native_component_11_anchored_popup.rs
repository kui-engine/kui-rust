// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_11_anchored_popup.rs; edit that file, not this copy.
//! Host logic of examples/native/component/11_anchored_popup/main.cpp: each .anchor button shows
//! #popup next to itself with its data-place placement (position_relative_to, offset 6, C++
//! defaults otherwise); clicking the same button again, or anywhere outside the anchors and the
//! popup, hides it.

use crate::canonical_runner::Context;
use kui::{Document, PositioningOptions};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

fn hide_popup(doc: &Document, open_anchor: &RefCell<String>) {
    if let Ok(Some(popup)) = doc.get_element_by_id("popup") {
        let _ = popup.set_style_property("display", "none", false);
    }
    open_anchor.borrow_mut().clear();
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let open_anchor = Rc::new(RefCell::new(String::new())); // "" while hidden

    for id in ["a-bottom", "a-top", "a-right", "a-left", "a-corner"] {
        // The original skips missing anchors.
        let Some(button) = ctx.document.get_element_by_id(id)? else {
            continue;
        };
        let doc = ctx.document.clone();
        let open_anchor = open_anchor.clone();
        button.add_event_listener("click", move |_event| {
            if *open_anchor.borrow() == id {
                hide_popup(&doc, &open_anchor);
                return;
            }
            let (Ok(Some(popup)), Ok(Some(anchor))) =
                (doc.get_element_by_id("popup"), doc.get_element_by_id(id))
            else {
                return;
            };
            let place = anchor.attribute("data-place").unwrap_or_default();
            if let Ok(Some(label)) = doc.get_element_by_id("pp-place") {
                let _ = label.set_text_content(&place);
            }
            // Show first: positioning is a no-op while the popup is display:none.
            let _ = popup.set_style_property("display", "block", false);
            let options = PositioningOptions {
                flip: true,
                clamp_to_viewport: true,
                offset: 6.0,
                viewport_margin: 8.0, // the C++ default (Rust's Default uses 0)
            };
            let _ = popup.position_relative_to(&anchor, &place, &options);
            *open_anchor.borrow_mut() = id.to_string();
        })?;
    }

    let root = ctx.document.root()?.ok_or("document has no root element")?;
    let doc = ctx.document.clone();
    root.add_event_listener("click", move |event| {
        if open_anchor.borrow().is_empty() {
            return;
        }
        // A non-element target cannot be walked; leave the popup alone.
        let Ok(Some(mut node)) = event.target(&doc) else {
            return;
        };
        loop {
            let is_popup = node.id().map(|id| id == "popup").unwrap_or(false);
            if is_popup || node.class_list_contains("anchor").unwrap_or(false) {
                return;
            }
            match node.parent() {
                Ok(Some(parent)) => node = parent,
                _ => break,
            }
        }
        hide_popup(&doc, &open_anchor);
    })?;
    Ok(())
}
