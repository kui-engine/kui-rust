//! 自定义元素的自绘、tick 与输入（C ABI 第 17 节）。单独一个进程（要 init 平台，并创建一个真实窗口）。

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use kui::{
    CornerRadii, CustomElementDefinition, CustomElementHooks, CustomMouseKind, DecorationStyle,
    ImeEventKind, InjectedKey, InjectedMouse, Modifiers, MouseButton, MouseInjection, Platform,
    Point, Rect, TextStyle, Window, WindowProps,
};

fn mouse(kind: InjectedMouse, delta_y: f32) -> MouseInjection {
    MouseInjection {
        kind,
        position: Point { x: 40.0, y: 30.0 },
        button: if delta_y == 0.0 {
            MouseButton::Left
        } else {
            MouseButton::None
        },
        modifiers: Modifiers::default(),
        click_count: if delta_y == 0.0 { 1 } else { 0 },
        delta: Point { x: 0.0, y: delta_y },
    }
}

fn custom_element_paint_tick_and_input() -> kui::Result<()> {
    let platform = Platform::init()?;
    let window = Window::new(&WindowProps {
        title: "kui-rust-custom-paint".into(),
        width: 640,
        height: 480,
        ..WindowProps::default()
    })?;
    let mut document = window.document()?;

    let paints = Rc::new(Cell::new(0));
    let size = Rc::new(Cell::new((0.0f32, 0.0f32)));
    let draw_ok = Rc::new(Cell::new(false));
    let ticks = Rc::new(Cell::new(0));
    let animating = Rc::new(Cell::new(true));
    let mouse_down = Rc::new(Cell::new(None));
    let wheel_dy = Rc::new(Cell::new(0.0f32));
    let key = Rc::new(RefCell::new(String::new()));
    let committed = Rc::new(RefCell::new(String::new()));

    let (p, s, d) = (Rc::clone(&paints), Rc::clone(&size), Rc::clone(&draw_ok));
    let (t, a, a2) = (
        Rc::clone(&ticks),
        Rc::clone(&animating),
        Rc::clone(&animating),
    );
    let (m, w, k, c) = (
        Rc::clone(&mouse_down),
        Rc::clone(&wheel_dy),
        Rc::clone(&key),
        Rc::clone(&committed),
    );
    document.define_custom_element_with(
        "x-rust-canvas",
        CustomElementDefinition::default(),
        CustomElementHooks {
            paint: Some(Box::new(move |_element, ctx| {
                p.set(p.get() + 1);
                s.set((ctx.width, ctx.height));
                let full = Rect {
                    x: 0.0,
                    y: 0.0,
                    width: ctx.width,
                    height: ctx.height,
                };
                let small = Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 20.0,
                    height: 20.0,
                };
                let ok = ctx.fill_rect(full, 0xFF00_00FF).is_ok()
                    && ctx.save().is_ok()
                    && ctx
                        .clip_rounded_rect(full, &CornerRadii::uniform(6.0))
                        .is_ok()
                    && ctx.translate(2.0, 2.0).is_ok()
                    && ctx.set_opacity(0.5).is_ok()
                    && ctx
                        .fill_rounded_rect(small, &CornerRadii::uniform(4.0), 0x0000_FFFF)
                        .is_ok()
                    && ctx
                        .stroke_rounded_rect(small, &CornerRadii::uniform(4.0), 1.0, 0x0000_00FF)
                        .is_ok()
                    && ctx
                        .draw_text(
                            "hi",
                            4.0,
                            40.0,
                            Some(&TextStyle {
                                font_size: 12.0,
                                font_weight: 700,
                                ..TextStyle::default()
                            }),
                            0x0000_00FF,
                        )
                        .is_ok()
                    && ctx
                        .draw_pixels(
                            &[0xFF00_FF00; 4],
                            2,
                            2,
                            Rect {
                                x: 30.0,
                                y: 30.0,
                                width: 8.0,
                                height: 8.0,
                            },
                        )
                        .is_ok()
                    && ctx.draw_pixels(&[0; 3], 2, 2, small).is_err()
                    && ctx
                        .draw_decoration_line(
                            0.0,
                            45.0,
                            40.0,
                            1.0,
                            DecorationStyle::Wavy,
                            0x00FF_00FF,
                        )
                        .is_ok()
                    && ctx.restore().is_ok()
                    && ctx.restore().is_err();
                d.set(ok);
            })),
            tick: Some(Box::new(move |element, _delta| {
                t.set(t.get() + 1);
                if a.get() {
                    let _ = element.request_repaint();
                }
            })),
            needs_tick: Some(Box::new(move |_element| a2.get())),
            mouse: Some(Box::new(move |_element, event| {
                if event.kind == CustomMouseKind::Down {
                    m.set(Some((event.x, event.y, event.button)));
                }
                false
            })),
            wheel: Some(Box::new(move |_element, event| {
                w.set(event.delta_y);
                true
            })),
            key: Some(Box::new(move |_element, event| {
                if event.down {
                    *k.borrow_mut() = event.key.clone();
                }
                true
            })),
            ime: Some(Box::new(move |_element, event| {
                if event.kind == ImeEventKind::Commit {
                    *c.borrow_mut() = event.committed_text.clone();
                }
                true
            })),
            ..CustomElementHooks::default()
        },
    )?;
    document.load_html(
        r#"<html><body style="margin:0"><x-rust-canvas id="cv" tabindex="0" style="display:block;position:absolute;left:10px;top:10px;width:120px;height:60px"></x-rust-canvas></body></html>"#,
    )?;
    for _ in 0..30 {
        if paints.get() > 0 && ticks.get() >= 3 {
            break;
        }
        platform.wait_events(0.02)?;
    }
    assert!(paints.get() > 0, "paint");
    assert_eq!(size.get(), (120.0, 60.0));
    assert!(draw_ok.get(), "drawing calls inside paint");
    assert!(ticks.get() >= 3, "ticks: {}", ticks.get());
    animating.set(false);
    for _ in 0..3 {
        platform.wait_events(0.02)?;
    }
    let stopped = ticks.get();
    for _ in 0..5 {
        platform.wait_events(0.02)?;
    }
    assert_eq!(ticks.get(), stopped, "needs_tick false stops the ticks");

    let element = document.get_element_by_id("cv")?.expect("cv");
    let metrics = element.measure_text("Hello", None)?;
    assert!(metrics.width > 0.0 && metrics.ascent > 0.0);
    let before = paints.get();
    element.invalidate_rect(Rect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    })?;
    for _ in 0..10 {
        if paints.get() > before {
            break;
        }
        platform.wait_events(0.02)?;
    }
    assert!(paints.get() > before, "invalidate_rect repaints");

    document.dispatch_mouse_event(&mouse(InjectedMouse::Pressed, 0.0))?;
    document.dispatch_mouse_event(&mouse(InjectedMouse::Released, 0.0))?;
    assert_eq!(mouse_down.get(), Some((30.0, 20.0, 0)));
    document.dispatch_mouse_event(&mouse(InjectedMouse::Wheel, 40.0))?;
    assert_eq!(wheel_dy.get(), 40.0);
    element.focus()?;
    document.dispatch_key_event(
        InjectedKey::Down,
        "a",
        "KeyA",
        Modifiers::default(),
        "a",
        false,
    )?;
    assert_eq!(*key.borrow(), "a");
    document.dispatch_ime_event(ImeEventKind::Start, "")?;
    document.dispatch_ime_event(ImeEventKind::Commit, "中")?;
    assert_eq!(*committed.borrow(), "中");
    drop(element);
    drop(document);
    Ok(())
}

fn main() {
    common::run(&[(
        "custom_element_paint_tick_and_input",
        custom_element_paint_tick_and_input,
    )]);
}
