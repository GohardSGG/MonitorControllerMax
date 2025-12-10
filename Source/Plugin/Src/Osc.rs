#![allow(non_snake_case)]

use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::collections::HashSet;
use crossbeam::channel::{unbounded, Sender, Receiver};
use rosc::{OscPacket, OscMessage, OscType, encoder};
use parking_lot::RwLock;

// InteractionManager 现在通过参数传递，不再使用全局单例
use crate::config_file::AppConfig;
use crate::config_manager::{STANDARD_CHANNEL_ORDER, Layout};
use crate::logger::InstanceLogger;

/// Blink Timer Interval (milliseconds)
const BLINK_INTERVAL_MS: u64 = 500;

/// Maximum queued OSC messages to prevent memory overflow
const MAX_QUEUE_SIZE: usize = 1000;

// ==================== 实例级共享状态 ====================
// 以下状态现在作为 OscSharedState 的字段，而非全局变量

/// OSC 实例共享状态（线程间共享，但实例间隔离）
pub struct OscSharedState {
    /// 当前音频布局的通道数
    pub channel_count: AtomicUsize,
    /// 当前布局的通道名称列表
    pub current_channel_names: RwLock<Vec<String>>,
    /// 之前布局的通道名称列表（用于清空已删除的通道）
    pub prev_channel_names: RwLock<Vec<String>>,
    /// 待激活的 Solo 模式标志
    pub pending_solo: AtomicBool,
    /// 待激活的 Mute 模式标志
    pub pending_mute: AtomicBool,
    /// 当前 Cut 状态（用于 toggle 支持）
    pub current_cut: AtomicBool,
    /// OSC 发送通道
    pub sender_tx: RwLock<Option<Sender<OscOutMessage>>>,
    /// Master Volume (使用 f32 的位表示存储在 AtomicU32 中)
    pub master_volume: AtomicU32,
    /// Dim 状态
    pub dim: AtomicBool,
    /// Cut 接收状态
    pub cut: AtomicBool,
    /// Mono 状态
    pub mono: AtomicBool,
    /// LFE +10dB 状态
    pub lfe_add_10db: AtomicBool,
    /// Low Boost 状态
    pub low_boost: AtomicBool,
    /// High Boost 状态
    pub high_boost: AtomicBool,
    /// 是否有待处理的变化
    pub has_pending: AtomicBool,
    /// OSC 接收端口是否成功绑定（用于UI显示）
    pub recv_port_bound: AtomicBool,
    /// 实例级日志器（线程安全）
    logger: Option<Arc<InstanceLogger>>,
}

impl OscSharedState {
    pub fn new() -> Self {
        Self {
            channel_count: AtomicUsize::new(0),
            current_channel_names: RwLock::new(Vec::new()),
            prev_channel_names: RwLock::new(Vec::new()),
            pending_solo: AtomicBool::new(false),
            pending_mute: AtomicBool::new(false),
            current_cut: AtomicBool::new(false),
            sender_tx: RwLock::new(None),
            master_volume: AtomicU32::new(0),
            dim: AtomicBool::new(false),
            cut: AtomicBool::new(false),
            mono: AtomicBool::new(false),
            lfe_add_10db: AtomicBool::new(false),
            low_boost: AtomicBool::new(false),
            high_boost: AtomicBool::new(false),
            has_pending: AtomicBool::new(false),
            recv_port_bound: AtomicBool::new(false),
            logger: None,
        }
    }

    /// 设置日志器（由 OscManager::init 调用）
    pub fn set_logger(&mut self, logger: Arc<InstanceLogger>) {
        self.logger = Some(logger);
    }

    /// 日志辅助方法
    fn log_info(&self, msg: &str) {
        if let Some(ref logger) = self.logger {
            logger.info("osc", msg);
        }
    }

    fn log_warn(&self, msg: &str) {
        if let Some(ref logger) = self.logger {
            logger.warn("osc", msg);
        }
    }

    // === 发送方法 ===

    /// 发送 Solo 模式按钮状态
    pub fn send_mode_solo(&self, on: bool) {
        self.send(OscOutMessage::ModeSolo { on });
    }

    /// 发送 Mute 模式按钮状态
    pub fn send_mode_mute(&self, on: bool) {
        self.send(OscOutMessage::ModeMute { on });
    }

    /// 发送通道 LED 状态（通过通道名称）
    pub fn send_channel_led_by_name(&self, ch_name: &str, state: ChannelLedState) {
        self.send(OscOutMessage::ChannelLed { channel: ch_name.to_string(), state });
    }

    /// 发送主音量
    pub fn send_master_volume(&self, value: f32) {
        self.send(OscOutMessage::MasterVolume { value });
    }

    /// 发送 Dim 状态
    pub fn send_dim(&self, on: bool) {
        self.send(OscOutMessage::Dim { on });
    }

