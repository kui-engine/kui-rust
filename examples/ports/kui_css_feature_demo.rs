// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_css_feature_demo.rs; edit that file, not this copy.
//! Host logic of examples/css_feature_demo/main.cpp: builds the "C++ Dynamic DOM" section into
//! #dynamic-section through the DOM API, updates #status-text, and binds the sidebar nav items
//! (active class + scroll the section into view). The original loads css_feature_test.html itself;
//! the runner loads that same file as this example's page.
//! The nav items scroll their section into view with `Element::scroll_into_view`.

use crate::canonical_runner::Context;
use kui::{Document, Element};
use std::error::Error;

/// The original's createElement + setTextContent + style()->setProperty.
fn make_el<'a>(
    doc: &'a Document,
    tag: &str,
    text: &str,
    styles: &[(&str, &str)],
) -> Option<Element<'a>> {
    let el = doc.create_element(tag).ok()?;
    if !text.is_empty() {
        let _ = el.set_text_content(text);
    }
    for (name, value) in styles {
        let _ = el.set_style_property(name, value, false);
    }
    Some(el)
}

fn append(parent: &Element<'_>, child: Option<Element<'_>>) {
    if let Some(child) = child {
        let _ = parent.append_child(&child);
    }
}

fn heading(doc: &Document, section: &Element<'_>, text: &str) {
    append(
        section,
        make_el(
            doc,
            "div",
            text,
            &[
                ("font-size", "12px"),
                ("color", "var(--c-dim)"),
                ("text-transform", "uppercase"),
                ("letter-spacing", "1px"),
                ("margin-bottom", "6px"),
            ],
        ),
    );
}

