// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_dom_tree.rs; edit that file, not this copy.
//! API tour: host-side DOM tree operations (examples/api_tours/dom_tree). On start the host calls
//! every lookup / traversal / tree-editing / HTML method once, checks the result against what the
//! C ABI documents and logs one row per check into #tour-log; #tour-result ends as "PASS n/n".
//! The panel buttons repeat the calls on the visible sandboxes.

use crate::canonical_runner::Context;
use kui::{Document, Element};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug). A failure is
//            logged as "KNOWN BUG" and kept out of the verdict; once it passes it counts as ok.

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    known: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            known: Cell::new(0),
            failed_names: RefCell::new(Vec::new()),
        }
    }

    fn row(&self, name: &str, result: &str, cell: &str) {
        let Ok(Some(log)) = self.doc.get_element_by_id("tour-log") else {
            return;
        };
        let Ok(row) = self.doc.create_element("tr") else {
            return;
        };
        let _ = row.set_attribute("data-check", name);
        let _ = row.set_attribute("data-result", result);
        if result != "ok" {
            let _ = row.class_list_add("bad");
        }
        for text in [name, cell] {
            if let Ok(td) = self.doc.create_element("td") {
                let _ = td.set_text_content(text);
                let _ = row.append_child(&td);
            }
        }
        let _ = log.append_child(&row);
    }

    fn check(&self, name: &str, ok: bool, detail: String) {
        self.total.set(self.total.get() + 1);
        if ok {
            self.row(name, "ok", "ok");
            return;
        }
        self.failed.set(self.failed.get() + 1);
        self.failed_names.borrow_mut().push(name.to_string());
        eprintln!("tour check failed: {name}: {detail}");
        self.row(name, "fail", &format!("FAIL: {detail}"));
    }

    fn known_bug(&self, name: &str, ok: bool, bug: &str, detail: String) {
        if ok {
            self.total.set(self.total.get() + 1);
            self.row(name, "ok", "ok");
            return;
        }
        self.known.set(self.known.get() + 1);
        eprintln!("tour known bug: {name}: {detail}");
        self.row(name, "known", &format!("KNOWN BUG: {bug} ({detail})"));
    }

    fn finish(&self) {
        let Ok(Some(result)) = self.doc.get_element_by_id("tour-result") else {
            return;
        };
        let (total, failed, known) = (self.total.get(), self.failed.get(), self.known.get());
        let mut text = if failed == 0 {
            format!("PASS {total}/{total}")
        } else {
            format!(
                "FAIL {failed}/{total}: {}",
                self.failed_names.borrow().join(", ")
            )
        };
        if known > 0 {
            text.push_str(&format!(" (known bugs: {known})"));
        }
        let _ = result.set_text_content(&text);
        let _ = result.set_attribute("data-total", &total.to_string());
        let _ = result.set_attribute("data-failed", &failed.to_string());
        let _ = result.set_attribute("data-known", &known.to_string());
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- Small readers (None-safe) ------------------------------------------------------------

fn text(e: Option<&Element<'_>>) -> String {
    e.and_then(|e| e.text_content().ok()).unwrap_or_default()
}

fn id(e: Option<&Element<'_>>) -> String {
    e.and_then(|e| e.id().ok()).unwrap_or_default()
}

fn tag(e: Option<&Element<'_>>) -> String {
    e.and_then(|e| e.tag_name().ok()).unwrap_or_default()
}

fn inner(e: Option<&Element<'_>>) -> String {
    e.and_then(|e| e.inner_html().ok()).unwrap_or_default()
}

fn children(e: Option<&Element<'_>>) -> usize {
    e.and_then(|e| e.child_count().ok()).unwrap_or(0)
}

fn join_texts(items: &[Element<'_>]) -> String {
    items
        .iter()
        .map(|item| text(Some(item)))
        .collect::<Vec<_>>()
        .join(",")
}

fn by_id<'d>(doc: &'d Document, element_id: &str) -> Option<Element<'d>> {
    doc.get_element_by_id(element_id).ok().flatten()
}

