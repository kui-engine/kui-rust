// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_file_explorer.rs; edit that file, not this copy.
//! Host logic of examples/file_explorer/main.cpp: lists a real directory into #file-grid (folders
//! first, then by name; icon text and class by extension), builds the #breadcrumb from the path
//! and a "Quick Access" #sidebar. Clicking a folder, a breadcrumb segment or a quick link
//! navigates there; clicking a file does nothing.
//!
//! The original starts in (and every quick link points at) the current directory; the
//! environment variable KUI_EXAMPLE_START_DIR overrides that start directory (test hook).
//! Instead of one listener per created element, each of the three containers listens once and
//! the clicked item carries its target directory in a data-path attribute.

use crate::canonical_runner::Context;
use kui::{Document, Element};
use std::error::Error;
use std::path::{Path, PathBuf};

struct Entry {
    name: String,
    is_dir: bool,
}

/// The original's getFileIcon / getIconClass (their extension lists differ: .svg and .rs only
/// get an icon text, not a class).
fn icon_for(entry: &Entry) -> (&'static str, &'static str) {
    if entry.is_dir {
        return ("[D]", "file-icon folder");
    }
    // Path::extension matches fs::path::extension: ".hidden" has none.
    let ext = Path::new(&entry.name)
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let is = |list: &[&str]| list.contains(&ext.as_str());
    let docs = [".txt", ".md", ".doc", ".pdf"];
    let text = if is(&[".png", ".jpg", ".bmp", ".gif", ".svg"]) {
        "[I]"
    } else if is(&[".cpp", ".h", ".c", ".py", ".js", ".rs"]) {
        "[<>]"
    } else if is(&docs) {
        "[T]"
    } else {
        "[F]"
    };
    let class = if is(&[".png", ".jpg", ".bmp", ".gif"]) {
        "file-icon image"
    } else if is(&[".cpp", ".h", ".c", ".py", ".js"]) {
        "file-icon code"
    } else if is(&docs) {
        "file-icon doc"
    } else {
        "file-icon file"
    };
    (text, class)
}

/// fs::directory_iterator: errors give an empty list; `Path::is_dir` follows links like
/// directory_entry::is_directory.
fn list_directory(dir: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)
        .map(|items| {
            items
                .flatten()
                .map(|item| Entry {
                    name: item.file_name().to_string_lossy().into_owned(),
                    is_dir: item.path().is_dir(),
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// createElement(tag) with class, text and data-path (each left out when empty).
fn make<'d>(
    doc: &'d Document,
    tag: &str,
    class: &str,
    text: &str,
    target: &str,
) -> Option<Element<'d>> {
    let element = doc.create_element(tag).ok()?;
    if !class.is_empty() {
        let _ = element.set_attribute("class", class);
    }
    if !text.is_empty() {
        let _ = element.set_text_content(text);
    }
    if !target.is_empty() {
        let _ = element.set_attribute("data-path", target);
    }
    Some(element)
}

fn append(parent: &Element<'_>, child: Option<Element<'_>>) {
    if let Some(child) = child {
        let _ = parent.append_child(&child);
    }
}

/// The element with that id, emptied.
fn cleared<'d>(doc: &'d Document, id: &str) -> Option<Element<'d>> {
    let element = doc.get_element_by_id(id).ok()??;
    while let Ok(Some(child)) = element.first_child() {
        let _ = element.remove_child(&child);
    }
    Some(element)
}

fn navigate_to(doc: &Document, path: &Path) {
    if let Some(bc) = cleared(doc, "breadcrumb") {
        let mut segments: Vec<&Path> = path.ancestors().collect();
        segments.reverse();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                append(&bc, make(doc, "span", "sep", "/", ""));
            }
            // The root has no file name and is shown as itself ("C:\", "/").
            let name = segment
                .file_name()
                .unwrap_or(segment.as_os_str())
                .to_string_lossy();
            append(
                &bc,
                make(doc, "span", "seg", &name, &segment.to_string_lossy()),
            );
        }
    }
    if let Some(grid) = cleared(doc, "file-grid") {
        let entries = list_directory(path);
        if entries.is_empty() {
            append(&grid, make(doc, "div", "empty", "Empty folder", ""));
        }
        for entry in &entries {
            let target = if entry.is_dir {
                path.join(&entry.name).to_string_lossy().into_owned()
            } else {
                String::new()
            };
            let Some(item) = make(doc, "div", "file-item", "", &target) else {
                continue;
            };
            let (icon, class) = icon_for(entry);
            append(&item, make(doc, "div", class, icon, ""));
            append(&item, make(doc, "div", "file-name", &entry.name, ""));
            append(&grid, Some(item));
        }
    }
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let start = match std::env::var_os("KUI_EXAMPLE_START_DIR").filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir()?,
    };
    let start = std::path::absolute(&start).unwrap_or(start);
    let start = PathBuf::from_iter(start.components()); // drops a trailing separator

    let doc = ctx.document;
    if let Some(sidebar) = cleared(doc, "sidebar") {
        append(
            &sidebar,
            make(doc, "div", "sidebar-section", "Quick Access", ""),
        );
        for (icon, name) in [("~", "Home"), ("D", "Desktop"), ("[>", "Documents")] {
            let Some(item) = make(doc, "div", "sidebar-item", "", &start.to_string_lossy()) else {
                continue;
            };
            append(&item, make(doc, "span", "sidebar-icon", icon, ""));
            append(&item, make(doc, "span", "", name, ""));
            append(&sidebar, Some(item));
        }
    }

    for id in ["breadcrumb", "sidebar", "file-grid"] {
        let doc = ctx.document.clone();
        ctx.document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?
            .add_event_listener("click", move |event| {
                let Ok(Some(target)) = event.target(&doc) else {
                    return;
                };
                let Ok(Some(item)) = target.closest("[data-path]") else {
                    return;
                };
                if let Ok(path) = item.attribute("data-path") {
                    if !path.is_empty() {
                        navigate_to(&doc, Path::new(&path));
                    }
                }
            })?;
    }
    navigate_to(doc, &start);
    Ok(())
}
