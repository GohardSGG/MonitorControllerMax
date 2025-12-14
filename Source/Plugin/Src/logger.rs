//! Instance-level logging system for MonitorControllerMax
//!
//! Each VST instance has its own independent log file.
//! No global state - fully instance-isolated.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::Local;
use parking_lot::RwLock;

/// Production 模式：禁用文件日志
#[cfg(feature = "production")]
const FILE_LOGGING_ENABLED: bool = false;

#[cfg(not(feature = "production"))]
const FILE_LOGGING_ENABLED: bool = true;

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    #[allow(dead_code)]
    Debug,
    #[allow(dead_code)]
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

/// Maximum number of log entries to keep in memory for UI display
const MAX_RECENT_LOGS: usize = 50;

/// Generate a unique instance ID using timestamp + random bits
/// No global counter needed - collision is practically impossible
pub fn generate_instance_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Use lower bits of nanoseconds for uniqueness
    format!("{:08x}", (nanos & 0xFFFFFFFF) as u32)
}

/// Instance-level logger - each VST instance owns one
pub struct InstanceLogger {
    /// File handle - None if log file couldn't be created (graceful degradation)
    file: Mutex<Option<File>>,
    #[allow(dead_code)]
    pub instance_id: String,
    /// Recent log entries for UI display (thread-safe)
    recent_logs: RwLock<VecDeque<String>>,
}

impl InstanceLogger {
    /// Create a new logger for a specific instance
    /// Gracefully degrades to memory-only logging if file creation fails
    pub fn new(instance_id: &str) -> Arc<Self> {
        let path = Self::get_log_path(instance_id);

        // Graceful degradation: if file can't be created, log only to memory
        let file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path) {
            Ok(f) => Some(f),
            Err(e) => {
                // Use eprintln as fallback - won't crash DAW
                eprintln!("[MCM] Failed to create log file {:?}: {}", path, e);
                None
            }
        };

        let logger = Arc::new(Self {
            file: Mutex::new(file),
            instance_id: instance_id.to_string(),
            recent_logs: RwLock::new(VecDeque::with_capacity(MAX_RECENT_LOGS)),
        });

        // Write initialization header (only if file exists)
        logger.write_header(&path);

        logger
    }

    /// Get the log file path for this instance
    fn get_log_path(instance_id: &str) -> PathBuf {
        #[cfg(target_os = "windows")]
        let primary_dir = PathBuf::from("C:/Plugins/MCM_Logs");

        #[cfg(target_os = "macos")]
        let primary_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Logs/MonitorControllerMax");

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let primary_dir = std::env::temp_dir().join("MonitorControllerMax_Logs");

        let log_dir = if fs::create_dir_all(&primary_dir).is_ok() {
            primary_dir
        } else {
            let fallback = std::env::temp_dir().join("MonitorControllerMax_Logs");
            let _ = fs::create_dir_all(&fallback);
            fallback
        };

        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        log_dir.join(format!("MCM_{}_{}.log", instance_id, timestamp))
    }

    /// Write initialization header to log file (skipped if file is None)
    fn write_header(&self, path: &PathBuf) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut f) = *guard {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(f, "");
                let _ = writeln!(f, "==================================================================");
                let _ = writeln!(f, "[{}] [INFO ] MonitorControllerMax Logger Initialized", timestamp);
                let _ = writeln!(f, "[{}] [INFO ] Instance ID: {}", timestamp, self.instance_id);
                let _ = writeln!(f, "[{}] [INFO ] Version: {} (Build: {})",
                    timestamp,
                    env!("CARGO_PKG_VERSION"),
                    env!("BUILD_TIMESTAMP"));
                let _ = writeln!(f, "[{}] [INFO ] Log File: {:?}", timestamp, path);
                let _ = writeln!(f, "==================================================================");
                let _ = f.flush();
            }
        }
    }

    /// Internal log function
    /// show_in_ui: 是否显示在 UI 日志面板
    fn log(&self, level: Level, module: &str, message: &str, show_in_ui: bool) {
        // Write to file (if available and enabled)
        if FILE_LOGGING_ENABLED {
            if let Ok(mut guard) = self.file.lock() {
                if let Some(ref mut f) = *guard {
                    let full_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(f, "[{}] [{}] [{}] {}", full_timestamp, level, module, message);
                    let _ = f.flush();
                }
            }
        }

        // Add to recent logs buffer (for UI display) - only for important logs
        if show_in_ui {
            let timestamp = Local::now().format("%H:%M:%S");
            let log_line = format!("[{}] [{}] {}", timestamp, module, message);
            let mut logs = self.recent_logs.write();
            if logs.len() >= MAX_RECENT_LOGS {
                logs.pop_front();
            }
            logs.push_back(log_line);
        }
    }

    /// Get recent log entries for UI display
    pub fn get_recent_logs(&self) -> Vec<String> {
        self.recent_logs.read().iter().cloned().collect()
    }

    /// 重要日志（显示在 UI + 写入文件）
    /// 用于：网络连接状态、角色切换、布局切换
    pub fn important(&self, module: &str, message: &str) {
        self.log(Level::Info, module, message, true);
    }

    /// Log at INFO level (仅写入文件，不显示在 UI)
    pub fn info(&self, module: &str, message: &str) {
        self.log(Level::Info, module, message, false);
    }

    /// Log at WARN level (仅写入文件，不显示在 UI)
    pub fn warn(&self, module: &str, message: &str) {
        self.log(Level::Warn, module, message, false);
    }

    /// Log at ERROR level (仅写入文件，不显示在 UI)
    pub fn error(&self, module: &str, message: &str) {
        self.log(Level::Error, module, message, false);
    }

    /// Log at DEBUG level
    #[allow(dead_code)]
    pub fn debug(&self, module: &str, message: &str) {
        self.log(Level::Debug, module, message, false);
    }

    /// Log at TRACE level
    #[allow(dead_code)]
    pub fn trace(&self, module: &str, message: &str) {
        self.log(Level::Trace, module, message, false);
    }
}