fn set_text(doc: &Document, element_id: &str, value: &str) {
    if let Some(element) = by_id(doc, element_id) {
        let _ = element.set_text_content(value);
    }
}

// ---- Document lookups: root / head / body, id, selectors, class and tag lists, hit test ----

fn check_lookups(t: &Tour) {
    let doc = &t.doc;
    let root = doc.root().ok().flatten();
    let head = doc.head().ok().flatten();
    let body = doc.body().ok().flatten();

    t.check(
        "doc.root",
        tag(root.as_ref()) == "html",
        format!("root tag '{}'", tag(root.as_ref())),
    );

    let title = head
        .as_ref()
        .and_then(|h| h.query_selector("title").ok().flatten());
    t.check(
        "doc.head",
        tag(head.as_ref()) == "head" && text(title.as_ref()) == "KUI API — DOM tree",
        format!(
            "head tag '{}', <title> '{}'",
            tag(head.as_ref()),
            text(title.as_ref())
        ),
    );

    let parent = body.as_ref().and_then(|b| b.parent().ok().flatten());
    t.check(
        "doc.body",
        body.is_some() && tag(parent.as_ref()) == "html",
        format!("body parent tag '{}'", tag(parent.as_ref())),
    );

    let lookup_box = by_id(doc, "lookup-box");
    t.check(
        "doc.get_element_by_id",
        id(lookup_box.as_ref()) == "lookup-box",
        format!("id '{}'", id(lookup_box.as_ref())),
    );
    let missing = doc.get_element_by_id("no-such-id");
    t.check(
        "doc.get_element_by_id-missing",
        matches!(missing, Ok(None)),
        format!(
            "result {:?}",
            missing
                .as_ref()
                .map(|e| e.is_some())
                .map_err(|e| e.to_string())
        ),
    );

    let hot = doc.query_selector("#lookup-box .hot").ok().flatten();
    t.check(
        "doc.query_selector",
        text(hot.as_ref()) == "beta",
        format!("first .hot text '{}'", text(hot.as_ref())),
    );

    let items = doc.query_selector_all(".chip").unwrap_or_default();
    let texts = join_texts(&items);
    t.check(
        "doc.query_selector_all",
        items.len() == 4 && texts == "alpha,beta,gamma,delta",
        format!("count {} texts '{texts}'", items.len()),
    );

    let items = doc.get_elements_by_class_name("hot").unwrap_or_default();
    let texts = join_texts(&items);
    t.check(
        "doc.get_elements_by_class_name",
        items.len() == 2 && texts == "beta,delta",
        format!("count {} texts '{texts}'", items.len()),
    );
    // kui_c.h: several space-separated classes must all match.
    let items = doc
        .get_elements_by_class_name("chip hot")
        .unwrap_or_default();
    let texts = join_texts(&items);
    t.known_bug(
        "doc.get_elements_by_class_name-multi",
        items.len() == 2 && texts == "beta,delta",
        "a multi-class query matches nothing",
        format!("count {} texts '{texts}'", items.len()),
    );

    let items = doc.get_elements_by_tag_name("em").unwrap_or_default();
    let texts = join_texts(&items);
    t.check(
        "doc.get_elements_by_tag_name",
        items.len() == 1 && texts == "delta",
        format!("count {} texts '{texts}'", items.len()),
    );

    if let Some(lookup_box) = lookup_box.as_ref() {
        let em = lookup_box.query_selector("em").ok().flatten();
        t.check(
            "el.query_selector",
            text(em.as_ref()) == "delta",
            format!("em text '{}'", text(em.as_ref())),
        );
        let items = lookup_box.query_selector_all("span").unwrap_or_default();
        let texts = join_texts(&items);
        t.check(
            "el.query_selector_all",
            items.len() == 3 && texts == "alpha,beta,gamma",
            format!("count {} texts '{texts}'", items.len()),
        );
    }

    // Hit test the centre of #hit-target in viewport coordinates.
    let target = by_id(doc, "hit-target");
    let rect = target.as_ref().and_then(|e| e.bounding_rect().ok());
    let (x, y, w, h) = rect.map_or((0.0, 0.0, 0.0, 0.0), |r| (r.x, r.y, r.width, r.height));
    let hit = doc
        .element_from_point(x + w / 2.0, y + h / 2.0)
        .ok()
        .flatten();
    t.check(
        "doc.element_from_point",
        id(hit.as_ref()) == "hit-target",
        format!(
            "rect {x:.0},{y:.0} {w:.0}x{h:.0} hit '{}'",
            id(hit.as_ref())
        ),
    );

    // An extra document reference (clone = kui_document_retain) is dropped again; still usable.
    let extra = doc.clone();
    drop(extra);
    let again = by_id(doc, "lookup-box");
    t.check(
        "doc.retain",
        again.is_some(),
        "document unusable after clone + drop".to_string(),
    );
}

