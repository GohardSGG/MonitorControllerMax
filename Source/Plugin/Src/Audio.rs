#![allow(non_snake_case)]

use nih_plug::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::Channel_Logic::ChannelLogic;
use crate::Config_Manager::ConfigManager;
use crate::Interaction::InteractionManager;

// ==================== 增益平滑状态（实例级）====================

/// 平滑系数：α = 0.1
/// 在 48kHz 下，约 50 采样（~1ms）达到 99% 目标值
const SMOOTHING_ALPHA: f32 = 0.1;

/// 增益平滑状态结构体（每个插件实例拥有独立的状态）
pub struct GainSmoothingState {
    /// 每通道当前增益状态（用于平滑过渡）
    /// 使用 AtomicU32 存储 f32 的位表示
    current_gains: [AtomicU32; MAX_CHANNELS],
}

impl GainSmoothingState {
    /// 创建新的增益平滑状态（初始增益为 1.0）
    pub fn new() -> Self {
        const INIT: AtomicU32 = AtomicU32::new(0x3F800000); // 1.0f32 的位表示
        Self {
            current_gains: [INIT; MAX_CHANNELS],
        }
    }

    /// 读取当前增益
    #[inline]
    pub fn load_gain(&self, ch: usize) -> f32 {
        f32::from_bits(self.current_gains[ch].load(Ordering::Relaxed))
    }

    /// 存储当前增益
    #[inline]
    pub fn store_gain(&self, ch: usize, value: f32) {
        self.current_gains[ch].store(value.to_bits(), Ordering::Relaxed);
    }
}

impl Default for GainSmoothingState {
    fn default() -> Self {
        Self::new()
    }
}

/// 音频处理核心逻辑
///
/// 注意：此函数只做音频处理，不涉及任何网络操作。
/// Master-Slave 同步由独立的网络线程处理（通过 InteractionManager）。
///
/// **优化**: 使用无分配的布局查询方法，避免在音频线程中分配内存
pub fn process_audio(
    buffer: &mut Buffer,
    params: &MonitorParams,
    gain_state: &GainSmoothingState,
    interaction: &Arc<InteractionManager>,
    layout_config: &ConfigManager,
) {
    let role = params.role.value();

    // 获取布局信息（使用无分配方法）
    let layout_idx = params.layout.value() as usize;
    let sub_layout_idx = params.sub_layout.value() as usize;

    // 无分配：直接获取 &str 引用
    let speaker_name = layout_config.get_speaker_name(layout_idx).unwrap_or("7.1.4");
    let sub_name = layout_config.get_sub_name(sub_layout_idx).unwrap_or("None");

    let layout = layout_config.get_layout(speaker_name, sub_name);

    // 计算 RenderState
    // - Master/Standalone: 从本地 InteractionManager 计算
    // - Slave: 同样从 InteractionManager 计算（已被网络线程同步更新）
    let render_state = match role {
        PluginRole::Master | PluginRole::Standalone => {
            // 本地计算
            ChannelLogic::compute(params, &layout, None, interaction)
        },
        PluginRole::Slave => {
            // Slave 的 InteractionManager 已被网络线程同步更新
            // 直接使用它计算状态
            ChannelLogic::compute(params, &layout, None, interaction)
        }
    };

    // 应用音频处理 (增益/静音) - 带平滑过渡
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
            let current_gain = gain_state.load_gain(ch_idx);
            let smoothed_gain = current_gain + (target_gain - current_gain) * SMOOTHING_ALPHA;
            gain_state.store_gain(ch_idx, smoothed_gain);

            // 应用平滑后的增益到采样点
            *sample *= smoothed_gain;
        }
    }
}
