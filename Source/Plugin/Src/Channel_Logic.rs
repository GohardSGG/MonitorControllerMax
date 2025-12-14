use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::Config_Manager::Layout;
use crate::Interaction::InteractionManager;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderState {
    pub master_gain: f32,
    pub channel_gains: [f32; MAX_CHANNELS],
    // Bitmask for simple mute status (for network efficiency)
    // 1 = Muted/Auto-Muted, 0 = Open
    pub channel_mute_mask: u32,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            master_gain: 1.0,
            channel_gains: [1.0; MAX_CHANNELS],
            channel_mute_mask: 0,
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
                osc.master_volume.unwrap_or_else(|| params.master_gain.value()),
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

        // 1. Global Level Processing (两种模式都生效)
        let global_gain = if cut_active {
            0.0
        } else if dim_active {
            master_gain * 0.1 // -20dB approx (0.1 is -20dB in amplitude? 20log(0.1) = -20)
        } else {
            master_gain
        };

        state.master_gain = global_gain;

        // 2. 获取 Lock-Free 快照（原子操作，无阻塞）
        let snapshot = interaction.get_snapshot();

        // 3. 双路径音频处理
        if snapshot.automation_mode {
            // ========== 自动化模式：直接读取 VST3 Enable 参数 ==========
            for i in 0..layout.total_channels {
                if i >= MAX_CHANNELS { break; }

                let enable = params.channels[i].enable.value();
                state.channel_gains[i] = if enable { 1.0 } else { 0.0 };

                if !enable {
                    state.channel_mute_mask |= 1 << i;
                }
            }
        } else {
            // ========== 手动模式：使用快照进行纯函数计算（无锁）==========
            for i in 0..layout.total_channels {
                if i >= MAX_CHANNELS { break; }

                // 查找通道信息
                let channel_info = layout.main_channels.iter()
                    .chain(layout.sub_channels.iter())
                    .find(|ch| ch.channel_index == i);

                if let Some(ch_info) = channel_info {
                    // 核心：使用快照的纯函数计算（无锁！）
                    let has_sound = snapshot.get_channel_state(&ch_info.name, i);
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
