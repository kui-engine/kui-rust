// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_animation_03_lottie_player.rs; edit that file, not this copy.
//! Host logic of examples/native/animation/03_lottie_player/main.cpp: the runner's setup_lottie
//! already points <lottie-player id="lottie"> at the bundled native_animation/pulse.json and plays
//! it; this reports the outcome in #status the way the original does. The runner only sets src
//! when it found the file, so an empty src means "not found".

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let mut text = "Bundled pulse.json was not found.".to_string();
    if let Some(player) = ctx.document.get_element_by_id("lottie")? {
        if !player.attribute("src").unwrap_or_default().is_empty() {
            text = if player.lottie_complete().unwrap_or(false) {
                let size = player.lottie_natural_size()?;
                format!(
                    "Playing bundled pulse.json · {} × {}",
                    size.width as u32, size.height as u32
                )
            } else {
                "Lottie decoder rejected pulse.json".to_string()
            };
        }
    }
    ctx.document
        .get_element_by_id("status")?
        .ok_or("page has no #status")?
        .set_text_content(&text)?;
    Ok(())
}
