// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_scrollbar_overlay_demo.rs; edit that file, not this copy.
//! Host logic of examples/scrollbar_overlay_demo/main.cpp: #toggle-target starts with
//! -kui-scrollbar-overlay: overlay and #toggle-btn flips it between overlay and auto (updating
//! #toggle-status); #btn-top / #btn-bottom scrollTo the top / bottom of #scrollzone.
//!
//! #btn-fling starts an inertial fling (`Element::start_momentum_scroll(0, 1.5)`, px/ms downward).
//! The original's registerBuiltinComponents runs before loadHTML and the page uses none, so it
//! is not repeated here.

use crate::canonical_runner::Context;
use kui::Document;
use std::cell::Cell;
use std::error::Error;

fn scroll_zone(doc: &Document, to_bottom: bool) {
    let Ok(Some(zone)) = doc.get_element_by_id("scrollzone") else {
        return;
    };
    let Ok(metrics) = zone.scroll_metrics() else {
        return;
    };
    // scroll-behavior:smooth on .scrollzone makes this animate.
    let top = if to_bottom { metrics.height } else { 0.0 };
    let _ = zone.set_scroll_position(metrics.left, top);
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document;
    match (
        doc.get_element_by_id("toggle-target")?,
        doc.get_element_by_id("toggle-btn")?,
    ) {
        (Some(target), Some(btn)) => {
            target.set_style_property("-kui-scrollbar-overlay", "overlay", false)?;
            let doc = doc.clone();
            let overlay_on = Cell::new(true);
            btn.add_event_listener("click", move |_event| {
                overlay_on.set(!overlay_on.get());
                let (mode, note) = if overlay_on.get() {
                    ("overlay", "  (scrollbar overlays content)")
                } else {
                    ("auto", "  (scrollbar reserves 8px)")
                };
                if let Ok(Some(target)) = doc.get_element_by_id("toggle-target") {
                    let _ = target.set_style_property("-kui-scrollbar-overlay", mode, false);
                }
                if let Ok(Some(status)) = doc.get_element_by_id("toggle-status") {
                    let _ = status.set_text_content(&format!(
                        "current: -kui-scrollbar-overlay: {mode}{note}"
                    ));
                }
            })?;
        }
        _ => eprintln!(
            "[scrollbar_overlay_demo] WARN: toggle elements missing; runtime switch disabled"
        ),
    }

    if doc.get_element_by_id("scrollzone")?.is_some() {
        for (id, to_bottom) in [("btn-top", false), ("btn-bottom", true)] {
            if let Some(button) = doc.get_element_by_id(id)? {
                let doc = doc.clone();
                button.add_event_listener("click", move |_event| scroll_zone(&doc, to_bottom))?;
            }
        }
        if let Some(button) = doc.get_element_by_id("btn-fling")? {
            let doc = doc.clone();
            button.add_event_listener("click", move |_event| {
                if let Ok(Some(zone)) = doc.get_element_by_id("scrollzone") {
                    let _ = zone.start_momentum_scroll(0.0, 1.5); // px/ms downward inertial fling
                }
            })?;
        }
    }
    Ok(())
}
