#![allow(non_snake_case)]

use crossbeam::channel::{bounded, Receiver, Sender};
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mcm_core::interaction::InteractionManager;
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::{MonitorParams, PluginRole};
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::config::AppConfig;
use mcm_protocol::osc_structs::{ChannelLedState, OscOutMessage};
use mcm_protocol::web_structs::WebSharedState;

/// Blink Timer Interval (milliseconds)
const BLINK_INTERVAL_MS: u64 = 500;

/// Maximum queued OSC messages to prevent memory overflow
const MAX_QUEUE_SIZE: usize = 1000;

// OscSharedState, ChannelLedState, OscOutMessage defined in Core/Protocol

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
    interaction: Option<Arc<InteractionManager>>,

    /// 实例级日志器
    logger: Option<Arc<InstanceLogger>>,

    /// Web 共享状态（用于获取 Web OSC 接收端口）
    web_state: Option<Arc<WebSharedState>>,

    /// 线程句柄
    send_thread: Option<JoinHandle<()>>,
    receive_thread: Option<JoinHandle<()>>,
    blink_thread: Option<JoinHandle<()>>,
}

impl OscManager {
    /// 创建新的 OscManager (未初始化)
    pub fn new() -> Self {
        Self::with_state(Arc::new(OscSharedState::new()))
    }

