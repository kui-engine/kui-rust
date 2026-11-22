// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_advanced_01_dom_manipulation.rs; edit that file, not this copy.
//! Host logic of examples/native/advanced/01_dom_manipulation/main.cpp: Add appends an "Item #n"
//! card to the container, Remove Last drops the last card, Clear All empties it; the counter
//! always shows the container's element count.

use crate::canonical_runner::Context;
use kui::{Document, Element};
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;

fn update_counter(doc: &Document, container: &Element<'_>) {
    let count = container.child_count().unwrap_or(0);
    if let Ok(Some(counter)) = doc.get_element_by_id("counter") {
        let _ = counter.set_text_content(&format!("Elements: {count}"));
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let next_id = Rc::new(Cell::new(1));
    let button = |id: &str| -> Result<Element<'_>, Box<dyn Error>> {
        Ok(ctx
            .document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?)
    };

    let doc = ctx.document.clone();
    button("addBtn")?.add_event_listener("click", move |_event| {
        let Ok(Some(container)) = doc.get_element_by_id("container") else {
            return;
        };
        let Ok(card) = doc.create_element("div") else {
            return;
        };
        let _ = card.set_class_name("card");
        let _ = card.set_text_content(&format!("Item #{}", next_id.get()));
        next_id.set(next_id.get() + 1);
        let _ = container.append_child(&card);
        update_counter(&doc, &container);
    })?;

    let doc = ctx.document.clone();
    button("removeBtn")?.add_event_listener("click", move |_event| {
        let Ok(Some(container)) = doc.get_element_by_id("container") else {
            return;
        };
        if let Ok(Some(last)) = container.last_child() {
            let _ = container.remove_child(&last);
            update_counter(&doc, &container);
        }
    })?;

    let doc = ctx.document.clone();
    button("clearBtn")?.add_event_listener("click", move |_event| {
        let Ok(Some(container)) = doc.get_element_by_id("container") else {
            return;
        };
        let _ = container.set_inner_html("");
        update_counter(&doc, &container);
    })?;
    Ok(())
}
