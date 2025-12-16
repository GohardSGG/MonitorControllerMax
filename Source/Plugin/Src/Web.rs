//! Web 控制器管理器
//!
//! 提供基于 WebSocket 的手机/平板遥控功能
//!
//! 设计原则：Web 控制器 = 虚拟硬件控制器
//! - 通过 OSC UDP 与插件通信（和 MonitorOSCPlugin 硬件一样）
//! - 发送命令到插件 OSC 接收端口 (7445)
//! - 从插件 OSC 发送接收状态（动态分配端口）

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::net::UdpSocket;
use std::time::Duration;
use std::collections::HashMap;

use axum::{
    Router,
    routing::get,
    response::{Html, IntoResponse},
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::StatusCode,
};
use tokio::sync::broadcast;
use futures::stream::StreamExt;
use futures::SinkExt;
use rosc::{OscPacket, OscMessage, OscType, encoder};
use parking_lot::RwLock;

use crate::Web_Protocol::{WebCommand, WebState, WebSharedState, ChannelState};
use crate::Web_Assets::Assets;
use crate::Logger::InstanceLogger;

/// Web 服务器状态推送频率（Hz）
const STATE_PUSH_HZ: u64 = 20;  // 20Hz = 50ms

/// 最大并发 WebSocket 连接数
#[allow(dead_code)]
const MAX_CONNECTIONS: usize = 8;

// ============================================================================
// Web OSC Client - 真正的 OSC 通信
// ============================================================================

/// Web OSC 客户端（像硬件一样通过 OSC 与插件通信）
struct WebOscClient {
    /// 发送命令到插件的 socket
    send_socket: UdpSocket,
    /// 接收状态从插件的 socket
    recv_socket: UdpSocket,
    /// 插件 OSC 接收端口（发送目标）
    target: String,
}

impl WebOscClient {
    /// 创建 OSC 客户端
    /// plugin_recv_port: 插件的 OSC 接收端口（默认 7445）
    fn new(plugin_recv_port: u16) -> Option<Self> {
        // 发送 socket（随机端口，用于发送命令到插件）
        let send_socket = UdpSocket::bind("0.0.0.0:0").ok()?;

        // 接收 socket（系统自动分配端口，用于接收插件发送的状态）
        let recv_socket = UdpSocket::bind("127.0.0.1:0").ok()?;
        recv_socket.set_read_timeout(Some(Duration::from_millis(5))).ok()?;

        Some(Self {
            send_socket,
            recv_socket,
            target: format!("127.0.0.1:{}", plugin_recv_port),
        })
    }

