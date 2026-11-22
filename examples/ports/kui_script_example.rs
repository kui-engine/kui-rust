// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_script_example.rs; edit that file, not this copy.
//! Host logic of examples/script_example/main.cpp: a browser for HTML files with embedded Lua.
//! It scans script_html/ next to the executable (the original's POST_BUILD copy of
//! tests/script/html; also --dir / --file / a positional path / --list), wraps each file's body
//! and first <style> in a page with a nav bar (prev / counter / next / title / file name / reload)
//! and a status bar, runs the file's <script> blocks in a fresh script engine, and shows the
//! #result / #detail text in the status bar.
//!
//! Differences from the original:
//! - The original adds its Left / Right / F5 keydown listener to doc->body() before any page is
//!   loaded, when the document has no body, so keyboard navigation never works. This port adds
//!   the listener to the body of every page it loads (what the original means to do).
//! - --list prints the list and quits right away, but the window already exists by then (the
//!   example host code runs after the runner opened the window).
//!
//! Like the original it ticks the script timers at 0 and 100 ms right after the scripts
//! (`ScriptEngine::tick_timers`), so short timers land before #result is read.

use crate::canonical_runner::Context;
use kui::{Document, Element, Platform, ScriptEngine, Window};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone)]
struct HtmlEntry {
    name: String,     // display name
    filename: String, // e.g. "01_basic_eval.html"
    path: PathBuf,    // full path
}

struct Browser {
    doc: Document,
    window: Rc<Window>,
    entries: RefCell<Vec<HtmlEntry>>,
    current: Cell<usize>,
    engine: RefCell<Option<ScriptEngine>>, // fresh engine per loaded entry
}

fn pretty_name(filename: &str) -> String {
    let mut s = match filename.rfind('.') {
        Some(dot) => &filename[..dot],
        None => filename,
    };
    // Remove leading number prefix like "01_"
    let bytes = s.as_bytes();
    if bytes.len() > 3 && bytes[2] == b'_' && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit()
    {
        s = &s[3..];
    }
    let mut result = String::new();
    let mut cap_next = true;
    for c in s.chars() {
        if c == '_' {
            result.push(' ');
            cap_next = true;
        } else {
            result.push(if cap_next { c.to_ascii_uppercase() } else { c });
            cap_next = false;
        }
    }
    result
}

/// The .html files of dir in name order.
fn scan_directory(dir: &Path) -> Vec<HtmlEntry> {
    let Ok(items) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = items
        .flatten()
        .filter(|item| item.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|item| item.file_name().to_string_lossy().into_owned())
        .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "html"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|f| HtmlEntry {
            name: pretty_name(&f),
            path: dir.join(&f),
            filename: f,
        })
        .collect()
}

const STATUS_HTML: &str = r#"
<div id="__status" style="
    display: flex; align-items: center; gap: 8px;
    padding: 4px 12px;
    background: #0f172a; border-top: 1px solid #334155;
    font-family: 'Segoe UI', sans-serif; font-size: 11px; color: #64748b;">
  <span>Result:</span>
  <span id="__result_display" style="font-weight: 700; color: #94a3b8;">—</span>
  <span id="__detail_display" style="flex: 1; color: #475569; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"></span>
</div>
"#;

const PAGE_HEAD: &str = r#"<!DOCTYPE html>
<html>
<head>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100vh; display: flex; flex-direction: column; background: #0f172a; }
#__content_area {
    flex: 1; overflow: auto; padding: 16px;
    background: #0f172a; color: #e2e8f0;
    font-family: 'Segoe UI', sans-serif;
}
</style>
</head>
<body>
"#;

