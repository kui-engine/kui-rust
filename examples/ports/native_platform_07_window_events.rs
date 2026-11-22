// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_07_window_events.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/07_window_events/main.cpp: window focus / blur /
//! focusin / focusout / movestart / move / moveend / dragregionpointerdown / close listeners
//! update the #b-focus / #b-move badges and prepend lines to #log; the custom title-bar buttons
//! minimize and close the (frameless) window.

use crate::canonical_runner::Context;
use kui::{Document, Window};
use std::error::Error;

/// Newest line first: #log = line + "\n" + #log.
fn log_line(doc: &Document, line: &str) {
    if let Ok(Some(log)) = doc.get_element_by_id("log") {
        let old = log.text_content().unwrap_or_default();
        let _ = log.set_text_content(&format!("{line}\n{old}"));
    }
}

fn set_badge(doc: &Document, id: &str, text: &str, add_active: bool, remove_active: bool) {
    if let Ok(Some(badge)) = doc.get_element_by_id(id) {
        let _ = badge.set_text_content(text);
        if remove_active {
            let _ = badge.class_list_remove("active");
        }
        if add_active {
            let _ = badge.class_list_add("active");
        }
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let events: [(&str, fn(&Document)); 9] = [
        ("focus", |doc| {
            set_badge(doc, "b-focus", "app: focus", true, true);
            log_line(doc, "[focus]  application activated");
        }),
        ("blur", |doc| {
            set_badge(doc, "b-focus", "app: blur", false, true);
            log_line(doc, "[blur]   application deactivated");
        }),
        ("focusin", |doc| {
            log_line(doc, "[focusin]  keyboard focus gained")
        }),
        ("focusout", |doc| {
            log_line(doc, "[focusout] keyboard focus lost")
        }),
        ("movestart", |doc| {
            set_badge(doc, "b-move", "move: dragging", false, false);
            log_line(
                doc,
                "[movestart] window drag begin (passed OS drag threshold)",
            );
        }),
        ("move", |doc| {
            set_badge(doc, "b-move", "move: moving", false, false)
        }),
        ("moveend", |doc| {
            set_badge(doc, "b-move", "move: idle", false, false);
            log_line(doc, "[moveend]  window drag end");
        }),
        ("dragregionpointerdown", |doc| {
            log_line(doc, "[dragregionpointerdown] title-bar pressed")
        }),
        ("close", |doc| {
            log_line(doc, "[close]  close requested (allowed)")
        }),
    ];
    for (name, handler) in events {
        let doc = ctx.document.clone();
        ctx.window
            .add_event_listener(name, false, move |_event| handler(&doc))?;
    }

    let buttons: [(&str, fn(&Window)); 2] = [
        ("btn-min", |w| {
            let _ = w.minimize();
        }),
        ("btn-close", |w| {
            let _ = w.close();
        }),
    ];
    for (id, action) in buttons {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let window = ctx.window.clone();
        button.add_event_listener("click", move |_event| action(&window))?;
    }
    log_line(
        ctx.document,
        "ready — try dragging the title bar / switching windows",
    );
    Ok(())
}