// ---- Traversal: parent, siblings, first / last child, closest and matches ---------------

fn check_traversal(t: &Tour) {
    let doc = &t.doc;
    let (Some(t2), Some(t4), Some(list)) =
        (by_id(doc, "t2"), by_id(doc, "t4"), by_id(doc, "trav-list"))
    else {
        t.check(
            "traversal.page",
            false,
            "missing #t2 / #t4 / #trav-list".to_string(),
        );
        return;
    };

    let got = t2.parent().ok().flatten();
    t.check(
        "el.parent",
        id(got.as_ref()) == "trav-list",
        format!("parent '{}'", id(got.as_ref())),
    );
    let got = t2.next_sibling().ok().flatten();
    t.check(
        "el.next_sibling",
        id(got.as_ref()) == "t3",
        format!("next '{}'", id(got.as_ref())),
    );
    let got = t2.previous_sibling().ok().flatten();
    t.check(
        "el.previous_sibling",
        id(got.as_ref()) == "t1",
        format!("previous '{}'", id(got.as_ref())),
    );
    let end = t4.next_sibling();
    t.check(
        "el.next_sibling-end",
        matches!(end, Ok(None)),
        "t4 has a next sibling".to_string(),
    );

    let first = list.first_child().ok().flatten();
    let last = list.last_child().ok().flatten();
    t.check(
        "el.first_last_child",
        id(first.as_ref()) == "t1" && id(last.as_ref()) == "t4" && children(Some(&list)) == 4,
        format!(
            "first '{}' last '{}' count {}",
            id(first.as_ref()),
            id(last.as_ref()),
            children(Some(&list))
        ),
    );

    let got = t2.closest("ul").ok().flatten();
    t.check(
        "el.closest-ancestor",
        id(got.as_ref()) == "trav-list",
        format!("closest(ul) '{}'", id(got.as_ref())),
    );
    let got = t2.closest("li").ok().flatten();
    t.check(
        "el.closest-inclusive",
        id(got.as_ref()) == "t2",
        format!("closest(li) '{}'", id(got.as_ref())),
    );
    let none = t2.closest("table");
    t.check(
        "el.closest-none",
        matches!(none, Ok(None)),
        "closest(table) found something".to_string(),
    );

    let yes = t2.matches("li.mid");
    t.check(
        "el.matches",
        matches!(yes, Ok(true)),
        format!("matches(li.mid) = {yes:?}"),
    );
    let no = t2.matches("#t1");
    t.check(
        "el.matches-false",
        matches!(no, Ok(false)),
        format!("matches(#t1) = {no:?}"),
    );
}

// ---- Create & insert (in the hidden #scratch): create, append, insert_before, clone, replace --

