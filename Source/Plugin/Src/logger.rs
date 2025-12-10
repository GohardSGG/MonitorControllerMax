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
    file: Mutex<File>,
    pub instance_id: String,
    /// Recent log entries for UI display (thread-safe)
    recent_logs: RwLock<VecDeque<String>>,
}

impl InstanceLogger {
    /// Create a new logger for a specific instance
    pub fn new(instance_id: &str) -> Arc<Self> {
        let path = Self::get_log_path(instance_id);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("Failed to create log file");

        let logger = Arc::new(Self {
            file: Mutex::new(file),
            instance_id: instance_id.to_string(),
            recent_logs: RwLock::new(VecDeque::with_capacity(MAX_RECENT_LOGS)),
        });

        // Write initialization header
        logger.write_header(&path);

        logger
    }

    /// Get the log file path for this instance
    fn get_log_path(instance_id: &str) -> PathBuf {
        let primary_dir = PathBuf::from("C:/Plugins/MCM_Logs");
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

    /// Write initialization header to log file
    fn write_header(&self, path: &PathBuf) {
        if let Ok(mut f) = self.file.lock() {
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

    /// Internal log function
    fn log(&self, level: Level, module: &str, message: &str) {
        let timestamp = Local::now().format("%H:%M:%S");
        let log_line = format!("[{}] [{}] {}", timestamp, module, message);

        // Write to file
        if let Ok(mut f) = self.file.lock() {
            let full_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(f, "[{}] [{}] [{}] {}", full_timestamp, level, module, message);
            let _ = f.flush();
        }

        // Add to recent logs buffer (for UI display)
        {
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

    /// Log at INFO level
    pub fn info(&self, module: &str, message: &str) {
        self.log(Level::Info, module, message);
    }

    /// Log at WARN level
    pub fn warn(&self, module: &str, message: &str) {
        self.log(Level::Warn, module, message);
    }

    /// Log at ERROR level
    pub fn error(&self, module: &str, message: &str) {
        self.log(Level::Error, module, message);
    }

    /// Log at DEBUG level
    pub fn debug(&self, module: &str, message: &str) {
        self.log(Level::Debug, module, message);
    }

    /// Log at TRACE level
    pub fn trace(&self, module: &str, message: &str) {
        self.log(Level::Trace, module, message);
    }
}
