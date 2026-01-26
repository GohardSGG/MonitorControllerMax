use crate::params::MAX_CHANNELS;
use serde::Deserialize;
use std::collections::HashMap;

// Embed the default config
const DEFAULT_CONFIG_JSON: &str = include_str!("../../Resource/Speaker_Config.json");

// UTF-8 BOM 字符
const UTF8_BOM: &str = "\u{FEFF}";

/// 标准通道顺序 - 唯一真实来源（Single Source of Truth）
/// Main 通道 (0-11) + SUB 通道 (12-15)
/// 注意：SUB 通道统一使用下划线格式 "SUB_F", "SUB_B" 等（不使用空格）
pub const STANDARD_CHANNEL_ORDER: &[&str] = &[
    // Main channels (7.1.4)
    "L", "R", "C", "LFE", "LSS", "RSS", "LRS", "RRS", "LTF", "RTF", "LTB", "RTB",
    // SUB channels (统一使用下划线格式)
    "SUB_F", "SUB_B", "SUB_L", "SUB_R",
];

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub grid_pos: u32,
    pub channel_index: usize, // 0-based index in the audio buffer
}

/// P5: 预计算的通道名称查找表条目
/// 使用固定大小字符串避免堆分配
#[derive(Debug, Clone, Copy)]
pub struct ChannelLookupEntry {
    /// 通道名称（固定 8 字节，足够存储 "SUB_F" 等）
    pub name: [u8; 8],
    /// 名称实际长度
    pub name_len: u8,
    /// 是否有效
    pub valid: bool,
}

impl Default for ChannelLookupEntry {
    fn default() -> Self {
        Self {
            name: [0; 8],
            name_len: 0,
            valid: false,
        }
    }
}

impl ChannelLookupEntry {
    /// 从字符串创建查找条目
    fn from_str(s: &str) -> Self {
        let mut entry = Self::default();
        let bytes = s.as_bytes();
        let len = bytes.len().min(8);
        entry.name[..len].copy_from_slice(&bytes[..len]);
        entry.name_len = len as u8;
        entry.valid = true;
        entry
    }