    /// 获取接收端口（插件需要知道这个端口来发送状态）
    fn recv_port(&self) -> u16 {
        self.recv_socket.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// 发送 OSC 消息到插件
    fn send(&self, addr: &str, value: f32) {
        let msg = OscMessage {
            addr: addr.to_string(),
            args: vec![OscType::Float(value)],
        };
        if let Ok(bytes) = encoder::encode(&OscPacket::Message(msg)) {
            let _ = self.send_socket.send_to(&bytes, &self.target);
        }
    }

    /// 轮询接收 OSC 消息
    fn poll(&self) -> Vec<(String, f32)> {
        let mut results = Vec::new();
        let mut buf = [0u8; 1024];

        // 非阻塞读取所有待处理的消息
        while let Ok((size, _)) = self.recv_socket.recv_from(&mut buf) {
            if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                match packet {
                    OscPacket::Message(msg) => {
                        if let Some(OscType::Float(v)) = msg.args.first() {
                            results.push((msg.addr.clone(), *v));
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        for p in bundle.content {
                            if let OscPacket::Message(msg) = p {
                                if let Some(OscType::Float(v)) = msg.args.first() {
                                    results.push((msg.addr.clone(), *v));
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }
}

// ============================================================================
// Web State Cache - 从 OSC 接收构建的状态缓存
// ============================================================================

/// Web 本地状态缓存（从 OSC 接收更新）
struct WebStateCache {
    /// 通道状态: 通道名 -> 状态 (0=off, 1=mute, 2=solo)
    channels: HashMap<String, u8>,
    /// Solo 模式激活
    solo_active: bool,
    /// Mute 模式激活
    mute_active: bool,
    /// Master Volume
    master_volume: f32,
    /// Dim 状态
    dim: bool,
    /// Cut 状态
    cut: bool,
    /// Mono 状态
    mono: bool,
    /// Low Boost 状态
    low_boost: bool,
    /// High Boost 状态
    high_boost: bool,
    /// LFE +10dB 状态
    lfe_add_10db: bool,
}

impl WebStateCache {
    fn new() -> Self {
        Self {
            channels: HashMap::new(),
            solo_active: false,
            mute_active: false,
            master_volume: 0.5,
            dim: false,
            cut: false,
            mono: false,
            low_boost: false,
            high_boost: false,
            lfe_add_10db: false,
        }
    }

    /// 从 OSC 消息更新状态
    fn update_from_osc(&mut self, addr: &str, value: f32) {
        match addr {
            "/Monitor/Master/Volume" => self.master_volume = value,
            "/Monitor/Master/Dim" => self.dim = value > 0.5,
            "/Monitor/Master/Cut" => self.cut = value > 0.5,
            "/Monitor/Mode/Solo" => self.solo_active = value > 0.5,
            "/Monitor/Mode/Mute" => self.mute_active = value > 0.5,
            "/Monitor/Master/Effect/Mono" => self.mono = value > 0.5,
            "/Monitor/Master/Effect/Low_Boost" => self.low_boost = value > 0.5,
            "/Monitor/Master/Effect/High_Boost" => self.high_boost = value > 0.5,
            "/Monitor/LFE/Add_10dB" => self.lfe_add_10db = value > 0.5,
            _ if addr.starts_with("/Monitor/Channel/") => {
                let name = &addr[17..];  // 跳过 "/Monitor/Channel/"
                self.channels.insert(name.to_string(), value as u8);
            }
            _ => {}
        }
    }

    /// 构建 WebState 用于推送到客户端
    fn to_web_state(&self) -> WebState {
        // 从 channels 构建 solo_mask 和 mute_mask
        let mut solo_mask: u32 = 0;
        let mut mute_mask: u32 = 0;

        let channels: Vec<ChannelState> = self.channels.iter().enumerate().map(|(idx, (name, &state))| {
            if state == 2 {
                solo_mask |= 1 << idx;
            } else if state == 1 {
                mute_mask |= 1 << idx;
            }

            ChannelState {
                name: name.clone(),
                index: idx,
                state,
                is_sub: name.starts_with("SUB"),
            }
        }).collect();

        // 根据 solo/mute 状态确定 primary
        let primary = if self.solo_active { 1 } else if self.mute_active { 2 } else { 0 };

        WebState {
            primary,
            compare: 0,
            solo_mask,
            mute_mask,
            master_volume: self.master_volume,
            dim: self.dim,
            cut: self.cut,
            mono: self.mono,
            low_boost: self.low_boost,
            high_boost: self.high_boost,
            lfe_add_10db: self.lfe_add_10db,
            channels,
        }
    }
}

// ============================================================================
// App State for Axum
// ============================================================================

#[derive(Clone)]
struct AppState {
    /// OSC 客户端（共享，用于发送命令）
    osc_client: Arc<WebOscClient>,
    /// 状态缓存（从 OSC 接收更新）
    state_cache: Arc<RwLock<WebStateCache>>,
    /// 状态广播通道
    broadcast_tx: broadcast::Sender<String>,
    /// 日志器
    logger: Arc<InstanceLogger>,
}

// ============================================================================
// WebManager
// ============================================================================

pub struct WebManager {
    /// 共享状态
    pub state: Arc<WebSharedState>,
    /// 运行标志
    is_running: Arc<AtomicBool>,
    /// 服务器线程句柄
    thread_handle: Option<JoinHandle<()>>,
    /// 实例日志器
    logger: Option<Arc<InstanceLogger>>,
    /// 插件 OSC 接收端口（用于发送命令）
    plugin_osc_recv_port: u16,
}

impl WebManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(WebSharedState::new()),
            is_running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            logger: None,
            plugin_osc_recv_port: 7445,  // 默认值
        }
    }

    /// 初始化并启动 Web 服务器
    /// plugin_osc_recv_port: 插件的 OSC 接收端口（默认 7445）
    pub fn init(
        &mut self,
        logger: Arc<InstanceLogger>,
        plugin_osc_recv_port: u16,
    ) {
        // 检查是否已在运行
        let thread_alive = self.thread_handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false);
        if self.is_running.load(Ordering::Acquire) && thread_alive {
            logger.info("web", "[Web] Server already running, skipping init");
            return;
        }

        // 清理已死的线程
        if self.is_running.load(Ordering::Acquire) && !thread_alive {
            logger.info("web", "[Web] Previous thread exited, cleaning up...");
            self.is_running.store(false, Ordering::Release);
            self.state.osc_recv_port.store(0, Ordering::Release);
            if let Some(h) = self.thread_handle.take() { let _ = h.join(); }
        }

        // 存储配置
        self.logger = Some(Arc::clone(&logger));
        self.plugin_osc_recv_port = plugin_osc_recv_port;

        // 设置运行标志
        self.is_running.store(true, Ordering::Release);
        self.state.is_running.store(true, Ordering::Release);

        // 获取本机 IP
        if let Ok(ip) = local_ip_address::local_ip() {
            *self.state.local_ip.write() = ip.to_string();
        }

        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&logger);
        let osc_recv_port = plugin_osc_recv_port;

        self.thread_handle = Some(thread::spawn(move || {
            // 创建单线程 Tokio Runtime
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    logger_clone.error("web", &format!("[Web] Failed to create Tokio runtime: {}", e));
                    is_running.store(false, Ordering::Release);
                    state.is_running.store(false, Ordering::Release);
                    return;
                }
            };

            rt.block_on(async move {
                Self::run_server(
                    is_running,
                    state,
                    logger_clone,
                    osc_recv_port,
                ).await;
            });
        }));
    }

    /// 运行 Axum 服务器
    async fn run_server(
        is_running: Arc<AtomicBool>,
        state: Arc<WebSharedState>,
        logger: Arc<InstanceLogger>,
        plugin_osc_recv_port: u16,
    ) {
        // 创建 OSC 客户端
        let osc_client = match WebOscClient::new(plugin_osc_recv_port) {
            Some(c) => Arc::new(c),
            None => {
                logger.error("web", "[Web] Failed to create OSC client");
                is_running.store(false, Ordering::Release);
                state.is_running.store(false, Ordering::Release);
                return;
            }
        };

        // 存储 OSC 接收端口（让插件 Osc.rs 发送线程知道）
        let web_osc_port = osc_client.recv_port();
        state.osc_recv_port.store(web_osc_port, Ordering::Release);
        logger.important("web", &format!("[Web] OSC client ready: send to :{}, recv on :{}",
            plugin_osc_recv_port, web_osc_port));

        // 绑定 HTTP 端口（系统自动分配）
        let listener = match tokio::net::TcpListener::bind("0.0.0.0:0").await {
            Ok(l) => l,
            Err(e) => {
                logger.error("web", &format!("[Web] Failed to bind HTTP: {}", e));
                is_running.store(false, Ordering::Release);
                state.is_running.store(false, Ordering::Release);
                state.osc_recv_port.store(0, Ordering::Release);
                return;
            }
        };

        let addr = listener.local_addr().unwrap();
        state.port.store(addr.port(), Ordering::Release);

        let local_ip = state.local_ip.read().clone();
        logger.important("web", &format!("[Web] HTTP server at http://{}:{}", local_ip, addr.port()));

        // 创建状态缓存
        let state_cache = Arc::new(RwLock::new(WebStateCache::new()));

        // 创建 broadcast channel 用于状态推送
        let (broadcast_tx, _) = broadcast::channel::<String>(16);

        // 创建 App State
        let app_state = AppState {
            osc_client: Arc::clone(&osc_client),
            state_cache: Arc::clone(&state_cache),
            broadcast_tx: broadcast_tx.clone(),
            logger: Arc::clone(&logger),
        };

        // 构建 Router
        let app = Router::new()
            .route("/", get(serve_index))
            .route("/style.css", get(serve_css))
            .route("/app.js", get(serve_js))
            .route("/ws", get(ws_handler))
            .with_state(app_state);

        // 启动 OSC 接收和状态推送任务
        let push_is_running = Arc::clone(&is_running);
        let push_osc_client = Arc::clone(&osc_client);
        let push_state_cache = Arc::clone(&state_cache);
        let push_logger = Arc::clone(&logger);
        tokio::spawn(async move {
            Self::osc_recv_and_push_task(
                push_is_running,
                push_osc_client,
                push_state_cache,
                broadcast_tx,
                push_logger,
            ).await;
        });

        // 运行 HTTP 服务器
        logger.info("web", "[Web] Axum server starting...");

        // 创建 shutdown signal
        let shutdown_is_running = Arc::clone(&is_running);
        let shutdown_signal = async move {
            while shutdown_is_running.load(Ordering::Acquire) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        };

        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
        {
            logger.error("web", &format!("[Web] Server error: {}", e));
        }

        logger.info("web", "[Web] Server stopped");
        state.is_running.store(false, Ordering::Release);
        state.port.store(0, Ordering::Release);
        state.osc_recv_port.store(0, Ordering::Release);
    }

    /// OSC 接收和状态推送任务
    async fn osc_recv_and_push_task(
        is_running: Arc<AtomicBool>,
        osc_client: Arc<WebOscClient>,
        state_cache: Arc<RwLock<WebStateCache>>,
        broadcast_tx: broadcast::Sender<String>,
        _logger: Arc<InstanceLogger>,
    ) {
        let interval = std::time::Duration::from_millis(1000 / STATE_PUSH_HZ);

        while is_running.load(Ordering::Acquire) {
            // 接收所有待处理的 OSC 消息
            let messages = osc_client.poll();

            // 更新状态缓存
            if !messages.is_empty() {
                let mut cache = state_cache.write();
                for (addr, value) in messages {
                    cache.update_from_osc(&addr, value);
                }
            }

            // 构建并广播状态
            let web_state = state_cache.read().to_web_state();
            if let Ok(json) = serde_json::to_string(&web_state) {
                let _ = broadcast_tx.send(json);
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// 关闭 Web 服务器
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Acquire) {
            return;
        }

        if let Some(ref logger) = self.logger {
            logger.info("web", "[Web] Shutting down server...");
        }

        // 清除 OSC 端口（让插件停止发送到此端口）
        self.state.osc_recv_port.store(0, Ordering::Release);

        self.is_running.store(false, Ordering::Release);
        self.state.is_running.store(false, Ordering::Release);

        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }

        self.state.port.store(0, Ordering::Release);
    }

    /// 检查服务器是否运行中
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Acquire)
    }

    /// 获取服务器地址
    #[allow(dead_code)]
    pub fn get_address(&self) -> Option<String> {
        self.state.get_address()
    }
}

