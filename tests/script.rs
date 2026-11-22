//! 脚本引擎控制（C ABI 第 16 节）：带返回值的执行、Rust 与 Lua 互相调用函数、全局变量、print、上限。

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use kui::{ScriptEngine, ScriptFunction, ScriptValue};

fn script_engine_control() -> kui::Result<()> {
    let engine = ScriptEngine::new()?;

    let output = engine.eval_value("return 40 + 2", "rust-test")?;
    assert!(output.success);
    assert_eq!(output.value, ScriptValue::Int(42));
    // 长错误文本不再被 4 KB 缓冲截断（以前会变成 Err）。
    let long = engine.eval(&format!("error('{}')", "x".repeat(5000)), "rust-test")?;
    assert!(!long.success && long.error.len() >= 5000);

    engine.register_function("rs_add", |args| match args {
        [ScriptValue::Int(a), ScriptValue::Int(b)] => Ok(ScriptValue::Int(a + b)),
        [ScriptValue::String(a), ScriptValue::String(b)] => {
            Ok(ScriptValue::String(format!("{a}{b}")))
        }
        _ => Err("rs_add wants two integers or two strings".into()),
    })?;
    assert_eq!(
        engine.eval_value("return rs_add(2, 3)", "")?.value,
        ScriptValue::Int(5)
    );
    assert_eq!(
        engine.eval_value("return rs_add('ab', 'cd')", "")?.value,
        ScriptValue::String("abcd".into())
    );
    let failed = engine.eval_value("rs_add(true)", "")?;
    assert!(
        !failed.success && failed.error.contains("two integers"),
        "{failed:?}"
    );

    // Lua 函数交给 Rust：调用它，并留一份到回调之外。
    let kept: Rc<RefCell<Option<ScriptFunction>>> = Rc::new(RefCell::new(None));
    let keep = Rc::clone(&kept);
    engine.register_function("rs_apply", move |args| {
        let [ScriptValue::Function(f), x] = args else {
            return Err("rs_apply(fn, x)".into());
        };
        keep.borrow_mut().get_or_insert_with(|| f.clone());
        f.call(std::slice::from_ref(x))
            .map_err(|error| error.to_string())
    })?;
    assert_eq!(
        engine
            .eval_value("return rs_apply(function(x) return x * 10 end, 4)", "")?
            .value,
        ScriptValue::Int(40)
    );
    let function = kept.borrow().clone().expect("kept function");
    assert_eq!(function.call(&[ScriptValue::Int(9)])?, ScriptValue::Int(90));

    // 递归调用同一个 Rust 闭包得到 Lua 错误，而不是静默返回 nil。
    engine.register_function("rs_reenter", |args| match args.first() {
        Some(ScriptValue::Function(f)) => f.call(&[]).map_err(|error| error.to_string()),
        _ => Ok(ScriptValue::Nil),
    })?;
    // （Lua 函数值内部的错误按原生规则变成 nil，所以在 Lua 里用 pcall 接住。）
    let reentered = engine.eval_value(
        "return rs_reenter(function() local ok, e = pcall(rs_reenter, function() end) reentry_error = e return ok end)",
        "",
    )?;
    assert_eq!(reentered.value, ScriptValue::Bool(false), "{reentered:?}");
    let ScriptValue::String(message) = engine.global("reentry_error")? else {
        panic!("reentry_error should be a string");
    };
    assert!(message.contains("re-entered"), "{message}");

    // Rust 闭包作为值交给 Lua；Lua 函数作为全局读出。
    let made = engine.create_function(|args| match args {
        [ScriptValue::Int(x)] => Ok(ScriptValue::Int(x * 100)),
        _ => Ok(ScriptValue::Nil),
    })?;
    engine.set_global("rs_made", &ScriptValue::Function(made.clone()))?;
    drop(made);
    assert_eq!(
        engine.eval_value("return rs_made(3)", "")?.value,
        ScriptValue::Int(300)
    );
    engine.eval_value("function mul(a, b) return a * b end", "")?;
    let product = engine.call_function("mul", &[ScriptValue::Int(6), ScriptValue::Int(7)])?;
    assert!(product.success);
    assert_eq!(product.value, ScriptValue::Int(42));
    let missing = engine.call_function("no_such_fn", &[])?;
    assert!(!missing.success && missing.error.contains("no_such_fn"));
    let ScriptValue::Function(mul) = engine.global("mul")? else {
        panic!("mul should be a function");
    };
    assert_eq!(
        mul.call(&[ScriptValue::Int(3), ScriptValue::Int(4)])?,
        ScriptValue::Int(12)
    );

    engine.set_global("g_text", &ScriptValue::String("text".into()))?;
    let marker = 1234usize as *mut std::ffi::c_void;
    engine.set_global("g_ud", &ScriptValue::UserData(marker))?;
    assert_eq!(engine.global("g_text")?, ScriptValue::String("text".into()));
    assert_eq!(engine.global("g_ud")?, ScriptValue::UserData(marker));
    assert_eq!(engine.global("missing")?, ScriptValue::Nil);

    let path = std::env::temp_dir().join(format!("kui_rust_eval_{}.lua", std::process::id()));
    std::fs::write(&path, "return 'from file'").expect("write lua file");
    let from_file = engine.eval_file(path.to_str().expect("utf-8 path"))?;
    let _ = std::fs::remove_file(&path);
    assert_eq!(from_file.value, ScriptValue::String("from file".into()));

    let printed = Rc::new(RefCell::new(String::new()));
    let sink = Rc::clone(&printed);
    engine.set_print_handler(move |message| *sink.borrow_mut() = message.to_owned())?;
    engine.eval_value("print('hi', 1, true)", "")?;
    assert_eq!(*printed.borrow(), "hi\t1\ttrue");
    engine.clear_print_handler()?;
    engine.eval_value("print('quiet')", "")?;
    assert_eq!(*printed.borrow(), "hi\t1\ttrue");

    let usage = engine.resource_usage()?;
    assert!(usage.memory_bytes > 0);
    assert_eq!(
        (usage.memory_limit, usage.instruction_limit),
        (256 * 1024 * 1024, 100_000_000)
    );
    engine.set_instruction_limit(100_000)?;
    let endless = engine.eval_value("while true do end", "")?;
    assert!(!endless.success && endless.error.contains("instruction limit"));
    engine.set_memory_limit(usage.memory_bytes + 1024 * 1024)?;
    let big = engine.eval_value("local s = string.rep('x', 8 * 1024 * 1024)", "")?;
    assert!(!big.success && big.error.contains("memory"), "{big:?}");
    engine.set_memory_limit(0)?;
    let usage = engine.resource_usage()?;
    assert_eq!((usage.memory_limit, usage.instruction_limit), (0, 100_000));

    drop(engine);
    let gone = function
        .call(&[ScriptValue::Int(1)])
        .expect_err("engine is gone");
    assert_eq!(gone.status(), kui::sys::KUI_ERR_INVALID_ARG);
    Ok(())
}

fn main() {
    common::run(&[("script_engine_control", script_engine_control)]);
}