    /// 获取通道名称作为 &str
    #[inline]
    pub fn as_str(&self) -> &str {
        if self.valid {
            // SAFETY: 我们只存储有效的 UTF-8 字符串
            unsafe { std::str::from_utf8_unchecked(&self.name[..self.name_len as usize]) }
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    #[allow(dead_code)]
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub main_channels: Vec<ChannelInfo>, // 主声道（在网格中显示）
    pub sub_channels: Vec<ChannelInfo>,  // SUB 声道（在上下轨道中显示）
    pub total_channels: usize,
    /// P5: O(1) 通道名称查找表
    /// channel_by_index[i] = 通道 i 的名称（用于快速查找）
    pub channel_by_index: [ChannelLookupEntry; MAX_CHANNELS],
}

#[derive(Deserialize, Debug)]
struct RawConfig {
    #[serde(rename = "Speaker")]
    speakers: HashMap<String, HashMap<String, serde_json::Value>>,
    #[serde(rename = "SUB")]
    subs: HashMap<String, HashMap<String, u32>>,
}

pub struct ConfigManager {
    raw_config: RawConfig,
    /// 缓存的排序后的 speaker 布局名称（避免重复排序和分配）
    cached_speaker_names: Vec<String>,
    /// 缓存的排序后的 SUB 布局名称（包含 "None"）
    cached_sub_names: Vec<String>,
}

impl ConfigManager {
    pub fn new() -> Self {
        // In a real scenario, we might try to load from a user file first.
        // For now, we just use the embedded default.
        // FAIL-SAFE: Do not panic here.

        // 移除可能存在的 UTF-8 BOM 字符
        let json_str = DEFAULT_CONFIG_JSON.trim_start_matches(UTF8_BOM);

        let raw_config: RawConfig = match serde_json::from_str(json_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                // If parsing fails, create an empty safe config to prevent crash
                // We can't log here easily if logger isn't ready, but we avoid panic.
                eprintln!("CRITICAL: Failed to parse default config: {}", e);
                RawConfig {
                    speakers: HashMap::new(),
                    subs: HashMap::new(),
                }
            }
        };

        // 预计算并缓存排序后的名称列表
        let mut cached_speaker_names: Vec<String> = raw_config.speakers.keys().cloned().collect();
        cached_speaker_names.sort();

        let mut cached_sub_names: Vec<String> = raw_config.subs.keys().cloned().collect();
        cached_sub_names.sort();
        cached_sub_names.insert(0, "None".to_string());

        Self {
            raw_config,
            cached_speaker_names,
            cached_sub_names,
        }
    }

    pub fn get_speaker_layouts(&self) -> Vec<String> {
        // 返回缓存的克隆（为了向后兼容保留此方法）
        self.cached_speaker_names.clone()
    }

    pub fn get_sub_layouts(&self) -> Vec<String> {
        // 返回缓存的克隆（为了向后兼容保留此方法）
        self.cached_sub_names.clone()
    }

    /// 获取 speaker 布局名称（无分配，用于音频线程）
    #[inline]
    pub fn get_speaker_name(&self, idx: usize) -> Option<&str> {
        self.cached_speaker_names.get(idx).map(|s| s.as_str())
    }

    /// 获取 SUB 布局名称（无分配，用于音频线程）
    #[inline]
    pub fn get_sub_name(&self, idx: usize) -> Option<&str> {
        self.cached_sub_names.get(idx).map(|s| s.as_str())
    }

    pub fn get_layout(&self, speaker_name: &str, sub_name: &str) -> Layout {
        let mut main_channels = Vec::new();
        let mut sub_channels = Vec::new();
        let mut channel_idx = 0;
        let mut width = 5;
        let mut height = 5;

        // Process Speaker Layout
        if let Some(layout_map) = self.raw_config.speakers.get(speaker_name) {
            // Extract Size
            if let Some(serde_json::Value::String(size_str)) = layout_map.get("Size") {
                if let Some((w, h)) = size_str.split_once('x') {
                    width = w.parse().unwrap_or(5);
                    height = h.parse().unwrap_or(5);
                }
            }

            // Extract Channels
            // We need a stable order for channel indices.
            // The JSON object is unordered. The C++ implementation relied on order?
            // "恢复之前的逻辑：按顺序递增分配通道索引。注意：这个逻辑依赖于JSON中属性的顺序，可能不稳定。"
            // In Rust, serde_json::Map preserves order if "preserve_order" feature is enabled.
            // But we are using HashMap here which does NOT preserve order.
            // To preserve order, we should assume standard channel orders (L, R, C, LFE...).
            // OR, we assume the user doesn't care about internal routing order as long as it's consistent.
            // BUT, routing usually follows a standard (e.g. SMPTE).
            // Let's try to sort by standard names or grid position?
            // Actually, for a Monitor Controller, the input order matters (DAW output order).
            // 5.1 is usually L, R, C, LFE, Ls, Rs.
            // If we iterate HashMap, L might come after R.
            // WE NEED TO FIX THIS. The config parsing must be deterministic and preferably standard-compliant.

            // For now, let's collect and sort by Grid Position? No, Grid Position is for UI.
            // L (1) should be ch 0. R (5) should be ch 1? No, 5.1 standard is L, R, C, LFE...
            // Let's look at the JSON again.
            // "L": 1, "R": 3 (in 2.0).
            // If we sort by keys, C comes before L.

            // 使用公共常量作为唯一真实来源（Single Source of Truth）
            let standard_order = STANDARD_CHANNEL_ORDER;

            // First pass: Standard channels
            for key in standard_order.iter() {
                if let Some(val) = layout_map.get(*key) {
                    if let Some(grid_pos) = val.as_u64() {
                        main_channels.push(ChannelInfo {
                            name: key.to_string(),
                            grid_pos: grid_pos as u32,
                            channel_index: channel_idx,
                        });
                        channel_idx += 1;
                    }
                }
            }

            // Second pass: Any other channels not in standard list?
            for (k, v) in layout_map {
                if k != "Size" && !standard_order.contains(&k.as_str()) {
                    if let Some(grid_pos) = v.as_u64() {
                        main_channels.push(ChannelInfo {
                            name: k.clone(),
                            grid_pos: grid_pos as u32,
                            channel_index: channel_idx,
                        });
                        channel_idx += 1;
                    }
                }
            }
        }

        // Process SUB Layout
        if sub_name != "None" {
            if let Some(sub_map) = self.raw_config.subs.get(sub_name) {
                // Similar issue with order.
                // Let's just sort keys alphabetically for SUBs? "SUB L", "SUB R".
                let mut keys: Vec<_> = sub_map.keys().collect();
                keys.sort();

                for key in keys {
                    let grid_pos = sub_map[key];
                    sub_channels.push(ChannelInfo {
                        name: key.clone(),
                        grid_pos: grid_pos, // 1-6 对应上下轨道的 6 个位置
                        channel_index: channel_idx,
                    });
                    channel_idx += 1;
                }
            }
        }

        let total_channels = main_channels.len() + sub_channels.len();

        // P5: 构建 O(1) 查找表
        let mut channel_by_index = [ChannelLookupEntry::default(); MAX_CHANNELS];
        for ch in main_channels.iter().chain(sub_channels.iter()) {
            if ch.channel_index < MAX_CHANNELS {
                channel_by_index[ch.channel_index] = ChannelLookupEntry::from_str(&ch.name);
            }
        }

        // H3: 空配置降级 - 如果没有任何通道，返回最小立体声配置
        if total_channels == 0 {
            eprintln!(
                "[MCM] WARNING: Empty layout '{}+{}', falling back to stereo",
                speaker_name, sub_name
            );
            let mut fallback_lookup = [ChannelLookupEntry::default(); MAX_CHANNELS];
            fallback_lookup[0] = ChannelLookupEntry::from_str("L");
            fallback_lookup[1] = ChannelLookupEntry::from_str("R");
            return Layout {
                name: "Fallback_Stereo".to_string(),
                width: 3,
                height: 1,
                main_channels: vec![
                    ChannelInfo {
                        name: "L".to_string(),
                        grid_pos: 1,
                        channel_index: 0,
                    },
                    ChannelInfo {
                        name: "R".to_string(),
                        grid_pos: 3,
                        channel_index: 1,
                    },
                ],
                sub_channels: vec![],
                total_channels: 2,
                channel_by_index: fallback_lookup,
            };
        }

        Layout {
            name: format!("{}+{}", speaker_name, sub_name),
            width,
            height,
            main_channels,
            sub_channels,
            total_channels,
            channel_by_index,
        }
    }

    /// 根据通道数自动查找最匹配的布局索引
    /// 返回匹配的 speaker layout 索引，如果没有找到则返回 None
    pub fn find_layout_for_channels(&self, target_channels: usize) -> Option<i32> {
        // 遍历所有 speaker 布局，找到通道数完全匹配的
        for (idx, speaker_name) in self.cached_speaker_names.iter().enumerate() {
            // 获取该布局的通道数（不包含 SUB）
            if let Some(layout_map) = self.raw_config.speakers.get(speaker_name) {
                // 计算通道数（排除 Size 字段）
                let channel_count = layout_map
                    .iter()
                    .filter(|(k, v)| *k != "Size" && v.is_u64())
                    .count();

                if channel_count == target_channels {
                    return Some(idx as i32);
                }
            }
        }

        // 如果没有精确匹配，查找最接近但不超过目标的布局
        let mut best_idx: Option<i32> = None;
        let mut best_count: usize = 0;

        for (idx, speaker_name) in self.cached_speaker_names.iter().enumerate() {
            if let Some(layout_map) = self.raw_config.speakers.get(speaker_name) {
                let channel_count = layout_map
                    .iter()
                    .filter(|(k, v)| *k != "Size" && v.is_u64())
                    .count();

                // 找最接近但不超过目标的
                if channel_count <= target_channels && channel_count > best_count {
                    best_count = channel_count;
                    best_idx = Some(idx as i32);
                }
            }
        }

        best_idx
    }
}
