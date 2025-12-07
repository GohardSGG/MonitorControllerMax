#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use parking_lot::RwLock;
use lazy_static::lazy_static;

/// 用户配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// OSC 发送端口 (默认 7444)
    pub osc_send_port: u16,

    /// OSC 接收端口 (默认 7445)
    pub osc_receive_port: u16,

    /// 默认 Speaker 布局名称
    pub default_speaker_layout: String,

    /// 默认 Sub 布局名称
    pub default_sub_layout: String,

    /// 日志目录
    pub log_directory: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osc_send_port: 7444,
            osc_receive_port: 7445,
            default_speaker_layout: "7.1.4".to_string(),
            default_sub_layout: "None".to_string(),
            log_directory: "".to_string(),  // 空 = 使用默认
        }
    }
}

impl AppConfig {
    /// 获取配置文件路径
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MonitorControllerMax")
            .join("config.json")
    }

    /// 从磁盘加载配置
    pub fn load_from_disk() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(config) => return config,
                        Err(e) => eprintln!("[Config] Parse error: {}", e),
                    }
                }
                Err(e) => eprintln!("[Config] Read error: {}", e),
            }
        }
        Self::default()
    }

    /// 保存配置到磁盘
    pub fn save_to_disk(&self) -> Result<(), String> {
        let path = Self::config_path();

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }
}

/// 全局配置单例
pub struct ConfigStore {
    config: RwLock<AppConfig>,
}

impl ConfigStore {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AppConfig::load_from_disk()),
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn update<F>(&self, f: F) where F: FnOnce(&mut AppConfig) {
        let mut config = self.config.write();
        f(&mut config);
    }

    pub fn save(&self) -> Result<(), String> {
        self.config.read().save_to_disk()
    }

    /// 更新并保存（原子操作）
    pub fn apply_and_save<F>(&self, f: F) -> Result<(), String>
    where F: FnOnce(&mut AppConfig)
    {
        self.update(f);
        self.save()
    }
}

lazy_static! {
    pub static ref APP_CONFIG: ConfigStore = ConfigStore::new();
}
