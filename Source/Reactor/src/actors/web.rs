//! Web 控制器管理器
//!
//! 提供基于 WebSocket 的手机/平板遥控功能
//!
//! 设计原则：Web 控制器 = 虚拟硬件控制器
//! - 移除 UDP 回环通信，直接通过内存操作 InteractionManager 和 Params
//! - 保持 20Hz 的状态推送频率

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures::stream::StreamExt;
use futures::SinkExt;
use tokio::sync::broadcast;

use crate::web_assets::Assets;
use mcm_core::interaction::InteractionManager;
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::MonitorParams;
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::web_structs::{ChannelState, WebCommand, WebSharedState, WebState};

/// Web 服务器状态推送频率（Hz）
const STATE_PUSH_HZ: u64 = 20;

// WebSharedState definitions removed, using mcm_protocol::web_structs::WebSharedState

// ============================================================================
// App State for Axum
// ============================================================================

#[derive(Clone)]
struct AppState {
    /// 交互管理器（用于直接控制）
    interaction: Arc<InteractionManager>,
    /// 参数管理（用于音量控制）
    params: Arc<MonitorParams>,
    /// OSC 共享状态（复用 pending 机制传递参数变更）
    osc_state: Arc<OscSharedState>,
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
}

impl WebManager {
    pub fn new(state: Arc<WebSharedState>) -> Self {
        Self {
            state,
            is_running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            logger: None,
        }
    }

    pub fn get_state(&self) -> Arc<WebSharedState> {
        Arc::clone(&self.state)
    }

    /// 初始化并启动 Web 服务器
    pub fn init(
        &mut self,
        logger: Arc<InstanceLogger>,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        osc_state: Arc<OscSharedState>,
    ) {
        // 检查是否已在运行
        let thread_alive = self
            .thread_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false);
        if self.is_running.load(Ordering::Acquire) && thread_alive {
            logger.info("web", "[Web] Server already running, skipping init");
            return;
        }

        // 清理已死的线程
        if self.is_running.load(Ordering::Acquire) && !thread_alive {
            logger.info("web", "[Web] Previous thread exited, cleaning up...");
            self.is_running.store(false, Ordering::Release);
            if let Some(h) = self.thread_handle.take() {
                let _ = h.join();
            }
        }

        // 存储配置
        self.logger = Some(Arc::clone(&logger));

        // 设置运行标志
        self.is_running.store(true, Ordering::Release);
        self.state.is_running.store(true, Ordering::Release);

        let state_local = Arc::clone(&self.state); // Clone for ip update

        // 获取本机 IP
        if let Ok(ip) = local_ip_address::local_ip() {
            *state_local.local_ip.write() = ip.to_string();
        }

