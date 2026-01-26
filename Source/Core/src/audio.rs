#![allow(non_snake_case)]

use crate::channel_logic::{ChannelLogic, OscOverride};
use crate::config_manager::Layout;
use crate::interaction::InteractionManager;
use crate::osc_state::OscSharedState;
use crate::params::{MonitorParams, PluginRole, MAX_CHANNELS};
use atomic_float::AtomicF32;
use nih_plug::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// ==================== P2 优化：批量增益平滑 ====================

/// 平滑系数：α = 0.1
/// 在 48kHz 下，约 50 采样（~1ms）达到 99% 目标值
const SMOOTHING_ALPHA: f32 = 0.1;

/// P2 优化：批量更新间隔
/// 每 8 个样本更新一次增益（而非每样本），减少 87% 的平滑计算
const SMOOTHING_UPDATE_INTERVAL: usize = 8;

/// 增益平滑状态结构体（每个插件实例拥有独立的状态）
/// P7: 使用 AtomicF32（无位转换开销）
#[repr(align(64))] // P3: 缓存行对齐，避免 false sharing
pub struct GainSmoothingState {
    /// 每通道当前增益状态（用于平滑过渡）
    /// P7: 直接使用 AtomicF32，消除 to_bits/from_bits 转换
    current_gains: [AtomicF32; MAX_CHANNELS],
}

impl GainSmoothingState {
    /// 创建新的增益平滑状态（初始增益为 1.0）
    pub fn new() -> Self {
        const INIT: AtomicF32 = AtomicF32::new(1.0);
        Self {
            current_gains: [INIT; MAX_CHANNELS],
        }
    }

    /// 读取当前增益
    #[inline]
    pub fn load_gain(&self, ch: usize) -> f32 {
        self.current_gains[ch].load(Ordering::Relaxed)
    }

    /// 存储当前增益
    #[inline]
    pub fn store_gain(&self, ch: usize, value: f32) {
        self.current_gains[ch].store(value, Ordering::Relaxed);
    }
}

impl Default for GainSmoothingState {
    fn default() -> Self {
        Self::new()
    }
}

/// 音频处理核心逻辑 - P2/P3/P4/P6/P7 极致优化版本
///
/// 优化点：
/// - P2: 批量增益平滑（每 8 样本更新，减少 87% 计算）
/// - P3: 本地增益缓存（栈上数组，避免原子操作）
/// - P4: Branchless 静音计算（消除分支预测失败）
/// - P6: 通道优先处理（as_slice），连续内存访问，LLVM 自动向量化
/// - P7: AtomicF32 无位转换
/// - 无内存分配，纯栈操作
#[allow(dead_code)]
pub fn process_audio(
    buffer: &mut Buffer,
    params: &MonitorParams,
    gain_state: &GainSmoothingState,
    interaction: &Arc<InteractionManager>,
    layout_config: &crate::config_manager::ConfigManager,
    osc_state: Option<&Arc<OscSharedState>>,
) {
    let role = params.role.value();

    // 获取布局信息（使用无分配方法）
    let layout_idx = params.layout.value() as usize;
    let sub_layout_idx = params.sub_layout.value() as usize;

    // 无分配：直接获取 &str 引用
    let speaker_name = layout_config
        .get_speaker_name(layout_idx)
        .unwrap_or("7.1.4");
    let sub_name = layout_config.get_sub_name(sub_layout_idx).unwrap_or("None");

    let layout = layout_config.get_layout(speaker_name, sub_name);

    // === P9 优化：使用合并的 get_override_snapshot（减少原子操作）===
    let osc_override = osc_state.and_then(|osc| {
        osc.get_override_snapshot()
            .map(|(vol, dim, cut)| OscOverride {
                master_volume: Some(vol),
                dim: Some(dim),
                cut: Some(cut),
            })
    });

    // 计算 RenderState
    let render_state = match role {
        PluginRole::Master | PluginRole::Standalone => {
            ChannelLogic::compute(params, &layout, None, interaction, osc_override)
        }
        PluginRole::Slave => ChannelLogic::compute(params, &layout, None, interaction, None),
    };

    // P2/P3 优化：使用本地增益缓存（栈上，无堆分配）
    let mut local_gains = [0.0f32; MAX_CHANNELS];
    let mut target_gains = [0.0f32; MAX_CHANNELS];

    let num_channels = layout.total_channels.min(MAX_CHANNELS);

    // 初始化本地增益 + P4: Branchless 预计算目标增益
    for ch_idx in 0..num_channels {
        local_gains[ch_idx] = gain_state.load_gain(ch_idx);

        // P4: Branchless 目标增益计算
        let is_muted_bit = ((render_state.channel_mute_mask >> ch_idx) & 1) as f32;
        let muted_multiplier = 1.0 - is_muted_bit;

        target_gains[ch_idx] =
            render_state.master_gain * render_state.channel_gains[ch_idx] * muted_multiplier;
    }

    // P6: 通道优先处理 - 使用 as_slice() 获取连续内存访问
    // 这比 iter_samples() 更高效，因为：
    // 1. 连续内存访问更好的 L1 缓存命中率
    // 2. LLVM 可以对内层循环自动向量化
    let channel_slices = buffer.as_slice();
    let block_len = channel_slices.first().map(|s| s.len()).unwrap_or(0);

    // P2: 计算平滑更新次数（每 8 样本更新一次）
    let num_updates = (block_len + SMOOTHING_UPDATE_INTERVAL - 1) / SMOOTHING_UPDATE_INTERVAL;

    for ch_idx in 0..num_channels.min(channel_slices.len()) {
        let samples = &mut channel_slices[ch_idx];
        let mut current_gain = local_gains[ch_idx];
        let target = target_gains[ch_idx];

        // P2/P6: 分块处理 - 每 8 样本更新一次增益
        for update_idx in 0..num_updates {
            // 更新平滑值（每块开始时）
            current_gain += (target - current_gain) * SMOOTHING_ALPHA;

            // 计算本块的样本范围
            let start = update_idx * SMOOTHING_UPDATE_INTERVAL;
            let end = (start + SMOOTHING_UPDATE_INTERVAL).min(block_len);

            // P6: 内层循环 - 连续内存访问，LLVM 可自动向量化
            // 注意：这个循环处理连续的 8 个样本，编译器会优化为 SIMD
            for sample in &mut samples[start..end] {
                *sample *= current_gain;
            }
        }

        // 保存最终增益值
        local_gains[ch_idx] = current_gain;
    }

    // P2: 只在 block 结束时写回原子状态
    for ch_idx in 0..num_channels {
        gain_state.store_gain(ch_idx, local_gains[ch_idx]);
    }
}

