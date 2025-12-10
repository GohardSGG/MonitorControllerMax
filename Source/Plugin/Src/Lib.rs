#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::Arc;

pub mod Components;
mod Editor;
mod Audio;
mod Params;
mod scale;
mod config_manager;
mod config_file;

mod network;
mod network_protocol;
mod channel_logic;
mod logger;
mod Interaction;
mod osc;

// Include auto-generated audio layouts from build.rs
mod Audio_Layouts {
    include!(concat!(env!("OUT_DIR"), "/Audio_Layouts.rs"));
}

use Params::MonitorParams;
use network::NetworkManager;
use osc::OscManager;
use Audio::GainSmoothingState;
use Interaction::InteractionManager;
use logger::InstanceLogger;
use config_file::AppConfig;
use config_manager::ConfigManager;

pub struct MonitorControllerMax {
    params: Arc<MonitorParams>,
    network: NetworkManager,
    osc: OscManager,
    gain_state: GainSmoothingState,
    interaction: Arc<InteractionManager>,
    /// 输出通道数（在 initialize 中记录，延迟初始化时使用）
    output_channels: usize,
    /// 是否需要延迟初始化
    needs_deferred_init: bool,
    /// 延迟初始化是否已完成
    deferred_init_done: bool,
    /// 上次的 Role（用于检测运行时切换）
    last_role: Option<Params::PluginRole>,
    /// 实例ID
    instance_id: String,
    /// 实例级日志器
    logger: Arc<InstanceLogger>,
    /// 实例级用户配置
    app_config: AppConfig,
    /// 实例级布局配置
    layout_config: Arc<ConfigManager>,
}

impl Default for MonitorControllerMax {
    fn default() -> Self {
        // Generate unique instance ID
        let instance_id = logger::generate_instance_id();

        // Create instance-specific logger
        let logger = InstanceLogger::new(&instance_id);

        // Load instance-specific configs
        let app_config = AppConfig::load_from_disk();
        let layout_config = Arc::new(ConfigManager::new());

        logger.info("monitor_controller_max", "Plugin instance created");

        Self {
            params: Arc::new(MonitorParams::default()),
            network: NetworkManager::new(),
            osc: OscManager::new(),
            gain_state: GainSmoothingState::new(),
            interaction: Arc::new(InteractionManager::new(Arc::clone(&logger))),
            output_channels: 2,
            needs_deferred_init: false,
            deferred_init_done: false,
            last_role: None,
            instance_id,
            logger,
            app_config,
            layout_config,
        }
    }
}

impl Plugin for MonitorControllerMax {
    const NAME: &'static str = "MonitorControllerMax";
    const VENDOR: &'static str = "GohardSGG";
    const URL: &'static str = "https://github.com/GohardSGG";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Audio IO layouts are auto-generated from Speaker_Config.json by build.rs
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = Audio_Layouts::GENERATED_AUDIO_IO_LAYOUTS;

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn nih_plug::params::Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn nih_plug::editor::Editor>> {
        Editor::create_editor(
            self.params.clone(),
            self.interaction.clone(),
            self.osc.get_state(),
            self.network.get_connection_status(),
            Arc::clone(&self.logger),
            self.app_config.clone(),
            Arc::clone(&self.layout_config),
        )
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.logger.info("monitor_controller_max", "Plugin initialize() called");

        // Debug: Log audio IO layout information
        let input_channels = audio_io_layout.main_input_channels.map(|n| n.get()).unwrap_or(0);
        let output_channels = audio_io_layout.main_output_channels.map(|n| n.get()).unwrap_or(0);
        self.logger.info("monitor_controller_max", &format!("[AudioIO] Input channels: {}, Output channels: {}", input_channels, output_channels));
        self.logger.info("monitor_controller_max", &format!("[AudioIO] Layout name: {}", audio_io_layout.name()));

        // 记录输出通道数，用于延迟初始化
        self.output_channels = output_channels as usize;

        // 延迟初始化：在 reset() 中执行 OSC/Network 初始化
        self.needs_deferred_init = true;
        self.deferred_init_done = false;

        self.logger.info("monitor_controller_max", &format!("[Initialize] Deferred init scheduled, output_channels={}", self.output_channels));

        true
    }