fn check_create_insert(t: &Tour) -> Result<(), Box<dyn Error>> {
    let doc = &t.doc;
    let scratch = by_id(doc, "scratch").ok_or("page has no #scratch")?;

    let a = doc.create_element("div")?;
    a.set_attribute("id", "sc-a")?;
    a.set_attribute("data-k", "v")?;
    scratch.append_child(&a)?;
    let got = by_id(doc, "sc-a");
    let value = got
        .as_ref()
        .and_then(|e| e.attribute("data-k").ok())
        .unwrap_or_default();
    t.check(
        "el.create_append",
        children(Some(&scratch)) == 1 && got.is_some() && value == "v",
        format!(
            "children {}, found {}, data-k '{value}'",
            children(Some(&scratch)),
            got.is_some()
        ),
    );

    let first = doc.create_element("p")?;
    first.set_attribute("id", "sc-first")?;
    let status = scratch.insert_before(&first, Some(&a));
    let got = scratch.first_child().ok().flatten();
    t.check(
        "el.insert_before",
        status.is_ok() && id(got.as_ref()) == "sc-first",
        format!("status {status:?}, first child '{}'", id(got.as_ref())),
    );

    let src = doc.create_element("div")?;
    src.set_attribute("id", "sc-src")?;
    src.set_inner_html("<b>bold</b> tail")?;
    scratch.append_child(&src)?;
    let deep = src.clone_node(true)?;
    let shallow = src.clone_node(false)?;

    let parent = deep.parent();
    let bold = deep.query_selector("b").ok().flatten();
    t.check(
        "el.clone-deep",
        text(Some(&deep)) == "bold tail" && matches!(parent, Ok(None)) && bold.is_some(),
        format!("text '{}', <b> {}", text(Some(&deep)), bold.is_some()),
    );
    t.check(
        "el.clone-shallow",
        text(Some(&shallow)).is_empty()
            && children(Some(&shallow)) == 0
            && tag(Some(&shallow)) == "div",
        format!(
            "text '{}', children {}, tag '{}'",
            text(Some(&shallow)),
            children(Some(&shallow)),
            tag(Some(&shallow))
        ),
    );

    // The deep copy (renamed, ids stay unique) takes #sc-a's place.
    deep.set_attribute("id", "sc-copy")?;
    let status = scratch.replace_child(&deep, &a);
    let old_parent = a.parent().ok().flatten();
    let new_parent = deep.parent().ok().flatten();
    let lookup = by_id(doc, "sc-a");
    t.check(
        "el.replace_child",
        status.is_ok()
            && old_parent.is_none()
            && lookup.is_none()
            && id(new_parent.as_ref()) == "scratch",
        format!(
            "status {status:?}, old parent {}, lookup {}",
            old_parent.is_some(),
            lookup.is_some()
        ),
    );
    Ok(())
}

// ---- Remove, attributes and handle references (still in #scratch) ----------------------

fn check_remove_attributes(t: &Tour) -> Result<(), Box<dyn Error>> {
    let doc = &t.doc;
    let missing = "the create & insert checks left no #sc-first / #sc-src / #sc-copy";
    let scratch = by_id(doc, "scratch").ok_or(missing)?;
    let first = by_id(doc, "sc-first").ok_or(missing)?;
    let src = by_id(doc, "sc-src").ok_or(missing)?;
    let copied = by_id(doc, "sc-copy").ok_or(missing)?;

    // A removed element leaves the document, but our handle keeps it alive and readable.
    let status = first.remove();
    let got = by_id(doc, "sc-first");
    t.check(
        "el.remove",
        status.is_ok() && got.is_none() && tag(Some(&first)) == "p",
        format!(
            "status {status:?}, still found {}, handle tag '{}'",
            got.is_some(),
            tag(Some(&first))
        ),
    );

    let before = children(Some(&scratch));
    let status = scratch.remove_child(&src);
    let parent = src.parent().ok().flatten();
    t.check(
        "el.remove_child",
        status.is_ok() && children(Some(&scratch)) + 1 == before && parent.is_none(),
        format!(
            "status {status:?}, children {before} -> {}",
            children(Some(&scratch))
        ),
    );

    copied.set_attribute("title", "hello")?;
    let title = copied.attribute("title").unwrap_or_default();
    t.check(
        "el.set_get_attribute",
        title == "hello",
        format!("title '{title}'"),
    );
    let has = copied.has_attribute("title");
    t.check(
        "el.has_attribute",
        matches!(has, Ok(true)),
        format!("has_attribute = {has:?}"),
    );
    let status = copied.remove_attribute("title");
    let has = copied.has_attribute("title");
    let read = copied.attribute("title");
    let not_found = matches!(&read, Err(e) if e.status() == kui::sys::KUI_ERR_NOT_FOUND);
    t.check(
        "el.remove_attribute",
        status.is_ok() && matches!(has, Ok(false)) && not_found,
        format!("status {status:?}, has {has:?}, get {read:?}"),
    );

    // clone (= kui_element_retain) adds a reference; dropping it leaves our handle working.
    let extra = copied.clone();
    drop(extra);
    t.check(
        "el.retain",
        id(Some(&copied)) == "sc-copy",
        format!("id '{}'", id(Some(&copied))),
    );
    Ok(())
}

