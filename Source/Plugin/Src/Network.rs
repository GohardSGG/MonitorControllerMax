use std::sync::Arc;
use std::thread;
use crossbeam::channel::{unbounded, Sender};
use crossbeam::atomic::AtomicCell;
use zeromq::{Socket, PubSocket, SubSocket, SocketSend, SocketRecv};
use bincode;
use tokio::runtime::Runtime;

use crate::network_protocol::NetworkRenderState;
use crate::{mcm_info, mcm_error};
// Removed unused import: RenderState

pub struct NetworkManager {
    // For Master: Send RenderState to the network thread
    pub sender: Option<Sender<NetworkRenderState>>,
    
    // For Slave: Read latest state from network thread
    // We use NetworkRenderState here to match the protocol, convertible to RenderState
    pub latest_state: Arc<AtomicCell<Option<NetworkRenderState>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            sender: None,
            latest_state: Arc::new(AtomicCell::new(None)),
        }
    }

    pub fn init_master(&mut self, port: u16) {
        let (tx, rx) = unbounded::<NetworkRenderState>();
        self.sender = Some(tx);

        thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                let mut socket = PubSocket::new();
                let endpoint = format!("tcp://0.0.0.0:{}", port);
                
                if let Err(e) = socket.bind(&endpoint).await {
                    mcm_error!("ZMQ Bind Error: {}", e);
                    return;
                }
                mcm_info!("ZMQ Publisher bound to {}", endpoint);

                while let Ok(state) = rx.recv() {
                    if let Ok(bytes) = bincode::serialize(&state) {
                        // Fire and forget
                        let _ = socket.send(bytes.into()).await;
                    }
                }
            });
        });
    }

    pub fn init_slave(&mut self, master_ip: &str, port: u16) {
        let state_cell = self.latest_state.clone();
        let endpoint = format!("tcp://{}:{}", master_ip, port);

        thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                let mut socket = SubSocket::new();
                if let Err(e) = socket.connect(&endpoint).await {
                    mcm_error!("ZMQ Connect Error: {}", e);
                    return;
                }
                if let Err(e) = socket.subscribe("").await {
                    mcm_error!("ZMQ Subscribe Error: {}", e);
                    return;
                }
                mcm_info!("ZMQ Subscriber connected to {}", endpoint);

                loop {
                    // This blocks (asynchronously) until data arrives
                    match socket.recv().await {
                        Ok(msg) => {
                            // ZMQ message might be multipart, we expect single part payload
                            if let Some(bytes) = msg.get(0) {
                                if let Ok(state) = bincode::deserialize::<NetworkRenderState>(bytes) {
                                    state_cell.store(Some(state));
                                }
                            }
                        }
                        Err(e) => {
                            mcm_error!("ZMQ Recv Error: {}", e);
                            // Simple retry delay?
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            });
        });
    }
}

