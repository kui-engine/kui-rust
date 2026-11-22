use kui::Platform;

fn main() -> kui::Result<()> {
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html(r#"<audio id="audio"></audio><kui-lottie id="lottie"></kui-lottie>"#)?;
    let audio = document.get_element_by_id("audio")?.expect("#audio");
    audio.set_src("sample.wav")?;
    audio.play()?;
    audio.pause()?;
    println!("media API PASS；解码能力由部署的 kui_media_* 后端决定");
    Ok(())
}
