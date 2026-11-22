// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_multi_window_animation.rs; edit that file, not this copy.
//! Host logic of examples/multi_window_animation/main.cpp: three 420x520 windows side by side,
//! centred on the primary monitor (CSS transitions, @keyframes, complex composited animations);
//! closing any window closes the other two.
//!
//! The generated page is the original's window 3, so this port loads window 1 into the main
//! window, turns the runner's second window into window 2 and creates window 3 itself. The main
//! window's close listener owns window 3, so it lives until the main window is dropped after the
//! event loop.

use crate::canonical_runner::Context;
use kui::{Window, WindowProps};
use std::cell::Cell;
use std::error::Error;
use std::rc::{Rc, Weak};

const WINDOW1_HTML: &str = r#"<html>
<head>
<style>
body {
    margin: 0; padding: 24px;
    background: #0f172a; color: #e2e8f0;
    font-family: 'Segoe UI', sans-serif;
}
h2 { color: #38bdf8; margin-bottom: 16px; font-size: 18px; }
.info { color: #64748b; font-size: 12px; margin-bottom: 20px; }

.card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
    transition: background 0.3s ease, border-color 0.3s ease, transform 0.2s ease;
    cursor: pointer;
}
.card:hover {
    background: #334155;
    border-color: #38bdf8;
    transform: translateX(8px);
}
.card-title {
    font-size: 14px; font-weight: 600; color: #f1f5f9;
    margin-bottom: 4px;
}
.card-desc { font-size: 12px; color: #94a3b8; }

.btn-row { display: flex; gap: 8px; margin-top: 16px; }
.btn {
    padding: 8px 20px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease, transform 0.15s ease;
    border: 1px solid transparent;
}
.btn-primary {
    background: #2563eb; color: white; border-color: #3b82f6;
}
.btn-primary:hover {
    background: #1d4ed8; transform: scale(1.05);
}
.btn-secondary {
    background: #1e293b; color: #94a3b8; border-color: #475569;
}
.btn-secondary:hover {
    background: #334155; color: #e2e8f0; transform: scale(1.05);
}

.progress-bar {
    height: 6px; background: #1e293b; border-radius: 3px;
    margin-top: 16px; overflow: hidden;
}
.progress-fill {
    height: 100%; width: 0%;
    background: linear-gradient(90deg, #38bdf8, #818cf8);
    border-radius: 3px;
    transition: width 2s ease-in-out;
}
.progress-fill.active { width: 100%; }
</style>
</head>
<body>
    <h2>Window 1 — CSS Transitions</h2>
    <div class="info">Hover over cards and buttons to see smooth CSS transitions</div>

    <div class="card">
        <div class="card-title">Smooth Background Transition</div>
        <div class="card-desc">background + border-color + transform on hover (0.3s ease)</div>
    </div>
    <div class="card">
        <div class="card-title">Interactive Card Element</div>
        <div class="card-desc">Each property transitions independently with its own timing</div>
    </div>
    <div class="card">
        <div class="card-title">Multi-Property Animation</div>
        <div class="card-desc">transform: translateX(8px) slides the card on hover</div>
    </div>

    <div class="btn-row">
        <div class="btn btn-primary">Primary Action</div>
        <div class="btn btn-secondary">Secondary</div>
        <div class="btn btn-secondary">Cancel</div>
    </div>

    <div class="progress-bar">
        <div class="progress-fill active"></div>
    </div>
</body>
</html>"#;

const WINDOW2_HTML: &str = r#"<html>
<head>
<style>
body {
    margin: 0; padding: 24px;
    background: #0f172a; color: #e2e8f0;
    font-family: 'Segoe UI', sans-serif;
}
h2 { color: #a78bfa; margin-bottom: 16px; font-size: 18px; }
.info { color: #64748b; font-size: 12px; margin-bottom: 20px; }

.anim-grid {
    display: flex; flex-wrap: wrap; gap: 20px;
    justify-content: center;
}
.anim-cell {
    width: 120px; height: 120px;
    display: flex; align-items: center; justify-content: center;
    background: #1e293b; border-radius: 12px;
    border: 1px solid #334155;
}
.anim-label {
    text-align: center; font-size: 11px; color: #64748b;
    margin-top: 6px;
}

@keyframes spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
}
@keyframes pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50%      { transform: scale(1.3); opacity: 0.7; }
}
@keyframes bounce {
    0%, 100% { transform: translateY(0); }
    50%      { transform: translateY(-20px); }
}
@keyframes colorShift {
    0%   { background: #ef4444; }
    33%  { background: #22c55e; }
    66%  { background: #3b82f6; }
    100% { background: #ef4444; }
}
@keyframes slideIn {
    from { transform: translateX(-40px); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
}
@keyframes fadeInOut {
    0%, 100% { opacity: 0.3; }
    50%      { opacity: 1; }
}

.spin-box {
    width: 40px; height: 40px; background: #38bdf8; border-radius: 6px;
    animation: spin 2s linear infinite;
}
.pulse-circle {
    width: 40px; height: 40px; background: #a78bfa; border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
}
.bounce-box {
    width: 40px; height: 40px; background: #34d399; border-radius: 8px;
    animation: bounce 1s ease-in-out infinite;
}
.color-box {
    width: 40px; height: 40px; border-radius: 6px;
    animation: colorShift 3s linear infinite;
}
.slide-box {
    width: 40px; height: 40px; background: #fb923c; border-radius: 6px;
    animation: slideIn 1.5s ease-out infinite;
}
.fade-circle {
    width: 40px; height: 40px; background: #f472b6; border-radius: 50%;
    animation: fadeInOut 2s ease-in-out infinite;
}
</style>
</head>
<body>
    <h2>Window 2 — @keyframes Animations</h2>
    <div class="info">Independent CSS animations running at 60fps</div>

    <div class="anim-grid">
        <div>
            <div class="anim-cell"><div class="spin-box"></div></div>
            <div class="anim-label">spin (2s linear)</div>
        </div>
        <div>
            <div class="anim-cell"><div class="pulse-circle"></div></div>
            <div class="anim-label">pulse (1.5s ease)</div>
        </div>
        <div>
            <div class="anim-cell"><div class="bounce-box"></div></div>
            <div class="anim-label">bounce (1s ease)</div>
        </div>
        <div>
            <div class="anim-cell"><div class="color-box"></div></div>
            <div class="anim-label">colorShift (3s)</div>
        </div>
        <div>
            <div class="anim-cell"><div class="slide-box"></div></div>
            <div class="anim-label">slideIn (1.5s)</div>
        </div>
        <div>
            <div class="anim-cell"><div class="fade-circle"></div></div>
            <div class="anim-label">fadeInOut (2s)</div>
        </div>
    </div>
</body>
</html>"#;

const WINDOW3_HTML: &str = r#"<html>
<head>
<style>
body {
    margin: 0; padding: 24px;
    background: #0f172a; color: #e2e8f0;
    font-family: 'Segoe UI', sans-serif;
}
h2 { color: #f472b6; margin-bottom: 16px; font-size: 18px; }
.info { color: #64748b; font-size: 12px; margin-bottom: 20px; }

@keyframes orbit {
    from { transform: rotate(0deg) translateX(60px) rotate(0deg); }
    to   { transform: rotate(360deg) translateX(60px) rotate(-360deg); }
}
@keyframes glow {
    0%, 100% { box-shadow: 0 0 8px #38bdf8; }
    50%      { box-shadow: 0 0 24px #38bdf8; }
}
@keyframes expandWidth {
    0%   { width: 40px; }
    50%  { width: 200px; }
    100% { width: 40px; }
}
@keyframes hueRotate {
    0%   { background: #ef4444; }
    25%  { background: #eab308; }
    50%  { background: #22c55e; }
    75%  { background: #3b82f6; }
    100% { background: #ef4444; }
}

.orbit-container {
    width: 160px; height: 160px; margin: 20px auto;
    position: relative;
}
.orbit-center {
    position: absolute;
    left: 60px; top: 60px;
    width: 40px; height: 40px;
    background: #1e293b; border: 2px solid #38bdf8;
    border-radius: 50%;
    animation: glow 2s ease-in-out infinite;
}
.orbit-planet {
    position: absolute;
    left: 68px; top: 68px;
    width: 24px; height: 24px;
    background: #f472b6; border-radius: 50%;
    animation: orbit 4s linear infinite;
}
.orbit-planet2 {
    position: absolute;
    left: 68px; top: 68px;
    width: 16px; height: 16px;
    background: #34d399; border-radius: 50%;
    animation: orbit 2.5s linear infinite reverse;
}

.bar-row { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
.bar-label { width: 80px; font-size: 12px; color: #94a3b8; }
.bar {
    height: 24px; border-radius: 4px;
    animation: expandWidth 3s ease-in-out infinite;
}
.bar.b1 { background: #ef4444; animation-delay: 0s; }
.bar.b2 { background: #f59e0b; animation-delay: 0.4s; }
.bar.b3 { background: #22c55e; animation-delay: 0.8s; }
.bar.b4 { background: #3b82f6; animation-delay: 1.2s; }

.color-strip {
    height: 8px; border-radius: 4px; margin-top: 20px;
    animation: hueRotate 4s linear infinite;
}

.stats-row {
    display: flex; gap: 12px; margin-top: 20px;
}
.stat-card {
    flex: 1; padding: 12px;
    background: #1e293b; border-radius: 8px;
    border: 1px solid #334155;
    text-align: center;
    transition: transform 0.2s ease, border-color 0.3s ease;
}
.stat-card:hover {
    transform: scale(1.05);
    border-color: #f472b6;
}
.stat-value { font-size: 20px; font-weight: 700; color: #f1f5f9; }
.stat-label { font-size: 11px; color: #64748b; margin-top: 4px; }
</style>
</head>
<body>
    <h2>Window 3 — Complex Composited Animations</h2>
    <div class="info">Multi-transform orbit + animated bars + hover transitions</div>

    <div class="orbit-container">
        <div class="orbit-center"></div>
        <div class="orbit-planet"></div>
        <div class="orbit-planet2"></div>
    </div>

    <div class="bar-row">
        <div class="bar-label">CPU</div>
        <div class="bar b1"></div>
    </div>
    <div class="bar-row">
        <div class="bar-label">Memory</div>
        <div class="bar b2"></div>
    </div>
    <div class="bar-row">
        <div class="bar-label">Network</div>
        <div class="bar b3"></div>
    </div>
    <div class="bar-row">
        <div class="bar-label">Disk I/O</div>
        <div class="bar b4"></div>
    </div>

    <div class="color-strip"></div>

    <div class="stats-row">
        <div class="stat-card">
            <div class="stat-value">60</div>
            <div class="stat-label">Target FPS</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">3</div>
            <div class="stat-label">Windows</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">12</div>
            <div class="stat-label">Animations</div>
        </div>
    </div>
</body>
</html>"#;

/// Prints the original's last line when window 3 goes away after the event loop.
struct Third {
    _window: Rc<Window>,
}
impl Drop for Third {
    fn drop(&mut self) {
        eprintln!("[MultiWindow] Event loop exited.");
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let (second, second_document) = ctx.second.ok_or("no second window")?;
    eprintln!("[MultiWindow] Starting multi-window animation demo...");
    ctx.platform.set_f12_devtools_enabled(true)?;

    // Platform::primaryMonitorSize(): 3 windows + gaps, centred.
    const WIN_W: i32 = 420;
    const WIN_H: i32 = 520;
    let screen = ctx.platform.primary_monitor_physical_size()?;
    let start_x = (screen.width as i32 - (WIN_W * 3 + 40)) / 2;
    let start_y = (screen.height as i32 - WIN_H) / 2;

    let titles = [
        "KUI — CSS Transitions",
        "KUI — @keyframes Animations",
        "KUI — Complex Composited",
    ];
    let third = Rc::new(
        Window::new(&WindowProps {
            title: titles[2].into(),
            width: WIN_W,
            height: WIN_H,
            ..WindowProps::default()
        })
        .map_err(|e| format!("[{}] Failed to create window: {e}", titles[2]))?,
    );
    let windows = [Rc::clone(ctx.window), Rc::clone(second), Rc::clone(&third)];
    let mut documents = [
        ctx.document.clone(),
        second_document.clone(),
        third.document()?,
    ];
    let pages = [WINDOW1_HTML, WINDOW2_HTML, WINDOW3_HTML];
    let all: Vec<Weak<Window>> = windows.iter().map(Rc::downgrade).collect();
    let closing = Rc::new(Cell::new(false));
    let mut keep = Some(Third { _window: third });
    for (index, window) in windows.iter().enumerate() {
        window.set_title(titles[index])?;
        window.set_position(start_x + (WIN_W + 20) * index as i32, start_y)?;
        documents[index].load_html(pages[index])?;
        // Closing any window closes the other two (the original's onClose).
        let (others, flag) = (all.clone(), Rc::clone(&closing));
        let owned = if index == 0 { keep.take() } else { None };
        window.add_event_listener("close", false, move |_| {
            let _keep = &owned;
            if flag.replace(true) {
                return;
            }
            for (other, weak) in others.iter().enumerate() {
                if other != index {
                    if let Some(w) = weak.upgrade() {
                        let _ = w.close();
                    }
                }
            }
        })?;
    }
    eprintln!("[MultiWindow] 3 windows created. Close any window to exit all.");
    Ok(())
}
