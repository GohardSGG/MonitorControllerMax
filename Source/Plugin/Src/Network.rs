use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use zeromq::{Socket, PubSocket, SubSocket, SocketSend, SocketRecv};
use bincode;
// P2 优化：使用 Builder 创建单线程 Runtime，不再需要直接 use Runtime

use crate::Network_Protocol::NetworkInteractionState;
use crate::Interaction::InteractionManager;
use crate::Logger::InstanceLogger;
use crate::Params::{MonitorParams, PluginRole};

pub struct NetworkManager {
    // 运行状态标志（线程退出控制）
    is_running: Arc<AtomicBool>,

    // Slave 连接状态（供 UI 显示）
    pub is_connected: Arc<AtomicBool>,

    // 最后接收的时间戳（用于防止乱序包）
    last_timestamp: Arc<AtomicU64>,

    // 线程句柄
    thread_handle: Option<JoinHandle<()>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            is_connected: Arc::new(AtomicBool::new(false)),
            last_timestamp: Arc::new(AtomicU64::new(0)),
            thread_handle: None,
        }
    }

    /// 获取 Slave 连接状态（供 Editor 使用）
    pub fn get_connection_status(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_connected)
    }

    /// 初始化 Master 模式
    /// 网络线程直接从 InteractionManager 读取状态，定期发送
    /// params 用于读取 master_gain/dim/cut
    /// D: 检测线程存活状态，死线程则允许重新初始化
    pub fn init_master(&mut self, port: u16, interaction: Arc<InteractionManager>, params: Arc<MonitorParams>, logger: Arc<InstanceLogger>) {
        // D: 检查线程是否实际在运行（使用 is_finished() 检测）
        let thread_alive = self.thread_handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false);

        // 如果线程真正在运行，先关闭
        if self.is_running.load(Ordering::Acquire) && thread_alive {
            self.shutdown();
        }

        // D: 线程已死但标志未清（C5 线程主动退出导致），清理状态
        if self.is_running.load(Ordering::Acquire) && !thread_alive {
            logger.info("network", "[Network Master] Previous thread exited, cleaning up for re-init...");
            self.is_running.store(false, Ordering::Release);
            if let Some(h) = self.thread_handle.take() { let _ = h.join(); }
        }

        self.is_running.store(true, Ordering::Release);

        let is_running = Arc::clone(&self.is_running);

        self.thread_handle = Some(thread::spawn(move || {
            // P2 优化：使用单线程 Runtime 减少开销
            // Master 网络线程只需要单线程，无需多线程 Runtime 的额外开销
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build() {
                Ok(rt) => rt,
                Err(e) => {
                    logger.error("network", &format!("Failed to create Tokio runtime: {}", e));
                    is_running.store(false, Ordering::Release);  // 清理标志，允许重新初始化
                    return;
                }
            };

            rt.block_on(async move {
                let mut socket = PubSocket::new();
                let endpoint = format!("tcp://0.0.0.0:{}", port);

                if let Err(e) = socket.bind(&endpoint).await {
                    logger.warn("network", &format!("ZMQ port {} unavailable: {}", port, e));
                    return;
                }
                logger.important("network", &format!("ZMQ Publisher bound to {}", endpoint));

                let mut send_count: u64 = 0;
                let mut send_error_count: u64 = 0;

                while is_running.load(Ordering::Acquire) {
                    // Role != Master 时暂停（不退出）
                    if params.role.value() != PluginRole::Master {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }

                    // 从 params 读取 master_gain/dim/cut/layout/sub_layout
                    let master_gain = params.master_gain.value();
                    let dim = params.dim.value();
                    let cut = params.cut.value();
                    let layout = params.layout.value();
                    let sub_layout = params.sub_layout.value();

                    // 从 InteractionManager 读取当前状态（包含 params 值）
                    let state = interaction.to_network_state(master_gain, dim, cut, layout, sub_layout);

                    // 序列化并发送
                    if let Ok(bytes) = bincode::serialize(&state) {
                        match socket.send(bytes.into()).await {
                            Ok(_) => {
                                send_count += 1;

                                // 每 500 次记录一次日志（约 10 秒一次，20ms 间隔）
                                if send_count % 500 == 0 {
                                    logger.info("network", &format!(
                                        "ZMQ Master sent {} packets, primary={}, compare={}",
                                        send_count, state.primary, state.compare
                                    ));
                                }
                            }
                            Err(e) => {
                                send_error_count += 1;
                                // 只在前几次和每 100 次记录错误，避免日志爆炸
                                if send_error_count <= 3 || send_error_count % 100 == 0 {
                                    logger.warn("network", &format!(
                                        "ZMQ Send Error #{}: {}",
                                        send_error_count, e
                                    ));
                                }
                            }
                        }
                    }

                    // 发送间隔 20ms (50Hz)
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }

                logger.info("network", "ZMQ Publisher thread stopped");
            });
        }));
    }

    /// 初始化 Slave 模式
    /// 网络线程收到数据后直接更新 InteractionManager
    /// 支持指数退避重连机制 + Role 参数检测 + 心跳超时
    /// D: 检测线程存活状态，死线程则允许重新初始化
    pub fn init_slave(&mut self, master_ip: &str, port: u16, interaction: Arc<InteractionManager>, params: Arc<MonitorParams>, logger: Arc<InstanceLogger>, app_config: crate::Config_File::AppConfig) {
        // D: 检查线程是否实际在运行（使用 is_finished() 检测）
        let thread_alive = self.thread_handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false);

        // 如果线程真正在运行，先关闭
        if self.is_running.load(Ordering::Acquire) && thread_alive {
            self.shutdown();
        }

        // D: 线程已死但标志未清（C5 线程主动退出导致），清理状态
        if self.is_running.load(Ordering::Acquire) && !thread_alive {
            logger.info("network", "[Network Slave] Previous thread exited, cleaning up for re-init...");
            self.is_running.store(false, Ordering::Release);
            if let Some(h) = self.thread_handle.take() { let _ = h.join(); }
        }

        let endpoint = format!("tcp://{}:{}", master_ip, port);
        self.is_running.store(true, Ordering::Release);
        self.is_connected.store(false, Ordering::Relaxed);
        self.last_timestamp.store(0, Ordering::Relaxed);

        let is_running = Arc::clone(&self.is_running);
        let is_connected = Arc::clone(&self.is_connected);
        let last_timestamp = Arc::clone(&self.last_timestamp);

        self.thread_handle = Some(thread::spawn(move || {
            // P2 优化：使用单线程 Runtime 减少开销
            // Slave 网络线程只需要单线程，无需多线程 Runtime 的额外开销
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build() {
                Ok(rt) => rt,
                Err(e) => {
                    logger.error("network", &format!("Failed to create Tokio runtime: {}", e));
                    is_running.store(false, Ordering::Release);  // 清理标志，允许重新初始化
                    return;
                }
            };

            rt.block_on(async move {
                let mut reconnect_delay_ms: u64 = 500;  // 初始延迟 500ms
                const MAX_RECONNECT_DELAY_MS: u64 = 5000;  // 最大延迟 5 秒
                const HEARTBEAT_TIMEOUT_MS: u64 = 2000;  // C6: 心跳超时 2 秒

                // 外层重连循环
                'reconnect_loop: while is_running.load(Ordering::Acquire) {
                    // Role != Slave 时暂停（不退出）
                    if params.role.value() != PluginRole::Slave {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue 'reconnect_loop;
                    }

                    let mut socket = SubSocket::new();

                    // 连接尝试
                    match socket.connect(&endpoint).await {
                        Ok(_) => {
                            reconnect_delay_ms = 500;  // 成功后重置延迟
                        }
                        Err(e) => {
                            logger.warn("network", &format!(
                                "ZMQ Connect failed, retry in {}ms: {}",
                                reconnect_delay_ms, e
                            ));
                            tokio::time::sleep(std::time::Duration::from_millis(reconnect_delay_ms)).await;
                            reconnect_delay_ms = (reconnect_delay_ms * 2).min(MAX_RECONNECT_DELAY_MS);
                            continue 'reconnect_loop;
                        }
                    }

                    if let Err(e) = socket.subscribe("").await {
                        logger.error("network", &format!("ZMQ Subscribe Error: {}", e));
                        tokio::time::sleep(std::time::Duration::from_millis(reconnect_delay_ms)).await;
                        reconnect_delay_ms = (reconnect_delay_ms * 2).min(MAX_RECONNECT_DELAY_MS);
                        continue 'reconnect_loop;
                    }

                    logger.important("network", &format!("ZMQ Subscriber connected to {}", endpoint));

                    let mut recv_count: u64 = 0;
                    let mut out_of_order_count: u64 = 0;
                    let mut consecutive_errors: u64 = 0;
                    let mut last_packet_time = std::time::Instant::now();  // C6: 心跳计时

                    // 内层接收循环
                    while is_running.load(Ordering::Acquire) {
                        // Role != Slave 时暂停（不退出），断开当前连接
                        if params.role.value() != PluginRole::Slave {
                            is_connected.store(false, Ordering::Relaxed);
                            break;  // 跳出内层循环，回到外层等待
                        }

                        let recv_future = socket.recv();
                        let timeout_future = tokio::time::timeout(
                            std::time::Duration::from_millis(100),
                            recv_future
                        );

                        match timeout_future.await {
                            Ok(Ok(msg)) => {
                                consecutive_errors = 0;  // 重置连续错误计数
                                last_packet_time = std::time::Instant::now();  // C6: 更新心跳时间
                                if let Some(bytes) = msg.get(0) {
                                    if let Ok(state) = bincode::deserialize::<NetworkInteractionState>(bytes) {
                                        if state.is_valid() {
                                            // 时间戳检查：防止乱序包
                                            let prev_ts = last_timestamp.load(Ordering::Relaxed);

                                            // M2: 时间戳跳跃检测（超过 1 小时视为异常，重置）
                                            let time_diff = state.timestamp.saturating_sub(prev_ts);
                                            if prev_ts != 0 && time_diff > 3600_000 {
                                                logger.warn("network", &format!(
                                                    "Timestamp jump detected: {} -> {} (diff={}ms), resetting",
                                                    prev_ts, state.timestamp, time_diff
                                                ));
                                                last_timestamp.store(0, Ordering::Relaxed);
                                            }

                                            // 正常乱序检查
                                            // C9 修复：使用 < 而非 <=，允许相同时间戳的包（高负载时可能发生）
                                            let current_prev = last_timestamp.load(Ordering::Relaxed);
                                            if state.timestamp < current_prev && current_prev != 0 {
                                                out_of_order_count += 1;
                                                if out_of_order_count <= 3 || out_of_order_count % 100 == 0 {
                                                    logger.warn("network", &format!(
                                                        "Out-of-order packet #{}: ts={} < prev={}",
                                                        out_of_order_count, state.timestamp, current_prev
                                                    ));
                                                }
                                                continue;
                                            }
                                            last_timestamp.store(state.timestamp, Ordering::Relaxed);

                                            // 直接更新 InteractionManager
                                            interaction.from_network_state(&state);
                                            recv_count += 1;

                                            // 收到有效数据时标记为已连接（包括重连后）
                                            if !is_connected.load(Ordering::Relaxed) {
                                                is_connected.store(true, Ordering::Relaxed);
                                                logger.important("network", &format!(
                                                    "ZMQ Slave: {} connected!",
                                                    if recv_count == 1 { "First packet received," } else { "Reconnected," }
                                                ));
                                            }

                                            // 每 500 次记录一次日志
                                            if recv_count % 500 == 0 {
                                                logger.info("network", &format!(
                                                    "ZMQ Slave received {} packets, primary={}, compare={}",
                                                    recv_count, state.primary, state.compare
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                consecutive_errors += 1;
                                logger.error("network", &format!("ZMQ Recv Error #{}: {}", consecutive_errors, e));
                                is_connected.store(false, Ordering::Relaxed);

                                // 连续 5 次错误后尝试重连
                                if consecutive_errors >= 5 {
                                    logger.warn("network", "Too many consecutive errors, reconnecting...");
                                    break;  // 跳出内层循环，触发重连
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                            Err(_) => {
                                // Timeout - C6: 检查心跳超时
                                if last_packet_time.elapsed().as_millis() as u64 > HEARTBEAT_TIMEOUT_MS {
                                    if is_connected.load(Ordering::Relaxed) {
                                        is_connected.store(false, Ordering::Relaxed);
                                        logger.important("network", "ZMQ Slave: Heartbeat timeout, reconnecting...");

                                        // C11 修复：清空网络状态，防止旧数据污染新连接
                                        interaction.clear_network_state();

                                        // 先清理 is_running 标志，允许热重载机制重新初始化
                                        is_running.store(false, Ordering::Release);

                                        // 请求网络重启（Lib.rs process() 会执行）
                                        interaction.request_network_restart(app_config.clone());

                                        // 退出线程，等待热重载机制重新启动
                                        break 'reconnect_loop;
                                    }
                                }
                                continue;
                            }
                        }
                    }

                    // 内层循环退出，可能需要重连
                    is_connected.store(false, Ordering::Relaxed);
                }

                is_connected.store(false, Ordering::Relaxed);
                logger.info("network", "ZMQ Subscriber thread stopped");
            });
        }));
    }

    /// 关闭网络管理器
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Acquire) {
            return;
        }

        // 停止线程
        self.is_running.store(false, Ordering::Release);
        self.is_connected.store(false, Ordering::Relaxed);

        // 等待线程结束
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for NetworkManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
