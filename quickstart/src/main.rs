use std::error::Error;

use kui::{Platform, Window, WindowProps};

fn main() -> Result<(), Box<dyn Error>> {
    let platform = Platform::init()?; // this thread is now the UI thread
    let window = Window::new(&WindowProps {
        title: "KUI Quick Start".into(),
        width: 560,
        height: 640,
        no_resize: true,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?; // a counted reference to the window's document

    let page = std::env::current_exe()?.with_file_name("app.html");
    document.load_html_file(page.to_str().ok_or("path is not UTF-8")?)?;

    let button = document.get_element_by_id("inc")?.ok_or("app.html has no #inc")?;
    let doc = document.clone(); // the listener must be 'static, so it keeps its own reference
    let mut clicks = 0;
    let listener = button.add_event_listener("click", move |_event| {
        clicks += 1;
        if let Ok(Some(count)) = doc.get_element_by_id("count") {
            let _ = count.set_text_content(&clicks.to_string()); // the DOM update
        }
    })?;

    window.show()?;
    platform.run()?; // returns when the window is closed

    button.remove_event_listener("click", listener)?; // frees the closure and its `doc`
    Ok(()) // then button, document, window and platform drop in reverse order
}