/// P8 优化版：使用预计算的 Layout 避免每帧堆分配
///
/// 此函数与 process_audio 功能相同，但接收预计算的 Layout 引用
/// 而不是每次调用 get_layout() 进行堆分配
#[inline(always)]
pub fn process_audio_with_layout(
    buffer: &mut Buffer,
    params: &MonitorParams,
    gain_state: &GainSmoothingState,
    interaction: &Arc<InteractionManager>,
    layout: &Layout,
    osc_state: Option<&Arc<OscSharedState>>,
) {
    let role = params.role.value();

    // === P9 优化：使用合并的 get_override_snapshot（减少原子操作）===
    let osc_override = osc_state.and_then(|osc| {
        osc.get_override_snapshot()
            .map(|(vol, dim, cut)| OscOverride {
                master_volume: Some(vol),
                dim: Some(dim),
                cut: Some(cut),
            })
    });

    // 计算 RenderState
    let render_state = match role {
        PluginRole::Master | PluginRole::Standalone => {
            ChannelLogic::compute(params, layout, None, interaction, osc_override)
        }
        PluginRole::Slave => ChannelLogic::compute(params, layout, None, interaction, None),
    };

    // P2/P3 优化：使用本地增益缓存（栈上，无堆分配）
    let mut local_gains = [0.0f32; MAX_CHANNELS];
    let mut target_gains = [0.0f32; MAX_CHANNELS];

    let num_channels = layout.total_channels.min(MAX_CHANNELS);

    // 初始化本地增益 + P4: Branchless 预计算目标增益
    for ch_idx in 0..num_channels {
        local_gains[ch_idx] = gain_state.load_gain(ch_idx);

        // P4: Branchless 目标增益计算
        let is_muted_bit = ((render_state.channel_mute_mask >> ch_idx) & 1) as f32;
        let muted_multiplier = 1.0 - is_muted_bit;

        target_gains[ch_idx] =
            render_state.master_gain * render_state.channel_gains[ch_idx] * muted_multiplier;
    }

    // P6: 通道优先处理 - 使用 as_slice() 获取连续内存访问
    let channel_slices = buffer.as_slice();
    let block_len = channel_slices.first().map(|s| s.len()).unwrap_or(0);

    // P2: 计算平滑更新次数（每 8 样本更新一次）
    let num_updates = (block_len + SMOOTHING_UPDATE_INTERVAL - 1) / SMOOTHING_UPDATE_INTERVAL;

    for ch_idx in 0..num_channels.min(channel_slices.len()) {
        let samples = &mut channel_slices[ch_idx];
        let mut current_gain = local_gains[ch_idx];
        let target = target_gains[ch_idx];

        // P2/P6: 分块处理 - 每 8 样本更新一次增益
        for update_idx in 0..num_updates {
            // 更新平滑值（每块开始时）
            current_gain += (target - current_gain) * SMOOTHING_ALPHA;

            // 计算本块的样本范围
            let start = update_idx * SMOOTHING_UPDATE_INTERVAL;
            let end = (start + SMOOTHING_UPDATE_INTERVAL).min(block_len);

            // P6: 内层循环 - 连续内存访问，LLVM 可自动向量化
            for sample in &mut samples[start..end] {
                *sample *= current_gain;
            }
        }

        // 保存最终增益值
        local_gains[ch_idx] = current_gain;
    }

    // P2: 只在 block 结束时写回原子状态
    for ch_idx in 0..num_channels {
        gain_state.store_gain(ch_idx, local_gains[ch_idx]);
    }
}