// ---- HTML & text: inner / outer HTML, insert_adjacent_html, text nodes ------------------

// ---- Form control state: the current value / checked / selected index, not the content attribute --

fn check_form_state(t: &Tour) -> Result<(), Box<dyn Error>> {
    let doc = &t.doc;
    let scratch = by_id(doc, "scratch").ok_or("page has no #scratch")?;
    let form_box = doc.create_element("div")?;
    scratch.append_child(&form_box)?;
    form_box.set_inner_html(r#"<input id="sc-fv" type="text" value="initial"><select id="sc-fs"><option value="s">S</option><option value="m" selected>M</option><option value="l">L</option></select><input id="sc-fc" type="checkbox"><i id="sc-fi">x</i>"#)?;
    let (input, select, checkbox, plain) = (
        by_id(doc, "sc-fv").ok_or("no #sc-fv")?,
        by_id(doc, "sc-fs").ok_or("no #sc-fs")?,
        by_id(doc, "sc-fc").ok_or("no #sc-fc")?,
        by_id(doc, "sc-fi").ok_or("no #sc-fi")?,
    );

    let initial = input.value().unwrap_or_default();
    input.set_value("Ada")?;
    let current = input.value().unwrap_or_default();
    let attribute = input.attribute("value").unwrap_or_default();
    t.check(
        "el.set_get_value",
        initial == "initial" && current == "Ada" && attribute == "initial",
        format!("initial '{initial}', current '{current}', attribute '{attribute}'"),
    );

    let before = select.selected_index().unwrap_or(-9);
    select.set_selected_index(2)?;
    let after = select.selected_index().unwrap_or(-9);
    let selected = select.value().unwrap_or_default();
    t.check(
        "el.set_get_selected_index",
        before == 1 && after == 2 && selected == "l",
        format!("before {before}, after {after}, value '{selected}'"),
    );

    let unchecked = !checkbox.checked().unwrap_or(true);
    checkbox.set_checked(true)?;
    let now = checkbox.checked().unwrap_or(false);
    t.check(
        "el.set_get_checked",
        unchecked && now,
        format!("unchecked first {unchecked}, now {now}"),
    );

    // An element with no form state: value() / selected_index() are not supported.
    let (value_err, index_err) = (plain.value().is_err(), plain.selected_index().is_err());
    t.check(
        "el.form-not-supported",
        value_err && index_err,
        format!("value() error {value_err}, selected_index() error {index_err}"),
    );
    Ok(())
}

fn check_html_text(t: &Tour) -> Result<(), Box<dyn Error>> {
    let doc = &t.doc;
    let scratch = by_id(doc, "scratch").ok_or("page has no #scratch")?;
    let html_box = doc.create_element("div")?;
    scratch.append_child(&html_box)?;

    html_box.set_inner_html("<i>a</i><i>b</i>")?;
    t.check(
        "el.set_inner_html",
        children(Some(&html_box)) == 2,
        format!("children {}", children(Some(&html_box))),
    );
    let html = inner(Some(&html_box));
    t.check(
        "el.get_inner_html",
        html == "<i>a</i><i>b</i>",
        format!("innerHTML '{html}'"),
    );

    html_box.set_inner_html(r#"<p id="sc-o" class="x">hi</p>"#)?;
    let outer = by_id(doc, "sc-o")
        .and_then(|e| e.outer_html().ok())
        .unwrap_or_default();
    t.check(
        "el.get_outer_html",
        outer.starts_with("<p")
            && outer.contains(r#"id="sc-o""#)
            && outer.contains(r#"class="x""#)
            && outer.ends_with("hi</p>"),
        format!("outerHTML '{outer}'"),
    );

    html_box.set_inner_html(r#"<span id="sc-t">T</span>"#)?;
    let target = by_id(doc, "sc-t");
    if let Some(target) = target.as_ref() {
        target.insert_adjacent_html("beforebegin", "<i>1</i>")?;
        target.insert_adjacent_html("afterbegin", "<i>2</i>")?;
        target.insert_adjacent_html("beforeend", "<i>3</i>")?;
        target.insert_adjacent_html("afterend", "<i>4</i>")?;
    }
    t.check(
        "el.insert_adjacent_html",
        text(Some(&html_box)) == "12T34"
            && text(target.as_ref()) == "2T3"
            && children(Some(&html_box)) == 3,
        format!(
            "box text '{}', target text '{}', box children {}",
            text(Some(&html_box)),
            text(target.as_ref()),
            children(Some(&html_box))
        ),
    );

    let span = doc.create_element("span")?;
    span.set_text_content("a")?;
    span.append_text("b")?;
    t.check(
        "el.append_text",
        text(Some(&span)) == "ab" && inner(Some(&span)) == "ab" && children(Some(&span)) == 0,
        format!(
            "text '{}', innerHTML '{}'",
            text(Some(&span)),
            inner(Some(&span))
        ),
    );

    // set_text_content never parses markup: it becomes one text node.
    span.set_text_content("<b>x</b>")?;
    t.check(
        "el.set_text_content",
        text(Some(&span)) == "<b>x</b>" && children(Some(&span)) == 0,
        format!(
            "text '{}', children {}",
            text(Some(&span)),
            children(Some(&span))
        ),
    );
    // HTML serialization escapes '<' '>' '&' in text, so innerHTML parses back to the same text.
    let html = inner(Some(&span));
    t.check(
        "el.get_inner_html-escapes-text",
        html == "&lt;b&gt;x&lt;/b&gt;",
        format!("innerHTML '{html}'"),
    );
    Ok(())
}

// ---- Panel buttons -----------------------------------------------------------------------

/// Registers a click handler on the element with that id; the handler gets its own document.
fn on<F>(ctx: &Context<'_>, element_id: &str, mut handler: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Document) + 'static,
{
    let doc = ctx.document.clone();
    let element = ctx
        .document
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener("click", move |_event| handler(&doc))?;
    Ok(())
}

fn bind_lookup_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-lookup-count", |doc| {
        let chips = doc.query_selector_all(".chip").map_or(0, |v| v.len());
        let hot = doc.get_elements_by_class_name("hot").map_or(0, |v| v.len());
        let em = doc.get_elements_by_tag_name("em").map_or(0, |v| v.len());
        set_text(
            doc,
            "lookup-out",
            &format!("chips={chips} | hot={hot} | em={em}"),
        );
    })?;
    on(ctx, "btn-lookup-hit", |doc| {
        let rect = by_id(doc, "hit-target").and_then(|e| e.bounding_rect().ok());
        let (x, y) = rect.map_or((0.0, 0.0), |r| (r.x + r.width / 2.0, r.y + r.height / 2.0));
        let hit = doc.element_from_point(x, y).ok().flatten();
        let name = hit.as_ref().map_or("(none)".to_string(), |e| id(Some(e)));
        set_text(doc, "lookup-out", &format!("element_from_point -> #{name}"));
    })?;
    on(ctx, "btn-lookup-root", |doc| {
        let root = doc.root().ok().flatten();
        let head = doc.head().ok().flatten();
        let body = doc.body().ok().flatten();
        let line = format!(
            "root={} | head={} | body={}",
            tag(root.as_ref()),
            tag(head.as_ref()),
            tag(body.as_ref())
        );
        set_text(doc, "lookup-out", &line);
    })
}

/// Moves the "cur" highlight to the next / previous item, wrapping at the ends.
fn move_highlight(doc: &Document, forward: bool) {
    let (Ok(Some(current)), Some(list)) = (
        doc.query_selector("#trav-list li.cur"),
        by_id(doc, "trav-list"),
    ) else {
        return;
    };
    let target = if forward {
        current
            .next_sibling()
            .ok()
            .flatten()
            .or_else(|| list.first_child().ok().flatten())
    } else {
        current
            .previous_sibling()
            .ok()
            .flatten()
            .or_else(|| list.last_child().ok().flatten())
    };
    let _ = current.class_list_remove("cur");
    if let Some(target) = target.as_ref() {
        let _ = target.class_list_add("cur");
    }
    set_text(doc, "trav-out", &format!("#{}", id(target.as_ref())));
}

fn bind_traversal_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-trav-next", |doc| move_highlight(doc, true))?;
    on(ctx, "btn-trav-prev", |doc| move_highlight(doc, false))?;
    on(ctx, "btn-trav-info", |doc| {
        let Ok(Some(current)) = doc.query_selector("#trav-list li.cur") else {
            return;
        };
        let parent = current.parent().ok().flatten();
        let panel = current.closest("section.panel").ok().flatten();
        let mid = if matches!(current.matches(".mid"), Ok(true)) {
            "yes"
        } else {
            "no"
        };
        let line = format!(
            "parent=#{} | closest(.panel)=#{} | matches(.mid)={mid}",
            id(parent.as_ref()),
            id(panel.as_ref())
        );
        set_text(doc, "trav-out", &line);
    })
}

fn new_item<'d>(doc: &'d Document, class: &str, value: &str) -> Option<Element<'d>> {
    let item = doc.create_element("p").ok()?;
    let _ = item.set_class_name(class);
    let _ = item.set_text_content(value);
    Some(item)
}

