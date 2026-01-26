use crossbeam::channel::{unbounded, Receiver, Sender};
use mcm_infra::logger::InstanceLogger;

use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;

pub mod actors;
pub mod web_assets;

use crate::actors::osc::OscManager;
use crate::actors::web::WebManager;
use mcm_core::interaction::InteractionManager;
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::MonitorParams;
use mcm_protocol::config::AppConfig;
use mcm_protocol::web_structs::WebSharedState;

/// Audio Thread -> Reactor Commands
#[derive(Debug)]
pub enum ReactorCommand {
    /// Start the Web Server
    StartWeb { port: u16 },
    /// Stop the Web Server
    StopWeb,
    /// Initialize/Start OSC
    InitOsc {
        channel_count: usize,
        current_cut: bool,
        config: AppConfig,
    },
    /// Send OSC Message
    SendOsc { addr: String, value: f32 },
    /// Broadcast state update to all clients (Web/OSC)
    BroadcastState {
        channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
    },
    /// Shutdown the Reactor (Plugin unloading)
    Shutdown,
}

/// The Reactor - Unified Async Runtime Manager
pub struct Reactor {
    /// Channel to send commands to Reactor
    tx: Sender<ReactorCommand>,
    /// Thread handle
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl Reactor {
    pub fn new(
        logger: Arc<InstanceLogger>,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        osc_state: Arc<OscSharedState>,
        web_state: Arc<WebSharedState>,
    ) -> Self {
        let (tx, rx) = unbounded::<ReactorCommand>();

        let interaction_clone = Arc::clone(&interaction);
        let params_clone = Arc::clone(&params);
        let osc_state_clone = Arc::clone(&osc_state);
        let web_state_clone = Arc::clone(&web_state);

        // Spawn background thread (non-audio)
        let thread_handle = thread::spawn(move || {
            Self::reactor_main(
                rx,
                logger,
                interaction_clone,
                params_clone,
                osc_state_clone,
                web_state_clone,
            );
        });

        Self {
            tx,
            thread_handle: Some(thread_handle),
        }
    }

    /// Send command to Reactor (Non-blocking, safe for Audio Thread)
    pub fn send(&self, cmd: ReactorCommand) {
        let _ = self.tx.try_send(cmd);
    }

    /// The Main Loop running in a background thread
    fn reactor_main(
        rx: Receiver<ReactorCommand>,
        logger: Arc<InstanceLogger>,
        interaction: Arc<InteractionManager>,
        params: Arc<MonitorParams>,
        osc_state: Arc<OscSharedState>,
        web_state: Arc<WebSharedState>,
    ) {
        logger.important("reactor", "Reactor Thread Started (Unified Tokio Runtime)");

        // Create Tokio Runtime
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                logger.error("reactor", &format!("Failed to create Runtime: {}", e));
                return;
            }
        };

        let logger_async = Arc::clone(&logger);
        rt.block_on(async move {
            // Shadowing logger with the clone for convenience
            let logger = logger_async;
            logger.info("reactor", "Tokio Runtime Active");

            // Actors state
            let mut web_manager = WebManager::new(Arc::clone(&web_state));
            let mut osc_manager = OscManager::with_state(Arc::clone(&osc_state));

            // Command Loop
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    ReactorCommand::Shutdown => {
                        logger.important("reactor", "Shutdown signal received");
                        web_manager.shutdown();
                        osc_manager.shutdown();
                        break;
                    }
                    ReactorCommand::StartWeb { port: _ } => {
                        logger.info("reactor", "CMD: StartWeb");
                        // WebManager handles port dynamic internally or via config passed later?
                        // For now we assume WebManager allocates random port.
                        web_manager.init(
                            Arc::clone(&logger),
                            Arc::clone(&interaction),
                            Arc::clone(&params),
                        );
                    }
                    ReactorCommand::StopWeb => {
                        logger.info("reactor", "CMD: StopWeb");
                        web_manager.shutdown();
                    }
                    ReactorCommand::InitOsc {
                        channel_count,
                        current_cut,
                        config,
                    } => {
                        logger.info("reactor", "CMD: InitOsc");
                        // WebState is needed for Osc init (for dual send)
                        let web_state = web_manager.get_state();

                        osc_manager.init(
                            channel_count,
                            0.0,   // Initial volume (unused in new init?)
                            false, // Initial dim
                            current_cut,
                            Arc::clone(&interaction),
                            Arc::clone(&params),
                            Arc::clone(&logger),
                            &config,
                            web_state,
                        );
                    }
                    ReactorCommand::SendOsc { addr: _, value: _ } => {
                        // osc_manager.send_message(...) - Not implemented in OscManager yet?
                        // Usually used for manual send. OscManager handles logic internally.
                    }
                    ReactorCommand::BroadcastState {
                        channel_count,
                        master_volume,
                        dim,
                        cut,
                    } => {
                        osc_manager.broadcast_state(channel_count, master_volume, dim, cut);
                    }
                }
            }
        });

        logger.important("reactor", "Reactor Thread Exiting");
    }
}
