// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_component_07_progress_bar.rs; edit that file, not this copy.
//! Host logic of examples/native/component/07_progress_bar/main.cpp: Start runs a 50 ms interval
//! that grows #pf1 / #lbl1 by 2% up to 100% (ignored while one is running); Reset stops it and
//! goes back to 0%.

use crate::canonical_runner::Context;
use kui::{Document, Platform, TimerId};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

fn show_progress(doc: &Document, percent: i32) {
    let value = format!("{percent}%");
    if let Ok(Some(bar)) = doc.get_element_by_id("pf1") {
        let _ = bar.set_style_property("width", &value, false);
    }
    if let Ok(Some(label)) = doc.get_element_by_id("lbl1") {
        let _ = label.set_text_content(&value);
    }
}

fn stop_timer(platform: &Platform, timer: &RefCell<Option<TimerId>>) {
    if let Some(id) = timer.borrow_mut().take() {
        let _ = platform.clear_timer(id);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let progress = Rc::new(Cell::new(0));
    let timer: Rc<RefCell<Option<TimerId>>> = Rc::new(RefCell::new(None));

    let start = ctx
        .document
        .get_element_by_id("startBtn")?
        .ok_or("page has no #startBtn")?;
    let reset = ctx
        .document
        .get_element_by_id("resetBtn")?
        .ok_or("page has no #resetBtn")?;

    {
        let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
        let (progress, timer) = (progress.clone(), timer.clone());
        start.add_event_listener("click", move |_event| {
            if timer.borrow().is_some() {
                return; // already running
            }
            progress.set(0);
            let (doc, tick_platform) = (doc.clone(), platform.clone());
            let (progress, tick_timer) = (progress.clone(), timer.clone());
            let id = platform.set_interval(50, move || {
                progress.set((progress.get() + 2).min(100));
                show_progress(&doc, progress.get());
                if progress.get() >= 100 {
                    stop_timer(&tick_platform, &tick_timer);
                }
            });
            *timer.borrow_mut() = id.ok();
        })?;
    }
    {
        let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
        reset.add_event_listener("click", move |_event| {
            stop_timer(&platform, &timer);
            progress.set(0);
            show_progress(&doc, 0);
        })?;
    }
    Ok(())
}
