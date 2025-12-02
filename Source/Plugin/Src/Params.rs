#![allow(non_snake_case)]

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;
use crate::config_manager::CONFIG;

// Define max channels constant. Must match array size.
pub const MAX_CHANNELS: usize = 32;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PluginRole {
    #[name = "Standalone"]
    Standalone,
    #[name = "Master (Source)"]
    Master,
    #[name = "Slave (Monitor)"]
    Slave,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum SoloMode {
    #[name = "SIP (Solo In Place)"]
    SIP,
    #[name = "PFL (Pre Fader Listen)"]
    PFL,
}

#[derive(Params)]
pub struct ChannelParams {
    #[id = "solo"]
    pub solo: BoolParam,
    #[id = "mute"]
    pub mute: BoolParam,
}

impl ChannelParams {
    pub fn new(index: usize) -> Self {
        let prefix = format!("Ch {}", index + 1);
        Self {
            solo: BoolParam::new(format!("{} Solo", prefix), false),
            mute: BoolParam::new(format!("{} Mute", prefix), false),
        }
    }
}

impl Default for ChannelParams {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Params)]
pub struct MonitorParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "master_gain"]
    pub master_gain: FloatParam,

    #[id = "dim"]
    pub dim: BoolParam,

    #[id = "cut"]
    pub cut: BoolParam,

    #[id = "role"]
    pub role: EnumParam<PluginRole>,

    #[id = "solo_mode"]
    pub solo_mode: EnumParam<SoloMode>,

    // Dynamic layout selector based on config
    #[id = "layout_idx"]
    pub layout: IntParam,

    // We also need SUB layout selector
    #[id = "sub_layout_idx"]
    pub sub_layout: IntParam,

    // Array of channel parameters (Solo/Mute only, no Trim)
    #[nested(array, group = "Channels")]
    pub channels: [ChannelParams; MAX_CHANNELS],

    // Allow Automation (Global Override)
    #[id = "allow_automation"]
    pub allow_automation: BoolParam,
}

impl Default for MonitorParams {
    fn default() -> Self {
        let speaker_layouts = CONFIG.get_speaker_layouts();
        let sub_layouts = CONFIG.get_sub_layouts();

        Self {
            editor_state: EguiState::from_size(720, 720), 

            master_gain: FloatParam::new(
                "Master Gain",
                util::db_to_gain(0.0),  // 默认 0 dB (unity gain)
                FloatRange::Skewed {
                    min: util::MINUS_INFINITY_GAIN,  // -∞ dB
                    max: util::db_to_gain(0.0),      // 0 dB (无增益)
                    factor: FloatRange::gain_skew_factor(-80.0, 0.0),
                },
            )
            .with_unit(" dB")
            .with_string_to_value(formatters::s2v_f32_gain_to_db())
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2)),
            
            dim: BoolParam::new("Dim", false),
            cut: BoolParam::new("Cut", false),
            
            role: EnumParam::new("Role", PluginRole::Standalone),
            solo_mode: EnumParam::new("Solo Mode", SoloMode::SIP),

            layout: IntParam::new(
                "Speaker Layout",
                0,
                IntRange::Linear { min: 0, max: (speaker_layouts.len().saturating_sub(1)) as i32 }
            ),
            
            sub_layout: IntParam::new(
                "Sub Layout",
                0,
                IntRange::Linear { min: 0, max: (sub_layouts.len().saturating_sub(1)) as i32 }
            ),

            channels: std::array::from_fn(|i| ChannelParams::new(i)),

            allow_automation: BoolParam::new("Allow Automation", false),
        }
    }
}