fn build_dynamic_section(doc: &Document) {
    let Ok(Some(container)) = doc.get_element_by_id("dynamic-section") else {
        return;
    };
    let Some(section) = make_el(
        doc,
        "div",
        "",
        &[
            ("background", "var(--c-surface)"),
            ("border", "1px solid var(--c-border)"),
            ("border-radius", "var(--radius)"),
            ("padding", "16px"),
            ("margin-bottom", "16px"),
        ],
    ) else {
        return;
    };
    append(
        &section,
        make_el(
            doc,
            "div",
            "C++ Dynamic DOM Manipulation",
            &[
                ("font-size", "15px"),
                ("font-weight", "700"),
                ("color", "var(--c-accent)"),
                ("margin-bottom", "10px"),
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", "8px"),
            ],
        ),
    );
    append(
        &section,
        make_el(
            doc,
            "div",
            "The following elements are created entirely via C++ API.",
            &[
                ("font-size", "11px"),
                ("color", "var(--c-dim)"),
                ("margin-bottom", "12px"),
            ],
        ),
    );

    // --- 1. Dynamic color palette ---
    heading(doc, &section, "1. Dynamic Color Palette (C++ generated)");
    if let Some(row) = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "flex"),
            ("gap", "4px"),
            ("flex-wrap", "wrap"),
            ("margin-bottom", "12px"),
        ],
    ) {
        for i in 0..16 {
            let hue = i * 22;
            let color = format!("hsl({hue}, 70%, 55%)");
            append(
                &row,
                make_el(
                    doc,
                    "div",
                    &hue.to_string(),
                    &[
                        ("width", "36px"),
                        ("height", "36px"),
                        ("background", &color),
                        ("border-radius", "6px"),
                        ("display", "flex"),
                        ("align-items", "center"),
                        ("justify-content", "center"),
                        ("font-size", "9px"),
                        ("color", "white"),
                        ("font-weight", "700"),
                        ("transition", "transform 0.2s ease, box-shadow 0.2s"),
                        ("cursor", "pointer"),
                    ],
                ),
            );
        }
        append(&section, Some(row));
    }

    // --- 2. Dynamic flex layout (proportional bars) ---
    heading(doc, &section, "2. Dynamic Flex Layout (proportional bars)");
    if let Some(row) = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "flex"),
            ("gap", "3px"),
            ("height", "30px"),
            ("margin-bottom", "12px"),
        ],
    ) {
        let labels = [
            "CSS 2.1",
            "Selectors",
            "Flexbox",
            "Grid",
            "Transforms",
            "Animations",
            "Colors",
        ];
        let values = [98, 85, 100, 95, 100, 100, 88];
        let colors = [
            "#e17055", "#fdcb6e", "#00b894", "#0984e3", "#6c5ce7", "#a29bfe", "#fd79a8",
        ];
        for i in 0..7 {
            let grow = values[i].to_string();
            append(
                &row,
                make_el(
                    doc,
                    "div",
                    labels[i],
                    &[
                        ("flex-grow", &grow),
                        ("background", colors[i]),
                        ("border-radius", "4px"),
                        ("display", "flex"),
                        ("align-items", "center"),
                        ("justify-content", "center"),
                        ("font-size", "9px"),
                        ("color", "white"),
                        ("font-weight", "700"),
                        ("overflow", "hidden"),
                        ("white-space", "nowrap"),
                        ("transition", "flex-grow 0.5s ease"),
                    ],
                ),
            );
        }
        append(&section, Some(row));
    }

    // --- 3. Dynamic grid dashboard ---
    heading(
        doc,
        &section,
        "3. Dynamic Grid Dashboard (C++ data binding)",
    );
    if let Some(grid) = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "grid"),
            ("grid-template-columns", "repeat(4, 1fr)"),
            ("gap", "8px"),
            ("margin-bottom", "12px"),
        ],
    ) {
        let stats = [
            ("CSS Properties", "307", "#6c5ce7", "+12"),
            ("Selectors", "44/52", "#00b894", "85%"),
            ("Layout Tests", "100%", "#0984e3", "Flex+Grid"),
            ("Animations", "11/11", "#fd79a8", "100%"),
        ];
        for (label, value, color, trend) in stats {
            let border = format!("3px solid {color}");
            let Some(card) = make_el(
                doc,
                "div",
                "",
                &[
                    ("background", "var(--c-card)"),
                    ("border-radius", "8px"),
                    ("padding", "12px"),
                    ("border-left", &border),
                ],
            ) else {
                continue;
            };
            append(
                &card,
                make_el(
                    doc,
                    "div",
                    label,
                    &[
                        ("font-size", "10px"),
                        ("color", "var(--c-dim)"),
                        ("margin-bottom", "4px"),
                    ],
                ),
            );
            append(
                &card,
                make_el(
                    doc,
                    "div",
                    value,
                    &[
                        ("font-size", "20px"),
                        ("font-weight", "800"),
                        ("color", color),
                    ],
                ),
            );
            append(
                &card,
                make_el(
                    doc,
                    "div",
                    trend,
                    &[
                        ("font-size", "10px"),
                        ("color", "var(--c-dim)"),
                        ("margin-top", "2px"),
                    ],
                ),
            );
            append(&grid, Some(card));
        }
        append(&section, Some(grid));
    }

    // --- 4. Programmatic style manipulation ---
    heading(doc, &section, "4. Programmatic Style Manipulation");
    if let Some(row) = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "flex"),
            ("gap", "6px"),
            ("flex-wrap", "wrap"),
            ("margin-bottom", "12px"),
        ],
    ) {
        let demos: [(&str, &[(&str, &str)]); 8] = [
            (
                "border-radius: 50%",
                &[
                    ("width", "40px"),
                    ("height", "40px"),
                    ("background", "var(--c-primary)"),
                    ("border-radius", "50%"),
                    ("display", "flex"),
                    ("align-items", "center"),
                    ("justify-content", "center"),
                    ("color", "white"),
                    ("font-size", "9px"),
                ],
            ),
            (
                "box-shadow",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-card)"),
                    ("border-radius", "6px"),
                    ("box-shadow", "0 4px 12px rgba(108,92,231,0.4)"),
                    ("font-size", "10px"),
                    ("color", "var(--c-dim)"),
                ],
            ),
            (
                "gradient bg",
                &[
                    ("padding", "8px"),
                    ("background", "linear-gradient(135deg, #6c5ce7, #fd79a8)"),
                    ("border-radius", "6px"),
                    ("color", "white"),
                    ("font-size", "10px"),
                ],
            ),
            (
                "transform",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-card)"),
                    ("border-radius", "6px"),
                    ("transform", "rotate(5deg) scale(1.05)"),
                    ("font-size", "10px"),
                    ("color", "var(--c-dim)"),
                ],
            ),
            (
                "opacity: 0.5",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-accent)"),
                    ("border-radius", "6px"),
                    ("opacity", "0.5"),
                    ("color", "white"),
                    ("font-size", "10px"),
                ],
            ),
            (
                "filter: blur",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-success)"),
                    ("border-radius", "6px"),
                    ("filter", "blur(1px)"),
                    ("color", "white"),
                    ("font-size", "10px"),
                ],
            ),
            (
                "text-shadow",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-card)"),
                    ("border-radius", "6px"),
                    ("text-shadow", "0 0 8px var(--c-primary)"),
                    ("font-size", "10px"),
                    ("color", "var(--c-secondary)"),
                ],
            ),
            (
                "outline",
                &[
                    ("padding", "8px"),
                    ("background", "var(--c-card)"),
                    ("border-radius", "6px"),
                    ("outline", "2px solid var(--c-accent)"),
                    ("outline-offset", "3px"),
                    ("font-size", "10px"),
                    ("color", "var(--c-dim)"),
                ],
            ),
        ];
        for (label, styles) in demos {
            append(&row, make_el(doc, "div", label, styles));
        }
        append(&section, Some(row));
    }

    // --- 5. Complex nested layout ---
    heading(
        doc,
        &section,
        "5. Complex Nested Layout (sidebar + grid + flex)",
    );
    if let Some(layout) = make_el(
        doc,
        "div",
        "",
        &[
            ("display", "flex"),
            ("gap", "8px"),
            ("height", "100px"),
            ("margin-bottom", "12px"),
        ],
    ) {
        if let Some(mini_sidebar) = make_el(
            doc,
            "div",
            "",
            &[
                ("width", "60px"),
                ("background", "var(--c-card)"),
                ("border-radius", "6px"),
                ("padding", "6px"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", "4px"),
            ],
        ) {
            for i in 0..4 {
                let first = i == 0;
                append(
                    &mini_sidebar,
                    make_el(
                        doc,
                        "div",
                        &(i + 1).to_string(),
                        &[
                            (
                                "background",
                                if first {
                                    "var(--c-primary)"
                                } else {
                                    "var(--c-border)"
                                },
                            ),
                            ("border-radius", "4px"),
                            ("padding", "2px"),
                            ("text-align", "center"),
                            ("font-size", "10px"),
                            ("color", if first { "white" } else { "var(--c-dim)" }),
                        ],
                    ),
                );
            }
            append(&layout, Some(mini_sidebar));
        }
        if let Some(content_grid) = make_el(
            doc,
            "div",
            "",
            &[
                ("flex", "1"),
                ("display", "grid"),
                ("grid-template-columns", "1fr 1fr"),
                ("grid-template-rows", "1fr 1fr"),
                ("gap", "4px"),
            ],
        ) {
            let cells = [
                ("var(--c-danger)", "Errors: 0"),
                ("var(--c-success)", "Tests: 730"),
                ("var(--c-primary)", "Coverage: 91%"),
                ("var(--c-warning)", "Build: OK"),
            ];
            for (color, label) in cells {
                append(
                    &content_grid,
                    make_el(
                        doc,
                        "div",
                        label,
                        &[
                            ("background", color),
                            ("border-radius", "6px"),
                            ("display", "flex"),
                            ("align-items", "center"),
                            ("justify-content", "center"),
                            ("font-size", "10px"),
                            ("color", "white"),
                            ("font-weight", "700"),
                        ],
                    ),
                );
            }
            append(&layout, Some(content_grid));
        }
        append(&section, Some(layout));
    }

    append(&container, Some(section));

    // Update status bar
    if let Ok(Some(status)) = doc.get_element_by_id("status-text") {
        let _ = status.set_text_content("Loaded: 16 sections, 307+ CSS properties tested");
    }
}