fn show_build_count(doc: &Document, build_box: &Element<'_>) {
    set_text(
        doc,
        "build-out",
        &format!("children={}", children(Some(build_box))),
    );
}

fn bind_build_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let serial = Rc::new(Cell::new(0));

    let next = serial.clone();
    on(ctx, "btn-build-append", move |doc| {
        let Some(build_box) = by_id(doc, "build-box") else {
            return;
        };
        next.set(next.get() + 1);
        if let Some(item) = new_item(doc, "item", &format!("item {}", next.get())) {
            let _ = build_box.append_child(&item);
        }
        show_build_count(doc, &build_box);
    })?;

    let next = serial.clone();
    on(ctx, "btn-build-prepend", move |doc| {
        let Some(build_box) = by_id(doc, "build-box") else {
            return;
        };
        next.set(next.get() + 1);
        if let Some(item) = new_item(doc, "item", &format!("first {}", next.get())) {
            let first = build_box.first_child().ok().flatten();
            let _ = build_box.insert_before(&item, first.as_ref()); // None appends
        }
        show_build_count(doc, &build_box);
    })?;

    let next = serial.clone();
    on(ctx, "btn-build-clone", move |doc| {
        let (Some(build_box), Some(seed)) = (by_id(doc, "build-box"), by_id(doc, "build-seed"))
        else {
            return;
        };
        if let Ok(copied) = seed.clone_node(true) {
            next.set(next.get() + 1);
            let _ = copied.set_attribute("id", &format!("seed-copy-{}", next.get()));
            let _ = build_box.append_child(&copied);
        }
        show_build_count(doc, &build_box);
    })?;

    let next = serial;
    on(ctx, "btn-build-replace", move |doc| {
        let Some(build_box) = by_id(doc, "build-box") else {
            return;
        };
        if let Ok(Some(first)) = build_box.first_child() {
            next.set(next.get() + 1);
            if let Some(item) = new_item(doc, "replaced", &format!("replaced {}", next.get())) {
                let _ = build_box.replace_child(&item, &first);
            }
        }
        show_build_count(doc, &build_box);
    })
}