    /// 发送 Cut 状态
    pub fn send_cut(&self, on: bool) {
        self.send(OscOutMessage::Cut { on });
    }

    /// 发送 Mono 状态
    pub fn send_mono(&self, on: bool) {
        self.send(OscOutMessage::Mono { on });
    }

    /// 发送 LFE +10dB 状态
    pub fn send_lfe_add_10db(&self, on: bool) {
        self.send(OscOutMessage::LfeAdd10dB { on });
    }

    /// 发送 Low Boost 状态
    pub fn send_low_boost(&self, on: bool) {
        self.send(OscOutMessage::LowBoost { on });
    }

    /// 发送 High Boost 状态
    pub fn send_high_boost(&self, on: bool) {
        self.send(OscOutMessage::HighBoost { on });
    }

    fn send(&self, msg: OscOutMessage) {
        if let Some(tx) = self.sender_tx.read().as_ref() {
            let _ = tx.try_send(msg);
        }
    }

    // === 接收方法 ===

    /// 设置 Master Volume (从 OSC 接收)
    pub fn set_master_volume(&self, value: f32) {
        self.master_volume.store(value.to_bits(), Ordering::Relaxed);
        self.has_pending.store(true, Ordering::Relaxed);
    }

    /// 设置 Dim (从 OSC 接收)
    pub fn set_dim(&self, on: bool) {
        self.dim.store(on, Ordering::Relaxed);
        self.has_pending.store(true, Ordering::Relaxed);
    }

    /// 设置 Cut (从 OSC 接收)
    pub fn set_cut(&self, on: bool) {
        self.cut.store(on, Ordering::Relaxed);
        self.has_pending.store(true, Ordering::Relaxed);
    }

    /// 设置 Mono (从 OSC 接收)
    pub fn set_mono(&self, on: bool) {
        self.mono.store(on, Ordering::Relaxed);
    }

    /// 获取 Mono 状态
    pub fn get_mono(&self) -> bool {
        self.mono.load(Ordering::Relaxed)
    }

    /// 设置 LFE +10dB (从 OSC 接收)
    pub fn set_lfe_add_10db(&self, on: bool) {
        self.lfe_add_10db.store(on, Ordering::Relaxed);
    }

    /// 获取 LFE +10dB 状态
    pub fn get_lfe_add_10db(&self) -> bool {
        self.lfe_add_10db.load(Ordering::Relaxed)
    }

    /// 设置 Low Boost (从 OSC 接收)
    pub fn set_low_boost(&self, on: bool) {
        self.low_boost.store(on, Ordering::Relaxed);
    }

    /// 获取 Low Boost 状态
    pub fn get_low_boost(&self) -> bool {
        self.low_boost.load(Ordering::Relaxed)
    }

    /// 设置 High Boost (从 OSC 接收)
    pub fn set_high_boost(&self, on: bool) {
        self.high_boost.store(on, Ordering::Relaxed);
    }

    /// 获取 High Boost 状态
    pub fn get_high_boost(&self) -> bool {
        self.high_boost.load(Ordering::Relaxed)
    }

    /// 获取并清除待处理的变化
    pub fn get_pending_changes(&self) -> Option<(f32, bool, bool)> {
        if !self.has_pending.swap(false, Ordering::Relaxed) {
            return None;
        }

        let volume = f32::from_bits(self.master_volume.load(Ordering::Relaxed));
        let dim = self.dim.load(Ordering::Relaxed);
        let cut = self.cut.load(Ordering::Relaxed);

        Some((volume, dim, cut))
    }

    /// 同步 Cut 状态
    pub fn sync_cut_state(&self, cut: bool) {
        self.current_cut.store(cut, Ordering::Relaxed);
    }

    /// 更新布局通道信息
    pub fn update_layout_channels(&self, layout: &Layout) {
        let mut prev = self.prev_channel_names.write();
        let mut curr = self.current_channel_names.write();

        // 保存旧列表
        *prev = curr.clone();

        // 从 layout 构建新列表
        let mut names = Vec::new();
        for ch in &layout.main_channels {
            names.push(ch.name.clone());
        }
        for ch in &layout.sub_channels {
            names.push(ch.name.clone());
        }

        self.log_info(&format!("[OSC] Layout channels updated: {} → {} channels", prev.len(), names.len()));

        *curr = names;
        self.channel_count.store(layout.total_channels, Ordering::Relaxed);
    }

