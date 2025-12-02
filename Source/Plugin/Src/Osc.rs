#![allow(non_snake_case)]

use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use crossbeam::channel::{unbounded, Sender, Receiver, TryRecvError};
use rosc::{OscPacket, OscMessage, OscType, encoder};
use log::{info, warn, error};
use parking_lot::RwLock;
use lazy_static::lazy_static;

use crate::Interaction::INTERACTION;
use crate::config_manager::CONFIG;

/// OSC Send Port (Plugin → Hardware)
const OSC_SEND_PORT: u16 = 7444;

/// OSC Receive Port (Hardware → Plugin)
const OSC_RECEIVE_PORT: u16 = 7445;

/// Blink Timer Interval (milliseconds)
const BLINK_INTERVAL_MS: u64 = 500;

/// Maximum queued OSC messages to prevent memory overflow
const MAX_QUEUE_SIZE: usize = 1000;

/// OSC 输出消息类型
#[derive(Debug, Clone)]
pub enum OscOutMessage {
    /// Solo LED 状态: channel name, on (1.0 = green, 0.0 = off)
    SoloLed { channel: String, on: bool },

    /// Mute LED 状态: channel name, on (1.0 = red, 0.0 = off)
    MuteLed { channel: String, on: bool },

    /// Solo Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeSolo { on: bool },

    /// Mute Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeMute { on: bool },

    /// Master Volume 值: 0.0 to 1.0
    MasterVolume { value: f32 },

    /// 广播所有状态 (初始化时使用)
    BroadcastAll { channel_count: usize },
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

    /// 发送通道 Solo LED 状态
    pub fn send_solo_led(&self, ch_idx: usize, on: bool) {
        let ch_name = OscManager::channel_index_to_name(ch_idx);
        self.send(OscOutMessage::SoloLed { channel: ch_name, on });
    }

    /// 发送通道 Mute LED 状态
    pub fn send_mute_led(&self, ch_idx: usize, on: bool) {
        let ch_name = OscManager::channel_index_to_name(ch_idx);
        self.send(OscOutMessage::MuteLed { channel: ch_name, on });
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
    pub fn init(&mut self, channel_count: usize) {
        if self.is_running.load(Ordering::Relaxed) {
            warn!("[OSC] Already running, skipping initialization");
            return;
        }

        info!("[OSC] Initializing OSC Manager with {} channels...", channel_count);

        // 存储通道数
        self.channel_count = channel_count;

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
        let _ = send_tx.try_send(OscOutMessage::BroadcastAll { channel_count });
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

    // ==================== 线程实现 ====================

    /// 发送线程 - 处理 UDP 7444 发送
    fn spawn_send_thread(rx: Receiver<OscOutMessage>, is_running: Arc<AtomicBool>) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("[OSC Send] Thread started, binding to 0.0.0.0:0 → broadcast to 127.0.0.1:{}", OSC_SEND_PORT);

            // 绑定 UDP Socket (发送到 127.0.0.1:7444)
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    error!("[OSC Send] Failed to bind socket: {}", e);
                    return;
                }
            };

            let target_addr = format!("127.0.0.1:{}", OSC_SEND_PORT);

            // 主循环
            while is_running.load(Ordering::Relaxed) {
                match rx.try_recv() {
                    Ok(msg) => {
                        Self::process_outgoing_message(&socket, &target_addr, msg);
                    }
                    Err(TryRecvError::Empty) => {
                        // 没有消息,短暂休眠
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(TryRecvError::Disconnected) => {
                        warn!("[OSC Send] Channel disconnected, exiting thread");
                        break;
                    }
                }
            }

            info!("[OSC Send] Thread stopped");
        })
    }

    /// 接收线程 - 处理 UDP 7445 接收
    fn spawn_receive_thread(is_running: Arc<AtomicBool>) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("[OSC Recv] Thread started, binding to 0.0.0.0:{}", OSC_RECEIVE_PORT);

            // 绑定 UDP Socket
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", OSC_RECEIVE_PORT)) {
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

                    // 根据当前模式决定闪烁哪个 LED
                    if INTERACTION.is_solo_blinking() {
                        let _ = tx.try_send(OscOutMessage::SoloLed {
                            channel: ch_name,
                            on: new_phase
                        });
                    } else if INTERACTION.is_mute_blinking() {
                        let _ = tx.try_send(OscOutMessage::MuteLed {
                            channel: ch_name,
                            on: new_phase
                        });
                    }
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
            OscOutMessage::SoloLed { channel, on } => {
                let addr = format!("/Monitor/Solo/{}", channel);
                Self::send_osc_float(socket, target, &addr, if on { 1.0 } else { 0.0 });
            }
            OscOutMessage::MuteLed { channel, on } => {
                let addr = format!("/Monitor/Mute/{}", channel);
                Self::send_osc_float(socket, target, &addr, if on { 1.0 } else { 0.0 });
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
            OscOutMessage::BroadcastAll { channel_count } => {
                Self::broadcast_all_states(socket, target, channel_count);
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
        }
    }

    fn handle_mode_mute(value: f32) {
        if value > 0.5 {
            info!("[OSC] Mode Mute pressed");
            INTERACTION.toggle_mute_mode();
        }
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
        }
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
    fn broadcast_all_states(socket: &UdpSocket, target: &str, channel_count: usize) {
        info!("[OSC] Broadcasting all states for {} channels...", channel_count);

        // 1. 所有通道的 Solo/Mute 状态
        for idx in 0..channel_count {
            let ch_name = Self::channel_index_to_name(idx);

            // Solo LED
            let solo_on = INTERACTION.is_channel_solo(idx);
            Self::send_osc_float(socket, target,
                &format!("/Monitor/Solo/{}", ch_name),
                if solo_on { 1.0 } else { 0.0 }
            );

            // Mute LED
            let mute_on = INTERACTION.is_channel_muted(idx);
            Self::send_osc_float(socket, target,
                &format!("/Monitor/Mute/{}", ch_name),
                if mute_on { 1.0 } else { 0.0 }
            );
        }

        // 2. 模式按钮状态
        Self::send_osc_float(socket, target, "/Monitor/Mode/Solo",
            if INTERACTION.is_solo_active() { 1.0 } else { 0.0 }
        );
        Self::send_osc_float(socket, target, "/Monitor/Mode/Mute",
            if INTERACTION.is_mute_active() { 1.0 } else { 0.0 }
        );

        // 3. Master Volume (TODO: 从参数读取)
        Self::send_osc_float(socket, target, "/Monitor/Master/Volume", 0.75);

        info!("[OSC] Broadcast complete");
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
