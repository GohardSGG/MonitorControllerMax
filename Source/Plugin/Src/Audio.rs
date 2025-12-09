#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::channel_logic::{ChannelLogic, RenderState};
use crate::network::NetworkManager;
use crate::network_protocol::NetworkRenderState;
use crate::config_manager::CONFIG;

// ==================== 增益平滑状态 ====================
// 使用 AtomicU32 存储 f32 的位表示，实现无锁线程安全

/// 每通道当前增益状态（用于平滑过渡）
static CURRENT_GAINS: [AtomicU32; MAX_CHANNELS] = {
    const INIT: AtomicU32 = AtomicU32::new(0x3F800000); // 1.0f32 的位表示
    [INIT; MAX_CHANNELS]
};

/// 平滑系数：α = 0.1
/// 在 48kHz 下，约 50 采样（~1ms）达到 99% 目标值
/// 足够快以保持监听控制器的响应性，又足够慢以避免咔哒声
const SMOOTHING_ALPHA: f32 = 0.1;

/// 读取当前增益
#[inline]
fn load_gain(ch: usize) -> f32 {
    f32::from_bits(CURRENT_GAINS[ch].load(Ordering::Relaxed))
}

/// 存储当前增益
#[inline]
fn store_gain(ch: usize, value: f32) {
    CURRENT_GAINS[ch].store(value.to_bits(), Ordering::Relaxed);
}

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

    // 2. 应用音频处理 (增益/静音) - 带平滑过渡
    // 遍历每个采样帧，对每个通道应用平滑增益
    for channel_samples in buffer.iter_samples() {
        for (ch_idx, sample) in channel_samples.into_iter().enumerate() {
            if ch_idx >= MAX_CHANNELS {
                break;
            }

            // 检查该通道是否被静音
            let is_muted = (render_state.channel_mute_mask >> ch_idx) & 1 == 1;

            // 获取该通道的目标增益
            let ch_gain = render_state.channel_gains[ch_idx];

            // 计算最终目标增益 = 主增益 × 通道增益 × (静音 ? 0 : 1)
            let target_gain = if is_muted {
                0.0
            } else {
                render_state.master_gain * ch_gain
            };

            // 指数平滑：current = current + (target - current) * α
            // α = 0.1，在 48kHz 下约 1ms 达到 99% 目标值
            let current_gain = load_gain(ch_idx);
            let smoothed_gain = current_gain + (target_gain - current_gain) * SMOOTHING_ALPHA;
            store_gain(ch_idx, smoothed_gain);

            // 应用平滑后的增益到采样点
            *sample *= smoothed_gain;
        }
    }
}
