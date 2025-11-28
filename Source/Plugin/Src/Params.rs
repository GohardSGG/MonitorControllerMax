#![allow(non_snake_case)]

use nih_plug::prelude::*;

#[derive(Params)]
pub struct MonitorParams {
    /// Master Gain (0% - 100%)
    #[id = "master_gain"]
    pub master_gain: FloatParam,

    /// Global Mute
    #[id = "global_mute"]
    pub global_mute: BoolParam,

    /// Global Dim
    #[id = "global_dim"]
    pub global_dim: BoolParam,

    // TODO: 添加 18 个通道的 Solo/Mute 状态参数
}

impl Default for MonitorParams {
    fn default() -> Self {
        Self {
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