const NAV_MAPPINGS: [(&str, &str); 15] = [
    ("nav-1", "sec-1"),
    ("nav-2", "sec-2"),
    ("nav-3", "sec-3"),
    ("nav-4", "sec-4"),
    ("nav-5", "sec-5"),
    ("nav-7", "sec-7"),
    ("nav-8", "sec-8"),
    ("nav-9", "sec-9"),
    ("nav-10", "sec-10"),
    ("nav-11", "sec-11"),
    ("nav-13", "sec-13"),
    ("nav-14", "sec-14"),
    ("nav-15", "sec-15"),
    ("nav-16", "sec-16"),
    ("nav-cpp", "dynamic-section"),
];

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let _ = ctx.document.register_builtin_components(); // before loadHTML in the original

    // Build the dynamic C++ section after HTML is loaded
    build_dynamic_section(ctx.document);

    // Bind sidebar nav-item click events (pairs whose nav or section is missing are skipped)
    for (index, (nav_id, sec_id)) in NAV_MAPPINGS.iter().enumerate() {
        let Some(nav) = ctx.document.get_element_by_id(nav_id)? else {
            continue;
        };
        if ctx.document.get_element_by_id(sec_id)?.is_none() {
            continue;
        }
        let doc = ctx.document.clone();
        nav.add_event_listener("click", move |_event| {
            // Update active state on all nav items
            for (id, _) in NAV_MAPPINGS {
                if let Ok(Some(n)) = doc.get_element_by_id(id) {
                    let _ = n.set_attribute("class", "nav-item");
                }
            }
            if let Ok(Some(n)) = doc.get_element_by_id(NAV_MAPPINGS[index].0) {
                let _ = n.set_attribute("class", "nav-item active");
            }
            // Scroll the section into view
            if let Ok(Some(section)) = doc.get_element_by_id(NAV_MAPPINGS[index].1) {
                let _ = section.scroll_into_view(true);
            }
        })?;
    }
    eprintln!("[CSS Demo] Running \u{2014} 16 CSS categories, 332 CSS properties");
    Ok(())
}
