#![allow(non_snake_case)]

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Params)]
pub struct MonitorParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "master_gain"]
    pub master_gain: FloatParam,

    #[id = "global_mute"]
    pub global_mute: BoolParam,

    #[id = "global_dim"]
    pub global_dim: BoolParam,
}

impl Default for MonitorParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(800, 600), 

            master_gain: FloatParam::new(
                "Master Gain",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(1)),
            
            global_mute: BoolParam::new("Mute", false),
            global_dim: BoolParam::new("Dim", false),
        }
    }
}
