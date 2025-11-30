use log::{LevelFilter, info, error};
use simplelog::{WriteLogger, Config};
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Once;
use std::panic;
use std::time::{SystemTime, UNIX_EPOCH};

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        setup_logger();
    });
}

fn setup_logger() {
    // 1. Try Primary Log Path: C:/Plugins/
    let primary_log_dir = PathBuf::from("C:/Plugins/");
    let mut log_dir = primary_log_dir.clone();
    
    // Generate Filename with Timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_filename = format!("MonitorControllerMax_{}.log", timestamp);

    // Try to create primary dir
    if let Err(e) = fs::create_dir_all(&primary_log_dir) {
        eprintln!("WARNING: Failed to create C:/Plugins/: {}. Falling back to TEMP.", e);
        // Fallback to Temp
        log_dir = std::env::temp_dir().join("MonitorControllerMax_Logs");
        if let Err(e2) = fs::create_dir_all(&log_dir) {
            eprintln!("CRITICAL: Failed to create fallback log dir: {}", e2);
            return; 
        }
    }

    let log_file_path = log_dir.join(&log_filename);
    
    // 2. Open File (Create new)
    let file = match File::create(&log_file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("CRITICAL: Failed to create log file at {:?}: {}", log_file_path, e);
            return;
        }
    };

    // 4. Init SimpleLog
    if let Err(e) = WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        file,
    ) {
        eprintln!("Failed to initialize logger: {}", e);
        return;
    }

    // 5. Set Panic Hook
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_else(|| "unknown".to_string());
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };
        
        let error_msg = format!("[PANIC] Thread panicked at '{}', {}", msg, location);
        error!("{}", error_msg);
        eprintln!("{}", error_msg);
        default_hook(info);
    }));

    info!("==================================================================");
    info!("MonitorControllerMax Initialized");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Log File: {:?}", log_file_path);
    info!("==================================================================");
}

