// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_frameless_01_basic_frameless.rs; edit that file, not this copy.
//! Host logic of examples/native/frameless/01_basic_frameless/main.cpp: the custom title bar
//! buttons minimize, maximize / restore and close the frameless window.

use crate::canonical_runner::Context;
use kui::Window;
use std::error::Error;
use std::rc::Rc;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let actions: [(&str, fn(&Window)); 3] = [
        ("minBtn", |w| drop(w.minimize())),
        ("maxBtn", |w| {
            let _ = if w.is_maximized().unwrap_or(false) {
                w.restore()
            } else {
                w.maximize()
            };
        }),
        ("closeBtn", |w| drop(w.close())),
    ];
    for (id, action) in actions {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let window: Rc<Window> = ctx.window.clone();
        button.add_event_listener("click", move |_event| action(&window))?;
    }
    Ok(())
}