fn show_rm_state(doc: &Document) {
    let Some(rm_box) = by_id(doc, "rm-box") else {
        return;
    };
    let flag = if matches!(rm_box.has_attribute("data-flag"), Ok(true)) {
        "on"
    } else {
        "removed"
    };
    set_text(
        doc,
        "rm-out",
        &format!("left={} | data-flag={flag}", children(Some(&rm_box))),
    );
}

fn bind_remove_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-rm-last", |doc| {
        if let Some(Ok(Some(last))) = by_id(doc, "rm-box").map(|b| b.last_child()) {
            let _ = last.remove();
        }
        show_rm_state(doc);
    })?;
    on(ctx, "btn-rm-first", |doc| {
        if let Some(rm_box) = by_id(doc, "rm-box") {
            if let Ok(Some(first)) = rm_box.first_child() {
                let _ = rm_box.remove_child(&first);
            }
        }
        show_rm_state(doc);
    })?;
    on(ctx, "btn-rm-attr", |doc| {
        if let Some(rm_box) = by_id(doc, "rm-box") {
            if matches!(rm_box.has_attribute("data-flag"), Ok(true)) {
                let _ = rm_box.remove_attribute("data-flag");
            } else {
                let _ = rm_box.set_attribute("data-flag", "on");
            }
        }
        show_rm_state(doc);
    })?;
    on(ctx, "btn-rm-retain", |doc| {
        let Some(rm_box) = by_id(doc, "rm-box") else {
            return;
        };
        let extra = rm_box.clone(); // kui_element_retain
        drop(extra);
        set_text(
            doc,
            "rm-out",
            &format!("retained handle -> #{}", id(Some(&rm_box))),
        );
    })
}

