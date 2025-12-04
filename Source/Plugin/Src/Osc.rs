#![allow(non_snake_case)]

use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use crossbeam::channel::{unbounded, Sender, Receiver};
use rosc::{OscPacket, OscMessage, OscType, encoder};
use log::{info, warn, error};
use parking_lot::RwLock;
use lazy_static::lazy_static;

use crate::Interaction::INTERACTION;
use crate::config_file::APP_CONFIG;

/// Blink Timer Interval (milliseconds)
const BLINK_INTERVAL_MS: u64 = 500;

/// Maximum queued OSC messages to prevent memory overflow
const MAX_QUEUE_SIZE: usize = 1000;

/// 当前音频布局的通道数（用于广播）
static CURRENT_CHANNEL_COUNT: AtomicUsize = AtomicUsize::new(0);

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

    /// 广播所有状态 (初始化时使用)
    BroadcastAll {
        channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
    },
}

/// 全局 OSC 发送器 (线程安全单例)
pub struct OscSender {
    tx: RwLock<Option<Sender<OscOutMessage>>>,
}

impl OscSender {
    pub const fn new() -> Self {
        Self {
            tx: RwLock::new(None),
        }
    }

    /// 注册发送通道 (由 OscManager::init 调用)
    pub fn register(&self, tx: Sender<OscOutMessage>) {
        *self.tx.write() = Some(tx);
    }

    /// 注销发送通道 (由 OscManager::shutdown 调用)
    pub fn unregister(&self) {
        *self.tx.write() = None;
    }

    /// 发送 Solo 模式按钮状态
    pub fn send_mode_solo(&self, on: bool) {
        self.send(OscOutMessage::ModeSolo { on });
    }

    /// 发送 Mute 模式按钮状态
    pub fn send_mode_mute(&self, on: bool) {
        self.send(OscOutMessage::ModeMute { on });
    }

    /// 发送通道 LED 状态（合并版）
    pub fn send_channel_led(&self, ch_idx: usize, state: ChannelLedState) {
        let ch_name = OscManager::channel_index_to_name(ch_idx);
        self.send(OscOutMessage::ChannelLed { channel: ch_name, state });
    }

    /// 发送主音量（0.0 ~ 1.0 线性值）
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

    /// 内部发送方法
    fn send(&self, msg: OscOutMessage) {
        if let Some(tx) = self.tx.read().as_ref() {
            let _ = tx.try_send(msg);
        }
    }
}

lazy_static! {
    /// 全局 OSC 发送器单例
    pub static ref OSC_SENDER: OscSender = OscSender::new();
}

/// OSC 接收状态 (从外部接收到的参数变化)
pub struct OscReceiver {
    /// Master Volume (使用 f32 的位表示存储在 AtomicU32 中)
    master_volume: AtomicU32,
    /// Dim 状态
    dim: AtomicBool,
    /// Cut 状态
    cut: AtomicBool,
    /// 是否有待处理的变化
    has_pending: AtomicBool,
}

impl OscReceiver {
    pub const fn new() -> Self {
        Self {
            master_volume: AtomicU32::new(0),  // 0.0 的位表示
            dim: AtomicBool::new(false),
            cut: AtomicBool::new(false),
            has_pending: AtomicBool::new(false),
        }
    }

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
}

lazy_static! {
    /// 全局 OSC 接收器单例
    pub static ref OSC_RECEIVER: OscReceiver = OscReceiver::new();
}

/// OSC 管理器 - 多线程架构
pub struct OscManager {
    /// 发送消息队列
    send_tx: Option<Sender<OscOutMessage>>,

    /// 运行状态标志 (原子操作)
    is_running: Arc<AtomicBool>,

    /// 闪烁相位 (true = 亮, false = 灭)
    blink_phase: Arc<AtomicBool>,

    /// 当前音频布局的通道数
    channel_count: usize,

    /// 线程句柄
    send_thread: Option<JoinHandle<()>>,
    receive_thread: Option<JoinHandle<()>>,
    blink_thread: Option<JoinHandle<()>>,
}

impl OscManager {
    /// 创建新的 OscManager (未初始化)
    pub fn new() -> Self {
        Self {
            send_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
            blink_phase: Arc::new(AtomicBool::new(false)),
            channel_count: 0,
            send_thread: None,
            receive_thread: None,
            blink_thread: None,
        }
    }

