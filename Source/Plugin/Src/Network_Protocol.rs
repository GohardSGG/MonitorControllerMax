use serde::{Serialize, Deserialize};
use crate::Params::MAX_CHANNELS;
use crate::channel_logic::RenderState;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct NetworkRenderState {
    pub master_gain: f32,
    pub channel_gains: [f32; MAX_CHANNELS],
    pub channel_mute_mask: u32,
    pub timestamp: u64,
    pub magic: u16,
}

impl NetworkRenderState {
    pub fn from_render_state(state: &RenderState) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            master_gain: state.master_gain,
            channel_gains: state.channel_gains,
            channel_mute_mask: state.channel_mute_mask,
            timestamp,
            magic: 0x4D43, // Fixed: 0xMC is not valid syntax
        }
    }
}