fn bind_html_buttons(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    on(ctx, "btn-html-reset", |doc| {
        let Some(html_box) = by_id(doc, "html-box") else {
            return;
        };
        let _ = html_box.set_inner_html(r#"<span id="html-core">core</span>"#);
        set_text(doc, "html-out", &inner(Some(&html_box)));
    })?;
    on(ctx, "btn-html-adjacent", |doc| {
        let Some(core) = by_id(doc, "html-core") else {
            return;
        };
        let _ = core.insert_adjacent_html("beforebegin", "<i>[</i>");
        let _ = core.insert_adjacent_html("afterbegin", "<i>(</i>");
        let _ = core.insert_adjacent_html("beforeend", "<i>)</i>");
        let _ = core.insert_adjacent_html("afterend", "<i>]</i>");
        let line = text(by_id(doc, "html-box").as_ref());
        set_text(doc, "html-out", &line);
    })?;
    on(ctx, "btn-html-text", |doc| {
        let Some(core) = by_id(doc, "html-core") else {
            return;
        };
        let _ = core.append_text("+");
        set_text(doc, "html-out", &text(Some(&core)));
    })?;
    on(ctx, "btn-html-outer", |doc| {
        let Some(core) = by_id(doc, "html-core") else {
            return;
        };
        set_text(doc, "html-out", &core.outer_html().unwrap_or_default());
    })
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let t = Tour::new(ctx.document);
    check_lookups(&t);
    check_traversal(&t);
    if let Err(error) = check_create_insert(&t) {
        t.check("create.setup", false, error.to_string());
    }
    if let Err(error) = check_remove_attributes(&t) {
        t.check("remove.setup", false, error.to_string());
    }
    if let Err(error) = check_html_text(&t) {
        t.check("html.setup", false, error.to_string());
    }
    if let Err(error) = check_form_state(&t) {
        t.check("form.setup", false, error.to_string());
    }
    t.finish();

    bind_lookup_buttons(ctx)?;
    bind_traversal_buttons(ctx)?;
    bind_build_buttons(ctx)?;
    bind_remove_buttons(ctx)?;
    bind_html_buttons(ctx)?;
    Ok(())
}
