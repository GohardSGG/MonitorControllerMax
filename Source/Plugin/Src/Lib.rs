#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

// Modules within Plugin crate
pub mod components;
pub mod editor;
pub mod keyboard_polling;
pub mod scale;

// Workspace Crates
use mcm_core::audio::{self, GainSmoothingState};
use mcm_core::config_manager::{ConfigManager, Layout};
use mcm_core::interaction::InteractionManager;
use mcm_core::osc_state::OscSharedState;
use mcm_core::params::MonitorParams;
use mcm_infra::config_loader;
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::config::AppConfig;
use mcm_protocol::web_structs::WebSharedState;
use mcm_reactor::{Reactor, ReactorCommand};

// Include auto-generated audio layouts from build.rs
mod audio_layouts {
    include!(concat!(env!("OUT_DIR"), "/Audio_Layouts.rs"));
}
use audio_layouts::GENERATED_AUDIO_IO_LAYOUTS;

pub struct MonitorControllerMax {
    params: Arc<MonitorParams>,

    // The Reactor (manages Web, OSC, Network)
    reactor: Arc<Reactor>,

    gain_state: GainSmoothingState,
    interaction: Arc<InteractionManager>,

    // Shared OSC State (accessible by Audio Thread and Reactor)
    // Shared OSC State (accessible by Audio Thread and Reactor)
    osc_state: Arc<OscSharedState>,

    // Web State
    web_state: Arc<WebSharedState>,

    // Network Status (ZMQ)
    network_connected: Arc<AtomicBool>,

    /// 输出通道数
    output_channels: usize,

    /// 实例ID
    #[allow(dead_code)]
    instance_id: String,

    /// 实例级日志器
    logger: Arc<InstanceLogger>,

    /// 实例级用户配置
    app_config: AppConfig,

    /// 实例级布局配置
    layout_config: Arc<ConfigManager>,

    /// P8: Layout 缓存（避免每帧堆分配）
    layout_cache: Option<Layout>,
    layout_cache_key: (i32, i32),

    /// 自动检测的布局索引
    auto_detected_layout: Option<i32>,
}

impl Default for MonitorControllerMax {
    fn default() -> Self {
        // Generate unique instance ID
        let instance_id = mcm_infra::logger::generate_instance_id();

        // Create instance-specific logger
        let logger = InstanceLogger::new(&instance_id);

        // Load instance-specific configs
        let app_config = config_loader::load_from_disk();
        let layout_config = Arc::new(ConfigManager::new());

        logger.info("monitor_controller_max", "Plugin instance created");

        let params = Arc::new(MonitorParams::default());
        let interaction = Arc::new(InteractionManager::new(Arc::clone(&logger)));
        let osc_state = Arc::new(OscSharedState::new());
        let web_state = Arc::new(WebSharedState::new());
        let network_connected = Arc::new(AtomicBool::new(false));

        // Initialize Reactor
        let reactor = Arc::new(Reactor::new(
            Arc::clone(&logger),
            Arc::clone(&interaction),
            Arc::clone(&params),
            Arc::clone(&osc_state),
            Arc::clone(&web_state),
        ));

        Self {
            params,
            reactor,
            gain_state: GainSmoothingState::new(),
            interaction,
            osc_state,
            web_state,
            network_connected,
            output_channels: 2,
            instance_id,
            logger,
            app_config,
            layout_config,
            layout_cache: None,
            layout_cache_key: (-1, -1),
            auto_detected_layout: None,
        }
    }
}

impl Plugin for MonitorControllerMax {
    const NAME: &'static str = "MonitorControllerMax";
    const VENDOR: &'static str = "GohardSGG";
    const URL: &'static str = "https://github.com/GohardSGG";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = GENERATED_AUDIO_IO_LAYOUTS;

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn nih_plug::params::Params> {
        self.params.clone()
    }

    fn editor(
        &mut self,
        _async_executor: AsyncExecutor<Self>,
    ) -> Option<Box<dyn nih_plug::editor::Editor>> {
        // TODO: Update Editor::create_editor signature to accept new types
        editor::create_editor(
            self.params.clone(),
            self.interaction.clone(),
            self.osc_state.clone(),
            self.network_connected.clone(),
            Arc::clone(&self.logger),
            self.app_config.clone(),
            Arc::clone(&self.layout_config),
            self.web_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.logger
            .info("monitor_controller_max", "Plugin initialize() called");

        let output_channels = audio_io_layout
            .main_output_channels
            .map(|n| n.get())
            .unwrap_or(0);
        self.output_channels = output_channels as usize;

        if output_channels > 2 {
            if let Some(detected_idx) = self
                .layout_config
                .find_layout_for_channels(output_channels as usize)
            {
                self.auto_detected_layout = Some(detected_idx);
            }
        }

        // Initialize OSC
        self.reactor.send(ReactorCommand::InitOsc {
            channel_count: self.output_channels,
            current_cut: self.params.cut.value(),
            config: self.app_config.clone(),
        });

        // Start Web Server
        let port = self.app_config.osc_receive_port; // Legacy logic, actually unused by WebManager logic
        self.reactor.send(ReactorCommand::StartWeb { port });

        true
    }

    fn reset(&mut self) {
        self.logger
            .info("monitor_controller_max", "Plugin reset() called");

        // Broadcast state
        self.reactor.send(ReactorCommand::BroadcastState {
            channel_count: self.output_channels,
            master_volume: self.params.master_gain.value(),
            dim: self.params.dim.value(),
            cut: self.params.cut.value(),
        });
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Prepare layout
        let layout_idx = self.params.layout.value();
        let sub_layout_idx = self.params.sub_layout.value();
        let cache_key = (layout_idx, sub_layout_idx);

        if cache_key != self.layout_cache_key || self.layout_cache.is_none() {
            let speaker_name = self
                .layout_config
                .get_speaker_name(layout_idx as usize)
                .unwrap_or("7.1.4");
            let sub_name = self
                .layout_config
                .get_sub_name(sub_layout_idx as usize)
                .unwrap_or("None");
            self.layout_cache = Some(self.layout_config.get_layout(speaker_name, sub_name));
            self.layout_cache_key = cache_key;

            // Layout changed, update OSC state
            self.osc_state
                .update_layout_channels(self.layout_cache.as_ref().unwrap());
        }

        let layout = match self.layout_cache.as_ref() {
            Some(l) => l,
            None => return ProcessStatus::Normal,
        };

        // Call Core Audio Processor
        audio::process_audio_with_layout(
            buffer,
            &self.params,
            &self.gain_state,
            &self.interaction,
            layout,
            Some(&self.osc_state),
        );

        ProcessStatus::Normal
    }
}

impl Drop for MonitorControllerMax {
    fn drop(&mut self) {
        self.logger
            .info("monitor_controller_max", "[Plugin] Shutting down...");
        // Signal Web shutdown via state
        self.web_state
            .is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.reactor.send(ReactorCommand::Shutdown);
    }
}

impl ClapPlugin for MonitorControllerMax {
    const CLAP_ID: &'static str = "com.gohardsgg.monitor-controller-max";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Monitor Controller Max");
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/GohardSGG");
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for MonitorControllerMax {
    const VST3_CLASS_ID: [u8; 16] = *b"MonitorCtrlMaxSG";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Tools,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(MonitorControllerMax);
nih_export_vst3!(MonitorControllerMax);