    fn reset(&mut self) {
        self.logger.info("monitor_controller_max", "Plugin reset() called");

        // 检测 Role 变化（DAW 恢复参数后可能改变）
        let current_role = self.params.role.value();
        let role_changed = self.last_role.map(|r| r != current_role).unwrap_or(false);

        if role_changed {
            self.logger.info("monitor_controller_max", &format!(
                "[Reset] Role changed: {:?} -> {:?}, triggering re-init",
                self.last_role, current_role
            ));

            // 关闭旧资源
            self.osc.shutdown();
            self.network.shutdown();

            // 重置初始化标志，让后续逻辑重新初始化
            self.deferred_init_done = false;
            self.needs_deferred_init = true;
        }

        // 执行延迟初始化
        if self.needs_deferred_init && !self.deferred_init_done {
            self.perform_deferred_init();
        }

        // 广播当前参数状态
        let role = self.params.role.value();
        if role != Params::PluginRole::Slave && self.deferred_init_done {
            let channel_count = self.osc.state.channel_count.load(std::sync::atomic::Ordering::Relaxed);
            let master_volume = self.params.master_gain.value();
            let dim = self.params.dim.value();
            let cut = self.params.cut.value();
            self.logger.info("monitor_controller_max", &format!("[Reset] Broadcasting state: vol={:.4}, dim={}, cut={}", master_volume, dim, cut));
            self.osc.broadcast_state(channel_count, master_volume, dim, cut);
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        Audio::process_audio(buffer, &self.params, &self.gain_state, &self.interaction, &self.layout_config);
        ProcessStatus::Normal
    }
}

impl MonitorControllerMax {
    /// 执行延迟初始化
    fn perform_deferred_init(&mut self) {
        let role = self.params.role.value();

        self.logger.info("monitor_controller_max", &format!("[DeferredInit] Role={:?}, output_channels={}", role, self.output_channels));

        // 根据 role 初始化网络
        match role {
            Params::PluginRole::Master => {
                self.network.init_master(9123, self.interaction.clone(), self.params.clone(), Arc::clone(&self.logger));
                let master_volume = self.params.master_gain.value();
                let dim = self.params.dim.value();
                let cut = self.params.cut.value();
                self.osc.init(self.output_channels, master_volume, dim, cut, self.interaction.clone(), Arc::clone(&self.logger), &self.app_config);
                self.logger.info("monitor_controller_max", "[DeferredInit] OSC initialized for Master mode");
            }
            Params::PluginRole::Slave => {
                self.network.init_slave("127.0.0.1", 9123, self.interaction.clone(), Arc::clone(&self.logger));
                self.logger.info("monitor_controller_max", "[DeferredInit] OSC disabled for Slave mode");
            }
            Params::PluginRole::Standalone => {
                let master_volume = self.params.master_gain.value();
                let dim = self.params.dim.value();
                let cut = self.params.cut.value();
                self.osc.init(self.output_channels, master_volume, dim, cut, self.interaction.clone(), Arc::clone(&self.logger), &self.app_config);
                self.logger.info("monitor_controller_max", "[DeferredInit] OSC initialized for Standalone mode");
            }
        }

        self.last_role = Some(role);
        self.deferred_init_done = true;
        self.needs_deferred_init = false;

        self.logger.info("monitor_controller_max", "[DeferredInit] Complete");
    }
}

impl ClapPlugin for MonitorControllerMax {
    const CLAP_ID: &'static str = "com.gohardsgg.monitor-controller-max";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("MonitorControllerMax Rust Edition");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Utility,
        ClapFeature::Stereo,
        ClapFeature::Surround,
        ClapFeature::Ambisonic,
    ];
}

impl Vst3Plugin for MonitorControllerMax {
    const VST3_CLASS_ID: [u8; 16] = *b"MonitorContrlMax";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Tools,
        Vst3SubCategory::Spatial,
    ];
}

impl Drop for MonitorControllerMax {
    fn drop(&mut self) {
        self.logger.info("monitor_controller_max", "[Plugin] Shutting down...");
        self.osc.shutdown();
    }
}

nih_export_clap!(MonitorControllerMax);
nih_export_vst3!(MonitorControllerMax);
