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
    "L", "R", "C", "LFE", "LSS", "RSS", "LRS", "RRS",
    "LTF", "RTF", "LTB", "RTB",
    // SUB channels (统一使用下划线格式)
    "SUB_F", "SUB_B", "SUB_L", "SUB_R",
];

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub grid_pos: u32,
    pub channel_index: usize, // 0-based index in the audio buffer
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub main_channels: Vec<ChannelInfo>,  // 主声道（在网格中显示）
    pub sub_channels: Vec<ChannelInfo>,   // SUB 声道（在上下轨道中显示）
    pub total_channels: usize,
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

        Self { raw_config }
    }

    pub fn get_speaker_layouts(&self) -> Vec<String> {
        let mut names: Vec<String> = self.raw_config.speakers.keys().cloned().collect();
        // Sort for consistency? Or keep them as is? HashMap is unordered.
        // Let's sort them to make the dropdown stable.
        // We might want custom sorting (2.0 < 5.1 < 7.1.4), but alpha sort is a start.
        names.sort();
        names
    }

    pub fn get_sub_layouts(&self) -> Vec<String> {
        let mut names: Vec<String> = self.raw_config.subs.keys().cloned().collect();
        names.sort();
        names.insert(0, "None".to_string());
        names
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

        Layout {
            name: format!("{}+{}", speaker_name, sub_name),
            width,
            height,
            main_channels,
            sub_channels,
            total_channels,
        }
    }
}