impl Default for WebManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WebManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ============================================================================
// Axum Handlers
// ============================================================================

/// 服务 index.html
async fn serve_index() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => Html(String::from_utf8_lossy(content.data.as_ref()).to_string()).into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// 服务 style.css
async fn serve_css() -> impl IntoResponse {
    match Assets::get("style.css") {
        Some(content) => (
            [(axum::http::header::CONTENT_TYPE, "text/css")],
            String::from_utf8_lossy(content.data.as_ref()).to_string()
        ).into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// 服务 app.js
async fn serve_js() -> impl IntoResponse {
    match Assets::get("app.js") {
        Some(content) => (
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            String::from_utf8_lossy(content.data.as_ref()).to_string()
        ).into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// WebSocket 处理器
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// 处理 WebSocket 连接
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 订阅状态广播
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    state.logger.info("web", "[Web] WebSocket client connected");

    // 发送任务：广播状态到客户端
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 接收任务：通过 OSC 发送命令到插件
    let osc_client = state.osc_client.clone();
    let logger = state.logger.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<WebCommand>(&text) {
                    Ok(cmd) => {
                        // 通过 OSC 发送命令（像硬件一样）
                        handle_command_via_osc(cmd, &osc_client, &logger);
                    }
                    Err(e) => {
                        logger.warn("web", &format!("[Web] Invalid command: {}", e));
                    }
                }
            }
        }
    });

    // 等待任一任务结束
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    state.logger.info("web", "[Web] WebSocket client disconnected");
}

