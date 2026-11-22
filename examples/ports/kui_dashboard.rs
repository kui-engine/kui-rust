// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_dashboard.rs; edit that file, not this copy.
//! Host logic of examples/dashboard/main.cpp: fills #table-body with seven .table-row rows, makes
//! the clicked nav item the only .active one, and Refresh writes a random 1400..1599 session
//! count into #stat-sessions.

use crate::canonical_runner::Context;
use std::collections::hash_map::RandomState;
use std::error::Error;
use std::hash::{BuildHasher, Hasher};

// (name, status, badge class, value, change, positive)
const ROWS: [(&str, &str, &str, &str, &str, bool); 7] = [
    (
        "Project Alpha",
        "Active",
        "badge-success",
        "$12,400",
        "+5.2%",
        true,
    ),
    (
        "Widget Pro",
        "Active",
        "badge-success",
        "$8,200",
        "+12.1%",
        true,
    ),
    (
        "API Gateway",
        "Warning",
        "badge-warning",
        "$4,800",
        "-1.5%",
        false,
    ),
    (
        "Mobile App",
        "Active",
        "badge-success",
        "$15,300",
        "+8.7%",
        true,
    ),
    (
        "Auth Service",
        "Critical",
        "badge-danger",
        "$2,100",
        "-15.3%",
        false,
    ),
    (
        "Data Pipeline",
        "Active",
        "badge-success",
        "$6,700",
        "+3.4%",
        true,
    ),
    (
        "ML Platform",
        "Warning",
        "badge-warning",
        "$9,500",
        "+0.2%",
        true,
    ),
];

const NAV_IDS: [&str; 4] = [
    "nav-overview",
    "nav-analytics",
    "nav-reports",
    "nav-settings",
];

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let body = ctx
        .document
        .get_element_by_id("table-body")?
        .ok_or("page has no #table-body")?;
    for (name, status, badge, value, change, positive) in ROWS {
        let row = ctx.document.create_element("div")?;
        row.set_class_name("table-row")?;
        let color = if positive {
            "var(--success)"
        } else {
            "var(--danger)"
        };
        row.set_inner_html(&format!(
            "<div class=\"col-name\">{name}</div>\
             <div class=\"col-status\"><span class=\"badge {badge}\">{status}</span></div>\
             <div class=\"col-value\">{value}</div>\
             <div class=\"col-change\" style=\"color: {color}\">{change}</div>"
        ))?;
        body.append_child(&row)?;
    }

    for id in NAV_IDS {
        let item = ctx
            .document
            .get_element_by_id(id)?
            .ok_or_else(|| format!("page has no #{id}"))?;
        let doc = ctx.document.clone();
        item.add_event_listener("click", move |event| {
            for nid in NAV_IDS {
                if let Ok(Some(el)) = doc.get_element_by_id(nid) {
                    let _ = el.class_list_remove("active");
                }
            }
            if let Ok(Some(target)) = event.target(&doc) {
                let _ = target.class_list_add("active");
            }
        })?;
    }

    let refresh = ctx
        .document
        .get_element_by_id("refreshBtn")?
        .ok_or("page has no #refreshBtn")?;
    let doc = ctx.document.clone();
    refresh.add_event_listener("click", move |_event| {
        // Stand-in for std::rand() without pulling a crate in.
        let random = RandomState::new().build_hasher().finish();
        if let Ok(Some(sessions)) = doc.get_element_by_id("stat-sessions") {
            let _ = sessions.set_text_content(&(1400 + random % 200).to_string());
        }
    })?;
    Ok(())
}
