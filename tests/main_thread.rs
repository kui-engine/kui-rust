//! 主线程投递：任意线程投递、只在 UI 线程执行；平台关闭前后的拒绝；主线程版异步 HTTP。
//! 单独一个测试文件（单独进程），因为它要走完整的 init → shutdown。

mod common;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use kui::{post_to_main_thread, HTTPClient, Platform};

fn posts_from_other_threads_run_on_the_ui_thread() -> kui::Result<()> {
    assert!(post_to_main_thread(|| {}).is_err(), "init 之前投递应被拒绝");

    let platform = Platform::init()?;
    let ui_thread = thread::current().id();
    let ran = Arc::new(AtomicUsize::new(0));
    let off_thread = Arc::new(AtomicBool::new(false));
    let posters: Vec<_> = (0..4)
        .map(|_| {
            let ran = Arc::clone(&ran);
            let off_thread = Arc::clone(&off_thread);
            thread::spawn(move || {
                for _ in 0..32 {
                    let ran = Arc::clone(&ran);
                    let off_thread = Arc::clone(&off_thread);
                    post_to_main_thread(move || {
                        if thread::current().id() != ui_thread {
                            off_thread.store(true, Ordering::SeqCst);
                        }
                        ran.fetch_add(1, Ordering::SeqCst);
                    })
                    .expect("平台运行时投递应成功");
                }
            })
        })
        .collect();
    for poster in posters {
        poster.join().unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while ran.load(Ordering::SeqCst) < 128 && Instant::now() < deadline {
        platform.wait_events(0.01)?;
    }
    assert_eq!(ran.load(Ordering::SeqCst), 128, "每个投递都应执行");
    assert!(!off_thread.load(Ordering::SeqCst), "回调必须在 UI 线程执行");

    // 主线程版 HTTP：闭包捕获 !Send 的 Rc，回调在 UI 线程上执行。
    let client = HTTPClient::new()?;
    let delivered = Rc::new(Cell::new(false));
    let seen = Rc::clone(&delivered);
    client.fetch_async_on_main_thread("not-a-url", 1.0, move |response| {
        assert_eq!(thread::current().id(), ui_thread);
        assert!(!response.expect("应得到响应对象").error.is_empty());
        seen.set(true);
    })?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !delivered.get() && Instant::now() < deadline {
        platform.wait_events(0.01)?;
    }
    assert!(delivered.get(), "主线程版 HTTP 回调应执行");
    drop(client);

    // 关闭时还在排队的任务被丢弃：不执行，闭包被释放。
    let late = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    struct CountDrop(Arc<AtomicUsize>);
    impl Drop for CountDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    for _ in 0..8 {
        let late = Arc::clone(&late);
        let guard = CountDrop(Arc::clone(&dropped));
        post_to_main_thread(move || {
            let _guard = &guard;
            late.fetch_add(1, Ordering::SeqCst);
        })?;
    }
    drop(platform);
    assert_eq!(late.load(Ordering::SeqCst), 0, "关闭时不应执行排队中的任务");
    assert_eq!(dropped.load(Ordering::SeqCst), 8, "被丢弃的任务应全部释放");

    assert!(post_to_main_thread(|| {}).is_err(), "关闭之后投递应被拒绝");
    Ok(())
}

fn main() {
    common::run(&[(
        "posts_from_other_threads_run_on_the_ui_thread",
        posts_from_other_threads_run_on_the_ui_thread,
    )]);
}
