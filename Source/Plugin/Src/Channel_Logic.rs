use crate::Params::{MonitorParams, PluginRole, MAX_CHANNELS};
use crate::config_manager::Layout;

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
        let role = override_role.unwrap_or(params.role.value());
        let master_gain = params.master_gain.value();
        let dim_active = params.dim.value();
        let cut_active = params.cut.value();
        
        let mut state = RenderState::default();
        
        // 1. Global Level Processing
        let global_gain = if cut_active {
            0.0
        } else if dim_active {
            master_gain * 0.1 // -20dB approx (0.1 is -20dB in amplitude? 20log(0.1) = -20)
        } else {
            master_gain
        };
        
        state.master_gain = global_gain;

        // 2. Identify Groups and Sets
        let mut solo_set_main = false;
        let mut solo_set_sub = false;
        
        // We need to know which indices correspond to SUBs.
        // The layout.channels info tells us.
        // We can pre-calculate a mask or iterate. Iterating 32 is fast.
        
        // First pass: Analyze State
        for i in 0..layout.total_channels {
            if i >= MAX_CHANNELS { break; }
            
            let _is_sub = layout.channels[i].name.contains("SUB") || layout.channels[i].name.contains("LFE");
            // Wait, LFE is Main (Group M) according to doc!
            // "LFE (.1 通道): ... 它必须参与所有的 Solo/Mute 逻辑 ... 属于 Group M"
            // So only "SUB" (Bass Management) is Group S.
            // Is "LFE" distinct from "SUB"? In config "5.1", we have "LFE".
            // In "SUB" layouts, we have "SUB", "SUB L", "SUB R".
            // So logic: name contains "SUB" -> Group S. Else -> Group M.
            
            let real_is_sub = layout.channels[i].name.contains("SUB");
            
            if params.channels[i].solo.value() {
                if real_is_sub {
                    solo_set_sub = true;
                } else {
                    solo_set_main = true;
                }
            }
        }

        // 3. Compute Per-Channel Gain
        for i in 0..layout.total_channels {
            if i >= MAX_CHANNELS { break; }
            
            let is_sub = layout.channels[i].name.contains("SUB");
            let user_mute = params.channels[i].mute.value();
            let user_solo = params.channels[i].solo.value();
            let channel_trim = params.channels[i].gain.value(); // Channel Trim

            // Logic Core (v4.0 Spec)
            let pass = match role {
                PluginRole::Master => {
                    // Master Logic (Source Control)
                    if user_mute {
                        0.0
                    } else {
                        // AND conditions
                        let _cond1 = (solo_set_main && !is_sub && user_solo) || (!solo_set_main);
                        // Explanation:
                        // If SoloSet_Main is NOT empty: Only allow if I am in SoloSet_Main (and I am Main).
                        // If I am SUB, this condition doesn't block me here? 
                        // Wait, spec says:
                        // "For Main Channel i: ... AND ( (SoloSet_Main not empty AND i in SoloSet) OR ... )"
                        
                        let is_main = !is_sub;
                        
                        // Condition A: Main Channel Logic
                        let allow_main = if is_main {
                            if solo_set_main {
                                user_solo // Must be explicitly soloed
                            } else if solo_set_sub {
                                true // "没 Solo 主，但 Solo SUB，全通喂饱"
                            } else {
                                true // No solos
                            }
                        } else {
                            true // Master doesn't filter SUBs (generated downstream usually, but if present here, let pass)
                        };
                        
                        if allow_main { 1.0 } else { 0.0 }
                    }
                },
                PluginRole::Slave => {
                    // Slave Logic (Monitor Control)
                    // Pre-check: If Master cut it, it's cut (handled by audio chain).
                    // We just calculate "Monitor Mute".
                    
                    if user_mute {
                        0.0
                    } else {
                        let is_main = !is_sub;
                        
                        if is_main {
                            // Main Channel
                            if solo_set_main {
                                if user_solo { 1.0 } else { 0.0 }
                            } else if solo_set_sub {
                                0.0 // Solo Only SUB -> Auto-Mute Main
                            } else {
                                1.0
                            }
                        } else {
                            // SUB Channel
                            // "Pass IF: ... AND (SoloSet_Sub is empty OR i in SoloSet_Sub) ..."
                            // "AND (Master has any main channel open?)" -> "联动豁免: 有源才有声"
                            // This "Has Source" check is implicit in physics, but we can enforce mute if we know source is dead.
                            // But for simple logic:
                            
                            if solo_set_sub {
                                if user_solo { 1.0 } else { 0.0 }
                            } else {
                                1.0 // Default open (Immunity)
                            }
                        }
                    }
                }
            };

            state.channel_gains[i] = if pass > 0.5 { channel_trim } else { 0.0 };
            
            if pass < 0.5 {
                state.channel_mute_mask |= 1 << i;
            }
        }
        
        state
    }
}

