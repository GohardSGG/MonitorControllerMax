use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use zeromq::{Socket, PubSocket, SubSocket, SocketSend, SocketRecv};
use bincode;
use tokio::runtime::Runtime;

use crate::network_protocol::NetworkInteractionState;
use crate::Interaction::InteractionManager;
use crate::logger::InstanceLogger;
use crate::Params::MonitorParams;

pub struct NetworkManager {
    // 运行状态标志（线程退出控制）
    is_running: Arc<AtomicBool>,

    // Slave 连接状态（供 UI 显示）
    pub is_connected: Arc<AtomicBool>,

    // 线程句柄
    thread_handle: Option<JoinHandle<()>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            is_connected: Arc::new(AtomicBool::new(false)),
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
    pub fn init_master(&mut self, port: u16, interaction: Arc<InteractionManager>, params: Arc<MonitorParams>, logger: Arc<InstanceLogger>) {
        // 如果已经运行，先关闭
        if self.is_running.load(Ordering::Relaxed) {
            self.shutdown();
        }

        self.is_running.store(true, Ordering::Relaxed);

        let is_running = Arc::clone(&self.is_running);

        self.thread_handle = Some(thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                let mut socket = PubSocket::new();
                let endpoint = format!("tcp://0.0.0.0:{}", port);

                if let Err(e) = socket.bind(&endpoint).await {
                    logger.warn("network", &format!("ZMQ port {} unavailable: {}", port, e));
                    return;
                }
                logger.info("network", &format!("ZMQ Publisher bound to {}", endpoint));

                let mut send_count: u64 = 0;

                while is_running.load(Ordering::Relaxed) {
                    // 从 params 读取 master_gain/dim/cut
                    let master_gain = params.master_gain.value();
                    let dim = params.dim.value();
                    let cut = params.cut.value();

                    // 从 InteractionManager 读取当前状态（包含 params 值）
                    let state = interaction.to_network_state(master_gain, dim, cut);

                    // 序列化并发送
                    if let Ok(bytes) = bincode::serialize(&state) {
                        let _ = socket.send(bytes.into()).await;
                        send_count += 1;

                        // 每 500 次记录一次日志（约 10 秒一次，20ms 间隔）
                        if send_count % 500 == 0 {
                            logger.info("network", &format!(
                                "ZMQ Master sent {} packets, primary={}, compare={}",
                                send_count, state.primary, state.compare
                            ));
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
    pub fn init_slave(&mut self, master_ip: &str, port: u16, interaction: Arc<InteractionManager>, logger: Arc<InstanceLogger>) {
        // 如果已经运行，先关闭
        if self.is_running.load(Ordering::Relaxed) {
            self.shutdown();
        }

        let endpoint = format!("tcp://{}:{}", master_ip, port);
        self.is_running.store(true, Ordering::Relaxed);
        self.is_connected.store(false, Ordering::Relaxed);

        let is_running = Arc::clone(&self.is_running);
        let is_connected = Arc::clone(&self.is_connected);

        self.thread_handle = Some(thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                let mut socket = SubSocket::new();
                if let Err(e) = socket.connect(&endpoint).await {
                    logger.error("network", &format!("ZMQ Connect Error: {}", e));
                    return;
                }
                if let Err(e) = socket.subscribe("").await {
                    logger.error("network", &format!("ZMQ Subscribe Error: {}", e));
                    return;
                }
                logger.info("network", &format!("ZMQ Subscriber connected to {}", endpoint));

                let mut recv_count: u64 = 0;

                // 使用非阻塞接收循环
                while is_running.load(Ordering::Relaxed) {
                    let recv_future = socket.recv();
                    let timeout_future = tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        recv_future
                    );

                    match timeout_future.await {
                        Ok(Ok(msg)) => {
                            if let Some(bytes) = msg.get(0) {
                                if let Ok(state) = bincode::deserialize::<NetworkInteractionState>(bytes) {
                                    if state.is_valid() {
                                        // 直接更新 InteractionManager
                                        interaction.from_network_state(&state);
                                        recv_count += 1;

                                        // 首次收到数据才标记为已连接
                                        if recv_count == 1 {
                                            is_connected.store(true, Ordering::Relaxed);
                                            logger.info("network", "ZMQ Slave: First packet received, connected!");
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
                            logger.error("network", &format!("ZMQ Recv Error: {}", e));
                            is_connected.store(false, Ordering::Relaxed);
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(_) => {
                            // Timeout, continue
                            continue;
                        }
                    }
                }

                is_connected.store(false, Ordering::Relaxed);
                logger.info("network", "ZMQ Subscriber thread stopped");
            });
        }));
    }

    /// 关闭网络管理器
    pub fn shutdown(&mut self) {
        if !self.is_running.load(Ordering::Relaxed) {
            return;
        }

        // 停止线程
        self.is_running.store(false, Ordering::Relaxed);
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
