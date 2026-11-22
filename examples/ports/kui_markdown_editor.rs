// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_markdown_editor.rs; edit that file, not this copy.
//! Host logic of examples/markdown_editor/main.cpp: the editor starts with the sample Markdown;
//! Clear empties it and Load Sample puts the sample back. After either, the preview shows the
//! converted HTML (as text, like the original: "for now, set as text") and #stats counts words
//! and characters of the content.
//!
//! The original never listens to the textarea: typing does not update the preview or the stats.
//! Its header checks compare a 3-char prefix with "### " and a 2-char prefix with "## ", so they
//! never match; this port does what they mean to (<h3> / <h2>).

use crate::canonical_runner::Context;
use kui::Document;
use std::error::Error;

const DEFAULT_MARKDOWN: &str = r#"# Welcome to KUI Markdown Editor

This is a **live preview** markdown editor built with KUI.

## Features

- Real-time markdown to HTML preview
- Dark theme with syntax highlighting
- Word and character counting
- Split pane layout

## Code Example

```cpp
#include <kui/kui.h>

int main() {
    kui::Platform::init();
    auto window = kui::Window::create({.title = "Hello"});
    return kui::Platform::run();
}
```

## Formatting

You can use **bold**, *italic*, and `inline code`.

> Blockquotes are also supported for callouts and notes.

---

### Lists

- Item one
- Item two
- Item three

Enjoy writing with KUI!
"#;

/// regex_replace(text, "<d>(.+?)<d>", "<open>$1<close>"): leftmost match, lazy body of >= 1 char.
fn replace_pairs(text: &str, delim: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(body) = rest.strip_prefix(delim) {
            let first = body.chars().next().map_or(0, char::len_utf8);
            let end = if first > 0 {
                body[first..].find(delim)
            } else {
                None
            };
            if let Some(end) = end {
                let end = first + end;
                out.push_str(open);
                out.push_str(&body[..end]);
                out.push_str(close);
                rest = &body[end + delim.len()..];
                continue;
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// The original's basic-subset converter, line by line (`split_terminator` yields no line after
/// the final newline, like std::getline).
fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_code = false;
    for line in md.split_terminator('\n') {
        if line.starts_with("```") {
            html.push_str(if in_code { "</pre>\n" } else { "<pre><code>" });
            in_code = !in_code;
        } else if in_code {
            for c in line.chars() {
                match c {
                    '<' => html.push_str("&lt;"),
                    '>' => html.push_str("&gt;"),
                    '&' => html.push_str("&amp;"),
                    _ => html.push(c),
                }
            }
            html.push('\n');
        } else if line == "---" || line == "***" || line == "___" {
            html.push_str("<hr>\n");
        } else if let Some(text) = line.strip_prefix("### ") {
            html.push_str(&format!("<h3>{text}</h3>\n"));
        } else if let Some(text) = line.strip_prefix("## ") {
            html.push_str(&format!("<h2>{text}</h2>\n"));
        } else if let Some(text) = line.strip_prefix("# ") {
            html.push_str(&format!("<h1>{text}</h1>\n"));
        } else if let Some(text) = line.strip_prefix("> ") {
            html.push_str(&format!("<blockquote><p>{text}</p></blockquote>\n"));
        } else if let Some(text) = line.strip_prefix("- ") {
            html.push_str(&format!("<li>{text}</li>\n"));
        } else if line.is_empty() {
            html.push_str("<br>\n");
        } else {
            let processed = replace_pairs(line, "**", "<strong>", "</strong>");
            let processed = replace_pairs(&processed, "*", "<em>", "</em>");
            let processed = replace_pairs(&processed, "`", "<code>", "</code>");
            html.push_str(&format!("<p>{processed}</p>\n"));
        }
    }
    if in_code {
        html.push_str("</pre>\n");
    }
    html
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Ok(Some(element)) = doc.get_element_by_id(id) {
        let _ = element.set_text_content(text);
    }
}

fn update_preview(doc: &Document, content: &str) {
    let Ok(Some(preview)) = doc.get_element_by_id("preview") else {
        return;
    };
    while let Ok(Some(child)) = preview.first_child() {
        let _ = preview.remove_child(&child);
    }
    if let Ok(wrapper) = doc.create_element("div") {
        let _ = wrapper.set_text_content(&markdown_to_html(content));
        let _ = preview.append_child(&wrapper);
    }
    let words = content
        .split([' ', '\n', '\r', '\t'])
        .filter(|word| !word.is_empty())
        .count();
    set_text(
        doc,
        "stats",
        &format!("{words} words, {} characters", content.len()),
    );
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    set_text(ctx.document, "editor", DEFAULT_MARKDOWN);
    for (id, content) in [("btn-clear", ""), ("btn-sample", DEFAULT_MARKDOWN)] {
        let doc = ctx.document.clone();
        ctx.document
            .get_element_by_id(id)?
            .ok_or(format!("page has no #{id}"))?
            .add_event_listener("click", move |_event| {
                set_text(&doc, "editor", content);
                update_preview(&doc, content);
            })?;
    }
    update_preview(ctx.document, DEFAULT_MARKDOWN);
    Ok(())
}