    /// 初始化 OSC (Master 或 Standalone 模式)
    pub fn init(&mut self, channel_count: usize, master_volume: f32, dim: bool, cut: bool) {
        if self.is_running.load(Ordering::Relaxed) {
            warn!("[OSC] Already running, skipping initialization");
            return;
        }

        info!("[OSC] Initializing OSC Manager with {} channels...", channel_count);

        // 存储通道数
        self.channel_count = channel_count;
        // 存储通道数供静态方法使用
        CURRENT_CHANNEL_COUNT.store(channel_count, Ordering::Relaxed);

        // 创建消息队列
        let (send_tx, send_rx) = unbounded::<OscOutMessage>();
        self.send_tx = Some(send_tx.clone());

        // 设置运行标志
        self.is_running.store(true, Ordering::Relaxed);

        // 启动三个线程
        let is_running_clone = Arc::clone(&self.is_running);
        let blink_phase_clone = Arc::clone(&self.blink_phase);

        // 1. 发送线程 (UDP 7444)
        self.send_thread = Some(Self::spawn_send_thread(send_rx, is_running_clone.clone()));

        // 2. 接收线程 (UDP 7445)
        self.receive_thread = Some(Self::spawn_receive_thread(is_running_clone.clone()));

        // 3. 闪烁定时器线程 (500ms)
        self.blink_thread = Some(Self::spawn_blink_thread(
            send_tx.clone(),
            is_running_clone,
            blink_phase_clone
        ));

        // 注册到全局发送器
        OSC_SENDER.register(send_tx.clone());

        info!("[OSC] All threads started successfully");

        // 广播初始状态
        let _ = send_tx.try_send(OscOutMessage::BroadcastAll {
            channel_count,
            master_volume,
            dim,
            cut,
        });
    }

