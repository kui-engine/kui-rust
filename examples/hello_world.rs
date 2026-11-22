use kui::{Platform, Window, WindowProps};

fn main() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "KUI Rust SDK".into(),
        width: 960,
        height: 640,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    document.load_html(
        r#"<main style="padding:48px;font-family:sans-serif"><h1>Rust SDK 已接入 KUI</h1><p>此程序仅通过公开 C ABI 工作。</p></main>"#,
    )?;
    window.show()?;
    let _ = platform.run()?;
    Ok(())
}
