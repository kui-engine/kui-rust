use kui::{Platform, ScriptEngine};

fn main() -> kui::Result<()> {
    let _platform = Platform::init()?;
    let mut document = kui::Document::new()?;
    document.load_html(r#"<canvas id="canvas" width="64" height="64"></canvas>"#)?;
    let engine = ScriptEngine::new()?;
    engine.bind_document(&document)?;
    let result = engine.eval(
        r#"local canvas=document:getElementById("canvas"); local ctx=canvas:getContext("2d"); ctx:fillRect(0,0,8,8)"#,
        "rust-canvas.lua",
    )?;
    if !result.success {
        eprintln!("Lua/Canvas 执行失败：{}", result.error);
        std::process::exit(1);
    }
    println!("Lua/Canvas PASS；WebGL 使用同一 ScriptEngine 边界");
    Ok(())
}
