//! 宿主集成类接口（C ABI 追加）：输入法开关、控制台、崩溃诊断。

use std::ffi::CString;

use crate::{bool_int, check, sys, Result};

/// 关闭调用线程的系统输入法，字母键和空格不再被中日韩输入法截走；仅 Windows，其他平台返回 `KUI_ERR_NOT_SUPPORTED`。
pub fn disable_ime() -> Result<()> {
    check(unsafe { sys::kui_platform_disable_ime() }, "disable_ime")
}

/// 控制台消息级别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// 调试。
    Debug,
    /// 信息。
    Info,
    /// 警告。
    Warn,
    /// 错误。
    Error,
}

/// 打开 / 关闭进程内控制台（DevTools 里看到的日志）。
pub fn console_set_enabled(enabled: bool) -> Result<()> {
    check(
        unsafe { sys::kui_console_set_enabled(bool_int(enabled)) },
        "console_set_enabled",
    )
}

/// 让每条消息同时实时打印到 stderr。
pub fn console_set_stderr_output(enabled: bool) -> Result<()> {
    check(
        unsafe { sys::kui_console_set_stderr_output(bool_int(enabled)) },
        "console_set_stderr_output",
    )
}

/// 低于该级别的消息被丢弃。
pub fn console_set_min_level(level: LogLevel) -> Result<()> {
    let raw = match level {
        LogLevel::Debug => sys::KUI_C_LOG_DEBUG,
        LogLevel::Info => sys::KUI_C_LOG_INFO,
        LogLevel::Warn => sys::KUI_C_LOG_WARN,
        LogLevel::Error => sys::KUI_C_LOG_ERROR,
    };
    check(
        unsafe { sys::kui_console_set_min_level(raw) },
        "console_set_min_level",
    )
}

fn optional_c_string(value: &str) -> Result<Option<CString>> {
    if value.is_empty() {
        return Ok(None);
    }
    CString::new(value)
        .map(Some)
        .map_err(|_| crate::error(sys::KUI_ERR_INVALID_ARG, "string contains NUL"))
}

fn c_ptr(value: &Option<CString>) -> *const std::ffi::c_char {
    value
        .as_ref()
        .map_or(std::ptr::null(), |text| text.as_ptr())
}

/// 堆栈追踪配置；`Default` 用库默认值（文件 + stderr、64 帧）。
#[derive(Clone, Debug, Default)]
pub struct StackTraceConfig {
    /// 输出位：1 文件、2 stderr、4 消息框；0 = 文件 + stderr。
    pub output_mode: u32,
    /// 日志文件路径；空 = 库默认。
    pub log_file_path: String,
    /// 最多写出的栈帧数；0 = 64。
    pub max_frames: i32,
    /// 应用名；空 = `kui_app`。
    pub app_name: String,
    /// Windows：同时写 `.dmp`。
    pub write_mini_dump: bool,
    /// dump 目录；空 = 库默认。
    pub dump_dir: String,
}

/// 安装未处理异常 / 信号的堆栈追踪写出；返回是否装上。尽早调用（启动线程之前）。
pub fn stack_trace_install(config: &StackTraceConfig) -> Result<bool> {
    let (log_file, app_name, dump_dir) = (
        optional_c_string(&config.log_file_path)?,
        optional_c_string(&config.app_name)?,
        optional_c_string(&config.dump_dir)?,
    );
    let raw = sys::KuiCStackTraceConfig {
        struct_size: std::mem::size_of::<sys::KuiCStackTraceConfig>(),
        output_mode: config.output_mode,
        log_file_path: c_ptr(&log_file),
        max_frames: config.max_frames,
        app_name: c_ptr(&app_name),
        write_mini_dump: bool_int(config.write_mini_dump),
        dump_dir: c_ptr(&dump_dir),
    };
    let mut installed = 0;
    check(
        unsafe { sys::kui_stack_trace_install(&raw, &mut installed) },
        "stack_trace_install",
    )?;
    Ok(installed != 0)
}

/// 卸载堆栈追踪写出。
pub fn stack_trace_uninstall() -> Result<()> {
    check(
        unsafe { sys::kui_stack_trace_uninstall() },
        "stack_trace_uninstall",
    )
}

/// 崩溃报告配置；`Default` 用库默认值（exe 旁的 crashpad_handler，不上传）。
#[derive(Clone, Debug, Default)]
pub struct CrashReporterConfig {
    /// `crashpad_handler` 的完整路径；空 = exe 旁。
    pub handler_exe: String,
    /// 崩溃库目录；空 = `<exe 目录>/crashes`。
    pub dump_dir: String,
    /// 写进每份报告的应用名；空 = `kui_app`。
    pub app_name: String,
    /// 写进每份报告的版本；空 = `0.0.0`。
    pub app_version: String,
    /// 上传地址；空 = 从不上传。
    pub report_url: String,
    /// 是否上传到 `report_url`。
    pub enable_upload: bool,
}

/// 安装 Crashpad 崩溃处理；返回是否装上（比如找不到 `crashpad_handler` 时为 `false`，调用本身仍成功）。
pub fn crash_reporter_install(config: &CrashReporterConfig) -> Result<bool> {
    let (handler, dump_dir, app_name, app_version, report_url) = (
        optional_c_string(&config.handler_exe)?,
        optional_c_string(&config.dump_dir)?,
        optional_c_string(&config.app_name)?,
        optional_c_string(&config.app_version)?,
        optional_c_string(&config.report_url)?,
    );
    let raw = sys::KuiCCrashReporterConfig {
        struct_size: std::mem::size_of::<sys::KuiCCrashReporterConfig>(),
        handler_exe: c_ptr(&handler),
        dump_dir: c_ptr(&dump_dir),
        app_name: c_ptr(&app_name),
        app_version: c_ptr(&app_version),
        report_url: c_ptr(&report_url),
        enable_upload: bool_int(config.enable_upload),
    };
    let mut installed = 0;
    check(
        unsafe { sys::kui_crash_reporter_install(&raw, &mut installed) },
        "crash_reporter_install",
    )?;
    Ok(installed != 0)
}

/// 卸载崩溃处理。
pub fn crash_reporter_uninstall() -> Result<()> {
    check(
        unsafe { sys::kui_crash_reporter_uninstall() },
        "crash_reporter_uninstall",
    )
}
