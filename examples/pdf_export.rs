use kui::{PDFOptions, Platform};

fn main() -> kui::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "kui-rust.pdf".into());
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html("<h1>KUI Rust PDF</h1>")?;
    let written = document.print_to_pdf(
        &output,
        &PDFOptions {
            title: "KUI Rust PDF".into(),
            ..PDFOptions::default()
        },
    )?;
    println!("PDF written: {written}");
    Ok(())
}