        let is_running = Arc::clone(&self.is_running);
        let state = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&logger);

        // Spawn server thread
        self.thread_handle = Some(thread::spawn(move || {
            // 创建单线程 Tokio Runtime
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    logger_clone.error(
                        "web",
                        &format!("[Web] Failed to create Tokio runtime: {}", e),
                    );
                    is_running.store(false, Ordering::Release);
                    state.is_running.store(false, Ordering::Release);
                    return;
                }
            };

            rt.block_on(async move {
                Self::run_server(is_running, state, logger_clone, interaction, params, osc_state).await;
            });
        }));
    }

    /// 运行 Axum 服务器
    async fn run_server(
        is_running: Arc<AtomicBool>,
        state: Arc<WebSharedState>,
        logger: Arc<InstanceLogger>,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        osc_state: Arc<OscSharedState>,
    ) {
        // 绑定 HTTP 端口（系统自动分配）
        let listener = match tokio::net::TcpListener::bind("0.0.0.0:0").await {
            Ok(l) => l,
            Err(e) => {
                logger.error("web", &format!("[Web] Failed to bind HTTP: {}", e));
                is_running.store(false, Ordering::Release);
                state.is_running.store(false, Ordering::Release);
                return;
            }
        };

        let addr = listener.local_addr().unwrap();
        state.port.store(addr.port(), Ordering::Release);

        let local_ip = state.local_ip.read().clone();
        logger.important(
            "web",
            &format!("[Web] HTTP server at http://{}:{}", local_ip, addr.port()),
        );

        // 创建 broadcast channel 用于状态推送
        let (broadcast_tx, _) = broadcast::channel::<String>(16);

        // 创建 App State
        let app_state = AppState {
            interaction: Arc::clone(&interaction),
            params: Arc::clone(&params),
            osc_state: Arc::clone(&osc_state),
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

        // 启动状态推送任务
        let push_is_running = Arc::clone(&is_running);
        let push_interaction = Arc::clone(&interaction);
        let push_params = Arc::clone(&params);
        let push_osc_state = Arc::clone(&osc_state);
        tokio::spawn(async move {
            Self::state_push_task(
                push_is_running,
                push_interaction,
                push_params,
                push_osc_state,
                broadcast_tx,
            )
            .await;
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
    }

    /// 状态推送任务（20Hz 广播状态到所有 WebSocket 客户端）
    async fn state_push_task(
        is_running: Arc<AtomicBool>,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        osc_state: Arc<OscSharedState>,
        broadcast_tx: broadcast::Sender<String>,
    ) {
        let interval = std::time::Duration::from_millis(1000 / STATE_PUSH_HZ);

        while is_running.load(Ordering::Acquire) {
            // 从 InteractionManager 读取状态
            let _primary = interaction.get_primary();
            let _compare = interaction.get_compare();
            let snapshot = interaction.get_snapshot();

            // 从 OscSharedState 读取音量/效果器状态
            let master_volume = osc_state.get_master_volume();

            // 构建 WebState
            let web_state = WebState {
                primary: snapshot.primary,
                compare: snapshot.compare,
                solo_mask: snapshot.solo_mask,
                mute_mask: snapshot.mute_mask,
                master_volume,
                dim: params.dim.value(),
                cut: params.cut.value(),
                mono: params.mono.value(),
                low_boost: params.low_boost.value(),
                high_boost: params.high_boost.value(),
                lfe_add_10db: params.lfe_add_10db.value(),
                channels: Self::build_channel_states(&interaction, &snapshot),
            };

            // 序列化并广播
            if let Ok(json) = serde_json::to_string(&web_state) {
                // send 失败只意味着没有订阅者，忽略
                let _ = broadcast_tx.send(json);
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// 构建通道状态列表
    fn build_channel_states(
        interaction: &InteractionManager,
        snapshot: &mcm_core::interaction::RenderSnapshot,
    ) -> Vec<ChannelState> {
        use mcm_core::interaction::ChannelMarker;

        // 获取当前布局的通道列表
        let _solo_set = interaction.get_solo_set();
        let _mute_set = interaction.get_mute_set();

        // 使用 get_channel_display 获取每个通道的显示状态
        let mut channels = Vec::new();

        // 标准通道顺序
        let all_channels = [
            "L", "R", "C", "LFE", "LSS", "RSS", "LRS", "RRS",
            "LTF", "RTF", "LTB", "RTB",
            "SUB_F", "SUB_B", "SUB_L", "SUB_R",
            "LBF", "RBF", "LBB", "RBB",
        ];

        for (i, name) in all_channels.iter().enumerate() {
            let display = interaction.get_channel_display(name);
            let state = match display.marker {
                Some(ChannelMarker::Solo) => 2, // 绿色
                Some(ChannelMarker::Mute) => 1, // 红色
                None => 0,                       // 无标记
            };
            channels.push(ChannelState {
                name: name.to_string(),
                index: i,
                state,
                is_sub: name.starts_with("SUB"),
            });
        }

        channels
    }

    /// 关闭 Web 服务器
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Acquire) {
            return;
        }

        if let Some(ref logger) = self.logger {
            logger.info("web", "[Web] Shutting down server...");
        }

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
        Some(content) => {
            Html(String::from_utf8_lossy(content.data.as_ref()).to_string()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// 服务 style.css
async fn serve_css() -> impl IntoResponse {
    match Assets::get("style.css") {
        Some(content) => (
            [(axum::http::header::CONTENT_TYPE, "text/css")],
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// 服务 app.js
async fn serve_js() -> impl IntoResponse {
    match Assets::get("app.js") {
        Some(content) => (
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// WebSocket 处理器
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
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

    // 接收任务：直接调用 InteractionManager
    let interaction = state.interaction.clone();
    let params = state.params.clone();
    let osc_state = state.osc_state.clone();
    let logger = state.logger.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<WebCommand>(&text) {
                    Ok(cmd) => {
                        handle_command_direct(cmd, &interaction, &params, &osc_state, &logger);
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

    state
        .logger
        .info("web", "[Web] WebSocket client disconnected");
}

/// 直接执行命令（复用 OscSharedState pending 机制传递参数变更）
fn handle_command_direct(
    cmd: WebCommand,
    interaction: &InteractionManager,
    params: &MonitorParams,
    osc_state: &OscSharedState,
    logger: &InstanceLogger,
) {
    // Slave 模式下拒绝所有 Web 命令（Slave 由 Master 完全控制）
    if params.role.value() == mcm_core::params::PluginRole::Slave {
        logger.info("web", "[Web] Command rejected (Slave mode)");
        return;
    }

    match cmd {
        // === 模式切换 ===
        WebCommand::ToggleSolo => {
            logger.info("web", "[Web] Toggle Solo");
            interaction.on_solo_button_click();
        }
        WebCommand::ToggleMute => {
            logger.info("web", "[Web] Toggle Mute");
            interaction.on_mute_button_click();
        }

        // === 通道操作 ===
        WebCommand::ChannelClick { channel } => {
            logger.info("web", &format!("[Web] Channel Click: {}", channel));
            interaction.on_channel_click(&channel);
        }

        // === 主控制（通过 OscSharedState pending 机制） ===
        WebCommand::SetVolume { value } => {
            let clamped = value.clamp(0.0, 1.0);
            logger.info("web", &format!("[Web] SetVolume {:.3}", clamped));
            osc_state.set_master_volume(clamped);
        }
        WebCommand::ToggleDim => {
            let current = osc_state.get_dim();
            logger.info("web", &format!("[Web] ToggleDim -> {}", !current));
            osc_state.set_dim(!current);
        }
        WebCommand::SetDim { on } => {
            logger.info("web", &format!("[Web] SetDim {}", on));
            osc_state.set_dim(on);
        }
        WebCommand::ToggleCut => {
            let current = osc_state.get_cut();
            logger.info("web", &format!("[Web] ToggleCut -> {}", !current));
            osc_state.set_cut(!current);
        }
        WebCommand::SetCut { on } => {
            logger.info("web", &format!("[Web] SetCut {}", on));
            osc_state.set_cut(on);
        }

        // === 效果器（通过 OscSharedState pending 机制） ===
        WebCommand::ToggleMono => {
            let current = osc_state.get_mono();
            logger.info("web", &format!("[Web] ToggleMono -> {}", !current));
            osc_state.set_mono(!current);
        }
        WebCommand::ToggleLowBoost => {
            let current = osc_state.get_low_boost();
            logger.info("web", &format!("[Web] ToggleLowBoost -> {}", !current));
            osc_state.set_low_boost(!current);
        }
        WebCommand::ToggleHighBoost => {
            let current = osc_state.get_high_boost();
            logger.info("web", &format!("[Web] ToggleHighBoost -> {}", !current));
            osc_state.set_high_boost(!current);
        }
        WebCommand::ToggleLfeAdd10dB => {
            let current = osc_state.get_lfe_add_10db();
            logger.info("web", &format!("[Web] ToggleLfeAdd10dB -> {}", !current));
            osc_state.set_lfe_add_10db(!current);
        }

        // === 通道组编码器 (Group Dial) ===
        WebCommand::GroupDial { group, direction } => {
            let channels = get_group_channels(&group);
            if channels.is_empty() {
                return;
            }
            // FIXME: 完整实现需要 InteractionManager 支持 Group Dial
            logger.info(
                "web",
                &format!(
                    "[Web] Group Dial {} ({}): Not fully implemented",
                    group, direction
                ),
            );
        }

        WebCommand::GroupClick { group } => {
            let channels = get_group_channels(&group);
            logger.info("web", &format!("[Web] Group Click {}", group));
            for ch in channels {
                interaction.on_channel_click(&ch);
            }
        }
    }
}

/// 获取通道组对应的通道列表（与 STANDARD_CHANNEL_ORDER 一致）
fn get_group_channels(group: &str) -> Vec<String> {
    match group.to_uppercase().as_str() {
        "FRONT" => vec!["L", "R"],
        "CENTER" => vec!["C"],
        "LFE" => vec!["LFE"],
        "SUB" => vec!["SUB_F", "SUB_B", "SUB_L", "SUB_R"],
        "SURROUND" => vec!["LSS", "RSS"],
        "REAR" => vec!["LRS", "RRS"],
        "TOP" => vec!["LTF", "RTF", "LTB", "RTB"],
        "BOTTOM" => vec!["LBF", "RBF", "LBB", "RBB"],
        _ => vec![],
    }
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}
