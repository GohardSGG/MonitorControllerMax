#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLedState {
    Off = 0,  // 不亮
    Mute = 1, // 红色
    Solo = 2, // 绿色
}

/// OSC 输出消息类型
#[derive(Debug, Clone)]
pub enum OscOutMessage {
    /// 通道 LED 状态: channel name, state (0=off, 1=mute/red, 2=solo/green)
    ChannelLed {
        channel: String,
        state: ChannelLedState,
    },

    /// Solo Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeSolo { on: bool },

    /// Mute Mode 按钮状态: on (1.0 = active/blinking, 0.0 = off)
    ModeMute { on: bool },

    /// Master Volume 值: 0.0 to 1.0
    MasterVolume { value: f32 },

    /// Dim 状态: on (1.0 = active, 0.0 = off)
    Dim { on: bool },

    /// Cut 状态: on (1.0 = active, 0.0 = off)
    Cut { on: bool },

    /// Mono 状态: on (1.0 = active, 0.0 = off)
    Mono { on: bool },

    /// LFE +10dB 状态: on (1.0 = active, 0.0 = off)
    LfeAdd10dB { on: bool },

    /// Low Boost 状态: on (1.0 = active, 0.0 = off)
    LowBoost { on: bool },

    /// High Boost 状态: on (1.0 = active, 0.0 = off)
    HighBoost { on: bool },

    /// 广播所有状态 (初始化时使用)
    BroadcastAll {
        channel_count: usize,
        master_volume: f32,
        dim: bool,
        cut: bool,
        /// 预计算的通道 LED 状态: (通道名, 状态) 其中状态 0=off, 1=mute, 2=solo
        channel_states: Vec<(String, u8)>,
        /// Solo 模式是否激活
        solo_active: bool,
        /// Mute 模式是否激活
        mute_active: bool,
    },
}
