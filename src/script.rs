//! 脚本引擎控制（C ABI 第 16 节，3.61.8）：带返回值的执行、调用 Lua 函数、把 Rust 闭包交给 Lua、
//! 全局变量、`print` 重定向、内存与指令上限。
//!
//! 全部只在 UI 线程上执行，闭包因此不要求 `Send`。交给 Lua 的闭包由 KUI 在不再引用时通知释放
//! （全局被替换并回收，或引擎销毁）。

use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::rc::Rc;

use crate::callbacks;
use crate::framework::{register, release_token};
use crate::{bytes, check, read_string, sys, Error, Result, ScriptEngine, ScriptResult};

/// Rust 与 Lua 之间传递的值（与原生 `ScriptValue` 一致；Lua 表没有对应形式，读出来是 `Nil`）。
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    /// nil。
    Nil,
    /// 布尔。
    Bool(bool),
    /// 整数。
    Int(i64),
    /// 浮点数。
    Float(f64),
    /// 字符串。
    String(String),
    /// 函数（Lua 函数，或交给 Lua 的 Rust 闭包）。
    Function(ScriptFunction),
    /// Lua 的 light userdata（不透明指针，KUI 不解引用）。
    UserData(*mut c_void),
}

/// 一次执行的结果：`success` 为假时 `error` 是 Lua 错误文本，`value` 是第一个返回值。
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptOutput {
    /// 是否成功。
    pub success: bool,
    /// Lua 错误文本（成功时为空）。
    pub error: String,
    /// 第一个返回值（没有返回值或失败时为 `Nil`）。
    pub value: ScriptValue,
}

/// 脚本引擎的内存用量与上限（上限 0 表示不限）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptResources {
    /// Lua 当前占用的字节数。
    pub memory_bytes: usize,
    /// 内存上限。
    pub memory_limit: usize,
    /// 每次执行的指令上限。
    pub instruction_limit: u64,
}

struct FunctionHandle {
    raw: NonNull<sys::KuiScriptFunction>,
    // 只在调用返回成功（引擎仍在）后用来读错误文本。
    engine: *mut sys::KuiScriptEngine,
}

impl Drop for FunctionHandle {
    fn drop(&mut self) {
        let _ = unsafe { sys::kui_script_function_release(self.raw.as_ptr()) };
    }
}

/// 可调用的脚本函数。克隆共享同一个句柄，最后一个克隆 drop 时释放；Lua 里仍在用的函数不受影响。
/// 引擎销毁后调用返回 `KUI_ERR_INVALID_ARG`。
#[derive(Clone)]
pub struct ScriptFunction {
    inner: Rc<FunctionHandle>,
    _not_send: PhantomData<Rc<()>>,
}

impl PartialEq for ScriptFunction {
    /// 同一个句柄（或它的克隆）才相等。
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl std::fmt::Debug for ScriptFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScriptFunction({:p})", self.inner.raw.as_ptr())
    }
}

impl ScriptFunction {
    fn from_owned(
        raw: *mut sys::KuiScriptFunction,
        engine: *mut sys::KuiScriptEngine,
    ) -> Result<Self> {
        let raw = NonNull::new(raw)
            .ok_or_else(|| crate::error(sys::KUI_ERR_INTERNAL, "ScriptFunction"))?;
        Ok(Self {
            inner: Rc::new(FunctionHandle { raw, engine }),
            _not_send: PhantomData,
        })
    }
    /// 调用函数。Rust 闭包返回 `Err` 时这里返回错误（文字相同）；Lua 函数内部出错得到 `Nil`（与原生一致）。
    pub fn call(&self, args: &[ScriptValue]) -> Result<ScriptValue> {
        let values = to_c(args);
        let mut success = 0;
        let mut out = empty();
        check(
            unsafe {
                sys::kui_script_function_call(
                    self.inner.raw.as_ptr(),
                    first(&values),
                    values.len(),
                    &mut success,
                    &mut out,
                )
            },
            "ScriptFunction::call",
        )?;
        let value = unsafe { from_c(self.inner.engine, &out) }?;
        if success == 0 {
            return Err(Error {
                status: sys::KUI_ERR_INTERNAL,
                context: "ScriptFunction::call",
                message: last_error(self.inner.engine),
            });
        }
        Ok(value)
    }
}

fn empty() -> sys::KuiCScriptValue {
    // 全零即 nil（空指针、长度 0）。
    unsafe { std::mem::zeroed() }
}

fn first(values: &[sys::KuiCScriptValue]) -> *const sys::KuiCScriptValue {
    if values.is_empty() {
        ptr::null()
    } else {
        values.as_ptr()
    }
}

