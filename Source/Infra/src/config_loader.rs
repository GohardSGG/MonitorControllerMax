use mcm_protocol::config::AppConfig;
use std::path::PathBuf;

/// 获取配置文件路径
pub fn config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MonitorControllerMax")
            .join("config.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MonitorControllerMax")
            .join("config.json")
    }
}

/// 从磁盘加载配置
pub fn load_from_disk() -> AppConfig {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return config,
                Err(e) => eprintln!("[Config] Parse error: {}", e),
            },
            Err(e) => eprintln!("[Config] Read error: {}", e),
        }
    }
    AppConfig::default()
}

/// 保存配置到磁盘
pub fn save_to_disk(config: &AppConfig) -> Result<(), String> {
    let path = config_path();

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