fn nav_html(current: usize, entries: &[HtmlEntry]) -> String {
    let (name, filename) = entries
        .get(current)
        .map(|e| (e.name.as_str(), e.filename.as_str()))
        .unwrap_or(("", ""));
    format!(
        r#"
<div id="__nav" style="
    display: flex; align-items: center; gap: 8px;
    padding: 6px 12px;
    background: #1e293b; border-bottom: 1px solid #334155;
    font-family: 'Segoe UI', sans-serif; position: relative;">
  <span style="font-size: 13px; font-weight: 700; color: #38bdf8;">KUI</span>
  <span style="font-size: 11px; color: #64748b;">Script</span>
  <div id="__btn_prev" style="
      padding: 2px 10px; font-size: 14px; color: #94a3b8;
      background: #0f172a; border: 1px solid #475569; border-radius: 4px;
      cursor: pointer;">&#9664;</div>
  <span id="__counter" style="font-size: 11px; color: #94a3b8; min-width: 50px; text-align: center;">{} / {}</span>
  <div id="__btn_next" style="
      padding: 2px 10px; font-size: 14px; color: #94a3b8;
      background: #0f172a; border: 1px solid #475569; border-radius: 4px;
      cursor: pointer;">&#9654;</div>
  <span id="__title" style="
      flex: 1; font-size: 13px; font-weight: 600; color: #e2e8f0;
      overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{name}</span>
  <span id="__filename" style="font-size: 10px; color: #475569;">{filename}</span>
  <div id="__btn_reload" style="
      padding: 2px 10px; font-size: 13px; color: #94a3b8;
      background: #0f172a; border: 1px solid #475569; border-radius: 4px;
      cursor: pointer;">&#8635;</div>
</div>
"#,
        current + 1,
        entries.len()
    )
}

fn text_of(doc: &Document, id: &str) -> Option<String> {
    let el = doc.get_element_by_id(id).ok()??;
    Some(el.text_content().unwrap_or_default())
}

/// Copies #result / #detail into the status bar (the original's post-load block).
fn update_status(doc: &Document) {
    if let Ok(Some(display)) = doc.get_element_by_id("__result_display") {
        match text_of(doc, "result") {
            Some(result) => {
                let _ = display.set_text_content(&result);
                let color = if result == "PASS" {
                    "#22c55e"
                } else {
                    "#ef4444"
                };
                let _ = display.set_style_property("color", color, false);
            }
            None => {
                let _ = display.set_text_content("(no #result element)");
                let _ = display.set_style_property("color", "#f59e0b", false);
            }
        }
    }
    if let (Some(detail), Ok(Some(el))) = (
        text_of(doc, "detail"),
        doc.get_element_by_id("__detail_display"),
    ) {
        let _ = el.set_text_content(&detail);
    }
}

