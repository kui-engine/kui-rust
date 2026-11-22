// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_showcase_07_music_player.rs; edit that file, not this copy.
//! Host logic of examples/native/showcase/07_music_player/main.cpp: #playBtn toggles its glyph
//! between play (U+25B6) and pause (U+23F8). No audio is involved in the original.

use crate::canonical_runner::Context;
use std::cell::Cell;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let play = ctx
        .document
        .get_element_by_id("playBtn")?
        .ok_or("page has no #playBtn")?;
    let doc = ctx.document.clone();
    let playing = Cell::new(false);
    play.add_event_listener("click", move |_event| {
        playing.set(!playing.get());
        if let Ok(Some(button)) = doc.get_element_by_id("playBtn") {
            let _ = button.set_text_content(if playing.get() {
                "\u{23f8}"
            } else {
                "\u{25b6}"
            });
        }
    })?;
    Ok(())
}