    /// 广播所有通道的 LED 状态
    pub fn broadcast_channel_states(&self, interaction: &crate::Interaction::InteractionManager) {
        let curr = self.current_channel_names.read();
        let prev = self.prev_channel_names.read();

        if curr.is_empty() {
            self.log_warn("[OSC] Channel names not initialized, skipping broadcast");
            return;
        }

        self.log_info(&format!("[OSC] Broadcasting LED states for {} channels...", curr.len()));

        // 广播当前布局的所有通道状态
        for name in curr.iter() {
            let state = if interaction.is_channel_solo(name) {
                ChannelLedState::Solo
            } else if interaction.is_channel_muted(name) {
                ChannelLedState::Mute
            } else {
                ChannelLedState::Off
            };

            self.send_channel_led_by_name(name, state);
        }

        // 清空已删除的通道
        let curr_set: HashSet<_> = curr.iter().collect();
        for name in prev.iter() {
            if !curr_set.contains(name) {
                self.log_info(&format!("[OSC] Clearing removed channel: {}", name));
                self.send_channel_led_by_name(name, ChannelLedState::Off);
            }
        }

        self.log_info(&format!("[OSC] Broadcast complete (cleared {} removed channels)",
              prev.iter().filter(|n| !curr_set.contains(n)).count()));
    }
}

impl Default for OscSharedState {
    fn default() -> Self {
        Self::new()
    }
}

/// 通道 LED 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLedState {
    Off = 0,    // 不亮
    Mute = 1,   // 红色
    Solo = 2,   // 绿色
}

/// OSC 输出消息类型
#[derive(Debug, Clone)]
pub enum OscOutMessage {
    /// 通道 LED 状态: channel name, state (0=off, 1=mute/red, 2=solo/green)
    ChannelLed { channel: String, state: ChannelLedState },

    /// Solo Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeSolo { on: bool },

    /// Mute Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeMute { on: bool },

    /// Master Volume 值: 0.0 to 1.0
    MasterVolume { value: f32 },

    /// Dim 状态: on (1.0 = active, 0.0 = off)
    Dim { on: bool },

    /// Cut 状态: on (1.0 = active, 0.0 = off)
    Cut { on: bool },

    /// Mono 状态: on (1.0 = active, 0.0 = off)
    Mono { on: bool },

    /// LFE +10dB 状态: on (1.0 = active, 0.0 = off)
    LfeAdd10dB { on: bool },

    /// Low Boost 状态: on (1.0 = active, 0.0 = off)
    LowBoost { on: bool },

    /// High Boost 状态: on (1.0 = active, 0.0 = off)
    HighBoost { on: bool },

    /// 广播所有状态 (初始化时使用)
    BroadcastAll {
        channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
        /// 预计算的通道 LED 状态: (通道名, 状态) 其中状态 0=off, 1=mute, 2=solo
        channel_states: Vec<(String, u8)>,
        /// Solo 模式是否激活
        solo_active: bool,
        /// Mute 模式是否激活
        mute_active: bool,
    },
}

// OscSender 和 OscReceiver 全局单例已删除 - 功能已移入 OscSharedState

/// OSC 管理器 - 多线程架构
pub struct OscManager {
    /// 实例级共享状态（替代全局变量）
    pub state: Arc<OscSharedState>,

    /// 运行状态标志 (原子操作)
    is_running: Arc<AtomicBool>,

    /// 闪烁相位 (true = 亮, false = 灭)
    blink_phase: Arc<AtomicBool>,

    /// 当前音频布局的通道数
    channel_count: usize,

    /// 交互管理器 (实例级，用于 Solo/Mute 状态)
    interaction: Option<Arc<crate::Interaction::InteractionManager>>,

    /// 实例级日志器
    logger: Option<Arc<InstanceLogger>>,

    /// 线程句柄
    send_thread: Option<JoinHandle<()>>,
    receive_thread: Option<JoinHandle<()>>,
    blink_thread: Option<JoinHandle<()>>,
}

impl OscManager {
    /// 创建新的 OscManager (未初始化)
    pub fn new() -> Self {
        Self {
            state: Arc::new(OscSharedState::new()),
            is_running: Arc::new(AtomicBool::new(false)),
            blink_phase: Arc::new(AtomicBool::new(false)),
            channel_count: 0,
            interaction: None,
            logger: None,
            send_thread: None,
            receive_thread: None,
            blink_thread: None,
        }
    }

    /// 获取共享状态的引用（供 Editor.rs 使用）
    pub fn get_state(&self) -> Arc<OscSharedState> {
        Arc::clone(&self.state)
    }