/// 通过 OSC 发送命令（像硬件控制器一样）
fn handle_command_via_osc(cmd: WebCommand, osc: &WebOscClient, logger: &InstanceLogger) {
    match cmd {
        // === 模式切换 ===
        WebCommand::ToggleSolo => {
            logger.info("web", "[Web->OSC] /Monitor/Mode/Solo 1.0");
            osc.send("/Monitor/Mode/Solo", 1.0);
        }
        WebCommand::ToggleMute => {
            logger.info("web", "[Web->OSC] /Monitor/Mode/Mute 1.0");
            osc.send("/Monitor/Mode/Mute", 1.0);
        }

        // === 通道操作 ===
        WebCommand::ChannelClick { channel } => {
            let addr = format!("/Monitor/Channel/{}", channel);
            logger.info("web", &format!("[Web->OSC] {} 1.0", addr));
            osc.send(&addr, 1.0);
        }

        // === 主控制 ===
        WebCommand::SetVolume { value } => {
            let clamped = value.clamp(0.0, 1.0);
            logger.info("web", &format!("[Web->OSC] /Monitor/Master/Volume {:.3}", clamped));
            osc.send("/Monitor/Master/Volume", clamped);
        }
        WebCommand::ToggleDim => {
            logger.info("web", "[Web->OSC] /Monitor/Master/Dim 1.0");
            osc.send("/Monitor/Master/Dim", 1.0);
        }
        WebCommand::SetDim { on } => {
            osc.send("/Monitor/Master/Dim", if on { 2.0 } else { 0.0 });
        }
        WebCommand::ToggleCut => {
            logger.info("web", "[Web->OSC] /Monitor/Master/Cut 1.0");
            osc.send("/Monitor/Master/Cut", 1.0);
        }
        WebCommand::SetCut { on } => {
            osc.send("/Monitor/Master/Cut", if on { 2.0 } else { 0.0 });
        }

        // === 效果器 ===
        WebCommand::ToggleMono => {
            logger.info("web", "[Web->OSC] /Monitor/Master/Effect/Mono 1.0");
            osc.send("/Monitor/Master/Effect/Mono", 1.0);
        }
        WebCommand::ToggleLowBoost => {
            logger.info("web", "[Web->OSC] /Monitor/Master/Effect/Low_Boost 1.0");
            osc.send("/Monitor/Master/Effect/Low_Boost", 1.0);
        }
        WebCommand::ToggleHighBoost => {
            logger.info("web", "[Web->OSC] /Monitor/Master/Effect/High_Boost 1.0");
            osc.send("/Monitor/Master/Effect/High_Boost", 1.0);
        }
        WebCommand::ToggleLfeAdd10dB => {
            logger.info("web", "[Web->OSC] /Monitor/LFE/Add_10dB 1.0");
            osc.send("/Monitor/LFE/Add_10dB", 1.0);
        }

        // === 通道组编码器（Group Dial）===
        WebCommand::GroupDial { group, direction } => {
            let channels = get_group_channels(&group);
            if channels.is_empty() {
                logger.warn("web", &format!("[Web] Unknown group: {}", group));
                return;
            }

            if direction > 0 {
                // 右转 = 有声音 → 预激活 Solo，然后添加通道
                logger.info("web", &format!("[Web->OSC] GroupDial {} 右转 (Solo)", group));
                osc.send("/Monitor/Mode/Solo", 2.0);  // 2.0 = 预激活模式
                for ch in channels {
                    osc.send(&format!("/Monitor/Channel/{}", ch), 10.0);  // 10.0 = 有声音
                }
            } else {
                // 左转 = 没声音 → 预激活 Mute，然后添加通道
                logger.info("web", &format!("[Web->OSC] GroupDial {} 左转 (Mute)", group));
                osc.send("/Monitor/Mode/Mute", 2.0);  // 2.0 = 预激活模式
                for ch in channels {
                    osc.send(&format!("/Monitor/Channel/{}", ch), 11.0);  // 11.0 = 没声音（增量）
                }
            }
        }

        WebCommand::GroupClick { group } => {
            let channels = get_group_channels(&group);
            if channels.is_empty() {
                logger.warn("web", &format!("[Web] Unknown group: {}", group));
                return;
            }

            // 按下 = 切换组内所有通道的状态
            logger.info("web", &format!("[Web->OSC] GroupClick {} (toggle)", group));
            for ch in channels {
                osc.send(&format!("/Monitor/Channel/{}", ch), 1.0);  // 1.0 = toggle
            }
        }
    }
}

/// 获取通道组对应的通道列表
fn get_group_channels(group: &str) -> Vec<&'static str> {
    match group.to_uppercase().as_str() {
        "FRONT" => vec!["L", "R"],
        "CENTER" => vec!["C"],
        "SUB" => vec!["SUB1", "SUB2"],
        "SURROUND" => vec!["LS", "RS"],
        "REAR" => vec!["LRS", "RRS"],
        "TOP" => vec!["TFL", "TFR", "TRL", "TRR"],
        "BOTTOM" => vec!["BFL", "BFR"],
        _ => vec![],
    }
}
