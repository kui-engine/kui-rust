use kui::Platform;

fn main() -> kui::Result<()> {
    let extension_dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let _platform = Platform::init()?;
    let document = kui::Document::new()?;
    let count = document.init_extensions(&extension_dir)?;
    println!("loaded {count} extension(s); 3D 通过 kui-3d-view 的 C ABI 方法表调用");
    Ok(())
}
