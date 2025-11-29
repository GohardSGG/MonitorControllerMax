#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::Arc;
use simplelog::*;
use std::fs::File;
use std::panic;
use std::io::Write;

pub mod Components; 
mod Editor;
mod Audio;
mod Registry;
mod Params;

use Params::MonitorParams;

pub struct MonitorControllerMax {
    params: Arc<MonitorParams>,
}

impl Default for MonitorControllerMax {
    fn default() -> Self {
        Self {
            params: Arc::new(MonitorParams::default()),
        }
    }
}

impl Plugin for MonitorControllerMax {
    const NAME: &'static str = "MonitorControllerMax";
    const VENDOR: &'static str = "GohardSGG";
    const URL: &'static str = "https://github.com/GohardSGG";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(18),
            main_output_channels: NonZeroU32::new(18),
            ..AudioIOLayout::const_default()
        },
    ];

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
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        panic::set_hook(Box::new(|info| {
            if let Ok(mut file) = File::options().create(true).append(true).open("C:/Plugins/MonitorControllerMax_Crash.log") {
                let _ = writeln!(file, "[CRASH] Panic occurred: {:?}", info);
            }
        }));

        if let Ok(file) = File::options().create(true).append(true).open("C:/Plugins/MonitorControllerMax_Debug.log") {
            let _ = WriteLogger::init(
                LevelFilter::Info,
                Config::default(),
                file,
            );
            log::info!("=== Plugin Initialized (v{}) [EGUI GPU MODE] ===", env!("CARGO_PKG_VERSION"));
        }

        Registry::GlobalRegistry::register_instance();
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        Audio::process_audio(buffer, &self.params);
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

nih_export_clap!(MonitorControllerMax);
nih_export_vst3!(MonitorControllerMax);