    /// 使用现有的 State 创建 OscManager
    pub fn with_state(state: Arc<OscSharedState>) -> Self {
        Self {
            state,
            is_running: Arc::new(AtomicBool::new(false)),
            blink_phase: Arc::new(AtomicBool::new(false)),
            channel_count: 0,
            interaction: None,
            logger: None,
            web_state: None,
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
    pub fn init(
        &mut self,
        channel_count: usize,
        _master_volume: f32,
        _dim: bool,
        cut: bool,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        logger: Arc<InstanceLogger>,
        app_config: &AppConfig,
        web_state: Arc<WebSharedState>,
    ) {
        let threads_alive = self
            .send_thread
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
            || self
                .receive_thread
                .as_ref()
                .map(|h| !h.is_finished())
                .unwrap_or(false)
            || self
                .blink_thread
                .as_ref()
                .map(|h| !h.is_finished())
                .unwrap_or(false);

        if self.is_running.load(Ordering::Acquire) && threads_alive {
            logger.warn("osc", "[OSC] Already running, skipping initialization");
            return;
        }

        if self.is_running.load(Ordering::Acquire) && !threads_alive {
            logger.info(
                "osc",
                "[OSC] Previous threads exited, cleaning up for re-init...",
            );
            self.is_running.store(false, Ordering::Release);
            if let Some(h) = self.send_thread.take() {
                let _ = h.join();
            }
            if let Some(h) = self.receive_thread.take() {
                let _ = h.join();
            }
            if let Some(h) = self.blink_thread.take() {
                let _ = h.join();
            }
            *self.state.sender_tx.write() = None;
        }

        logger.info(
            "osc",
            &format!(
                "[OSC] Initializing OSC Manager with {} channels...",
                channel_count
            ),
        );

        self.logger = Some(Arc::clone(&logger));
        self.web_state = Some(Arc::clone(&web_state));
        self.channel_count = channel_count;
        self.interaction = Some(interaction.clone());

        // Update state
        self.state
            .channel_count
            .store(channel_count, Ordering::Relaxed);
        self.state.current_cut.store(cut, Ordering::Relaxed);
        self.state.set_logger(Arc::clone(&logger)); // Ensure logger is set in shared state

        let (send_tx, send_rx) = bounded::<OscOutMessage>(MAX_QUEUE_SIZE);
        *self.state.sender_tx.write() = Some(send_tx.clone());

        self.is_running.store(true, Ordering::Release);

        let send_port = app_config.osc_send_port;
        let recv_port = app_config.osc_receive_port;

        let init_role = params.role.value();

        let is_running_clone = Arc::clone(&self.is_running);
        let blink_phase_clone = Arc::clone(&self.blink_phase);
        let state_clone = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&logger);
        let params_clone = Arc::clone(&params);
        let web_state_clone = Arc::clone(&web_state);

        // 1. Send Thread
        self.send_thread = Some(Self::spawn_send_thread(
            send_rx,
            is_running_clone.clone(),
            Arc::clone(&params_clone),
            init_role,
            Arc::clone(&logger_clone),
            send_port,
            web_state_clone,
        ));

        // 2. Receive Thread
        self.receive_thread = Some(Self::spawn_receive_thread(
            is_running_clone.clone(),
            interaction.clone(),
            Arc::clone(&state_clone),
            Arc::clone(&params_clone),
            init_role,
            Arc::clone(&logger_clone),
            recv_port,
        ));

        // 3. Blink Thread
        self.blink_thread = Some(Self::spawn_blink_thread(
            send_tx.clone(),
            is_running_clone,
            blink_phase_clone,
            interaction.clone(),
            Arc::clone(&state_clone),
            Arc::clone(&params_clone),
            init_role,
            Arc::clone(&logger_clone),
        ));

        logger.info("osc", "[OSC] All threads started successfully");
    }

    /// Broadcast current state to hardware
    pub fn broadcast_state(&self, channel_count: usize, master_volume: f32, dim: bool, cut: bool) {
        if let Some(ref tx) = *self.state.sender_tx.read() {
            let channel_states = if let Some(ref interaction) = self.interaction {
                let channel_names = self.state.current_channel_names.read();
                channel_names
                    .iter()
                    .map(|name| {
                        let state = if interaction.is_channel_solo(name) {
                            2u8
                        } else if interaction.is_channel_muted(name) {
                            1u8
                        } else {
                            0u8
                        };
                        (name.clone(), state)
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let (solo_active, mute_active) = if let Some(ref interaction) = self.interaction {
                (interaction.is_solo_active(), interaction.is_mute_active())
            } else {
                (false, false)
            };

            if let Some(ref logger) = self.logger {
                logger.info(
                    "osc",
                    &format!(
                        "[OSC] Broadcasting state: vol={:.2}, dim={}, cut={}",
                        master_volume, dim, cut
                    ),
                );
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

    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Acquire) {
            return;
        }

        if let Some(ref logger) = self.logger {
            logger.info("osc", "[OSC] Shutting down OSC Manager...");
        }

        self.is_running.store(false, Ordering::Release);

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

    // ==================== Thread Implementations ====================

    fn spawn_send_thread(
        rx: Receiver<OscOutMessage>,
        is_running: Arc<AtomicBool>,
        params: Arc<MonitorParams>,
        _init_role: PluginRole,
        logger: Arc<InstanceLogger>,
        send_port: u16,
        web_state: Arc<WebSharedState>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info(
                "osc",
                &format!(
                    "[OSC Send] Thread started, binding to 0.0.0.0:0 -> hardware:127.0.0.1:{}",
                    send_port
                ),
            );

            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    logger.error("osc", &format!("[OSC Send] Failed to bind socket: {}", e));
                    return;
                }
            };

            let target_hardware = format!("127.0.0.1:{}", send_port);

            while is_running.load(Ordering::Acquire) {
                if params.role.value() == PluginRole::Slave {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(msg) => {
                        let web_port = web_state.osc_recv_port.load(Ordering::Relaxed);
                        let target_web = if web_port > 0 {
                            Some(format!("127.0.0.1:{}", web_port))
                        } else {
                            None
                        };

                        Self::process_outgoing_message_dual(
                            &socket,
                            &target_hardware,
                            target_web.as_deref(),
                            msg,
                            &logger,
                        );

                        while let Ok(msg) = rx.try_recv() {
                            let web_port = web_state.osc_recv_port.load(Ordering::Relaxed);
                            let target_web = if web_port > 0 {
                                Some(format!("127.0.0.1:{}", web_port))
                            } else {
                                None
                            };
                            Self::process_outgoing_message_dual(
                                &socket,
                                &target_hardware,
                                target_web.as_deref(),
                                msg,
                                &logger,
                            );
                        }
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Timeout) => continue,
                    Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                        logger.warn("osc", "[OSC Send] Channel disconnected, exiting thread");
                        break;
                    }
                }
            }
            logger.info("osc", "[OSC Send] Thread stopped");
        })
    }

    fn spawn_receive_thread(
        is_running: Arc<AtomicBool>,
        interaction: Arc<InteractionManager>,
        state: Arc<OscSharedState>,
        params: Arc<MonitorParams>,
        _init_role: PluginRole,
        logger: Arc<InstanceLogger>,
        recv_port: u16,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info(
                "osc",
                &format!(
                    "[OSC Recv] Thread started, binding to 0.0.0.0:{}",
                    recv_port
                ),
            );

            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", recv_port)) {
                Ok(s) => {
                    state.recv_port_bound.store(true, Ordering::Relaxed);
                    logger.info(
                        "osc",
                        &format!("[OSC Recv] Port {} bound successfully", recv_port),
                    );
                    s
                }
                Err(e) => {
                    state.recv_port_bound.store(false, Ordering::Relaxed);
                    is_running.store(false, Ordering::Release);
                    logger.info(
                        "osc",
                        &format!("[OSC Recv] Port {} unavailable: {}", recv_port, e),
                    );
                    return;
                }
            };

            if let Err(e) = socket.set_read_timeout(Some(Duration::from_millis(100))) {
                logger.error("osc", &format!("[OSC Recv] Failed to set timeout: {}", e));
                is_running.store(false, Ordering::Release);
                return;
            }

            let mut buf = [0u8; 1024];

            while is_running.load(Ordering::Acquire) {
                if params.role.value() == PluginRole::Slave {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                match socket.recv_from(&mut buf) {
                    Ok((size, _src)) => {
                        Self::process_incoming_packet(&buf[..size], &interaction, &state, &logger);
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue
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

    fn spawn_blink_thread(
        tx: Sender<OscOutMessage>,
        is_running: Arc<AtomicBool>,
        blink_phase: Arc<AtomicBool>,
        interaction: Arc<InteractionManager>,
        _state: Arc<OscSharedState>,
        params: Arc<MonitorParams>,
        _init_role: PluginRole,
        logger: Arc<InstanceLogger>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            logger.info(
                "osc",
                &format!(
                    "[OSC Blink] Thread started, interval = {}ms",
                    BLINK_INTERVAL_MS
                ),
            );

            while is_running.load(Ordering::Acquire) {
                if params.role.value() == PluginRole::Slave {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                thread::sleep(Duration::from_millis(BLINK_INTERVAL_MS));

                let new_phase = !blink_phase.load(Ordering::Relaxed);
                blink_phase.store(new_phase, Ordering::Relaxed);

                let blinking_channels = interaction.get_blinking_channels();

                for ch_name in blinking_channels {
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
                        state,
                    });
                }

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

    // ==================== Message Processing ====================

    fn process_outgoing_message_dual(
        socket: &UdpSocket,
        target_hardware: &str,
        target_web: Option<&str>,
        msg: OscOutMessage,
        logger: &InstanceLogger,
    ) {
        match msg {
            OscOutMessage::ChannelLed { channel, state } => {
                let addr = format!("/Monitor/Channel/{}", channel);
                Self::send_osc_float_dual(
                    socket,
                    target_hardware,
                    target_web,
                    &addr,
                    state as u8 as f32,
                    logger,
                );
            }
            OscOutMessage::ModeSolo { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Mode/Solo",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::ModeMute { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Mode/Mute",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::MasterVolume { value } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Volume",
                value,
                logger,
            ),
            OscOutMessage::Dim { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Dim",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::Cut { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Cut",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::Mono { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Effect/Mono",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::LfeAdd10dB { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/LFE/Add_10dB",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::LowBoost { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Effect/Low_Boost",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::HighBoost { on } => Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                "/Monitor/Master/Effect/High_Boost",
                if on { 1.0 } else { 0.0 },
                logger,
            ),
            OscOutMessage::BroadcastAll {
                channel_count,
                master_volume,
                dim,
                cut,
                channel_states,
                solo_active,
                mute_active,
            } => {
                Self::broadcast_all_states_dual(
                    socket,
                    target_hardware,
                    target_web,
                    channel_count,
                    master_volume,
                    dim,
                    cut,
                    &channel_states,
                    solo_active,
                    mute_active,
                    logger,
                );
            }
        }
    }

    fn process_incoming_packet(
        data: &[u8],
        interaction: &InteractionManager,
        state: &OscSharedState,
        logger: &InstanceLogger,
    ) {
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

    fn handle_osc_message(
        msg: OscMessage,
        interaction: &InteractionManager,
        state: &OscSharedState,
        logger: &InstanceLogger,
    ) {
        let addr = msg.addr.as_str();
        let value = match msg.args.first() {
            Some(OscType::Float(v)) => *v,
            Some(OscType::Int(v)) => *v as f32,
            _ => {
                return;
            }
        };

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
            let ch_name = &addr[17..];
            Self::handle_channel_click(ch_name, value, interaction, state, logger);
        }
    }

    // Logic handlers call OscSharedState methods
    fn handle_mode_solo(
        value: f32,
        interaction: &InteractionManager,
        state: &OscSharedState,
        logger: &InstanceLogger,
    ) {
        if value > 0.5 {
            let state_val = value.round() as u8;
            if state_val == 1 {
                logger.info("osc", "[OSC] Mode Solo toggle");
                interaction.toggle_solo_mode();
                state.send_mode_solo(interaction.is_solo_active());
                state.send_mode_mute(interaction.is_mute_active());
                state.broadcast_channel_states(interaction);
            } else if state_val == 2 && !interaction.is_solo_active() {
                state.pending_solo.store(true, Ordering::Relaxed);
            }
        }
    }

    fn handle_mode_mute(
        value: f32,
        interaction: &InteractionManager,
        state: &OscSharedState,
        logger: &InstanceLogger,
    ) {
        if value > 0.5 {
            let state_val = value.round() as u8;
            if state_val == 1 {
                logger.info("osc", "[OSC] Mode Mute toggle");
                interaction.toggle_mute_mode();
                state.send_mode_solo(interaction.is_solo_active());
                state.send_mode_mute(interaction.is_mute_active());
                state.broadcast_channel_states(interaction);
            } else if state_val == 2 && !interaction.is_mute_active() {
                state.pending_mute.store(true, Ordering::Relaxed);
            }
        }
    }

    fn handle_master_volume(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let clamped = value.clamp(0.0, 1.0);
        logger.info("osc", &format!("[OSC] Master volume: {:.3}", clamped));
        state.set_master_volume(clamped);
    }

    fn handle_dim(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let state_val = value.round() as u8;
        let new_dim = if state_val == 1 {
            let cur = state.dim.load(Ordering::Relaxed);
            !cur
        } else {
            state_val >= 2
        };
        logger.info("osc", &format!("[OSC] Dim set: {}", new_dim));
        state.set_dim(new_dim);
        state.send_dim(new_dim);
    }

    fn handle_cut(value: f32, state: &OscSharedState, logger: &InstanceLogger) {
        let state_val = value.round() as u8;
        let new_cut = if state_val == 1 {
            let cur = state.current_cut.load(Ordering::Relaxed);
            !cur
        } else {
            state_val >= 2
        };
        logger.info("osc", &format!("[OSC] Cut set: {}", new_cut));
        state.current_cut.store(new_cut, Ordering::Relaxed);
        state.set_cut(new_cut);
        state.send_cut(new_cut);
    }

    fn handle_mono(value: f32, state: &OscSharedState, _logger: &InstanceLogger) {
        state.set_mono(value > 0.5);
        state.send_mono(value > 0.5);
    }

    fn handle_lfe_add_10db(value: f32, state: &OscSharedState, _logger: &InstanceLogger) {
        state.set_lfe_add_10db(value > 0.5);
        state.send_lfe_add_10db(value > 0.5);
    }

    fn handle_low_boost(value: f32, state: &OscSharedState, _logger: &InstanceLogger) {
        state.set_low_boost(value > 0.5);
        state.send_low_boost(value > 0.5);
    }

    fn handle_high_boost(value: f32, state: &OscSharedState, _logger: &InstanceLogger) {
        state.set_high_boost(value > 0.5);
        state.send_high_boost(value > 0.5);
    }

    fn handle_channel_click(
        channel_name: &str,
        value: f32,
        interaction: &InteractionManager,
        state: &OscSharedState,
        _logger: &InstanceLogger,
    ) {
        // ... (simplified logic call interaction)
        // Check exist
        let channel_exists = state
            .current_channel_names
            .read()
            .contains(&channel_name.to_string());
        if !channel_exists {
            return;
        }

        let state_val = value.round() as u8;

        // Group Dial Logic (10, 11, 12)
        if state_val >= 10 {
            // Handle pending
            let pending_solo = state.pending_solo.swap(false, Ordering::Relaxed);
            let pending_mute = state.pending_mute.swap(false, Ordering::Relaxed);
            if pending_solo && !interaction.is_solo_active() {
                interaction.toggle_solo_mode();
            }
            if pending_mute && !interaction.is_mute_active() {
                interaction.toggle_mute_mode();
            }
        }

        match state_val {
            1 => interaction.handle_click(channel_name),
            0 | 2 => interaction.set_channel_state(channel_name, state_val),
            10 => interaction.set_channel_sound(channel_name, true, true),
            11 => interaction.set_channel_sound(channel_name, false, false),
            12 => interaction.set_channel_sound(channel_name, false, true),
            _ => {}
        }

        state.broadcast_channel_states(interaction);
        state.send_mode_solo(interaction.is_solo_active());
        state.send_mode_mute(interaction.is_mute_active());
    }

    fn send_osc_float_dual(
        socket: &UdpSocket,
        target_hardware: &str,
        target_web: Option<&str>,
        addr: &str,
        value: f32,
        logger: &InstanceLogger,
    ) {
        let msg = OscMessage {
            addr: addr.to_string(),
            args: vec![OscType::Float(value)],
        };
        let packet = OscPacket::Message(msg);
        if let Ok(bytes) = encoder::encode(&packet) {
            let _ = socket.send_to(&bytes, target_hardware);
            if let Some(w) = target_web {
                let _ = socket.send_to(&bytes, w);
            }
        } else if let Err(e) = encoder::encode(&packet) {
            logger.error("osc", &format!("Encode error: {}", e));
        }
    }

    fn broadcast_all_states_dual(
        socket: &UdpSocket,
        target_hardware: &str,
        target_web: Option<&str>,
        _channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
        channel_states: &[(String, u8)],
        solo_active: bool,
        mute_active: bool,
        logger: &InstanceLogger,
    ) {
        Self::send_osc_float_dual(
            socket,
            target_hardware,
            target_web,
            "/Monitor/Mode/Solo",
            if solo_active { 1.0 } else { 0.0 },
            logger,
        );
        Self::send_osc_float_dual(
            socket,
            target_hardware,
            target_web,
            "/Monitor/Mode/Mute",
            if mute_active { 1.0 } else { 0.0 },
            logger,
        );
        Self::send_osc_float_dual(
            socket,
            target_hardware,
            target_web,
            "/Monitor/Master/Volume",
            master_volume,
            logger,
        );
        Self::send_osc_float_dual(
            socket,
            target_hardware,
            target_web,
            "/Monitor/Master/Dim",
            if dim { 1.0 } else { 0.0 },
            logger,
        );
        Self::send_osc_float_dual(
            socket,
            target_hardware,
            target_web,
            "/Monitor/Master/Cut",
            if cut { 1.0 } else { 0.0 },
            logger,
        );
        for (ch, st) in channel_states {
            Self::send_osc_float_dual(
                socket,
                target_hardware,
                target_web,
                &format!("/Monitor/Channel/{}", ch),
                *st as f32,
                logger,
            );
        }
    }
}

impl Default for OscManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OscManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
