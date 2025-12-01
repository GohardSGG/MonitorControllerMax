//! Direct file logging system for MonitorControllerMax
//!
//! This logger writes directly to a file, bypassing the `log` crate's global logger
//! to avoid conflicts with NIH-plug's logging system.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::panic;
use chrono::Local;

/// Global file handle protected by mutex for thread-safe logging
static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Error => write!(f, "ERROR"),
            Level::Warn  => write!(f, "WARN "),
            Level::Info  => write!(f, "INFO "),
            Level::Debug => write!(f, "DEBUG"),
            Level::Trace => write!(f, "TRACE"),
        }
    }
}

/// Get the log file path with timestamp
fn get_log_file_path() -> PathBuf {
    let primary_log_dir = PathBuf::from("C:/Plugins/MCM_Logs");
    let log_dir = if fs::create_dir_all(&primary_log_dir).is_ok() {
        primary_log_dir
    } else {
        let fallback = std::env::temp_dir().join("MonitorControllerMax_Logs");
        let _ = fs::create_dir_all(&fallback);
        fallback
    };

    // Generate filename with timestamp: MCM_2025-11-30_22-30-15.log
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    log_dir.join(format!("MCM_{}.log", timestamp))
}

/// Initialize the logger. Safe to call multiple times.
pub fn init() {
    LOG_FILE.get_or_init(|| {
        let log_file_path = get_log_file_path();

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .expect("Failed to open log file");

        let mutex = Mutex::new(file);

        // Set up panic hook
        setup_panic_hook();

        // Write initialization message
        if let Ok(mut f) = mutex.lock() {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(f, "");
            let _ = writeln!(f, "==================================================================");
            let _ = writeln!(f, "[{}] [INFO ] MonitorControllerMax Logger Initialized", timestamp);
            let _ = writeln!(f, "[{}] [INFO ] Version: {}", timestamp, env!("CARGO_PKG_VERSION"));
            let _ = writeln!(f, "[{}] [INFO ] Log File: {:?}", timestamp, log_file_path);
            let _ = writeln!(f, "==================================================================");
            let _ = f.flush();
        }

        mutex
    });
}

/// Setup panic hook to log panics
fn setup_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let location = info.location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => (*s).to_string(),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "Box<Any>".to_string(),
            },
        };

        // Log to file
        log_internal(Level::Error, "PANIC", &format!("Thread panicked at '{}', {}", msg, location));

        // Also print to stderr
        eprintln!("[PANIC] Thread panicked at '{}', {}", msg, location);

        default_hook(info);
    }));
}

/// Internal logging function
pub fn log_internal(level: Level, module: &str, message: &str) {
    if let Some(mutex) = LOG_FILE.get() {
        if let Ok(mut file) = mutex.lock() {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] [{}] [{}] {}", timestamp, level, module, message);
            let _ = file.flush();
        }
    }
}

/// Log macros for convenient usage
#[macro_export]
macro_rules! mcm_error {
    ($($arg:tt)*) => {
        $crate::logger::log_internal(
            $crate::logger::Level::Error,
            module_path!(),
            &format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! mcm_warn {
    ($($arg:tt)*) => {
        $crate::logger::log_internal(
            $crate::logger::Level::Warn,
            module_path!(),
            &format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! mcm_info {
    ($($arg:tt)*) => {
        $crate::logger::log_internal(
            $crate::logger::Level::Info,
            module_path!(),
            &format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! mcm_debug {
    ($($arg:tt)*) => {
        $crate::logger::log_internal(
            $crate::logger::Level::Debug,
            module_path!(),
            &format!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! mcm_trace {
    ($($arg:tt)*) => {
        $crate::logger::log_internal(
            $crate::logger::Level::Trace,
            module_path!(),
            &format!($($arg)*)
        )
    };
}
