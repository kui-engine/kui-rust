// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/native_platform_04_file_dialog.rs; edit that file, not this copy.
//! Host logic of examples/native/platform/04_file_dialog/main.cpp: Open File / Save File / Select
//! Folder show the native dialogs and write the chosen path, or "... cancelled.", into #result.

use crate::canonical_runner::Context;
use kui::{FileDialogOptions, Platform};
use std::error::Error;

type Dialog = fn(&Platform) -> kui::Result<Option<String>>;

fn open(platform: &Platform) -> kui::Result<Option<String>> {
    platform.open_file_dialog(&FileDialogOptions {
        title: "Open a document".into(),
        filters: vec!["*.html".into(), "*.css".into(), "*.txt".into()],
        ..FileDialogOptions::default()
    })
}

fn save(platform: &Platform) -> kui::Result<Option<String>> {
    platform.save_file_dialog(&FileDialogOptions {
        title: "Save a document".into(),
        default_path: "example.html".into(),
        filters: vec!["*.html".into()],
        ..FileDialogOptions::default()
    })
}

fn folder(platform: &Platform) -> kui::Result<Option<String>> {
    platform.select_folder_dialog("Select a folder")
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let buttons: [(&str, Dialog, &'static str); 3] = [
        ("openBtn", open, "Open cancelled."),
        ("saveBtn", save, "Save cancelled."),
        ("folderBtn", folder, "Folder selection cancelled."),
    ];
    for (id, dialog, cancelled) in buttons {
        let button = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let (doc, platform) = (ctx.document.clone(), ctx.platform.clone());
        button.add_event_listener("click", move |_event| {
            let text = match dialog(&platform) {
                Ok(Some(path)) => path,
                _ => cancelled.to_string(),
            };
            if let Ok(Some(result)) = doc.get_element_by_id("result") {
                let _ = result.set_text_content(&text);
            }
        })?;
    }
    Ok(())
}
