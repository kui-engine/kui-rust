//! 集成测试的主线程运行器。
//!
//! macOS 只允许在主线程（`main` 函数所在的线程）初始化 AppKit、创建窗口，而 libtest 把每个 `#[test]`
//! 放在子线程里跑（`--test-threads=1` 也一样），带窗口的测试在 macOS 上一律失败（`Window::new` 报
//! internal error）。所以 tests/ 下的每个文件在 Cargo.toml 里设 `harness = false`，由文件里的 `main`
//! 调 `run`：在主线程上按顺序逐个执行，一个失败（返回错误或 panic）不影响后面的，有失败就以 101 退出
//! （与 libtest 相同）。三个平台用同一套，测试的线程模型与应用（UI 线程就是主线程）一致。
//!
//! 命令行与 libtest 兼容的部分：`--list` 列出名字；其余 `-` 开头的参数（`--test-threads=1` 等）忽略，
//! 带值的写法（`--test-threads 1`）连值一起跳过；剩下的参数是名字过滤（子串匹配）。

use std::panic::{self, AssertUnwindSafe};

pub type Test = (&'static str, fn() -> kui::Result<()>);

/// 带值、值可能单独成一个参数的 libtest 选项。
const OPTIONS_WITH_VALUE: &[&str] = &[
    "--test-threads",
    "--skip",
    "--format",
    "--color",
    "--logfile",
    "--shuffle-seed",
    "-Z",
];

pub fn run(tests: &[Test]) {
    let mut filters = Vec::new();
    let mut list = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--list" {
            list = true;
        } else if OPTIONS_WITH_VALUE.contains(&arg.as_str()) {
            args.next();
        } else if !arg.starts_with('-') {
            filters.push(arg);
        }
    }
    let selected: Vec<&Test> = tests
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();
    if list {
        for (name, _) in &selected {
            println!("{name}: test");
        }
        return;
    }

    println!(
        "\nrunning {} test{}",
        selected.len(),
        if selected.len() == 1 { "" } else { "s" }
    );
    let mut failures = Vec::new();
    for (name, test) in &selected {
        let error = match panic::catch_unwind(AssertUnwindSafe(|| test())) {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(format!("Error: {error:?}")),
            Err(payload) => Some(match payload.downcast_ref::<&str>() {
                Some(message) => format!("panicked: {message}"),
                None => match payload.downcast_ref::<String>() {
                    Some(message) => format!("panicked: {message}"),
                    None => "panicked".to_string(),
                },
            }),
        };
        match error {
            None => println!("test {name} ... ok"),
            Some(error) => {
                println!("test {name} ... FAILED");
                failures.push((*name, error));
            }
        }
    }
    if !failures.is_empty() {
        println!("\nfailures:");
        for (name, error) in &failures {
            println!("\n---- {name} ----\n{error}");
        }
    }
    println!(
        "\ntest result: {}. {} passed; {} failed; 0 ignored; 0 measured; {} filtered out\n",
        if failures.is_empty() { "ok" } else { "FAILED" },
        selected.len() - failures.len(),
        failures.len(),
        tests.len() - selected.len()
    );
    if !failures.is_empty() {
        std::process::exit(101);
    }
}
