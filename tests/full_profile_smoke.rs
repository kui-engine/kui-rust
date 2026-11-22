mod common;

use kui::{Platform, ScriptEngine};

fn document_dom_style_and_lua() -> kui::Result<()> {
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html(r#"<html><body><div id="probe" class="a">ready</div></body></html>"#)?;
    let probe = document.get_element_by_id("probe")?.expect("#probe 应存在");
    assert_eq!(probe.text_content()?, "ready");
    probe.set_style_property("color", "rgb(1, 2, 3)", false)?;
    assert!(!probe.style_property("color")?.is_empty());

    let engine = ScriptEngine::new()?;
    engine.bind_document(&document)?;
    let result = engine.eval("local answer = 6 * 7", "rust-sdk-smoke.lua")?;
    assert!(result.success, "Lua 执行失败：{}", result.error);
    Ok(())
}

/// 空字符串参数：`bytes("")` 以前把空切片的悬空指针（长度 0）传给 C ABI，C 按约定当 NUL 结尾读、`strlen` 读崩。
fn empty_string_arguments_do_not_crash() -> kui::Result<()> {
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html(r#"<html><body><div id="probe">text</div></body></html>"#)?;
    let probe = document.get_element_by_id("probe")?.expect("#probe");
    probe.set_text_content("")?;
    assert_eq!(probe.text_content()?, "");
    probe.set_attribute("data-empty", "")?;
    assert!(probe.has_attribute("data-empty")?);
    probe.set_style_property("color", "", false)?;
    probe.set_inner_html("")?;
    assert_eq!(probe.child_count()?, 0);
    Ok(())
}

fn main() {
    common::run(&[
        ("document_dom_style_and_lua", document_dom_style_and_lua),
        (
            "empty_string_arguments_do_not_crash",
            empty_string_arguments_do_not_crash,
        ),
    ]);
}
