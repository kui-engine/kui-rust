// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_02_dpi_scaling.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/02_dpi_scaling/main.cpp: writes the window's device
//! pixel ratio into #dpr (std::to_string formatting, six decimals) and the truncated percentage
//! into #scale.

use crate::canonical_runner::Context;
use std::error::Error;

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let dpr = ctx.window.device_pixel_ratio()?;
    if let Some(element) = ctx.document.get_element_by_id("dpr")? {
        element.set_text_content(&format!("{dpr:.6}"))?;
    }
    if let Some(element) = ctx.document.get_element_by_id("scale")? {
        element.set_text_content(&format!("{}%", (dpr * 100.0f32) as i32))?;
    }
    Ok(())
}