fn listen(
    b: &Rc<Browser>,
    el: Option<Element<'_>>,
    event: &str,
    action: fn(&Rc<Browser>, &kui::Event<'_>),
) {
    let Some(el) = el else { return };
    let browser = b.clone();
    let _ = el.add_event_listener(event, move |e| action(&browser, e));
}

fn go_prev(b: &Rc<Browser>) {
    if b.current.get() > 0 {
        load_entry(b, b.current.get() - 1);
    }
}

fn go_next(b: &Rc<Browser>) {
    if b.current.get() + 1 < b.entries.borrow().len() {
        load_entry(b, b.current.get() + 1);
    }
}

/// Keyboard navigation: Left / Right arrows to switch, F5 to reload.
fn add_keyboard_listener(b: &Rc<Browser>) {
    listen(b, b.doc.body().ok().flatten(), "keydown", |b, e| {
        match e.key().unwrap_or_default().as_str() {
            "ArrowLeft" | "Left" => go_prev(b),
            "ArrowRight" | "Right" => go_next(b),
            "F5" => load_entry(b, b.current.get()),
            _ => {}
        }
    });
}

fn wire_nav_events(b: &Rc<Browser>) {
    listen(
        b,
        b.doc.get_element_by_id("__btn_prev").ok().flatten(),
        "click",
        |b, _| go_prev(b),
    );
    listen(
        b,
        b.doc.get_element_by_id("__btn_next").ok().flatten(),
        "click",
        |b, _| go_next(b),
    );
    listen(
        b,
        b.doc.get_element_by_id("__btn_reload").ok().flatten(),
        "click",
        |b, _| load_entry(b, b.current.get()),
    );
    add_keyboard_listener(b);
}

/// Builds the wrapper page for entry index and runs its scripts (the original's loadEntry).
fn load_entry(b: &Rc<Browser>, index: usize) {
    let Some(entry) = b.entries.borrow().get(index).cloned() else {
        return;
    };
    b.current.set(index);
    let content = match std::fs::read(&entry.path) {
        Ok(bytes) if !bytes.is_empty() => String::from_utf8_lossy(&bytes).into_owned(),
        _ => {
            eprintln!("[Script Example] Failed to read: {}", entry.path.display());
            return;
        }
    };
    let lower = content.to_ascii_lowercase(); // same byte offsets, like the original's ::tolower

    let mut page = String::from(PAGE_HEAD);
    page += &nav_html(index, &b.entries.borrow());
    page += "<div id=\"__content_area\">\n";
    // Also extract the first <style> block
    if let (Some(start), Some(end)) = (lower.find("<style>"), lower.find("</style>")) {
        if start <= end {
            page += &content[start..end + 8];
            page += "\n";
        }
    }
    // Inner body content (or the whole file when it has no body tags)
    match (lower.find("<body"), lower.find("</body>")) {
        (Some(open), Some(close)) => {
            if let Some(tag_end) = content[open..].find('>').map(|i| open + i) {
                if tag_end < close {
                    page += &content[tag_end + 1..close];
                }
            }
        }
        _ => page += &content,
    }
    page += "\n</div>\n";
    page += STATUS_HTML;
    page += "\n</body>\n</html>";

    let _ = b.doc.clone().load_html(&page);
    let _ = b
        .window
        .set_title(&format!("KUI Script Example — {}", entry.name));

    // Create / reset the script engine and run the <script> blocks of the original file
    if let Ok(engine) = ScriptEngine::new() {
        drop(b.engine.replace(Some(engine)));
        if let Some(engine) = b.engine.borrow().as_ref() {
            let _ = engine.bind_document(&b.doc);
            let (mut search, mut script_index) = (0, 0);
            while let Some(open) = lower[search..].find("<script>").map(|i| search + i) {
                let Some(close) = lower[open..].find("</script>").map(|i| open + i) else {
                    break;
                };
                let _ = engine.eval(
                    &content[open + 8..close],
                    &format!("{}#{script_index}", entry.filename),
                );
                script_index += 1;
                search = close + 9;
            }
            // Tick timers: the original ticks 0 and 100 ms right after the scripts.
            let _ = engine.tick_timers(0.0);
            let _ = engine.tick_timers(100.0);
        }
    }

    // Update status bar with the script result
    update_status(&b.doc);

    wire_nav_events(b);
    eprintln!("[Script Example] Loaded: {}", entry.filename);
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let _ = ctx.document.register_builtin_components();
    let browser = Rc::new(Browser {
        doc: ctx.document.clone(),
        window: ctx.window.clone(),
        entries: RefCell::new(Vec::new()),
        current: Cell::new(0),
        engine: RefCell::new(None),
    });

    // Parse arguments
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut file_to_load, mut dir_to_scan, mut list_mode) =
        (None::<PathBuf>, None::<PathBuf>, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => list_mode = true,
            "--file" if i + 1 < args.len() => {
                i += 1;
                file_to_load = Some(PathBuf::from(&args[i]));
            }
            "--dir" if i + 1 < args.len() => {
                i += 1;
                dir_to_scan = Some(PathBuf::from(&args[i]));
            }
            arg if !arg.starts_with('-') => {
                // Positional: either a file or a directory
                if Path::new(arg).is_file() {
                    file_to_load = Some(PathBuf::from(arg));
                } else {
                    dir_to_scan = Some(PathBuf::from(arg));
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Default: scan script_html / tests/script/html relative to the exe, then the cwd
    if dir_to_scan.is_none() && file_to_load.is_none() {
        const DEFAULT_DIRS: [&str; 7] = [
            "script_html",
            "assets/script_html",
            "tests/script/html",
            "../tests/script/html",
            "../../tests/script/html",
            "../../../tests/script/html",
            "../../../../tests/script/html",
        ];
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from));
        dir_to_scan = exe_dir
            .and_then(|dir| {
                DEFAULT_DIRS
                    .iter()
                    .map(|d| dir.join(d))
                    .find(|p| p.is_dir())
            })
            .or_else(|| DEFAULT_DIRS.iter().map(PathBuf::from).find(|p| p.is_dir()));
    }
    if let Some(dir) = &dir_to_scan {
        *browser.entries.borrow_mut() = scan_directory(dir);
    }

    if list_mode {
        let entries = browser.entries.borrow();
        println!("Available HTML files ({}):", entries.len());
        for (i, e) in entries.iter().enumerate() {
            println!("  [{i:2}] {:<30} {}", e.name, e.filename);
        }
        ctx.platform.quit(0)?;
        return Ok(());
    }

    if let Some(path) = file_to_load {
        // loadSingleFile: a one-entry list
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        *browser.entries.borrow_mut() = vec![HtmlEntry {
            name: pretty_name(&filename),
            filename,
            path,
        }];
        load_entry(&browser, 0);
    } else if !browser.entries.borrow().is_empty() {
        load_entry(&browser, 0);
    } else {
        // The packaged page is the original's "No HTML files found" page; keep it.
        add_keyboard_listener(&browser);
    }
    Ok(())
}
