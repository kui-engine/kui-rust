// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_06_timer_intervals.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/06_timer_intervals/main.cpp: Start runs a 100 ms
//! interval that bumps #counter and sets #bar's width to count % 100 (ignored while running);
//! Stop clears it; Reset clears it and goes back to 0. Each action writes a line into #log.

use crate::canonical_runner::Context;
use kui::{Document, Element, Platform, TimerId};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(element)) = doc.get_element_by_id(id) {
        let _ = element.set_text_content(text);
    }
}

fn update_ui(doc: &Document, count: i32) {
    set_text(doc, "counter", &count.to_string());
    if let Ok(Some(bar)) = doc.get_element_by_id("bar") {
        let _ = bar.set_style_property("width", &format!("{}%", count % 100), false);
    }
}

/// Clears the running interval; true when there was one.
fn stop_timer(platform: &Platform, timer: &RefCell<Option<TimerId>>) -> bool {
    match timer.borrow_mut().take() {
        Some(id) => {
            let _ = platform.clear_timer(id);
            true
        }
        None => false,
    }
}

fn button<'d>(doc: &'d Document, id: &str) -> Result<Element<'d>, Box<dyn Error>> {
    Ok(doc
        .get_element_by_id(id)?
        .ok_or_else(|| format!("page has no #{id}"))?)
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let count = Rc::new(Cell::new(0));
    let timer: Rc<RefCell<Option<TimerId>>> = Rc::new(RefCell::new(None));
    let doc = ctx.document;
    let (start, stop, reset) = (
        button(doc, "startBtn")?,
        button(doc, "stopBtn")?,
        button(doc, "resetBtn")?,
    );

    {
        let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
        let (count, timer) = (count.clone(), timer.clone());
        start.add_event_listener("click", move |_event| {
            if timer.borrow().is_some() {
                return;
            }
            set_text(&doc, "log", "Timer started");
            let (doc, count) = (doc.clone(), count.clone());
            let id = platform.set_interval(100, move || {
                count.set(count.get() + 1);
                update_ui(&doc, count.get());
            });
            *timer.borrow_mut() = id.ok();
        })?;
    }
    {
        let (doc, platform, timer) = (ctx.document.clone(), ctx.platform.clone(), timer.clone());
        stop.add_event_listener("click", move |_event| {
            if stop_timer(&platform, &timer) {
                set_text(&doc, "log", "Timer stopped");
            }
        })?;
    }
    {
        let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
        reset.add_event_listener("click", move |_event| {
            stop_timer(&platform, &timer);
            count.set(0);
            update_ui(&doc, 0);
            set_text(&doc, "log", "Timer reset");
        })?;
    }
    Ok(())
}
