use serde::{Serialize, Deserialize};
use std::collections::HashSet;

/// 标准通道名称列表（用于位掩码映射）
pub const STANDARD_CHANNELS: &[&str] = &[
    // Main 通道 (0-11)
    "L", "R", "C", "LFE", "LSS", "RSS", "LRS", "RRS", "LTF", "RTF", "LTB", "RTB",
    // SUB 通道 (12-15)
    "SUB_F", "SUB_B", "SUB_L", "SUB_R",
];

/// Master-Slave 同步的交互状态
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct NetworkInteractionState {
    /// 主模式: 0=None, 1=Solo, 2=Mute
    pub primary: u8,
    /// 比较模式: 0=None, 1=Solo, 2=Mute
    pub compare: u8,
    /// Solo 通道集合（位掩码，对应 STANDARD_CHANNELS）
    pub solo_mask: u32,
    /// Mute 通道集合（位掩码）
    pub mute_mask: u32,
    /// SUB User Mute 集合（位掩码，bit 0-3 对应 SUB_F/B/L/R）
    pub user_mute_sub_mask: u8,
    /// 主音量 (0.0-1.0)
    pub master_gain: f32,
    /// Dim 开关
    pub dim: bool,
    /// Cut 开关
    pub cut: bool,
    // === v2.5.0 新增字段 ===
    /// 布局索引（Speaker Layout）
    pub layout: i32,
    /// SUB 布局索引
    pub sub_layout: i32,
    /// Solo 记忆标志（Compare 模式中用户是否修改过 Solo 集合）
    pub solo_has_memory: bool,
    /// Mute 记忆标志（Compare 模式中用户是否修改过 Mute 集合）
    pub mute_has_memory: bool,
    /// 是否处于自动化模式
    pub automation_mode: bool,
    // ======================
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 魔数校验
    pub magic: u16,
}

impl NetworkInteractionState {
    pub const MAGIC: u16 = 0x4D43; // "MC"

    /// 从通道名称集合创建位掩码
    pub fn channel_set_to_mask(set: &HashSet<String>) -> u32 {
        let mut mask: u32 = 0;
        for (idx, name) in STANDARD_CHANNELS.iter().enumerate() {
            if set.contains(*name) {
                mask |= 1 << idx;
            }
        }
        mask
    }

    /// 从位掩码还原通道名称集合
    pub fn mask_to_channel_set(mask: u32) -> HashSet<String> {
        let mut set = HashSet::new();
        for (idx, name) in STANDARD_CHANNELS.iter().enumerate() {
            if (mask >> idx) & 1 == 1 {
                set.insert(name.to_string());
            }
        }
        set
    }

    /// SUB 通道名称到位索引的映射
    pub fn sub_name_to_bit(name: &str) -> Option<u8> {
        match name {
            "SUB_F" => Some(0),
            "SUB_B" => Some(1),
            "SUB_L" => Some(2),
            "SUB_R" => Some(3),
            _ => None,
        }
    }

    /// 从 SUB 集合创建位掩码
    pub fn sub_set_to_mask(set: &HashSet<String>) -> u8 {
        let mut mask: u8 = 0;
        for name in set.iter() {
            if let Some(bit) = Self::sub_name_to_bit(name) {
                mask |= 1 << bit;
            }
        }
        mask
    }

    /// 从位掩码还原 SUB 集合
    pub fn mask_to_sub_set(mask: u8) -> HashSet<String> {
        let mut set = HashSet::new();
        let sub_names = ["SUB_F", "SUB_B", "SUB_L", "SUB_R"];
        for (idx, name) in sub_names.iter().enumerate() {
            if (mask >> idx) & 1 == 1 {
                set.insert(name.to_string());
            }
        }
        set
    }

    /// 创建带时间戳的新状态
    pub fn with_timestamp(mut self) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.magic = Self::MAGIC;
        self
    }

    /// 验证魔数
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

