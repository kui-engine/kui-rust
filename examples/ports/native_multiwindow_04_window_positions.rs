// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_multiwindow_04_window_positions.rs; edit that file, not this copy.
//! Host logic of examples/native/multiwindow/04_window_positions/main.cpp: moves the window to the
//! primary monitor's work area + (48, 48), then a 200 ms interval writes the live OS position into
//! #position.

use crate::canonical_runner::Context;
use kui::Window;
use std::error::Error;
use std::rc::Rc;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    if let Ok(monitor) = ctx.platform.primary_monitor() {
        ctx.window.set_position(
            (monitor.work_area.x + 48.0) as i32,
            (monitor.work_area.y + 48.0) as i32,
        )?;
    }
    let window: Rc<Window> = ctx.window.clone();
    let doc = ctx.document.clone();
    ctx.platform.set_interval(200, move || {
        let Ok((x, y)) = window.position() else {
            return;
        };
        if let Ok(Some(el)) = doc.get_element_by_id("position") {
            let _ = el.set_text_content(&format!("Live window position: ({x}, {y})"));
        }
    })?;
    Ok(())
}
