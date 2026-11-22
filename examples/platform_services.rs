use kui::Platform;

fn main() -> kui::Result<()> {
    let platform = Platform::init()?;
    println!("{} {}", platform.os_name()?, platform.os_version()?);
    Ok(())
}