    /// 初始化 OSC (Master 或 Standalone 模式)
    pub fn init(&mut self, channel_count: usize, master_volume: f32, dim: bool, cut: bool, interaction: Arc<crate::Interaction::InteractionManager>, logger: Arc<InstanceLogger>, app_config: &AppConfig) {
        if self.is_running.load(Ordering::Relaxed) {
            logger.warn("osc", "[OSC] Already running, skipping initialization");
            return;
        }

        logger.info("osc", &format!("[OSC] Initializing OSC Manager with {} channels...", channel_count));

        // 存储日志器
        self.logger = Some(Arc::clone(&logger));

        // 存储通道数和交互管理器
        self.channel_count = channel_count;
        self.interaction = Some(interaction.clone());

        // 使用实例级状态而非全局变量
        self.state.channel_count.store(channel_count, Ordering::Relaxed);
        self.state.current_cut.store(cut, Ordering::Relaxed);

        // 创建消息队列
        let (send_tx, send_rx) = unbounded::<OscOutMessage>();
        *self.state.sender_tx.write() = Some(send_tx.clone());

        // 设置运行标志
        self.is_running.store(true, Ordering::Relaxed);

        // 获取端口配置
        let send_port = app_config.osc_send_port;
        let recv_port = app_config.osc_receive_port;

        // 启动三个线程
        let is_running_clone = Arc::clone(&self.is_running);
        let blink_phase_clone = Arc::clone(&self.blink_phase);
        let state_clone = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&logger);

        // 1. 发送线程 (UDP 7444)
        self.send_thread = Some(Self::spawn_send_thread(send_rx, is_running_clone.clone(), Arc::clone(&logger_clone), send_port));

        // 2. 接收线程 (UDP 7445) - 传递 interaction 和 state
        self.receive_thread = Some(Self::spawn_receive_thread(
            is_running_clone.clone(),
            interaction.clone(),
            Arc::clone(&state_clone),
            Arc::clone(&logger_clone),
            recv_port
        ));

        // 3. 闪烁定时器线程 (500ms) - 传递 interaction 和 state
        self.blink_thread = Some(Self::spawn_blink_thread(
            send_tx.clone(),
            is_running_clone,
            blink_phase_clone,
            interaction.clone(),
            Arc::clone(&state_clone),
            Arc::clone(&logger_clone)
        ));