/// 借用 `values` 的 C 视图：字符串与函数句柄指向 `values` 里的数据。
fn to_c(values: &[ScriptValue]) -> Vec<sys::KuiCScriptValue> {
    values.iter().map(value_to_c).collect()
}

fn value_to_c(value: &ScriptValue) -> sys::KuiCScriptValue {
    let mut out = empty();
    match value {
        ScriptValue::Nil => out.type_ = sys::KUI_C_SCRIPT_NIL,
        ScriptValue::Bool(v) => {
            out.type_ = sys::KUI_C_SCRIPT_BOOL;
            out.bool_value = i32::from(*v);
        }
        ScriptValue::Int(v) => {
            out.type_ = sys::KUI_C_SCRIPT_INT;
            out.int_value = *v;
        }
        ScriptValue::Float(v) => {
            out.type_ = sys::KUI_C_SCRIPT_FLOAT;
            out.float_value = *v;
        }
        ScriptValue::String(v) => {
            let (raw, len) = bytes(v);
            out.type_ = sys::KUI_C_SCRIPT_STRING;
            out.string_value = raw;
            out.string_len = len;
        }
        ScriptValue::Function(f) => {
            out.type_ = sys::KUI_C_SCRIPT_FUNCTION;
            out.function_value = f.inner.raw.as_ptr();
        }
        ScriptValue::UserData(v) => {
            out.type_ = sys::KUI_C_SCRIPT_USERDATA;
            out.userdata = *v;
        }
    }
    out
}

/// 复制一个 C 值；函数被 retain 成独立的句柄。
unsafe fn from_c(
    engine: *mut sys::KuiScriptEngine,
    value: &sys::KuiCScriptValue,
) -> Result<ScriptValue> {
    Ok(match value.type_ {
        sys::KUI_C_SCRIPT_BOOL => ScriptValue::Bool(value.bool_value != 0),
        sys::KUI_C_SCRIPT_INT => ScriptValue::Int(value.int_value),
        sys::KUI_C_SCRIPT_FLOAT => ScriptValue::Float(value.float_value),
        sys::KUI_C_SCRIPT_STRING => ScriptValue::String(if value.string_value.is_null() {
            String::new()
        } else {
            let slice = unsafe {
                std::slice::from_raw_parts(value.string_value.cast::<u8>(), value.string_len)
            };
            String::from_utf8_lossy(slice).into_owned()
        }),
        sys::KUI_C_SCRIPT_FUNCTION => {
            let mut owned = ptr::null_mut();
            check(
                unsafe { sys::kui_script_function_retain(value.function_value, &mut owned) },
                "ScriptValue::Function",
            )?;
            ScriptValue::Function(ScriptFunction::from_owned(owned, engine)?)
        }
        sys::KUI_C_SCRIPT_USERDATA => ScriptValue::UserData(value.userdata),
        _ => ScriptValue::Nil,
    })
}

fn last_error(engine: *mut sys::KuiScriptEngine) -> String {
    read_string("ScriptEngine::last_error", |buf, cap, out| unsafe {
        sys::kui_script_engine_get_last_error(engine, buf, cap, out)
    })
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Rust 闭包交给 Lua
// ---------------------------------------------------------------------------

type ScriptFn = Box<dyn FnMut(&[ScriptValue]) -> std::result::Result<ScriptValue, String>>;

struct ScriptFnState {
    f: ScriptFn,
    engine: *mut sys::KuiScriptEngine,
    // 上一次的返回值：KUI 在 trampoline 返回后立刻复制，下一次调用时替换。
    text: Vec<u8>,
    function: Option<ScriptFunction>,
}

impl ScriptFnState {
    fn write_text(&mut self, text: &str, out: &mut sys::KuiCScriptValue) {
        self.text = text.as_bytes().to_vec();
        out.type_ = sys::KUI_C_SCRIPT_STRING;
        out.string_value = self.text.as_ptr().cast();
        out.string_len = self.text.len();
    }
}

const REENTERED: &str = "a Rust function cannot be re-entered while it is running";

unsafe extern "C" fn script_function_trampoline(
    args: *const sys::KuiCScriptValue,
    count: usize,
    out: *mut sys::KuiCScriptValue,
    user_data: *mut c_void,
) -> sys::KuiStatus {
    let out = unsafe { &mut *out };
    let mut status = sys::KUI_ERR_INTERNAL;
    let mut ran = false;
    callbacks::invoke::<ScriptFnState>(user_data as usize, |state| {
        ran = true;
        let raw = if count == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(args, count) }
        };
        let values = match raw
            .iter()
            .map(|value| unsafe { from_c(state.engine, value) })
            .collect::<Result<Vec<_>>>()
        {
            Ok(values) => values,
            Err(error) => {
                state.write_text(&error.to_string(), out);
                return;
            }
        };
        match (state.f)(&values) {
            Ok(ScriptValue::String(text)) => {
                state.write_text(&text, out);
                status = sys::KUI_OK;
            }
            Ok(ScriptValue::Function(function)) => {
                *out = value_to_c(&ScriptValue::Function(function.clone()));
                state.function = Some(function);
                status = sys::KUI_OK;
            }
            Ok(value) => {
                *out = value_to_c(&value);
                status = sys::KUI_OK;
            }
            Err(message) => state.write_text(&message, out),
        }
    });
    if !ran {
        out.type_ = sys::KUI_C_SCRIPT_STRING;
        out.string_value = REENTERED.as_ptr().cast();
        out.string_len = REENTERED.len();
    }
    status
}

