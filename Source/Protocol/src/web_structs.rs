use serde::{Deserialize, Serialize};

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

// ============================================================================
// WebSharedState (Shared between Reactor and Editor)
// ============================================================================

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

/// Web 服务器共享状态（线程安全）
pub struct WebSharedState {
    // ... (fields)
    // ===== 服务器状态 =====
    /// HTTP 服务器端口（0 表示未启动）
    pub port: AtomicU16,
    /// 服务器是否运行中
    pub is_running: AtomicBool,
    /// 本机 IP 地址
    pub local_ip: RwLock<String>,

    // ===== OSC 通信端口 =====
    /// Web OSC 接收端口（动态分配，0 表示未启动）
    pub osc_recv_port: AtomicU16,

    // ===== 当前布局 =====
    /// 当前通道列表
    pub channel_names: RwLock<Vec<String>>,

    // ===== 状态标志 =====
    /// OSC 接收端口是否绑定成功
    pub osc_recv_port_bound: AtomicBool,
    /// 当前连接的客户端数量
    pub clients_count: AtomicU16,
}

impl WebSharedState {
    pub fn new() -> Self {
        Self {
            port: AtomicU16::new(0),
            is_running: AtomicBool::new(false),
            local_ip: RwLock::new(String::new()),
            osc_recv_port: AtomicU16::new(0),
            channel_names: RwLock::new(Vec::new()),
            osc_recv_port_bound: AtomicBool::new(false),
            clients_count: AtomicU16::new(0),
        }
    }

    /// 获取服务器地址
    pub fn get_address(&self) -> Option<String> {
        if self.is_running.load(Ordering::Relaxed) {
            let port = self.port.load(Ordering::Relaxed);
            let ip = self.local_ip.read().clone();
            // 如果 ip 为空，默认为 localhost? 或者显示 IP unknown
            if ip.is_empty() {
                Some(format!("127.0.0.1:{}", port))
            } else {
                Some(format!("{}:{}", ip, port))
            }
        } else {
            None
        }
    }
}
impl Default for WebSharedState {
    fn default() -> Self {
        Self::new()
    }
}
