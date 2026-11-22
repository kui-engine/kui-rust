// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_html_08_timer_animation.rs; edit that file, not this copy.
//! Host logic of examples/native/html/08_timer_animation/main.cpp: a 1 s interval writes the time
//! into #clock (%H:%M:%S) and #date (%Y-%m-%d %A), sets #progress's width to sec*100/59 % and
//! counts "Ticks: N" in #counter.
//!
//! Deviation: the original shows local time. The Rust standard library has no time zone support
//! (and neither the SDK nor the Lua sandbox offers a local clock), so this port shows UTC, like
//! the runner's own timestamps.

use crate::canonical_runner::Context;
use kui::Document;
use std::cell::Cell;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// (year, month, day, weekday) of a day count since 1970-01-01 (H. Hinnant's civil_from_days).
fn civil_from_days(days: i64) -> (i64, i64, i64, usize) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, (days + 4).rem_euclid(7) as usize) // 1970-01-01 was a Thursday
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(element)) = doc.get_element_by_id(id) {
        let _ = element.set_text_content(text);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let doc = ctx.document.clone();
    let ticks = Cell::new(0);
    ctx.platform.set_interval(1000, move || {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let (secs_of_day, days) = (now.rem_euclid(86_400), now.div_euclid(86_400));
        let (year, month, day, weekday) = civil_from_days(days);
        let sec = secs_of_day % 60;
        set_text(
            &doc,
            "clock",
            &format!(
                "{:02}:{:02}:{:02}",
                secs_of_day / 3600,
                secs_of_day / 60 % 60,
                sec
            ),
        );
        set_text(
            &doc,
            "date",
            &format!("{year:04}-{month:02}-{day:02} {}", WEEKDAYS[weekday]),
        );
        if let Ok(Some(progress)) = doc.get_element_by_id("progress") {
            let _ = progress.set_style_property("width", &format!("{}%", sec * 100 / 59), false);
        }
        ticks.set(ticks.get() + 1);
        set_text(&doc, "counter", &format!("Ticks: {}", ticks.get()));
    })?;
    Ok(())
}