    /// 关闭 OSC 系统
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Relaxed) {
            return;
        }

        info!("[OSC] Shutting down OSC Manager...");

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

        self.send_tx = None;

        // 注销全局发送器
        OSC_SENDER.unregister();

        info!("[OSC] OSC Manager shutdown complete");
    }

    /// 发送 OSC 消息 (非阻塞)
    pub fn send(&self, msg: OscOutMessage) {
        if let Some(tx) = &self.send_tx {
            if let Err(e) = tx.try_send(msg) {
                warn!("[OSC] Failed to queue message: {:?}", e);
            }
        }
    }

    /// 获取当前闪烁相位
    pub fn get_blink_phase(&self) -> bool {
        self.blink_phase.load(Ordering::Relaxed)
    }

    /// 更新当前音频布局的通道数（当 GUI 布局变化时调用）
    pub fn update_channel_count(new_count: usize) {
        CURRENT_CHANNEL_COUNT.store(new_count, Ordering::Relaxed);
        info!("[OSC] Channel count updated to: {}", new_count);
    }

    // ==================== 线程实现 ====================

    /// 发送线程 - 处理 UDP 发送
    fn spawn_send_thread(rx: Receiver<OscOutMessage>, is_running: Arc<AtomicBool>) -> JoinHandle<()> {
        thread::spawn(move || {
            let config = APP_CONFIG.get();
            let send_port = config.osc_send_port;

            info!("[OSC Send] Thread started, binding to 0.0.0.0:0 → broadcast to 127.0.0.1:{}", send_port);

            // 绑定 UDP Socket
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    error!("[OSC Send] Failed to bind socket: {}", e);
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
                        Self::process_outgoing_message(&socket, &target_addr, msg);

                        // 批量处理队列中所有待发消息（避免消息间延迟）
                        while let Ok(msg) = rx.try_recv() {
                            Self::process_outgoing_message(&socket, &target_addr, msg);
                        }
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                        // 超时正常，继续等待
                        continue;
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                        warn!("[OSC Send] Channel disconnected, exiting thread");
                        break;
                    }
                }
            }

            info!("[OSC Send] Thread stopped");
        })
    }

    /// 接收线程 - 处理 UDP 接收
    fn spawn_receive_thread(is_running: Arc<AtomicBool>) -> JoinHandle<()> {
        thread::spawn(move || {
            let config = APP_CONFIG.get();
            let recv_port = config.osc_receive_port;

            info!("[OSC Recv] Thread started, binding to 0.0.0.0:{}", recv_port);

            // 绑定 UDP Socket
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", recv_port)) {
                Ok(s) => s,
                Err(e) => {
                    error!("[OSC Recv] Failed to bind socket: {}", e);
                    return;
                }
            };

            // 设置非阻塞模式
            if let Err(e) = socket.set_read_timeout(Some(Duration::from_millis(100))) {
                error!("[OSC Recv] Failed to set timeout: {}", e);
                return;
            }

            let mut buf = [0u8; 1024];

            // 主循环
            while is_running.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((size, _src)) => {
                        Self::process_incoming_packet(&buf[..size]);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                               || e.kind() == std::io::ErrorKind::TimedOut => {
                        // 超时正常,继续循环
                        continue;
                    }
                    Err(e) => {
                        error!("[OSC Recv] Socket error: {}", e);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            info!("[OSC Recv] Thread stopped");
        })
    }

    /// 闪烁定时器线程 - 每 500ms 切换一次相位
    fn spawn_blink_thread(
        tx: Sender<OscOutMessage>,
        is_running: Arc<AtomicBool>,
        blink_phase: Arc<AtomicBool>
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("[OSC Blink] Thread started, interval = {}ms", BLINK_INTERVAL_MS);

            while is_running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(BLINK_INTERVAL_MS));

                // 切换相位
                let new_phase = !blink_phase.load(Ordering::Relaxed);
                blink_phase.store(new_phase, Ordering::Relaxed);

                // 获取需要闪烁的通道
                let blinking_channels = INTERACTION.get_blinking_channels();

                // 发送闪烁更新
                for ch_idx in blinking_channels {
                    let ch_name = Self::channel_index_to_name(ch_idx);

                    // 闪烁时交替亮/灭
                    let state = if new_phase {
                        if INTERACTION.is_solo_blinking() {
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
                if INTERACTION.is_solo_blinking() {
                    let _ = tx.try_send(OscOutMessage::ModeSolo { on: new_phase });
                }
                if INTERACTION.is_mute_blinking() {
                    let _ = tx.try_send(OscOutMessage::ModeMute { on: new_phase });
                }
            }

            info!("[OSC Blink] Thread stopped");
        })
    }

    // ==================== 消息处理 ====================

    /// 处理发送的 OSC 消息
    fn process_outgoing_message(socket: &UdpSocket, target: &str, msg: OscOutMessage) {
        match msg {
            OscOutMessage::ChannelLed { channel, state } => {
                let addr = format!("/Monitor/Channel/{}", channel);
                Self::send_osc_float(socket, target, &addr, state as u8 as f32);
            }
            OscOutMessage::ModeSolo { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Mode/Solo", if on { 1.0 } else { 0.0 });
            }
            OscOutMessage::ModeMute { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Mode/Mute", if on { 1.0 } else { 0.0 });
            }
            OscOutMessage::MasterVolume { value } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Volume", value);
            }
            OscOutMessage::Dim { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Dim", if on { 1.0 } else { 0.0 });
            }
            OscOutMessage::Cut { on } => {
                Self::send_osc_float(socket, target, "/Monitor/Master/Cut", if on { 1.0 } else { 0.0 });
            }
            OscOutMessage::BroadcastAll { channel_count, master_volume, dim, cut } => {
                Self::broadcast_all_states(socket, target, channel_count, master_volume, dim, cut);
            }
        }
    }

    /// 处理接收的 OSC 数据包
    fn process_incoming_packet(data: &[u8]) {
        let packet = match rosc::decoder::decode_udp(data) {
            Ok((_, packet)) => packet,
            Err(e) => {
                warn!("[OSC Recv] Failed to decode packet: {}", e);
                return;
            }
        };

        match packet {
            OscPacket::Message(msg) => Self::handle_osc_message(msg),
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    if let OscPacket::Message(msg) = packet {
                        Self::handle_osc_message(msg);
                    }
                }
            }
        }
    }

    /// 处理单个 OSC 消息
    fn handle_osc_message(msg: OscMessage) {
        let addr = msg.addr.as_str();

        // 提取浮点值 (假设所有消息都是单个 float)
        let value = match msg.args.first() {
            Some(OscType::Float(v)) => *v,
            Some(OscType::Int(v)) => *v as f32,
            _ => {
                warn!("[OSC Recv] Invalid message args: {:?}", msg.args);
                return;
            }
        };

        // 路由到相应处理
        if addr == "/Monitor/Mode/Solo" {
            Self::handle_mode_solo(value);
        } else if addr == "/Monitor/Mode/Mute" {
            Self::handle_mode_mute(value);
        } else if addr == "/Monitor/Master/Volume" {
            Self::handle_master_volume(value);
        } else if addr == "/Monitor/Master/Dim" {
            Self::handle_dim(value);
        } else if addr == "/Monitor/Master/Cut" {
            Self::handle_cut(value);
        } else if addr.starts_with("/Monitor/Channel/") {
            let ch_name = &addr[17..]; // 跳过 "/Monitor/Channel/"
            Self::handle_channel_click(ch_name, value);
        } else if addr.starts_with("/Monitor/Solo/") {
            let ch_name = &addr[14..]; // 跳过 "/Monitor/Solo/"
            Self::handle_solo_channel(ch_name, value);
        } else if addr.starts_with("/Monitor/Mute/") {
            let ch_name = &addr[14..]; // 跳过 "/Monitor/Mute/"
            Self::handle_mute_channel(ch_name, value);
        } else {
            warn!("[OSC Recv] Unknown address: {}", addr);
        }
    }

    // ==================== OSC 消息处理器 ====================

    fn handle_mode_solo(value: f32) {
        if value > 0.5 {
            info!("[OSC] Mode Solo pressed");
            INTERACTION.toggle_solo_mode();

            // 回传模式按钮状态给硬件
            let is_active = INTERACTION.is_solo_active();
            OSC_SENDER.send_mode_solo(is_active);

            // 广播所有通道 LED 状态
            Self::broadcast_channel_states();
        }
    }

    fn handle_mode_mute(value: f32) {
        if value > 0.5 {
            info!("[OSC] Mode Mute pressed");
            INTERACTION.toggle_mute_mode();

            // 回传模式按钮状态给硬件
            let is_active = INTERACTION.is_mute_active();
            OSC_SENDER.send_mode_mute(is_active);

            // 广播所有通道 LED 状态
            Self::broadcast_channel_states();
        }
    }

    fn handle_channel_click(channel_name: &str, value: f32) {
        let ch_idx = match Self::channel_name_to_index(channel_name) {
            Some(idx) => idx,
            None => {
                warn!("[OSC] Unknown channel name: {}", channel_name);
                return;
            }
        };

        // 根据 value 区分语义
        let state = value.round() as u8;

        if state == 1 {
            // value ≈ 1.0 → 点击事件（toggle）
            info!("[OSC] Channel {} click (toggle)", channel_name);
            INTERACTION.handle_click(ch_idx);
        } else {
            // value ≈ 0.0 或 2.0 → 目标状态（直接设置）
            info!("[OSC] Channel {} set state → {}", channel_name, state);
            INTERACTION.set_channel_state(ch_idx, state);
        }

        Self::broadcast_channel_states();
    }

    fn handle_solo_channel(channel_name: &str, value: f32) {
        let ch_idx = match Self::channel_name_to_index(channel_name) {
            Some(idx) => idx,
            None => {
                warn!("[OSC] Unknown channel name: {}", channel_name);
                return;
            }
        };

        if value > 0.5 {
            info!("[OSC] Solo channel pressed: {} (index {})", channel_name, ch_idx);
            INTERACTION.handle_click(ch_idx);
            Self::broadcast_channel_states();
        }
    }

    fn handle_mute_channel(channel_name: &str, value: f32) {
        let ch_idx = match Self::channel_name_to_index(channel_name) {
            Some(idx) => idx,
            None => {
                warn!("[OSC] Unknown channel name: {}", channel_name);
                return;
            }
        };

        if value > 0.5 {
            info!("[OSC] Mute channel pressed: {} (index {})", channel_name, ch_idx);
            INTERACTION.handle_click(ch_idx);
            Self::broadcast_channel_states();
        }
    }

    fn handle_master_volume(value: f32) {
        // 限制范围 0.0 ~ 1.0
        let clamped = value.clamp(0.0, 1.0);
        info!("[OSC] Master volume received: {:.3}", clamped);
        OSC_RECEIVER.set_master_volume(clamped);
    }

    fn handle_dim(value: f32) {
        let on = value > 0.5;
        info!("[OSC] Dim received: {}", on);
        OSC_RECEIVER.set_dim(on);
    }

    fn handle_cut(value: f32) {
        let on = value > 0.5;
        info!("[OSC] Cut received: {}", on);
        OSC_RECEIVER.set_cut(on);
    }

    // ==================== 工具函数 ====================

    /// 发送单个 Float OSC 消息
    fn send_osc_float(socket: &UdpSocket, target: &str, addr: &str, value: f32) {
        let msg = OscMessage {
            addr: addr.to_string(),
            args: vec![OscType::Float(value)],
        };

        let packet = OscPacket::Message(msg);

        match encoder::encode(&packet) {
            Ok(bytes) => {
                if let Err(e) = socket.send_to(&bytes, target) {
                    warn!("[OSC Send] Failed to send to {}: {}", target, e);
                }
            }
            Err(e) => {
                error!("[OSC Send] Failed to encode message: {}", e);
            }
        }
    }

    /// 广播所有当前状态
    fn broadcast_all_states(socket: &UdpSocket, target: &str, channel_count: usize, master_volume: f32, dim: bool, cut: bool) {
        info!("[OSC] Broadcasting all states for {} channels...", channel_count);

        // 1. 所有通道的 LED 状态（三态：0=off, 1=mute, 2=solo）
        for idx in 0..channel_count {
            let ch_name = Self::channel_index_to_name(idx);

            // 确定通道状态
            let state = if INTERACTION.is_channel_solo(idx) {
                ChannelLedState::Solo  // 2 = 绿色
            } else if INTERACTION.is_channel_muted(idx) {
                ChannelLedState::Mute  // 1 = 红色
            } else {
                ChannelLedState::Off   // 0 = 不亮
            };

            Self::send_osc_float(socket, target,
                &format!("/Monitor/Channel/{}", ch_name),
                state as u8 as f32
            );
        }

        // 2. 模式按钮状态
        Self::send_osc_float(socket, target, "/Monitor/Mode/Solo",
            if INTERACTION.is_solo_active() { 1.0 } else { 0.0 }
        );
        Self::send_osc_float(socket, target, "/Monitor/Mode/Mute",
            if INTERACTION.is_mute_active() { 1.0 } else { 0.0 }
        );

        // 3. Master Volume (从参数传入)
        Self::send_osc_float(socket, target, "/Monitor/Master/Volume", master_volume);

        // 4. Dim 状态
        Self::send_osc_float(socket, target, "/Monitor/Master/Dim",
            if dim { 1.0 } else { 0.0 }
        );

        // 5. Cut 状态
        Self::send_osc_float(socket, target, "/Monitor/Master/Cut",
            if cut { 1.0 } else { 0.0 }
        );

        info!("[OSC] Broadcast complete");
    }

    /// 广播所有通道的 LED 状态（三态：0=off, 1=mute, 2=solo）
    /// 用于通道点击后同步所有受影响的通道状态
    pub fn broadcast_channel_states() {
        let channel_count = CURRENT_CHANNEL_COUNT.load(Ordering::Relaxed);
        if channel_count == 0 {
            warn!("[OSC] Channel count not initialized, skipping broadcast");
            return;
        }

        info!("[OSC] Broadcasting all channel LED states for {} channels...", channel_count);

        for ch_idx in 0..channel_count {
            let state = if INTERACTION.is_channel_solo(ch_idx) {
                ChannelLedState::Solo  // 2 = 绿色
            } else if INTERACTION.is_channel_muted(ch_idx) {
                ChannelLedState::Mute  // 1 = 红色
            } else {
                ChannelLedState::Off   // 0 = 不亮
            };

            let ch_name = Self::channel_index_to_name(ch_idx);
            info!("[OSC] Channel {} → {:?}", ch_name, state);
            OSC_SENDER.send_channel_led(ch_idx, state);
        }
    }

    /// 通道索引 → 名称映射 (匹配旧版 C++ 实现)
    pub fn channel_index_to_name(idx: usize) -> String {
        let names = ["L", "R", "C", "LFE", "LR", "RR", "LSS", "RSS",
                     "LRS", "RRS", "LTF", "RTF", "LTB", "RTB",
                     "SUB_F", "SUB_B", "SUB_L", "SUB_R"];
        names.get(idx).unwrap_or(&"UNKNOWN").to_string()
    }

    /// 通道名称 → 索引映射
    fn channel_name_to_index(name: &str) -> Option<usize> {
        let names = ["L", "R", "C", "LFE", "LR", "RR", "LSS", "RSS",
                     "LRS", "RRS", "LTF", "RTF", "LTB", "RTB",
                     "SUB_F", "SUB_B", "SUB_L", "SUB_R"];
        names.iter().position(|&n| n == name)
    }
}

impl Drop for OscManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
