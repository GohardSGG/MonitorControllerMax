#![allow(non_snake_case)]

use nih_plug::prelude::*;
use crate::Params::MonitorParams;

/// 音频处理核心逻辑
/// 
/// # 参数
/// * `buffer` - 音频缓冲区
/// * `params` - 参数引用
pub fn process_audio(buffer: &mut Buffer, params: &MonitorParams) {
    let gain = params.master_gain.value();
    let is_muted = params.global_mute.value();
    
    // 快速路径：如果静音，直接清空缓冲区
    if is_muted {
        for channel in buffer.as_slice() {
            channel.fill(0.0);
        }
        return;
    }

    // 应用 Dim
    let final_gain = if params.global_dim.value() {
        gain * 0.158 // -16dB approx
    } else {
        gain
    };

    // 应用增益
    if (final_gain - 1.0).abs() > f32::EPSILON {
        for channel in buffer.as_slice() {
            for sample in channel.iter_mut() {
                *sample *= final_gain;
            }
        }
    }
}

