#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::Arc;
use simplelog::*;
use std::fs::File;

// 引入模块
pub mod Components; // 公开组件模块，供 Editor 使用
mod Editor;
mod Audio;
mod Registry;
mod Params;

// 为了避免与 nih_plug::prelude::Params trait 冲突，
// 我们使用完整的路径或重命名引入
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

    // 工业级音频配置：支持 18x18 通道
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

    // 明确指定返回类型为 nih_plug::params::Params Trait 对象
    fn params(&self) -> Arc<dyn nih_plug::params::Params> {
        self.params.clone()
    }

    // 明确指定 Editor Trait
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn nih_plug::editor::Editor>> {
        log::info!("Creating editor...");
        Editor::create_editor(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // 初始化文件日志系统
        // 注意：在多实例加载时可能会有冲突，这里简单的 append 模式
        if let Ok(file) = File::options().create(true).append(true).open("C:/Plugins/MonitorControllerMax_Debug.log") {
            let _ = WriteLogger::init(
                LevelFilter::Info,
                Config::default(),
                file,
            );
            log::info!("=== Plugin Initialized (v{}) ===", env!("CARGO_PKG_VERSION"));
        } else {
            // 如果 C:/Plugins 不存在或无权限，尝试临时目录
            // 这里为了调试崩溃，我们假设您已经创建了该目录
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
        // 调用音频处理模块
        Audio::process_audio(buffer, &self.params);
        ProcessStatus::Normal
    }
}

// 实现 ClapPlugin Trait (完整实现)
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
