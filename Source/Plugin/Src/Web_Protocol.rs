//! Web 控制器协议定义
//!
//! 定义 WebSocket 消息格式和共享状态结构

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use parking_lot::RwLock;

// ============================================================================
// WebSocket 命令（客户端 → 服务器）
// ============================================================================

/// 来自 Web 客户端的命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebCommand {
    // 模式切换
    ToggleSolo,
    ToggleMute,

    // 通道操作
    ChannelClick { channel: String },

    // 全局控制
    SetVolume { value: f32 },
    ToggleDim,
    ToggleCut,
    SetDim { on: bool },
    SetCut { on: bool },

    // 效果器
    ToggleMono,
    ToggleLowBoost,
    ToggleHighBoost,
    ToggleLfeAdd10dB,

    // 通道组编码器（Group Dial）
    // direction: 1=右转(有声音/Solo), -1=左转(没声音/Mute)
    GroupDial { group: String, direction: i8 },
    // 编码器按下（切换组内所有通道的 Mute）
    GroupClick { group: String },
}

// ============================================================================
// WebSocket 状态推送（服务器 → 客户端）
// ============================================================================

/// 完整状态推送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebState {
    /// 主模式: 0=None, 1=Solo, 2=Mute
    pub primary: u8,
    /// 比较模式
    pub compare: u8,
    /// Solo 通道掩码
    pub solo_mask: u32,
    /// Mute 通道掩码
    pub mute_mask: u32,
    /// Master Volume (0.0-1.0)
    pub master_volume: f32,
    /// Dim 状态
    pub dim: bool,
    /// Cut 状态
    pub cut: bool,
    /// Mono 状态
    pub mono: bool,
    /// Low Boost 状态
    pub low_boost: bool,
    /// High Boost 状态
    pub high_boost: bool,
    /// LFE +10dB 状态
    pub lfe_add_10db: bool,
    /// 通道列表
    pub channels: Vec<ChannelState>,
}

/// 单个通道状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelState {
    /// 通道名称
    pub name: String,
    /// 通道索引
    pub index: usize,
    /// 通道状态: 0=off, 1=mute(红), 2=solo(绿)
    pub state: u8,
    /// 是否为 SUB 通道
    pub is_sub: bool,
}

// ============================================================================
// Web 共享状态
// ============================================================================

/// Web 服务器共享状态（线程安全）
pub struct WebSharedState {
    // ===== 服务器状态 =====
    /// HTTP 服务器端口（0 表示未启动）
    pub port: AtomicU16,
    /// 服务器是否运行中
    pub is_running: AtomicBool,
    /// 本机 IP 地址
    pub local_ip: RwLock<String>,

    // ===== OSC 通信端口 =====
    /// Web OSC 接收端口（动态分配，0 表示未启动）
    /// 插件 Osc.rs 发送线程会检查此端口，如果 > 0 则同时发送到此端口
    pub osc_recv_port: AtomicU16,

    // ===== 当前布局 =====
    /// 当前通道列表
    pub channel_names: RwLock<Vec<String>>,
}

impl WebSharedState {
    pub fn new() -> Self {
        Self {
            port: AtomicU16::new(0),
            is_running: AtomicBool::new(false),
            local_ip: RwLock::new(String::from("127.0.0.1")),
            osc_recv_port: AtomicU16::new(0),
            channel_names: RwLock::new(Vec::new()),
        }
    }

    /// 获取服务器地址
    pub fn get_address(&self) -> Option<String> {
        if self.is_running.load(Ordering::Acquire) {
            let port = self.port.load(Ordering::Relaxed);
            let ip = self.local_ip.read().clone();
            Some(format!("{}:{}", ip, port))
        } else {
            None
        }
    }

    /// 检查服务器是否运行中
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Acquire)
    }

    /// 更新通道列表
    pub fn update_channels(&self, channels: Vec<String>) {
        *self.channel_names.write() = channels;
    }
}

impl Default for WebSharedState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Web 重启动作
// ============================================================================

/// Web 服务器重启动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebRestartAction {
    /// 启动服务器
    Start,
    /// 停止服务器
    Stop,
}
