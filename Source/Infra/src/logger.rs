//! Instance-level logging system for MonitorControllerMax
//!
//! Each VST instance has its own independent log file.
//! No global state - fully instance-isolated.
//!
//! # Real-time Safety
//! This logger is designed for audio thread usage.
//! - `info/warn/error` methods are non-blocking (push to channel).
//! - File IO happens in a dedicated background thread.
//! - String formatting is minimized in the hot path.

use chrono::Local;
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

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
            Level::Warn => write!(f, "WARN "),
            Level::Info => write!(f, "INFO "),
            Level::Debug => write!(f, "DEBUG"),
            Level::Trace => write!(f, "TRACE"),
        }
    }
}

/// Log Message sent to background thread
enum LogMsg {
    /// Standard log entry
    Entry {
        level: Level,
        module: &'static str,
        message: String,
        show_in_ui: bool,
    },
    /// Flush signal (for shutdown)
    Flush,
}

/// Maximum number of log entries to keep in memory for UI display
const MAX_RECENT_LOGS: usize = 50;

/// Generate a unique instance ID using timestamp + random bits
pub fn generate_instance_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:08x}", (nanos & 0xFFFFFFFF) as u32)
}

/// Instance-level logger - each VST instance owns one
pub struct InstanceLogger {
    /// Channel sender for non-blocking logging
    tx: Sender<LogMsg>,
    #[allow(dead_code)]
    pub instance_id: String,
    /// Recent log entries for UI display (thread-safe)
    recent_logs: Arc<RwLock<VecDeque<String>>>,
    /// Handle to the background thread (joined on drop implicitly via detach or we can store it)
    /// We detach for simplicity as Drop order of Arc<Logger> is complex
    _thread_handle: Option<thread::JoinHandle<()>>,
}

impl InstanceLogger {
    /// Create a new logger for a specific instance
    /// Spawns a background thread for file IO
    pub fn new(instance_id: &str) -> Arc<Self> {
        let path = Self::get_log_path(instance_id);
        let (tx, rx) = bounded::<LogMsg>(4096);
        let recent_logs = Arc::new(RwLock::new(VecDeque::with_capacity(MAX_RECENT_LOGS)));

        let recent_logs_clone = Arc::clone(&recent_logs);
        let instance_id_clone = instance_id.to_string();

        // Spawn background worker
        let thread_handle = thread::spawn(move || {
            Self::log_worker(rx, path, instance_id_clone, recent_logs_clone);
        });

        Arc::new(Self {
            tx,
            instance_id: instance_id.to_string(),
            recent_logs,
            _thread_handle: Some(thread_handle),
        })
    }

    /// Background worker function
    fn log_worker(
        rx: Receiver<LogMsg>,
        path: PathBuf,
        instance_id: String,
        recent_logs: Arc<RwLock<VecDeque<String>>>,
    ) {
        // Open file safely
        let mut file = if FILE_LOGGING_ENABLED {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| eprintln!("[MCM] Failed to open log: {}", e))
                .ok()
        } else {
            None
        };

        // Write header
        if let Some(ref mut f) = file {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(
                f,
                "\n=================================================================="
            );
            let _ = writeln!(
                f,
                "[{}] [INFO ] MonitorControllerMax Logger Initialized",
                timestamp
            );
            let _ = writeln!(f, "[{}] [INFO ] Instance ID: {}", timestamp, instance_id);
            let _ = writeln!(
                f,
                "=================================================================="
            );
        }

        // Event loop
        while let Ok(msg) = rx.recv() {
            match msg {
                LogMsg::Entry {
                    level,
                    module,
                    message,
                    show_in_ui,
                } => {
                    let timestamp_str = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

                    // 1. Write to file
                    if let Some(ref mut f) = file {
                        let _ = writeln!(
                            f,
                            "[{}] [{}] [{}] {}",
                            timestamp_str, level, module, message
                        );
                    }

                    // 2. Update UI buffer (if needed)
                    if show_in_ui {
                        let ui_time = Local::now().format("%H:%M:%S");
                        let log_line = format!("[{}] [{}] {}", ui_time, module, message);
                        let mut logs = recent_logs.write();
                        if logs.len() >= MAX_RECENT_LOGS {
                            logs.pop_front();
                        }
                        logs.push_back(log_line);
                    }
                }
                LogMsg::Flush => {
                    if let Some(ref mut f) = file {
                        let _ = f.flush();
                    }
                }
            }
        }
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

    /// Internal log function (Non-blocking)
    fn log(&self, level: Level, module: &'static str, message: String, show_in_ui: bool) {
        // Try send to avoid blocking audio thread if queue is full
        let msg = LogMsg::Entry {
            level,
            module,
            message,
            show_in_ui,
        };
        let _ = self.tx.try_send(msg);
    }

    /// Get recent log entries for UI display
    pub fn get_recent_logs(&self) -> Vec<String> {
        self.recent_logs.read().iter().cloned().collect()
    }

    /// 重要日志（显示在 UI + 写入文件）
    pub fn important(&self, module: &'static str, message: &str) {
        self.log(Level::Info, module, message.to_string(), true);
    }

    /// Log at INFO level (仅写入文件)
    pub fn info(&self, module: &'static str, message: &str) {
        self.log(Level::Info, module, message.to_string(), false);
    }

    /// Log at WARN level (仅写入文件)
    pub fn warn(&self, module: &'static str, message: &str) {
        self.log(Level::Warn, module, message.to_string(), false);
    }

    /// Log at ERROR level (仅写入文件)
    pub fn error(&self, module: &'static str, message: &str) {
        self.log(Level::Error, module, message.to_string(), false);
    }

    #[allow(dead_code)]
    pub fn debug(&self, module: &'static str, message: &str) {
        self.log(Level::Debug, module, message.to_string(), false);
    }

    /// Explicit flush (e.g. on shutdown)
    pub fn flush(&self) {
        let _ = self.tx.send(LogMsg::Flush);
    }
}

impl Drop for InstanceLogger {
    fn drop(&mut self) {
        // Best effort flush
        let _ = self.tx.send(LogMsg::Flush);
    }
}
