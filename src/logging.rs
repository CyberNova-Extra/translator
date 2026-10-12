use std::ffi::CStr;
use std::io::Write;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

// ggml_log_level: NONE=0 DEBUG=1 INFO=2 WARN=3 ERROR=4 CONT=5（续行）
const CONT: i32 = 5;

#[derive(Clone, Copy, Default, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum LogLevel {
    Off,
    Error,
    #[default]
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    fn threshold(self) -> i32 {
        match self {
            LogLevel::Off => i32::MAX,
            LogLevel::Error => 4,
            LogLevel::Warn => 3,
            LogLevel::Info => 2,
            LogLevel::Debug => 1,
        }
    }
}

static THRESHOLD: AtomicI32 = AtomicI32::new(3);
static LAST_PASSED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn log_callback(
    level: llama_cpp_sys_2::ggml_log_level,
    text: *const c_char,
    _user_data: *mut c_void,
) {
    let level = level as i32;
    let pass = if level == CONT {
        LAST_PASSED.load(Ordering::Relaxed)
    } else {
        let p = level >= THRESHOLD.load(Ordering::Relaxed);
        LAST_PASSED.store(p, Ordering::Relaxed);
        p
    };
    if pass && !text.is_null() {
        let bytes = unsafe { CStr::from_ptr(text) }.to_bytes();
        let _ = std::io::stderr().write_all(bytes);
    }
}

/// 接管 llama.cpp / ggml 的日志输出，按级别过滤（默认 warn，静默 info 噪音）
pub fn init(level: LogLevel) {
    THRESHOLD.store(level.threshold(), Ordering::Relaxed);
    unsafe {
        llama_cpp_sys_2::llama_log_set(Some(log_callback), std::ptr::null_mut());
        llama_cpp_sys_2::ggml_log_set(Some(log_callback), std::ptr::null_mut());
    }
}
