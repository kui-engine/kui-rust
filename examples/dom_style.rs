use kui::Platform;

fn main() -> kui::Result<()> {
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html(r#"<div id="probe">DOM</div>"#)?;
    let probe = document.get_element_by_id("probe")?.expect("#probe");
    probe.set_style_property("color", "red", false)?;
    println!("DOM/style PASS: {}", probe.style_property("color")?);
    Ok(())
}
