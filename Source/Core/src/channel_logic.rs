use crate::config_manager::Layout;
use crate::interaction::InteractionManager;
use crate::params::{MonitorParams, PluginRole, MAX_CHANNELS};

/// P3: 缓存行对齐的 RenderState
/// 64 字节对齐确保热数据在同一缓存行，减少 L3 未命中
#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct RenderState {
    pub master_gain: f32,
    pub channel_mute_mask: u32,
    // P3: 填充到 64 字节边界，确保 channel_gains 从新缓存行开始
    _padding: [u8; 56],
    pub channel_gains: [f32; MAX_CHANNELS],
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            master_gain: 1.0,
            channel_mute_mask: 0,
            _padding: [0; 56],
            channel_gains: [1.0; MAX_CHANNELS],
        }
    }
}

/// OSC 覆盖值（用于 Editor 关闭时仍能响应 OSC 控制）
#[derive(Clone, Copy, Debug, Default)]
pub struct OscOverride {
    pub master_volume: Option<f32>,
    pub dim: Option<bool>,
    pub cut: Option<bool>,
}

pub struct ChannelLogic;

impl ChannelLogic {
    /// Pure function to compute RenderState from Params and Layout
    /// `override_role`: If Some, use this role instead of params.role
    /// `interaction`: Reference to the InteractionManager for channel state
    /// `osc_override`: Optional OSC override values (used when Editor is closed)
    ///
    /// **音频线程优化**: 使用 Lock-Free 快照，避免任何锁操作
    #[inline]
    pub fn compute(
        params: &MonitorParams,
        layout: &Layout,
        override_role: Option<PluginRole>,
        interaction: &InteractionManager,
        osc_override: Option<OscOverride>,
    ) -> RenderState {
        let _role = override_role.unwrap_or(params.role.value());

        // 使用 OSC 覆盖值（如果有），否则使用 DAW 参数
        let (master_gain, dim_active, cut_active) = if let Some(osc) = osc_override {
            (
                osc.master_volume
                    .unwrap_or_else(|| params.master_gain.value()),
                osc.dim.unwrap_or_else(|| params.dim.value()),
                osc.cut.unwrap_or_else(|| params.cut.value()),
            )
        } else {
            (
                params.master_gain.value(),
                params.dim.value(),
                params.cut.value(),
            )
        };

        let mut state = RenderState::default();

        // P11 优化：Branchless 全局增益计算
        // 原代码（有分支）：
        // let global_gain = if cut_active { 0.0 }
        //     else if dim_active { master_gain * 0.1 }
        //     else { master_gain };
        //
        // 优化后（无分支）：
        // cut_active -> cut_mul = 0.0, 否则 1.0
        // dim_active -> dim_mul = 0.1, 否则 1.0
        // global_gain = master_gain * cut_mul * dim_mul
        let cut_mul = 1.0 - (cut_active as u8 as f32); // cut=true -> 0.0, cut=false -> 1.0
        let dim_mul = 1.0 - 0.9 * (dim_active as u8 as f32); // dim=true -> 0.1, dim=false -> 1.0
        let global_gain = master_gain * cut_mul * dim_mul;

        state.master_gain = global_gain;

        // 2. 获取 Lock-Free 快照（原子操作，无阻塞）
        let snapshot = interaction.get_snapshot();

        // 3. 双路径音频处理
        if snapshot.automation_mode {
            // ========== 自动化模式：直接读取 VST3 Enable 参数 ==========
            for i in 0..layout.total_channels {
                if i >= MAX_CHANNELS {
                    break;
                }

                let enable = params.channels[i].enable.value();
                state.channel_gains[i] = if enable { 1.0 } else { 0.0 };

                if !enable {
                    state.channel_mute_mask |= 1 << i;
                }
            }
        } else {
            // ========== 手动模式：使用快照进行纯函数计算（无锁）==========
            for i in 0..layout.total_channels {
                if i >= MAX_CHANNELS {
                    break;
                }

                // P5: O(1) 通道名称查找（替代 O(n) 的 find()）
                let lookup = &layout.channel_by_index[i];
                if lookup.valid {
                    let ch_name = lookup.as_str();
                    // 核心：使用快照的纯函数计算（无锁！）
                    let has_sound = snapshot.get_channel_state(ch_name, i);
                    let pass = if has_sound { 1.0 } else { 0.0 };

                    state.channel_gains[i] = pass;

                    if pass < 0.5 {
                        state.channel_mute_mask |= 1 << i;
                    }
                }
            }
        }

        state
    }
}
