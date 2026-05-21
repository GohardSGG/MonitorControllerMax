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

/// 平滑系数：α = 0.005
/// 在 48kHz 下，约 500 采样（~10ms）达到 99% 目标值
const SMOOTHING_ALPHA: f32 = 0.005;

// const SMOOTHING_UPDATE_INTERVAL: usize = 8; // Deprecated by Real P0 Fix

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
        const INIT: AtomicF32 = AtomicF32::new(0.0); // 默认静音，避免启动爆音
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
    // 注意：效果器开关（low_boost 等）通过 pending 机制同步到 params，不放在 OscOverride 里
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
    // let block_len = ... (Unused)

    // P2: REVERTED Block smoothing (Zipper Noise) -> Per-sample smoothing

    for ch_idx in 0..num_channels.min(channel_slices.len()) {
        let samples = &mut channel_slices[ch_idx];
        let mut current_gain = local_gains[ch_idx];
        let target = target_gains[ch_idx];

        // P2/P6 Real Fix: Per-sample smoothing (High Fidelity)
        for sample in samples.iter_mut() {
            current_gain += (target - current_gain) * SMOOTHING_ALPHA;
            *sample *= current_gain;
        }

        // 保存最终增益值
        local_gains[ch_idx] = current_gain;
    }

    // P2: 只在 block 结束时写回原子状态
    // P2: 只在 block 结束时写回原子状态
    // P0 修复：脏检查优化原子写入，减少总线流量
    for ch_idx in 0..num_channels {
        let new_val = local_gains[ch_idx];
        if (gain_state.load_gain(ch_idx) - new_val).abs() > 1e-5 {
            gain_state.store_gain(ch_idx, new_val);
        }
    }
}
