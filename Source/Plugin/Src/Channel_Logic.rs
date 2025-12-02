use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::config_manager::Layout;
use crate::Interaction::get_interaction_manager;

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

pub struct ChannelLogic;

impl ChannelLogic {
    /// Pure function to compute RenderState from Params and Layout
    /// `override_role`: If Some, use this role instead of params.role
    pub fn compute(params: &MonitorParams, layout: &Layout, override_role: Option<PluginRole>) -> RenderState {
        let _role = override_role.unwrap_or(params.role.value());
        let master_gain = params.master_gain.value();
        let dim_active = params.dim.value();
        let cut_active = params.cut.value();

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

        // 2. 获取 InteractionManager
        let interaction = get_interaction_manager();

        // 3. 双路径音频处理
        if interaction.is_automation_mode() {
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
            // ========== 手动模式：使用 InteractionManager 状态机 ==========
            for i in 0..layout.total_channels {
                if i >= MAX_CHANNELS { break; }

                // 查找通道信息
                let channel_info = layout.main_channels.iter()
                    .chain(layout.sub_channels.iter())
                    .find(|ch| ch.channel_index == i);

                let is_sub = channel_info.map(|ch| ch.name.contains("SUB")).unwrap_or(false);

                // 核心：直接使用 InteractionManager 的状态
                let display = interaction.get_channel_display(i, is_sub);
                let pass = if display.has_sound { 1.0 } else { 0.0 };

                state.channel_gains[i] = pass;

                if pass < 0.5 {
                    state.channel_mute_mask |= 1 << i;
                }
            }
        }

        state
    }
}

