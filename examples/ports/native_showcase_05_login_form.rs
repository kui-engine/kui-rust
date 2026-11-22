// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_showcase_05_login_form.rs; edit that file, not this copy.
//! Host logic of examples/native/showcase/05_login_form/main.cpp: Sign In shows an error in #msg
//! when the email or password is empty, otherwise a welcome for the email.
//!
//! The C++ original reads both inputs with getAttribute("value"), which is the content attribute
//! and stays empty while the user types, so it always reports the error. This port reads the live
//! values (`Element::value`).

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

fn input_value(doc: &Document, id: &str) -> String {
    match doc.get_element_by_id(id) {
        Ok(Some(input)) => input.value().unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let login = ctx
        .document
        .get_element_by_id("loginBtn")?
        .ok_or("page has no #loginBtn")?;
    let doc = ctx.document.clone();

    login.add_event_listener("click", move |_event| {
        let (email, pass) = (input_value(&doc, "email"), input_value(&doc, "pass"));
        let Ok(Some(msg)) = doc.get_element_by_id("msg") else {
            return;
        };
        if email.is_empty() || pass.is_empty() {
            let _ = msg.set_class_name("msg msg-err show");
            let _ = msg.set_text_content("Please fill in all fields.");
        } else {
            let _ = msg.set_class_name("msg msg-ok show");
            let _ = msg.set_text_content(&format!("Login successful! Welcome, {email}"));
        }
    })?;
    Ok(())
}
