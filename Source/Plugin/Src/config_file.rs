#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 用户配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// OSC 发送端口 (默认 7444)
    #[serde(default = "default_osc_send_port")]
    pub osc_send_port: u16,

    /// OSC 接收端口 (默认 7445)
    #[serde(default = "default_osc_receive_port")]
    pub osc_receive_port: u16,

    /// Network 端口 (默认 9123) - Master/Slave 通信
    #[serde(default = "default_network_port")]
    pub network_port: u16,

    /// Master IP 地址 (默认 127.0.0.1) - Slave 连接目标
    #[serde(default = "default_master_ip")]
    pub master_ip: String,

    /// 默认 Speaker 布局名称
    #[serde(default = "default_speaker_layout")]
    pub default_speaker_layout: String,

    /// 默认 Sub 布局名称
    #[serde(default = "default_sub_layout")]
    pub default_sub_layout: String,

    /// 日志目录
    #[serde(default)]
    pub log_directory: String,
}

// serde 默认值函数
fn default_osc_send_port() -> u16 { 7444 }
fn default_osc_receive_port() -> u16 { 7445 }
fn default_network_port() -> u16 { 9123 }
fn default_master_ip() -> String { "127.0.0.1".to_string() }
fn default_speaker_layout() -> String { "7.1.4".to_string() }
fn default_sub_layout() -> String { "None".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osc_send_port: 7444,
            osc_receive_port: 7445,
            network_port: 9123,
            master_ip: "127.0.0.1".to_string(),
            default_speaker_layout: "7.1.4".to_string(),
            default_sub_layout: "None".to_string(),
            log_directory: "".to_string(),
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
    #[allow(dead_code)]
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
