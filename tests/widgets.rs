//! 弹出层、`<input>` 选择器与游戏音频（C ABI 第 12 节）。单独一个进程（要 init 平台，并创建一个真实窗口）。
//! 测试进程默认 `KUI_MEDIA_NO_DEVICE=1`，所以音频只验证不需要设备的路径。

mod common;

use std::cell::Cell;
use std::rc::Rc;

use kui::{
    AudioGroup, AudioPlayOptions, MenuItem, PickerMode, Platform, PopupPosition, PopupType, Rect,
    TooltipTheme, Window, WindowProps,
};

fn popups_picker_and_audio() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-widgets".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;
    document.load_html(
        r#"<html><body><input id="d" type="date" value="2026-11-16"><input id="t" type="text"><audio id="a"></audio><lottie-player id="l"></lottie-player></body></html>"#,
    )?;
    for _ in 0..5 {
        platform.wait_events(0.01)?;
    }

    let popups = document.popups();
    let anchor = Rect {
        x: 10.0,
        y: 10.0,
        width: 40.0,
        height: 20.0,
    };
    assert!(popups.show_tooltip("hello", anchor, PopupPosition::Bottom)? != 0);
    let activated = Rc::new(Cell::new(0u32));
    let seen = Rc::clone(&activated);
    let items = vec![
        MenuItem {
            text: "Copy".into(),
            id: 1,
            ..MenuItem::default()
        },
        MenuItem {
            separator: true,
            ..MenuItem::default()
        },
    ];
    assert!(popups.show_context_menu(20.0, 30.0, &items, move |id| seen.set(id))? != 0);
    assert!(popups.has(PopupType::ContextMenu)?);
    popups.dismiss_all()?;
    assert!(!popups.any()?);
    popups.set_tooltip_theme(TooltipTheme {
        background: 0x1122_3344,
        foreground: 0xffff_ffff,
        border: 0,
        radius: 4.0,
    })?;
    let theme = popups.tooltip_theme()?;
    assert_eq!((theme.background, theme.radius), (0x1122_3344, 4.0));
    popups.set_tooltip_delay(200.0)?;
    assert_eq!(popups.tooltip_delay()?, 200.0);
    assert!(
        !popups.default_context_menu_enabled()?,
        "内置右键菜单应默认关闭"
    );
    popups.set_default_context_menu_enabled(true)?;
    assert!(popups.default_context_menu_enabled()?);
    popups.set_default_context_menu_enabled(false)?;

    let date = document.get_element_by_id("d")?.expect("#d");
    let text = document.get_element_by_id("t")?.expect("#t");
    date.open_input_picker(PickerMode::None)?;
    let picker = document.input_picker();
    let state = picker.state()?;
    assert!(state.open && state.mode == PickerMode::Date);
    assert_eq!((state.visible_year, state.visible_month), (2026, 11));
    assert_eq!(picker.target()?.expect("target").id()?, "d");
    picker.set_month(2027, 2)?;
    assert_eq!(picker.state()?.visible_month, 2);
    assert!(picker.set_month(2027, 13).is_err());
    picker.close()?;
    assert!(picker.target()?.is_none());
    assert!(
        text.open_input_picker(PickerMode::None).is_err(),
        "文本框没有面板"
    );

    let audio = document.audio();
    if !audio.available()? {
        assert!(audio
            .play("missing.wav", &AudioPlayOptions::default())
            .is_err());
    }
    audio.set_group(AudioGroup::Music, 0.5, true)?;
    assert_eq!(audio.group(AudioGroup::Music)?, (0.5, true));
    assert!(audio.info(12345)?.is_none());
    assert_eq!(audio.counts(None)?.0, 0);
    audio.stop_all(Some(AudioGroup::Sfx))?;
    audio.unload_all()?;

    // 第 13 节：媒体与 Lottie 播放控制。
    let media = document.get_element_by_id("a")?.expect("#a");
    media.set_media_loop(true)?;
    media.set_media_autoplay(true)?;
    assert_eq!(media.media_playback_flags()?, (true, true, false));
    media.load_media()?;
    let lottie = document.get_element_by_id("l")?.expect("#l");
    lottie.set_lottie_speed(2.0)?;
    lottie.set_lottie_loop(true)?;
    lottie.seek_lottie(0.25)?;
    let state = lottie.lottie_state()?;
    assert!(state.looping && state.speed == 2.0 && state.position == 0.25);
    assert_eq!(state.load_state, kui::LottieLoadState::Idle);
    lottie.stop_lottie()?;
    assert_eq!(lottie.lottie_state()?.position, 0.0);
    assert!(media.stop_lottie().is_err(), "<audio> 不是 Lottie");

    // 第 14 节：扩展与 DevTools 生命周期。
    let dir = std::env::temp_dir().join("kui-rust-ext-empty");
    std::fs::create_dir_all(&dir).ok();
    let dir = dir.to_string_lossy().into_owned();
    document.init_extensions(&dir)?;
    assert!(!document.load_extension("./no-such-extension.dll")?);
    assert!(document.unload_extension("kui-no-such").is_err());
    document.watch_extensions(Some(&dir))?;
    // Linux / macOS 的内核还没有目录监视（T-DEBT-POSIX-EXTENSION-HOT-RELOAD-NOT-IMPLEMENTED），如实报告未监视：
    // 断言它当绊线，内核实现后这里会失败，届时同一提交改回与 Windows 相同的断言。
    assert_eq!(document.extension_state()?.1, cfg!(windows));
    document.watch_extensions(None)?;
    assert_eq!(document.extension_state()?, (0, false));
    if window.start_devtools(0)? {
        let (running, port, _) = window.devtools_state()?;
        assert!(running && port != 0);
        window.stop_devtools()?;
        assert!(!window.devtools_state()?.0);
    }
    Ok(())
}

fn main() {
    common::run(&[("popups_picker_and_audio", popups_picker_and_audio)]);
}
