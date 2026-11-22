// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/repro_kui011.rs; edit that file, not this copy.
//! Host logic of examples/repro_kui011/main.cpp (KUI-011 crash repro): a 16 ms interval rebuilds
//! the main window's #list with setInnerHTML (24 rows every 8th tick, otherwise 6) while a
//! frameless 380x200 "modal" window is created, pumped by a nested message loop, destroyed and
//! re-created, round after round. Prints "NO CRASH" and exits 0 after KUI011_ROUNDS rounds
//! (default 400); KUI011_TIMEOUT_SEC (default 120) exits 3 instead.
//!
//! Like the original, everything runs before the regular event loop, pumping messages itself:
//! poll / wait_events stand for pumpFor's PeekMessage loop, and each nested-loop step is one
//! wait_events call (GetMessageW returns one message, so KUI011_NESTED_MSGS counts batches here,
//! not single messages). Not ported: the Win32 crash handler / minidump (a crash simply kills
//! the process), KUI011_SIMPLE (would mean reloading the generated main page) and
//! InvalidateRect (setInnerHTML already schedules the repaint). The main window is disabled
//! during the modal with `Window::set_enabled` (EnableWindow, Windows only) and the dialog is
//! created hidden (`WindowProps::hidden`), as the original does. The runner's placeholder second
//! window is hidden: the original creates its dialog afresh every round.

use crate::canonical_runner::Context;
use kui::{Platform, Window, WindowProps};
use std::cell::Cell;
use std::error::Error;
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::{Duration, Instant};

const DIALOG_HTML: &str = r#"<html><head><style>
  body { margin:0; background:#11111b; color:#cdd6f4;
         font-family:'Segoe UI',sans-serif; }
  .box { padding:18px; transform:translateZ(0); opacity:0.99;
         box-shadow:0 12px 40px rgba(0,0,0,0.6); }
  .btn { margin-top:14px; padding:8px 18px; background:#89b4fa; color:#11111b;
         border-radius:6px; display:inline-block; transform:scale(1); }
</style></head><body>
  <div class="box">
    <div>确认要执行该操作吗？（自绘模态消息框）</div>
    <div class="btn" id="ok">确定</div>
  </div>
</body></html>
"#;

fn env_int(name: &str, fallback: u32) -> u32 {
    match std::env::var(name).ok().and_then(|v| v.parse::<u32>().ok()) {
        Some(value) if value > 0 => value,
        _ => fallback,
    }
}

fn pump_for(platform: &Platform, duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        let _ = platform.wait_events(0.001);
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let platform = ctx.platform;
    let rounds = env_int("KUI011_ROUNDS", 400);
    let nested_msgs = env_int("KUI011_NESTED_MSGS", 80);
    let watchdog_sec = env_int("KUI011_TIMEOUT_SEC", 120);
    let started = Instant::now();
    if let Some((second, _)) = ctx.second {
        let _ = second.hide();
    }

    let tick = Rc::new(Cell::new(0u32));
    let timer = {
        let (doc, tick) = (ctx.document.clone(), tick.clone());
        platform.set_interval(16, move || {
            tick.set(tick.get() + 1);
            let t = tick.get();
            let Ok(Some(list)) = doc.get_element_by_id("list") else {
                return;
            };
            let n = if t % 8 == 0 { 24 } else { 6 };
            let mut html = String::new();
            for i in 0..n {
                let _ = write!(
                    html,
                    "<div class=\"row layerA\"><span class=\"pill\">#{t}.{i}</span> 扫描扇区 {}</div>",
                    (t * 31 + i) % 100000
                );
            }
            let _ = list.set_inner_html(&html);
        })?
    };

    pump_for(platform, Duration::from_millis(300)); // let the main window settle
    eprintln!("[repro] start: rounds={rounds} nestedMsgs={nested_msgs} timeout={watchdog_sec}s");

    let mut round = 0;
    while round < rounds {
        if started.elapsed() > Duration::from_secs(u64::from(watchdog_sec)) {
            eprintln!("[repro] WATCHDOG TIMEOUT ({watchdog_sec}s) — exiting (no crash)");
            platform.quit(3)?;
            return Ok(());
        }
        let props = WindowProps {
            title: "modal".into(),
            width: 380,
            height: 200,
            centered: true,
            frameless: true,
            no_decorations: true,
            hidden: true, // visible = false, as the original
            ..WindowProps::default()
        };
        let Ok(dialog) = Window::new(&props) else {
            eprintln!("[repro] dlg create failed @round {round}");
            break;
        };
        if let Ok(mut dialog_doc) = dialog.document() {
            let _ = dialog_doc.load_html(DIALOG_HTML);
        }
        let _ = ctx.window.set_enabled(false); // standard modal: disable the parent (Windows only)
        let _ = dialog.show();

        // Nested loop: dispatches every window's messages (main timer, paints) while the dialog
        // is up; after nested_msgs steps the "user" closes it.
        for _ in 0..nested_msgs {
            let _ = platform.wait_events(0.1);
        }

        let _ = ctx.window.set_enabled(true);
        drop(dialog); // dlg.reset(): destroys the window and its document
        pump_for(platform, Duration::from_millis(20)); // back to the main loop for a few frames

        if round % 25 == 0 {
            eprintln!("[repro] round {round} / {rounds} (tick={})", tick.get());
        }
        round += 1;
    }

    eprintln!(
        "[repro] NO CRASH after {round} rounds (tick={})",
        tick.get()
    );
    let _ = platform.clear_timer(timer);
    platform.quit(0)?;
    Ok(())
}
