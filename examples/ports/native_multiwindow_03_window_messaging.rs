// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_multiwindow_03_window_messaging.rs; edit that file, not this copy.
//! Host logic of examples/native/multiwindow/03_window_messaging/main.cpp: Window A's Send button
//! counts messages, logs "Sent message #N" in #log and "Received message #N from Window A" in
//! Window B's #log2.
//!
//! The generated page only carries Window A; the runner's second window shows a placeholder, so
//! this port loads the original Window B page (and title) into it first.

use crate::canonical_runner::Context;
use kui::Document;
use std::cell::Cell;
use std::error::Error;

const WINDOW_B_HTML: &str = r#"<!DOCTYPE html><html><head><style>
*{margin:0;padding:0;box-sizing:border-box}
body{padding:24px;font-family:"Segoe UI",sans-serif;background:#064e3b;color:#f8fafc}
h2{margin-bottom:16px}
.card{background:rgba(255,255,255,.1);border-radius:12px;padding:20px;border:1px solid rgba(255,255,255,.1)}
.card h3{font-size:14px;margin-bottom:8px}
.log{background:rgba(0,0,0,.3);border-radius:8px;padding:12px;font-size:12px;color:#a7f3d0;font-family:"Consolas",monospace;min-height:60px}
</style></head><body>
<h2>Window B — Receiver</h2>
<div class="card"><h3>Messages</h3><div class="log" id="log2">Waiting for messages...</div></div>
</body></html>"#;

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(element)) = doc.get_element_by_id(id) {
        let _ = element.set_text_content(text);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let (second_window, second_document) = ctx.second.ok_or("no second window")?;
    let mut d2 = second_document.clone();
    d2.load_html(WINDOW_B_HTML)?;
    second_window.set_title("KUI — Window B")?;

    let send = ctx
        .document
        .get_element_by_id("sendBtn")?
        .ok_or("page has no #sendBtn")?;
    let d1 = ctx.document.clone();
    let count = Cell::new(0);
    send.add_event_listener("click", move |_event| {
        count.set(count.get() + 1);
        set_text(&d1, "log", &format!("Sent message #{}", count.get()));
        set_text(
            &d2,
            "log2",
            &format!("Received message #{} from Window A", count.get()),
        );
    })?;
    Ok(())
}