type PrintFn = Box<dyn FnMut(&str)>;

unsafe extern "C" fn print_trampoline(
    message: *const std::ffi::c_char,
    len: usize,
    user_data: *mut c_void,
) {
    let text = if message.is_null() || len == 0 {
        String::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(message.cast::<u8>(), len) };
        String::from_utf8_lossy(slice).into_owned()
    };
    callbacks::invoke::<PrintFn>(user_data as usize, |f| f(&text));
}

impl ScriptEngine {
    fn finish_run(&self, success: i32, out: &sys::KuiCScriptValue) -> Result<ScriptOutput> {
        let value = unsafe { from_c(self.raw.as_ptr(), out) }?;
        let success = success != 0;
        Ok(ScriptOutput {
            success,
            error: if success {
                String::new()
            } else {
                last_error(self.raw.as_ptr())
            },
            value,
        })
    }
    pub(crate) fn eval_result(&self, code: &str, source: &str) -> Result<ScriptResult> {
        let output = self.eval_value(code, source)?;
        Ok(ScriptResult {
            success: output.success,
            error: output.error,
        })
    }
    /// 运行到期的 Lua 定时器（`setTimeout` / `setInterval`）；`now_ms` 与 `Platform::now` 同一时钟（毫秒）。
    /// 引擎自己不会走时间：加载了带定时器脚本的宿主每帧调一次。
    pub fn tick_timers(&self, now_ms: f64) -> Result<()> {
        check(
            unsafe { sys::kui_script_engine_tick_timers(self.raw.as_ptr(), now_ms) },
            "ScriptEngine::tick_timers",
        )
    }
    /// 执行 Lua，并取第一个返回值。
    pub fn eval_value(&self, code: &str, source: &str) -> Result<ScriptOutput> {
        let (code, code_len) = bytes(code);
        let (source, source_len) = bytes(source);
        let mut success = 0;
        let mut out = empty();
        check(
            unsafe {
                sys::kui_script_engine_eval_value(
                    self.raw.as_ptr(),
                    code,
                    code_len,
                    source,
                    source_len,
                    &mut success,
                    &mut out,
                )
            },
            "ScriptEngine::eval_value",
        )?;
        self.finish_run(success, &out)
    }
    /// 执行一个 Lua 文件。
    pub fn eval_file(&self, path: &str) -> Result<ScriptOutput> {
        let (path, len) = bytes(path);
        let mut success = 0;
        let mut out = empty();
        check(
            unsafe {
                sys::kui_script_engine_eval_file(
                    self.raw.as_ptr(),
                    path,
                    len,
                    &mut success,
                    &mut out,
                )
            },
            "ScriptEngine::eval_file",
        )?;
        self.finish_run(success, &out)
    }
    /// 调用 Lua 全局函数；函数不存在或出错时 `success` 为假、`error` 是错误文本。
    pub fn call_function(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptOutput> {
        let (name, len) = bytes(name);
        let values = to_c(args);
        let mut success = 0;
        let mut out = empty();
        check(
            unsafe {
                sys::kui_script_engine_call_function(
                    self.raw.as_ptr(),
                    name,
                    len,
                    first(&values),
                    values.len(),
                    &mut success,
                    &mut out,
                )
            },
            "ScriptEngine::call_function",
        )?;
        self.finish_run(success, &out)
    }
    fn new_state<F>(&self, f: F) -> usize
    where
        F: FnMut(&[ScriptValue]) -> std::result::Result<ScriptValue, String> + 'static,
    {
        register(ScriptFnState {
            f: Box::new(f),
            engine: self.raw.as_ptr(),
            text: Vec::new(),
            function: None,
        })
    }
    /// 把 Rust 闭包设为 Lua 全局函数（同名再登记会替换）。闭包返回 `Err(文字)` 时在 Lua 里抛出同样文字的错误；
    /// 同一个闭包不能在自己执行期间被再次调用（会得到 Lua 错误）。
    pub fn register_function<F>(&self, name: &str, f: F) -> Result<()>
    where
        F: FnMut(&[ScriptValue]) -> std::result::Result<ScriptValue, String> + 'static,
    {
        let (name, len) = bytes(name);
        let token = self.new_state(f);
        let status = unsafe {
            sys::kui_script_engine_register_function(
                self.raw.as_ptr(),
                name,
                len,
                Some(script_function_trampoline),
                token as *mut c_void,
                Some(release_token),
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "ScriptEngine::register_function")
    }
    /// 把 Rust 闭包包成可以作为值交给 Lua 的函数。
    pub fn create_function<F>(&self, f: F) -> Result<ScriptFunction>
    where
        F: FnMut(&[ScriptValue]) -> std::result::Result<ScriptValue, String> + 'static,
    {
        let token = self.new_state(f);
        let mut raw = ptr::null_mut();
        let status = unsafe {
            sys::kui_script_function_create(
                self.raw.as_ptr(),
                Some(script_function_trampoline),
                token as *mut c_void,
                Some(release_token),
                &mut raw,
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "ScriptEngine::create_function")?;
        ScriptFunction::from_owned(raw, self.raw.as_ptr())
    }
    /// 设置 Lua 全局变量。
    pub fn set_global(&self, name: &str, value: &ScriptValue) -> Result<()> {
        let (name, len) = bytes(name);
        let converted = value_to_c(value);
        check(
            unsafe { sys::kui_script_engine_set_global(self.raw.as_ptr(), name, len, &converted) },
            "ScriptEngine::set_global",
        )
    }
    /// 读取 Lua 全局变量；不存在为 `Nil`。
    pub fn global(&self, name: &str) -> Result<ScriptValue> {
        let (name, len) = bytes(name);
        let mut out = empty();
        check(
            unsafe { sys::kui_script_engine_get_global(self.raw.as_ptr(), name, len, &mut out) },
            "ScriptEngine::global",
        )?;
        unsafe { from_c(self.raw.as_ptr(), &out) }
    }
    /// 接收 Lua `print()` 的输出（参数以制表符分隔），替换上一个。
    pub fn set_print_handler<F>(&self, handler: F) -> Result<()>
    where
        F: FnMut(&str) + 'static,
    {
        let token = register::<PrintFn>(Box::new(handler));
        let status = unsafe {
            sys::kui_script_engine_set_print_handler(
                self.raw.as_ptr(),
                Some(print_trampoline),
                token as *mut c_void,
                Some(release_token),
            )
        };
        if status != sys::KUI_OK {
            callbacks::unregister(token);
        }
        check(status, "ScriptEngine::set_print_handler")
    }
    /// 恢复默认 `print()`（只进 DevTools 控制台）。
    pub fn clear_print_handler(&self) -> Result<()> {
        check(
            unsafe {
                sys::kui_script_engine_set_print_handler(
                    self.raw.as_ptr(),
                    None,
                    ptr::null_mut(),
                    None,
                )
            },
            "ScriptEngine::clear_print_handler",
        )
    }
    /// 内存上限（字节，0 表示不限；默认 256 MiB）。
    pub fn set_memory_limit(&self, bytes: usize) -> Result<()> {
        check(
            unsafe { sys::kui_script_engine_set_memory_limit(self.raw.as_ptr(), bytes) },
            "ScriptEngine::set_memory_limit",
        )
    }
    /// 每次执行的指令上限（0 表示不限；默认 1 亿），从下一次执行起生效。
    pub fn set_instruction_limit(&self, max: u64) -> Result<()> {
        check(
            unsafe { sys::kui_script_engine_set_instruction_limit(self.raw.as_ptr(), max) },
            "ScriptEngine::set_instruction_limit",
        )
    }
    /// 内存用量与上限。
    pub fn resource_usage(&self) -> Result<ScriptResources> {
        let mut usage = ScriptResources {
            memory_bytes: 0,
            memory_limit: 0,
            instruction_limit: 0,
        };
        check(
            unsafe {
                sys::kui_script_engine_get_resource_usage(
                    self.raw.as_ptr(),
                    &mut usage.memory_bytes,
                    &mut usage.memory_limit,
                    &mut usage.instruction_limit,
                )
            },
            "ScriptEngine::resource_usage",
        )?;
        Ok(usage)
    }
}
