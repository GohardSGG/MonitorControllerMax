#![allow(non_snake_case)]

use nih_plug::prelude::*;
// Removed unused Arc and AtomicCell imports
use crate::Params::{MonitorParams, PluginRole};
use crate::channel_logic::{ChannelLogic, RenderState};
use crate::network::NetworkManager;
use crate::network_protocol::NetworkRenderState;
use crate::config_manager::CONFIG;

// Global Network Manager (Lazy initialized in Lib.rs or manually managed)
// For simplicity, we can use a lazy_static here or put it in the plugin struct.
// Since nih_plug plugins are re-instantiated, putting it in plugin struct is better.
// But we need to initialize it.
// Let's assume Lib.rs initializes it. Or Audio.rs does.
// Audio processor needs access to it.

// For now, let's create a thread-local or static for simplicity in demo, 
// BUT for production, this should be owned by the plugin instance.
// In Lib.rs, we didn't add NetworkManager to struct MonitorControllerMax. We should.

/// 音频处理核心逻辑
pub fn process_audio(
    buffer: &mut Buffer, 
    params: &MonitorParams, 
    network: &mut NetworkManager
) {
    let role = params.role.value();
    
    // 1. Determine RenderState
    let render_state = match role {
        PluginRole::Master | PluginRole::Standalone => {
            // A. Compute Logic Locally
            // Standalone behaves like Master but without network broadcasting
            let layout_idx = params.layout.value() as usize;
            let sub_layout_idx = params.sub_layout.value() as usize;

            let speaker_names = CONFIG.get_speaker_layouts();
            let sub_names = CONFIG.get_sub_layouts();

            let speaker_name = speaker_names.get(layout_idx).map(|s| s.as_str()).unwrap_or("7.1.4");
            let sub_name = sub_names.get(sub_layout_idx).map(|s| s.as_str()).unwrap_or("None");

            let layout = CONFIG.get_layout(speaker_name, sub_name);

            let state = ChannelLogic::compute(params, &layout, None);

            // B. Broadcast to Network (only for Master, not Standalone)
            if role == PluginRole::Master {
                if let Some(sender) = &network.sender {
                    let net_state = NetworkRenderState::from_render_state(&state);
                    // Try send, don't block audio thread
                    // unbounded channel is non-blocking
                    let _ = sender.send(net_state);
                }
            }

            state
        },
        PluginRole::Slave => {
            // A. Read from Network Cache (Fail-Safe)
            // "State Retention": If no new data, use last known good state.
            // AtomicCell load is lock-free.
            if let Some(net_state) = network.latest_state.load() {
                // Convert NetworkRenderState back to RenderState
                RenderState {
                    master_gain: net_state.master_gain,
                    channel_gains: net_state.channel_gains,
                    channel_mute_mask: net_state.channel_mute_mask,
                }
            } else {
                // Initial state / Disconnected for too long?
                // Default to mute or unity? Default trait is unity gain.
                // Safest is maybe Mute? Or Unity?
                // Let's use default (Unity, no mute) or Mute.
                // If I am a slave and I haven't heard from Master, I should probably shut up.
                let mut s = RenderState::default();
                s.master_gain = 0.0; // Safety mute
                s
            }
        }
    };

    // 2. Apply Audio Processing (Gain/Mute)
    let _num_samples = buffer.samples();
    let _num_channels = buffer.channels();

    for (channel_idx, channel_data) in buffer.iter_samples().enumerate() {
        if channel_idx >= crate::Params::MAX_CHANNELS {
            break;
        }

        // Check Mute Mask
        let is_muted = (render_state.channel_mute_mask >> channel_idx) & 1 == 1;
        
        // Get Channel Gain (includes Trim)
        let ch_gain = render_state.channel_gains[channel_idx];
        
        // Final Gain = Master * Channel * (Mute ? 0 : 1)
        // Note: Master Gain is already applied in Logic? 
        // No, Logic computed `global_gain` into `state.master_gain`.
        // Slave logic needs to apply it.
        
        let target_gain = if is_muted {
            0.0
        } else {
            render_state.master_gain * ch_gain
        };

        // Apply to all samples (No smoothing for now, add smoothing later)
        // In nih_plug, params have smoothers, but here we calculated gain manually.
        // For production, we need a smoothed gain follower.
        // For prototype, direct multiply.
        
        for sample in channel_data {
            *sample *= target_gain;
        }
    }
}