        logger.info("osc", "[OSC] All threads started successfully");
    }

    /// 广播当前状态到硬件 (在 DAW 恢复参数后调用)
    pub fn broadcast_state(&self, channel_count: usize, master_volume: f32, dim: bool, cut: bool) {
        if let Some(ref tx) = *self.state.sender_tx.read() {
            // 预计算通道 LED 状态
            let channel_states = if let Some(ref interaction) = self.interaction {
                let channel_names = self.state.current_channel_names.read();
                channel_names.iter().map(|name| {
                    let state = if interaction.is_channel_solo(name) {
                        2u8  // Solo = 绿色
                    } else if interaction.is_channel_muted(name) {
                        1u8  // Mute = 红色
                    } else {
                        0u8  // Off = 不亮
                    };
                    (name.clone(), state)
                }).collect()
            } else {
                Vec::new()
            };

            // 预计算模式状态
            let (solo_active, mute_active) = if let Some(ref interaction) = self.interaction {
                (interaction.is_solo_active(), interaction.is_mute_active())
            } else {
                (false, false)
            };

            if let Some(ref logger) = self.logger {
                logger.info("osc", &format!("[OSC] Broadcasting state: vol={:.2}, dim={}, cut={}", master_volume, dim, cut));
            }
            let _ = tx.try_send(OscOutMessage::BroadcastAll {
                channel_count,
                master_volume,
                dim,
                cut,
                channel_states,
                solo_active,
                mute_active,
            });
        }
    }

    /// 关闭 OSC 系统
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Relaxed) {
            return;
        }

        if let Some(ref logger) = self.logger {
            logger.info("osc", "[OSC] Shutting down OSC Manager...");
        }

        // 停止所有线程
        self.is_running.store(false, Ordering::Relaxed);

        // 等待线程结束
        if let Some(handle) = self.send_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.receive_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.blink_thread.take() {
            let _ = handle.join();
        }

        *self.state.sender_tx.write() = None;

        if let Some(ref logger) = self.logger {
            logger.info("osc", "[OSC] OSC Manager shutdown complete");
        }
    }

    // ==================== 线程实现 ====================

    /// 发送线程 - 处理 UDP 发送
    fn spawn_send_thread(rx: Receiver<OscOutMessage>, is_running: Arc<AtomicBool>, logger: Arc<InstanceLogger>, send_port: u16) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info("osc", &format!("[OSC Send] Thread started, binding to 0.0.0.0:0 → broadcast to 127.0.0.1:{}", send_port));

            // 绑定 UDP Socket
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    logger.error("osc", &format!("[OSC Send] Failed to bind socket: {}", e));
                    return;
                }
            };

            let target_addr = format!("127.0.0.1:{}", send_port);

            // 主循环：使用阻塞接收 + 批量处理，消除轮询延迟
            while is_running.load(Ordering::Relaxed) {
                // 阻塞等待第一条消息（100ms 超时用于检查运行状态）
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(msg) => {
                        // 立即处理第一条消息
                        Self::process_outgoing_message(&socket, &target_addr, msg, &logger);

                        // 批量处理队列中所有待发消息（避免消息间延迟）
                        while let Ok(msg) = rx.try_recv() {
                            Self::process_outgoing_message(&socket, &target_addr, msg, &logger);
                        }
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                        // 超时正常，继续等待
                        continue;
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                        logger.warn("osc", "[OSC Send] Channel disconnected, exiting thread");
                        break;
                    }
                }
            }

            logger.info("osc", "[OSC Send] Thread stopped");
        })
    }

    /// 接收线程 - 处理 UDP 接收
    fn spawn_receive_thread(
        is_running: Arc<AtomicBool>,
        interaction: Arc<crate::Interaction::InteractionManager>,
        state: Arc<OscSharedState>,
        logger: Arc<InstanceLogger>,
        recv_port: u16
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info("osc", &format!("[OSC Recv] Thread started, binding to 0.0.0.0:{}", recv_port));

            // 绑定 UDP Socket（单一控制者模式：端口冲突时优雅退出）
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", recv_port)) {
                Ok(s) => {
                    // 绑定成功，标记端口已绑定
                    state.recv_port_bound.store(true, Ordering::Relaxed);
                    logger.info("osc", &format!("[OSC Recv] Port {} bound successfully", recv_port));
                    s
                }
                Err(e) => {
                    // 绑定失败，可能是另一个实例已占用端口（这是预期行为）
                    state.recv_port_bound.store(false, Ordering::Relaxed);
                    logger.info("osc", &format!("[OSC Recv] Port {} unavailable (another instance may be using it): {}", recv_port, e));
                    return;
                }
            };

            // 设置非阻塞模式
            if let Err(e) = socket.set_read_timeout(Some(Duration::from_millis(100))) {
                logger.error("osc", &format!("[OSC Recv] Failed to set timeout: {}", e));
                return;
            }

            let mut buf = [0u8; 1024];

            // 主循环
            while is_running.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((size, _src)) => {
                        Self::process_incoming_packet(&buf[..size], &interaction, &state, &logger);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                               || e.kind() == std::io::ErrorKind::TimedOut => {
                        // 超时正常,继续循环
                        continue;
                    }
                    Err(e) => {
                        logger.error("osc", &format!("[OSC Recv] Socket error: {}", e));
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            logger.info("osc", "[OSC Recv] Thread stopped");
        })
    }

    /// 闪烁定时器线程 - 每 500ms 切换一次相位
    fn spawn_blink_thread(
        tx: Sender<OscOutMessage>,
        is_running: Arc<AtomicBool>,
        blink_phase: Arc<AtomicBool>,
        interaction: Arc<crate::Interaction::InteractionManager>,
        _state: Arc<OscSharedState>,  // 预留供将来使用
        logger: Arc<InstanceLogger>
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info("osc", &format!("[OSC Blink] Thread started, interval = {}ms", BLINK_INTERVAL_MS));

            while is_running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(BLINK_INTERVAL_MS));

                // 切换相位
                let new_phase = !blink_phase.load(Ordering::Relaxed);
                blink_phase.store(new_phase, Ordering::Relaxed);

                // 获取需要闪烁的通道
                let blinking_channels = interaction.get_blinking_channels();

                // 发送闪烁更新
                for ch_name in blinking_channels {

                    // 闪烁时交替亮/灭
                    let state = if new_phase {
                        if interaction.is_solo_blinking() {
                            ChannelLedState::Solo
                        } else {
                            ChannelLedState::Mute
                        }
                    } else {
                        ChannelLedState::Off
                    };

                    let _ = tx.try_send(OscOutMessage::ChannelLed {
                        channel: ch_name,
                        state
                    });
                }

                // 模式按钮闪烁
                if interaction.is_solo_blinking() {
                    let _ = tx.try_send(OscOutMessage::ModeSolo { on: new_phase });
                }
                if interaction.is_mute_blinking() {
                    let _ = tx.try_send(OscOutMessage::ModeMute { on: new_phase });
                }
            }

            logger.info("osc", "[OSC Blink] Thread stopped");
        })
    }

    // ==================== 消息处理 ====================

    /// 处理发送的 OSC 消息
    fn process_outgoing_message(socket: &UdpSocket, target: &str, msg: OscOutMessage, logger: &InstanceLogger) {
        match msg {
            OscOutMessage::ChannelLed { channel, state } => {
                let addr = format!("/Monitor/Channel/{}", channel);
                Self::send_osc_float(socket, target, &addr, state as u8 as f32, logger);
            }
            OscOutMessage::ModeSolo { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Mode/Solo", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::ModeMute { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Mode/Mute", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::MasterVolume { value } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Volume", value, logger);
            }
            OscOutMessage::Dim { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Dim", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::Cut { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Cut", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::Mono { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Effect/Mono", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::LfeAdd10dB { on } => {
                Self::send_osc_float(socket, target, "/Monitor/LFE/Add_10dB", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::LowBoost { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Effect/Low_Boost", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::HighBoost { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Effect/High_Boost", if on { 1.0 } else { 0.0 }, logger);
            }
            OscOutMessage::BroadcastAll { channel_count, master_volume, dim, cut, channel_states, solo_active, mute_active } => {
                Self::broadcast_all_states(socket, target, channel_count, master_volume, dim, cut, &channel_states, solo_active, mute_active, logger);
            }
        }
    }

    /// 处理接收的 OSC 数据包
    fn process_incoming_packet(data: &[u8], interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        let packet = match rosc::decoder::decode_udp(data) {
            Ok((_, packet)) => packet,
            Err(e) => {
                logger.warn("osc", &format!("[OSC Recv] Failed to decode packet: {}", e));
                return;
            }
        };

        match packet {
            OscPacket::Message(msg) => Self::handle_osc_message(msg, interaction, state, logger),
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    if let OscPacket::Message(msg) = packet {
                        Self::handle_osc_message(msg, interaction, state, logger);
                    }
                }
            }
        }
    }

    /// 处理单个 OSC 消息
    fn handle_osc_message(msg: OscMessage, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        let addr = msg.addr.as_str();

        // 提取浮点值 (假设所有消息都是单个 float)
        let value = match msg.args.first() {
            Some(OscType::Float(v)) => *v,
            Some(OscType::Int(v)) => *v as f32,
            _ => {
                logger.warn("osc", &format!("[OSC Recv] Invalid message args: {:?}", msg.args));
                return;
            }
        };

        // 路由到相应处理（逐步迁移到使用 state）
        if addr == "/Monitor/Mode/Solo" {
            Self::handle_mode_solo(value, interaction, state, logger);
        } else if addr == "/Monitor/Mode/Mute" {
            Self::handle_mode_mute(value, interaction, state, logger);
        } else if addr == "/Monitor/Master/Volume" {
            Self::handle_master_volume(value, state, logger);
        } else if addr == "/Monitor/Master/Dim" {
            Self::handle_dim(value, state, logger);
        } else if addr == "/Monitor/Master/Cut" {
            Self::handle_cut(value, state, logger);
        } else if addr == "/Monitor/Master/Effect/Mono" {
            Self::handle_mono(value, state, logger);
        } else if addr == "/Monitor/LFE/Add_10dB" {
            Self::handle_lfe_add_10db(value, state, logger);
        } else if addr == "/Monitor/Master/Effect/Low_Boost" {
            Self::handle_low_boost(value, state, logger);
        } else if addr == "/Monitor/Master/Effect/High_Boost" {
            Self::handle_high_boost(value, state, logger);
        } else if addr.starts_with("/Monitor/Channel/") {
            let ch_name = &addr[17..]; // 跳过 "/Monitor/Channel/"
            Self::handle_channel_click(ch_name, value, interaction, state, logger);
        } else if addr.starts_with("/Monitor/Solo/") {
            let ch_name = &addr[14..]; // 跳过 "/Monitor/Solo/"
            Self::handle_solo_channel(ch_name, value, interaction, state, logger);
        } else if addr.starts_with("/Monitor/Mute/") {
            let ch_name = &addr[14..]; // 跳过 "/Monitor/Mute/"
            Self::handle_mute_channel(ch_name, value, interaction, state, logger);
        } else {
            logger.warn("osc", &format!("[OSC Recv] Unknown address: {}", addr));
        }
    }

    // ==================== OSC 消息处理器 ====================

    fn handle_mode_solo(value: f32, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        if value > 0.5 {
            let state_val = value.round() as u8;

            match state_val {
                1 => {
                    // value=1 → toggle（Mode 按钮点击）
                    logger.info("osc", "[OSC] Mode Solo toggle");
                    interaction.toggle_solo_mode();
                    // 使用实例状态发送LED状态
                    state.send_mode_solo(interaction.is_solo_active());
                    state.send_mode_mute(interaction.is_mute_active());
                    state.broadcast_channel_states(interaction);
                }
                2 => {
                    // value=2 → Group_Dial 预激活，延迟到通道消息处理
                    if !interaction.is_solo_active() {
                        state.pending_solo.store(true, Ordering::Relaxed);
                        logger.info("osc", "[OSC] Mode Solo pending activation");
                    }
                    // 不发送LED，等待通道消息
                    return;
                }
                _ => {
                    logger.warn("osc", &format!("[OSC] Mode Solo unknown value: {}", value));
                }
            }
        }
    }

    fn handle_mode_mute(value: f32, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        if value > 0.5 {
            let state_val = value.round() as u8;

            match state_val {
                1 => {
                    // value=1 → toggle（Mode 按钮点击）
                    logger.info("osc", "[OSC] Mode Mute toggle");
                    interaction.toggle_mute_mode();
                    // 使用实例状态发送LED状态
                    state.send_mode_solo(interaction.is_solo_active());
                    state.send_mode_mute(interaction.is_mute_active());
                    state.broadcast_channel_states(interaction);
                }
                2 => {
                    // value=2 → Group_Dial 预激活，延迟到通道消息处理
                    if !interaction.is_mute_active() {
                        state.pending_mute.store(true, Ordering::Relaxed);
                        logger.info("osc", "[OSC] Mode Mute pending activation");
                    }
                    // 不发送LED，等待通道消息
                    return;
                }
                _ => {
                    logger.warn("osc", &format!("[OSC] Mode Mute unknown value: {}", value));
                }
            }
        }
    }

    fn handle_channel_click(channel_name: &str, value: f32, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        // 检查通道是否在当前布局中（使用实例状态）
        let channel_exists = state.current_channel_names.read().contains(&channel_name.to_string());
        let state_val = value.round() as u8;

        // 处理 Group_Dial 消息 (value=10/11/12)
        if state_val == 10 || state_val == 11 || state_val == 12 {
            let pending_solo = state.pending_solo.swap(false, Ordering::Relaxed);
            let pending_mute = state.pending_mute.swap(false, Ordering::Relaxed);

            if !channel_exists {
                // 通道无效，清除待激活状态，什么都不做
                logger.info("osc", &format!("[OSC] Channel {} not in layout, pending mode cancelled", channel_name));
                return;
            }

            // 通道有效，先激活待激活的模式
            if pending_solo && !interaction.is_solo_active() {
                interaction.toggle_solo_mode();
                logger.info("osc", "[OSC] Mode Solo activated (deferred)");
            }
            if pending_mute && !interaction.is_mute_active() {
                interaction.toggle_mute_mode();
                logger.info("osc", "[OSC] Mode Mute activated (deferred)");
            }
        }

        if !channel_exists {
            logger.info("osc", &format!("[OSC] Channel {} not in current layout, ignored", channel_name));
            return;
        }

        // 根据 value 区分语义
        match state_val {
            1 => {
                // value=1 → 点击事件（toggle）
                logger.info("osc", &format!("[OSC] Channel {} click (toggle)", channel_name));
                interaction.handle_click(channel_name);
            }
            0 | 2 => {
                // value=0/2 → 目标状态（用于单通道精确控制）
                logger.info("osc", &format!("[OSC] Channel {} set state → {}", channel_name, state_val));
                interaction.set_channel_state(channel_name, state_val);
            }
            10 => {
                // value=10 → 有声音（Group_Dial 右转）- 加入通道，可退出空模式
                logger.info("osc", &format!("[OSC] Channel {} set sound → HAS SOUND", channel_name));
                interaction.set_channel_sound(channel_name, true, true);
            }
            11 => {
                // value=11 → 没声音，增量移除（不退出模式即使变空）
                logger.info("osc", &format!("[OSC] Channel {} set sound → NO SOUND (incremental)", channel_name));
                interaction.set_channel_sound(channel_name, false, false);
            }
            12 => {
                // value=12 → 没声音，可退出空模式
                logger.info("osc", &format!("[OSC] Channel {} set sound → NO SOUND (can exit)", channel_name));
                interaction.set_channel_sound(channel_name, false, true);
            }
            _ => {
                logger.warn("osc", &format!("[OSC] Channel {} unknown state value: {}", channel_name, state_val));
            }
        }

        // 广播通道状态
        state.broadcast_channel_states(interaction);

        // 广播模式状态
        state.send_mode_solo(interaction.is_solo_active());
        state.send_mode_mute(interaction.is_mute_active());
    }

    fn handle_solo_channel(channel_name: &str, value: f32, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        // 验证通道是否在当前布局中
        let channel_names = state.current_channel_names.read();
        if !channel_names.contains(&channel_name.to_string()) {
            logger.warn("osc", &format!("[OSC] Unknown channel name: {}", channel_name));
            return;
        }
        drop(channel_names);

        if value > 0.5 {
            logger.info("osc", &format!("[OSC] Solo channel pressed: {}", channel_name));
            interaction.handle_click(channel_name);
            state.broadcast_channel_states(interaction);
        }
    }

    fn handle_mute_channel(channel_name: &str, value: f32, interaction: &crate::Interaction::InteractionManager, state: &OscSharedState, logger: &InstanceLogger) {
        // 验证通道是否在当前布局中
        let channel_names = state.current_channel_names.read();
        if !channel_names.contains(&channel_name.to_string()) {
            logger.warn("osc", &format!("[OSC] Unknown channel name: {}", channel_name));
            return;
        }
        drop(channel_names);

        if value > 0.5 {
            logger.info("osc", &format!("[OSC] Mute channel pressed: {}", channel_name));
            interaction.handle_click(channel_name);
            state.broadcast_channel_states(interaction);
        }
    }

    fn handle_master_volume(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        // 限制范围 0.0 ~ 1.0
        let clamped = value.clamp(0.0, 1.0);
        logger.info("osc", &format!("[OSC] Master volume received: {:.3}", clamped));
        state.set_master_volume(clamped);
    }

    fn handle_dim(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let on = value > 0.5;
        logger.info("osc", &format!("[OSC] Dim received: {}", on));
        state.set_dim(on);
    }

    fn handle_cut(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let state_val = value.round() as u8;

        let new_cut = match state_val {
            1 => {
                // value=1 → toggle（旋钮按下）
                let current = state.current_cut.load(Ordering::Relaxed);
                let toggled = !current;
                logger.info("osc", &format!("[OSC] Cut toggle: {} -> {}", current, toggled));
                toggled
            }
            0 => {
                // value=0 → 关闭
                logger.info("osc", "[OSC] Cut set: false");
                false
            }
            _ => {
                // value>=2 → 开启
                logger.info("osc", "[OSC] Cut set: true");
                true
            }
        };

        state.current_cut.store(new_cut, Ordering::Relaxed);
        state.set_cut(new_cut);
    }

    fn handle_mono(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let on = value > 0.5;
        logger.info("osc", &format!("[OSC] ===== Mono received: {} =====", on));
        state.set_mono(on);
        // 发送反馈
        state.send_mono(on);
    }

    fn handle_lfe_add_10db(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let on = value > 0.5;
        logger.info("osc", &format!("[OSC] ===== LFE +10dB received: {} =====", on));
        state.set_lfe_add_10db(on);
        // 发送反馈
        state.send_lfe_add_10db(on);
    }

    fn handle_low_boost(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let on = value > 0.5;
        logger.info("osc", &format!("[OSC] ===== Low Boost received: {} =====", on));
        state.set_low_boost(on);
        // 发送反馈
        state.send_low_boost(on);
    }

    fn handle_high_boost(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let on = value > 0.5;
        logger.info("osc", &format!("[OSC] ===== High Boost received: {} =====", on));
        state.set_high_boost(on);
        // 发送反馈
        state.send_high_boost(on);
    }

    // ==================== 工具函数 ====================

    /// 发送单个 Float OSC 消息
    fn send_osc_float(socket: &UdpSocket, target: &str, addr: &str, value: f32, logger: &InstanceLogger) {
        let msg = OscMessage {
            addr: addr.to_string(),
            args: vec![OscType::Float(value)],
        };

        let packet = OscPacket::Message(msg);

        match encoder::encode(&packet) {
            Ok(bytes) => {
                if let Err(e) = socket.send_to(&bytes, target) {
                    logger.warn("osc", &format!("[OSC Send] Failed to send to {}: {}", target, e));
                }
            }
            Err(e) => {
                logger.error("osc", &format!("[OSC Send] Failed to encode message: {}", e));
            }
        }
    }

    /// 广播所有当前状态
    fn broadcast_all_states(
        socket: &UdpSocket,
        target: &str,
        channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
        channel_states: &[(String, u8)],
        solo_active: bool,
        mute_active: bool,
        logger: &InstanceLogger
    ) {
        logger.info("osc", &format!("[OSC] Broadcasting all states for {} channels...", channel_count));

        // 1. 所有通道的 LED 状态（三态：0=off, 1=mute, 2=solo）
        for (ch_name, state) in channel_states {
            Self::send_osc_float(socket, target,
                &format!("/Monitor/Channel/{}", ch_name),
                *state as f32,
                logger
            );
        }

        // 2. 模式按钮状态
        Self::send_osc_float(socket, target, "/Monitor/Mode/Solo",
            if solo_active { 1.0 } else { 0.0 },
            logger
        );
        Self::send_osc_float(socket, target, "/Monitor/Mode/Mute",
            if mute_active { 1.0 } else { 0.0 },
            logger
        );

        // 3. Master Volume (从参数传入)
        Self::send_osc_float(socket, target, "/Monitor/Master/Volume", master_volume, logger);

        // 4. Dim 状态
        Self::send_osc_float(socket, target, "/Monitor/Master/Dim",
            if dim { 1.0 } else { 0.0 },
            logger
        );

        // 5. Cut 状态
        Self::send_osc_float(socket, target, "/Monitor/Master/Cut",
            if cut { 1.0 } else { 0.0 },
            logger
        );

        logger.info("osc", "[OSC] Broadcast complete");
    }
}

impl Drop for OscManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
