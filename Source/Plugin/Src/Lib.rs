#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::Arc;

pub mod Components;
mod Editor;
mod Audio;
mod Registry;
mod Params;
mod scale;
mod config_manager;

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

pub struct MonitorControllerMax {
    params: Arc<MonitorParams>,
    network: NetworkManager,
    osc: OscManager,
}

impl Default for MonitorControllerMax {
    fn default() -> Self {
        // Initialize logger FIRST, before any other initialization
        logger::init();

        Self {
            params: Arc::new(MonitorParams::default()),
            network: NetworkManager::new(),
            osc: OscManager::new(),
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
        Editor::create_editor(self.params.clone())
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        mcm_info!("Plugin initialize() called");

        // Debug: Log audio IO layout information
        let input_channels = audio_io_layout.main_input_channels.map(|n| n.get()).unwrap_or(0);
        let output_channels = audio_io_layout.main_output_channels.map(|n| n.get()).unwrap_or(0);
        mcm_info!("[AudioIO] Input channels: {}, Output channels: {}", input_channels, output_channels);
        mcm_info!("[AudioIO] Layout name: {}", audio_io_layout.name());
        mcm_info!("[AudioIO] Main input name: {}", audio_io_layout.main_input_name());
        mcm_info!("[AudioIO] Main output name: {}", audio_io_layout.main_output_name());

        // Debug: Log all available layouts
        mcm_info!("[AudioIO] Available layouts count: {}", Self::AUDIO_IO_LAYOUTS.len());
        for (i, layout) in Self::AUDIO_IO_LAYOUTS.iter().enumerate() {
            let ch = layout.main_input_channels.map(|n| n.get()).unwrap_or(0);
            mcm_info!("[AudioIO] Layout[{}]: {} ({} channels)", i, layout.name(), ch);
        }

        Registry::GlobalRegistry::register_instance();
        
        // Initialize Network based on Role
        // TODO: Port should be configurable via params or config
        let role = self.params.role.value();
        match role {
            Params::PluginRole::Master => {
                self.network.init_master(9123);
                // Initialize OSC for Master (sends to hardware + receives from hardware)
                self.osc.init(output_channels as usize);
                mcm_info!("[OSC] Initialized for Master mode");
            }
            Params::PluginRole::Slave => {
                self.network.init_slave("127.0.0.1", 9123);
                // Slave mode does NOT use OSC (no hardware control)
                mcm_info!("[OSC] Disabled for Slave mode");
            }
            Params::PluginRole::Standalone => {
                // No network initialization - pure local mode
                // Initialize OSC for Standalone (local hardware control)
                self.osc.init(output_channels as usize);
                mcm_info!("[OSC] Initialized for Standalone mode");
            }
        }

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        Audio::process_audio(buffer, &self.params, &mut self.network);
        ProcessStatus::Normal
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
        mcm_info!("[Plugin] Shutting down...");
        // OSC Manager will auto-shutdown via its own Drop impl
        // but we can explicitly shutdown for cleaner logs
        self.osc.shutdown();
    }
}

nih_export_clap!(MonitorControllerMax);
nih_export_vst3!(MonitorControllerMax);
