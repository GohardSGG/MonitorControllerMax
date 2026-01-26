//! InteractionManager - 交互状态机
//!
//! 实现 v4.0 规范的核心交互逻辑：
//! - 主模式: SoloActive (常亮绿), MuteActive (常亮红)
//! - 比较模式: 在主模式基础上叠加另一个模式 (闪烁)
//! - 通道操作: 始终操作当前激活的 Context (闪烁的那个优先)

use crossbeam::atomic::AtomicCell;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mcm_infra::logger::InstanceLogger;
use mcm_protocol::config::AppConfig;
use mcm_protocol::network_structs::NetworkInteractionState;
use mcm_protocol::web_structs::WebRestartAction;

// ========== Lock-Free 音频线程快照 ==========

/// 音频线程使用的 Lock-Free 快照
/// 所有字段都是简单值类型，可以原子复制
/// C12 修复：明确 16 字节对齐，确保 AtomicCell 在所有平台上正确工作
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct RenderSnapshot {
    /// 主模式: 0=None, 1=Solo, 2=Mute
    pub primary: u8,
    /// 比较模式: 0=None, 1=Solo, 2=Mute
    pub compare: u8,
    /// 是否自动化模式
    pub automation_mode: bool,
    /// 填充到 4 字节边界
    _padding1: u8,
    /// Solo 通道掩码（位图）
    pub solo_mask: u32,
    /// Mute 通道掩码（位图）
    pub mute_mask: u32,
    /// SUB User Mute 掩码（位图，bit 0-3）
    pub user_mute_sub_mask: u8,
    /// 填充到 16 字节
    _padding2: [u8; 3],
}

impl RenderSnapshot {
    /// 根据通道名获取显示状态（纯函数，无锁）
    /// 返回 (has_sound, is_solo_marker, is_mute_marker)
    #[inline]
    pub fn get_channel_state(&self, ch_name: &str, ch_idx: usize) -> bool {
        // Idle 状态 = 全通
        if self.primary == 0 {
            return true;
        }

        let is_sub = ch_name.starts_with("SUB");

        // SUB 通道逻辑
        if is_sub {
            // 检查 User Mute（优先级最高）
            let sub_bit = match ch_name {
                "SUB_F" => 0,
                "SUB_B" => 1,
                "SUB_L" => 2,
                "SUB_R" => 3,
                _ => return true,
            };
            if (self.user_mute_sub_mask >> sub_bit) & 1 == 1 {
                return false; // User Muted
            }

            // SUB 使用 Primary 模式的集合
            let (context_is_solo, active_mask) = if self.primary == 1 {
                (true, self.solo_mask)
            } else {
                (false, self.mute_mask)
            };

            let is_in_set = (active_mask >> ch_idx) & 1 == 1;
            let sub_mask = active_mask >> 12; // SUB 通道从索引 12 开始
            let sub_set_has_any = sub_mask & 0xF != 0;
            let main_set_has_any = active_mask & 0xFFF != 0;

            if !main_set_has_any && !sub_set_has_any {
                return true; // 空集合 = 全通
            }

            if context_is_solo {
                if sub_set_has_any {
                    is_in_set
                } else {
                    true // 豁免权
                }
            } else {
                // Mute context
                if sub_set_has_any {
                    !is_in_set
                } else {
                    true // 豁免权
                }
            }
        } else {
            // Main 通道逻辑：比较模式优先
            let (context_is_solo, active_mask) = if self.compare == 1 {
                (true, self.solo_mask)
            } else if self.compare == 2 {
                (false, self.mute_mask)
            } else if self.primary == 1 {
                (true, self.solo_mask)
            } else {
                (false, self.mute_mask)
            };

            let is_in_set = (active_mask >> ch_idx) & 1 == 1;
            let main_set_has_any = active_mask & 0xFFF != 0;

            if !main_set_has_any {
                return true; // 空集合 = 全通
            }

            if context_is_solo {
                is_in_set
            } else {
                !is_in_set
            }
        }
    }
}

/// 主模式 - 先进入的模式，常亮
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryMode {
    /// 无主模式
    None,
    /// Solo 为主模式 (绿色常亮)
    Solo,
    /// Mute 为主模式 (红色常亮)
    Mute,
}

impl Default for PrimaryMode {
    fn default() -> Self {
        PrimaryMode::None
    }
}

/// 比较模式 - 后进入的模式，闪烁
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// 无比较模式
    None,
    /// Solo 为比较模式 (绿色闪烁)
    Solo,
    /// Mute 为比较模式 (红色闪烁)
    Mute,
}

impl Default for CompareMode {
    fn default() -> Self {
        CompareMode::None
    }
}

/// 通道集合 - 使用通道名称存储（基于 HashSet）
#[derive(Debug, Clone, Default)]
pub struct ChannelSet {
    /// 通道名称集合（存储通道名如 "L", "R", "LBF", "SUB_F" 等）
    channels: std::collections::HashSet<String>,
}

impl ChannelSet {
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.channels.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// 切换通道状态
    pub fn toggle(&mut self, ch_name: &str) {
        if self.channels.contains(ch_name) {
            self.channels.remove(ch_name);
        } else {
            self.channels.insert(ch_name.to_string());
        }
    }

    /// 设置通道状态（true=加入集合，false=移除）
    pub fn set(&mut self, ch_name: &str, on: bool) {
        if on {
            self.channels.insert(ch_name.to_string());
        } else {
            self.channels.remove(ch_name);
        }
    }

    /// 检查通道是否在集合中
    pub fn contains(&self, ch_name: &str) -> bool {
        self.channels.contains(ch_name)
    }

    /// 获取所有通道名称的迭代器
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.channels.iter()
    }

    /// 检查集合中是否有任何 SUB 通道
    pub fn has_any_sub(&self) -> bool {
        self.channels.iter().any(|name| name.starts_with("SUB"))
    }

    /// 检查集合中是否有任何 Main 通道（非 SUB）
    pub fn has_any_main(&self) -> bool {
        self.channels.iter().any(|name| !name.starts_with("SUB"))
    }
}

/// 双击检测器
pub struct DoubleClickDetector {
    last_click_time: Option<Instant>,
    last_click_channel: Option<usize>,
    threshold: Duration,
}

impl DoubleClickDetector {
    pub fn new() -> Self {
        Self {
            last_click_time: None,
            last_click_channel: None,
            threshold: Duration::from_millis(300),
        }
    }

    /// 检测是否为双击，返回 true 表示双击
    pub fn check(&mut self, channel: usize) -> bool {
        let now = Instant::now();

        if let (Some(last_time), Some(last_ch)) = (self.last_click_time, self.last_click_channel) {
            if last_ch == channel && now.duration_since(last_time) < self.threshold {
                self.last_click_time = None;
                self.last_click_channel = None;
                return true;
            }
        }

        self.last_click_time = Some(now);
        self.last_click_channel = Some(channel);
        false
    }
}

/// 交互状态管理器
pub struct InteractionManager {
    /// 主模式 (常亮的那个)
    primary: RwLock<PrimaryMode>,

    /// 比较模式 (闪烁的那个)
    compare: RwLock<CompareMode>,

    /// Solo 通道集合 (Context A)
    solo_set: RwLock<ChannelSet>,

    /// Mute 通道集合 (Context B)
    mute_set: RwLock<ChannelSet>,

    /// Solo 上下文是否有用户记忆 (用户在比较模式中修改过)
    solo_has_memory: RwLock<bool>,

    /// Mute 上下文是否有用户记忆
    mute_has_memory: RwLock<bool>,

    /// User Mute SUB (SUB 双击/长按强制静音) - 存储被静音的 SUB 通道名称
    user_mute_sub: RwLock<std::collections::HashSet<String>>,

    /// 双击检测器
    double_click: RwLock<DoubleClickDetector>,

    /// 闪烁计数器 (用于动画)
    blink_counter: AtomicU32,

    /// 自动化模式 (是否处于自动化控制模式)
    automation_mode: RwLock<bool>,

    /// 实例级日志器
    logger: Arc<InstanceLogger>,

    // ========== 网络同步状态 (Slave 接收 Master 数据用) ==========
    /// 网络接收的主音量
    network_master_gain: RwLock<Option<f32>>,
    /// 网络接收的 Dim 状态
    network_dim: RwLock<Option<bool>>,
    /// 网络接收的 Cut 状态
    network_cut: RwLock<Option<bool>>,
    /// 网络接收的布局索引
    network_layout: RwLock<Option<i32>>,
    /// 网络接收的 SUB 布局索引
    network_sub_layout: RwLock<Option<i32>>,

    // ========== OSC Hot Reload ==========
    /// 待应用的新配置（用于 OSC 端口热重载）
    osc_restart_config: RwLock<Option<AppConfig>>,
    /// P1 优化：快速路径标志，避免每个 block 都获取 RwLock
    osc_restart_pending: AtomicBool,

    // ========== Network Hot Reload ==========
    /// 待应用的新配置（用于 Network 端口/IP 热重载）
    network_restart_config: RwLock<Option<AppConfig>>,
    /// P1 优化：快速路径标志，避免每个 block 都获取 RwLock
    network_restart_pending: AtomicBool,

    // ========== Web Hot Reload ==========
    /// Web 服务器重启动作
    web_restart_action: RwLock<Option<WebRestartAction>>,
    /// P1 优化：快速路径标志
    web_restart_pending: AtomicBool,

    // ========== Lock-Free 音频线程快照 ==========
    /// 音频线程使用的原子快照（无锁读取）
    render_snapshot: AtomicCell<RenderSnapshot>,
}

impl InteractionManager {
    pub fn new(logger: Arc<InstanceLogger>) -> Self {
        Self {
            primary: RwLock::new(PrimaryMode::None),
            compare: RwLock::new(CompareMode::None),
            solo_set: RwLock::new(ChannelSet::new()),
            mute_set: RwLock::new(ChannelSet::new()),
            solo_has_memory: RwLock::new(false),
            mute_has_memory: RwLock::new(false),
            user_mute_sub: RwLock::new(std::collections::HashSet::new()),
            double_click: RwLock::new(DoubleClickDetector::new()),
            blink_counter: AtomicU32::new(0),
            automation_mode: RwLock::new(false),
            logger,
            // 网络同步状态初始化
            network_master_gain: RwLock::new(None),
            network_dim: RwLock::new(None),
            network_cut: RwLock::new(None),
            network_layout: RwLock::new(None),
            network_sub_layout: RwLock::new(None),
            // OSC Hot Reload 初始化
            osc_restart_config: RwLock::new(None),
            osc_restart_pending: AtomicBool::new(false),
            // Network Hot Reload 初始化
            network_restart_config: RwLock::new(None),
            network_restart_pending: AtomicBool::new(false),
            // Web Hot Reload 初始化
            web_restart_action: RwLock::new(None),
            web_restart_pending: AtomicBool::new(false),
            // Lock-Free 快照初始化
            render_snapshot: AtomicCell::new(RenderSnapshot::default()),
        }
    }

    // ========== 状态查询 ==========

    #[allow(dead_code)]
    pub fn is_idle(&self) -> bool {
        *self.primary.read() == PrimaryMode::None
    }

    pub fn get_primary(&self) -> PrimaryMode {
        *self.primary.read()
    }

    pub fn get_compare(&self) -> CompareMode {
        *self.compare.read()
    }

    /// SOLO 按钮是否常亮
    pub fn is_solo_steady(&self) -> bool {
        *self.primary.read() == PrimaryMode::Solo
    }

    /// SOLO 按钮是否闪烁
    pub fn is_solo_blinking(&self) -> bool {
        *self.compare.read() == CompareMode::Solo
    }

    /// MUTE 按钮是否常亮
    pub fn is_mute_steady(&self) -> bool {
        *self.primary.read() == PrimaryMode::Mute
    }

    /// MUTE 按钮是否闪烁
    pub fn is_mute_blinking(&self) -> bool {
        *self.compare.read() == CompareMode::Mute
    }

    /// 获取 Solo 集合
    #[allow(dead_code)]
    pub fn get_solo_set(&self) -> ChannelSet {
        self.solo_set.read().clone()
    }

    /// 获取 Mute 集合
    #[allow(dead_code)]
    pub fn get_mute_set(&self) -> ChannelSet {
        self.mute_set.read().clone()
    }

    /// 检查通道是否在 Solo 集合中
    #[allow(dead_code)]
    pub fn is_in_solo_set(&self, ch_name: &str) -> bool {
        self.solo_set.read().contains(ch_name)
    }

    /// 检查通道是否在 Mute 集合中
    #[allow(dead_code)]
    pub fn is_in_mute_set(&self, ch_name: &str) -> bool {
        self.mute_set.read().contains(ch_name)
    }

    // ========== Lock-Free 快照方法（音频线程使用）==========

    /// 获取当前快照（音频线程调用，无锁）
    /// P14: 使用 #[inline(always)] 确保音频线程调用被内联
    #[inline(always)]
    pub fn get_snapshot(&self) -> RenderSnapshot {
        self.render_snapshot.load()
    }

    /// 更新快照（UI/网络线程在状态变化后调用）
    /// M1: 优化为快速读取所有锁，然后释放锁后再计算
    /// P3 优化：原地计算掩码，不再克隆 HashSet
    pub fn update_snapshot(&self) {
        // P3 优化：直接在持有锁时计算掩码，避免 HashSet 克隆
        let primary = *self.primary.read();
        let compare = *self.compare.read();
        let automation = *self.automation_mode.read();

        // 原地计算掩码（不克隆 HashSet）
        let solo_mask = Self::channel_set_to_mask(&self.solo_set.read().channels);
        let mute_mask = Self::channel_set_to_mask(&self.mute_set.read().channels);
        let user_mute_sub_mask = Self::sub_set_to_mask(&self.user_mute_sub.read());

        let snapshot = RenderSnapshot {
            primary: match primary {
                PrimaryMode::None => 0,
                PrimaryMode::Solo => 1,
                PrimaryMode::Mute => 2,
            },
            compare: match compare {
                CompareMode::None => 0,
                CompareMode::Solo => 1,
                CompareMode::Mute => 2,
            },
            automation_mode: automation,
            _padding1: 0,
            solo_mask,
            mute_mask,
            user_mute_sub_mask,
            _padding2: [0; 3],
        };

        // 原子写入快照
        self.render_snapshot.store(snapshot);
    }

    /// 通道名称集合转位掩码
    fn channel_set_to_mask(channels: &HashSet<String>) -> u32 {
        let channel_names = [
            "L", "R", "C", "LFE", "LSS", "RSS", "LRS", "RRS", "LTF", "RTF", "LTB", "RTB", "SUB_F",
            "SUB_B", "SUB_L", "SUB_R",
        ];
        let mut mask: u32 = 0;
        for (i, name) in channel_names.iter().enumerate() {
            if channels.contains(*name) {
                mask |= 1 << i;
            }
        }
        mask
    }

    /// SUB 集合转位掩码
    fn sub_set_to_mask(subs: &HashSet<String>) -> u8 {
        let mut mask: u8 = 0;
        if subs.contains("SUB_F") {
            mask |= 1 << 0;
        }
        if subs.contains("SUB_B") {
            mask |= 1 << 1;
        }
        if subs.contains("SUB_L") {
            mask |= 1 << 2;
        }
        if subs.contains("SUB_R") {
            mask |= 1 << 3;
        }
        mask
    }

    // ========== 自动化模式管理 ==========

    /// 检查是否处于自动化模式
    pub fn is_automation_mode(&self) -> bool {
        *self.automation_mode.read()
    }

    /// 进入自动化模式（清空所有状态机状态）
    pub fn enter_automation_mode(&self) {
        *self.solo_set.write() = ChannelSet::new();
        *self.mute_set.write() = ChannelSet::new();
        *self.primary.write() = PrimaryMode::None;
        *self.compare.write() = CompareMode::None;
        *self.solo_has_memory.write() = false;
        *self.mute_has_memory.write() = false;
        self.user_mute_sub.write().clear();
        *self.automation_mode.write() = true;
        self.update_snapshot();
    }

    /// 退出自动化模式（保持清空状态）
    pub fn exit_automation_mode(&self) {
        *self.automation_mode.write() = false;
        // 不恢复任何状态，保持 Idle
        self.update_snapshot();
    }

    /// H2: 布局变化时清理旧状态（防止旧通道状态污染新布局）
    /// 在手动模式下，布局切换时调用
    pub fn clear_on_layout_change(&self) {
        // 清空 Solo/Mute 集合
        self.solo_set.write().channels.clear();
        self.mute_set.write().channels.clear();
        self.user_mute_sub.write().clear();

        // 重置模式状态为 Idle
        *self.primary.write() = PrimaryMode::None;
        *self.compare.write() = CompareMode::None;
        *self.solo_has_memory.write() = false;
        *self.mute_has_memory.write() = false;

        // 更新快照
        self.update_snapshot();
    }

    // ========== 辅助函数 ==========

    /// 拷贝集合到比较模式 (只拷贝 Main 通道，SUB 不参与)
    /// 逻辑：被 Solo 的通道 -> 变成被 Mute 的通道（相同的通道集合）
    fn copy_set(&self, source: &ChannelSet) -> ChannelSet {
        // 只拷贝 Main 通道（过滤掉 SUB）
        let channels: HashSet<String> = source
            .channels
            .iter()
            .filter(|name| !name.starts_with("SUB"))
            .cloned()
            .collect();

        ChannelSet { channels }
    }

    // ========== 全局按钮操作 ==========

    /// SOLO 按钮点击
    pub fn on_solo_button_click(&self) {
        // 先读取当前状态
        let current_primary = *self.primary.read();
        let current_compare = *self.compare.read();

        // 根据状态决定要做什么
        enum Action {
            SetSoloActive,
            ExitToIdle,
            EnterSoloCompare,
            ExitSoloCompare,
            ExitAll,
            None,
        }

        let action = match (current_primary, current_compare) {
            (PrimaryMode::None, CompareMode::None) => Action::SetSoloActive,
            (PrimaryMode::Solo, CompareMode::None) => Action::ExitToIdle,
            (PrimaryMode::Mute, CompareMode::None) => Action::EnterSoloCompare,
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitSoloCompare, // 点击 Compare 按钮，只退出 Compare
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitAll, // 点击 Primary 按钮，完全退出
            _ => Action::None,
        };

        // 执行操作（分开获取锁，避免死锁）
        match action {
            Action::SetSoloActive => {
                *self.primary.write() = PrimaryMode::Solo;
            }
            Action::ExitToIdle => {
                // 彻底退出到 Idle
                *self.primary.write() = PrimaryMode::None;
                *self.compare.write() = CompareMode::None;
                self.solo_set.write().clear();
                self.mute_set.write().clear();
                *self.solo_has_memory.write() = false;
                *self.mute_has_memory.write() = false;
                self.user_mute_sub.write().clear();
            }
            Action::EnterSoloCompare => {
                // 从 Mute Active 进入 Solo Compare
                // 关键：自动反转逻辑（拷贝 mute_set 到 solo_set）
                let has_memory = *self.solo_has_memory.read();

                if !has_memory {
                    // 如果 Solo 没有记忆，执行自动反转（拷贝）
                    let mute_set = self.mute_set.read().clone();
                    let copied = self.copy_set(&mute_set);
                    *self.solo_set.write() = copied;
                }
                // 如果有记忆，保留上次的 solo_set

                *self.compare.write() = CompareMode::Solo;
            }
            Action::ExitSoloCompare => {
                // 退出 Solo Compare，回到 Mute Active
                *self.compare.write() = CompareMode::None;
                // 注意：不清除 solo_set，保留记忆
            }
            Action::ExitAll => {
                // 在比较模式下点击常亮按钮 = 完全退出
                *self.primary.write() = PrimaryMode::None;
                *self.compare.write() = CompareMode::None;
                self.solo_set.write().clear();
                self.mute_set.write().clear();
                *self.solo_has_memory.write() = false;
                *self.mute_has_memory.write() = false;
                self.user_mute_sub.write().clear();
            }
            Action::None => {}
        }

        // 记录状态变化（在所有操作完成后）
        let new_primary = *self.primary.read();
        let new_compare = *self.compare.read();
        let solo_count = self.solo_set.read().channels.len();
        let mute_count = self.mute_set.read().channels.len();
        self.logger.info(
            "interaction",
            &format!(
                "[SM] SOLO: ({:?},{:?})->({:?},{:?}) solo_count={} mute_count={}",
                current_primary, current_compare, new_primary, new_compare, solo_count, mute_count
            ),
        );

        // 更新 Lock-Free 快照
        self.update_snapshot();
    }

    /// MUTE 按钮点击
    pub fn on_mute_button_click(&self) {
        // 先读取当前状态
        let current_primary = *self.primary.read();
        let current_compare = *self.compare.read();

        // 根据状态决定要做什么
        enum Action {
            SetMuteActive,
            ExitToIdle,
            EnterMuteCompare,
            ExitMuteCompare,
            ExitAll,
            None,
        }

        let action = match (current_primary, current_compare) {
            (PrimaryMode::None, CompareMode::None) => Action::SetMuteActive,
            (PrimaryMode::Mute, CompareMode::None) => Action::ExitToIdle,
            (PrimaryMode::Solo, CompareMode::None) => Action::EnterMuteCompare,
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitMuteCompare, // 点击 Compare 按钮，只退出 Compare
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitAll, // 点击 Primary 按钮，完全退出
            _ => Action::None,
        };

        // 执行操作（分开获取锁，避免死锁）
        match action {
            Action::SetMuteActive => {
                *self.primary.write() = PrimaryMode::Mute;
            }
            Action::ExitToIdle => {
                // 彻底退出到 Idle
                *self.primary.write() = PrimaryMode::None;
                *self.compare.write() = CompareMode::None;
                self.solo_set.write().clear();
                self.mute_set.write().clear();
                *self.solo_has_memory.write() = false;
                *self.mute_has_memory.write() = false;
                self.user_mute_sub.write().clear();
            }
            Action::EnterMuteCompare => {
                // 从 Solo Active 进入 Mute Compare
                // 关键：自动反转逻辑（拷贝 solo_set 到 mute_set）
                let has_memory = *self.mute_has_memory.read();

                if !has_memory {
                    // 如果 Mute 没有记忆，执行自动反转（拷贝）
                    let solo_set = self.solo_set.read().clone();
                    let copied = self.copy_set(&solo_set);
                    *self.mute_set.write() = copied;
                }
                // 如果有记忆，保留上次的 mute_set

                *self.compare.write() = CompareMode::Mute;
            }
            Action::ExitMuteCompare => {
                // 退出 Mute Compare，回到 Solo Active
                *self.compare.write() = CompareMode::None;
                // 注意：不清除 mute_set，保留记忆
            }
            Action::ExitAll => {
                // 在比较模式下点击常亮按钮 = 完全退出
                *self.primary.write() = PrimaryMode::None;
                *self.compare.write() = CompareMode::None;
                self.solo_set.write().clear();
                self.mute_set.write().clear();
                *self.solo_has_memory.write() = false;
                *self.mute_has_memory.write() = false;
                self.user_mute_sub.write().clear();
            }
            Action::None => {}
        }

        // 记录状态变化（在所有操作完成后）
        let new_primary = *self.primary.read();
        let new_compare = *self.compare.read();
        let solo_count = self.solo_set.read().channels.len();
        let mute_count = self.mute_set.read().channels.len();
        self.logger.info(
            "interaction",
            &format!(
                "[SM] MUTE: ({:?},{:?})->({:?},{:?}) solo_count={} mute_count={}",
                current_primary, current_compare, new_primary, new_compare, solo_count, mute_count
            ),
        );

        // 更新 Lock-Free 快照
        self.update_snapshot();
    }

    // ========== 通道操作 ==========

    /// 获取当前应该操作的 Context 类型
    /// 返回 None 表示 Idle，不应操作
    /// 比较模式优先（闪烁的那个）
    fn get_active_context(&self) -> Option<ActiveContext> {
        let primary = *self.primary.read();
        let compare = *self.compare.read();

        // 比较模式优先
        match compare {
            CompareMode::Solo => return Some(ActiveContext::Solo),
            CompareMode::Mute => return Some(ActiveContext::Mute),
            CompareMode::None => {}
        }

        // 否则看主模式
        match primary {
            PrimaryMode::Solo => Some(ActiveContext::Solo),
            PrimaryMode::Mute => Some(ActiveContext::Mute),
            PrimaryMode::None => None,
        }
    }

    /// 通道点击
    pub fn on_channel_click(&self, ch_name: &str) -> bool {
        let primary = *self.primary.read();
        let compare = *self.compare.read();
        let ctx = self.get_active_context();

        let result = match ctx {
            None => {
                // Idle 状态，什么都不做
                false
            }
            Some(ActiveContext::Solo) => {
                // 修改 Solo 集合
                self.solo_set.write().toggle(ch_name);

                // 记忆标记逻辑
                match (primary, compare) {
                    (PrimaryMode::Solo, CompareMode::None) => {
                        // Solo 是主模式，修改后脏化 Mute 记忆
                        *self.mute_has_memory.write() = false;
                    }
                    (PrimaryMode::Mute, CompareMode::Solo) => {
                        // Solo 是比较模式，设置 Solo 记忆
                        *self.solo_has_memory.write() = true;
                    }
                    _ => {}
                }

                true
            }
            Some(ActiveContext::Mute) => {
                // 修改 Mute 集合
                self.mute_set.write().toggle(ch_name);

                // 记忆标记逻辑
                match (primary, compare) {
                    (PrimaryMode::Mute, CompareMode::None) => {
                        // Mute 是主模式，修改后脏化 Solo 记忆
                        *self.solo_has_memory.write() = false;
                    }
                    (PrimaryMode::Solo, CompareMode::Mute) => {
                        // Mute 是比较模式，设置 Mute 记忆
                        *self.mute_has_memory.write() = true;
                    }
                    _ => {}
                }

                true
            }
        };

        // 记录通道点击日志
        if result {
            let solo_count = self
                .solo_set
                .read()
                .iter()
                .filter(|n| !n.starts_with("SUB"))
                .count();
            let mute_count = self
                .mute_set
                .read()
                .iter()
                .filter(|n| !n.starts_with("SUB"))
                .count();
            self.logger.info(
                "interaction",
                &format!(
                    "[CH] {} click: solo_count={} mute_count={}",
                    ch_name, solo_count, mute_count
                ),
            );

            // 更新 Lock-Free 快照
            self.update_snapshot();
        }

        result
    }

    /// 直接设置通道状态（用于 OSC 目标状态模式）
    /// state: 0=Off, 1=Mute, 2=Solo（在当前上下文中）
    ///
    /// 通道索引映射：
    /// - 0-11: Main 通道 (L, R, C, LFE, LSS, RSS, LRS, RRS, LTF, RTF, LTB, RTB)
    /// - 12-15: SUB 通道 (SUB_F, SUB_B, SUB_L, SUB_R) → 内部索引 0-3
    pub fn set_channel_state(&self, ch_name: &str, state: u8) {
        let ctx = self.get_active_context();

        match ctx {
            None => {
                // Idle 状态，忽略
            }
            Some(ActiveContext::Solo) => {
                let mut solo_set = self.solo_set.write();
                match state {
                    0 => {
                        // Off = 移除 Solo
                        solo_set.set(ch_name, false);
                    }
                    2 => {
                        // Solo = 加入 Solo
                        solo_set.set(ch_name, true);
                    }
                    _ => {}
                }
            }
            Some(ActiveContext::Mute) => {
                let mut mute_set = self.mute_set.write();
                match state {
                    0 => {
                        // Off = 移除 Mute
                        mute_set.set(ch_name, false);
                    }
                    1 => {
                        // Mute = 加入 Mute
                        mute_set.set(ch_name, true);
                    }
                    _ => {}
                }
            }
        }
        // 更新 Lock-Free 快照
        self.update_snapshot();
    }

    /// 设置通道声音状态（语义层，用于 Group_Dial）
    /// has_sound: true = 有声音, false = 没声音
    /// can_exit_if_empty: true = 集合变空时自动退出模式, false = 仅增量操作
    /// 根据当前 ActiveContext 正确解释语义
    pub fn set_channel_sound(&self, ch_name: &str, has_sound: bool, can_exit_if_empty: bool) {
        let ctx = self.get_active_context();

        match ctx {
            Some(ActiveContext::Solo) => {
                // Solo 上下文：有声音 = 加入 Solo，没声音 = 移除 Solo
                self.solo_set.write().set(ch_name, has_sound);
            }
            Some(ActiveContext::Mute) => {
                // Mute 上下文：有声音 = 移除 Mute，没声音 = 加入 Mute
                self.mute_set.write().set(ch_name, !has_sound);
            }
            None => {
                // Idle 状态，忽略（C# 应该先发送模式激活）
                return;
            }
        }

        // 仅当 can_exit_if_empty=true 时检查并退出空模式
        // 这允许增量操作（value=11）不会意外退出模式
        if can_exit_if_empty {
            self.check_and_exit_empty_mode();
        }

        // 更新 Lock-Free 快照
        self.update_snapshot();
    }

    /// 检查集合是否变空，自动退出对应模式
    fn check_and_exit_empty_mode(&self) {
        let solo_empty = self.solo_set.read().is_empty();
        let mute_empty = self.mute_set.read().is_empty();

        // 检查 Solo 模式
        if solo_empty && *self.primary.read() == PrimaryMode::Solo {
            *self.primary.write() = PrimaryMode::None;
            self.logger.info(
                "interaction",
                "[Mode] Solo set empty, auto-exiting Solo mode",
            );
        }

        // 检查 Mute 模式
        if mute_empty && *self.primary.read() == PrimaryMode::Mute {
            *self.primary.write() = PrimaryMode::None;
            self.logger.info(
                "interaction",
                "[Mode] Mute set empty, auto-exiting Mute mode",
            );
        }
    }

    /// SUB 双击 - User Mute (强制静音，优先级最高)
    pub fn on_sub_double_click(&self, ch_name: &str) -> bool {
        if ch_name.starts_with("SUB") {
            let mut user_mute = self.user_mute_sub.write();
            let was_muted = user_mute.contains(ch_name);

            if was_muted {
                user_mute.remove(ch_name);
                self.logger.info(
                    "interaction",
                    &format!("[CH] {} dblclick: user_mute removed", ch_name),
                );
            } else {
                user_mute.insert(ch_name.to_string());
                self.logger.info(
                    "interaction",
                    &format!("[CH] {} dblclick: user_mute added", ch_name),
                );
            }
            // 更新 Lock-Free 快照
            self.update_snapshot();
            true
        } else {
            false
        }
    }

    /// 检测 SUB 点击类型
    pub fn detect_sub_click(&self, ch: usize) -> SubClickType {
        let mut detector = self.double_click.write();
        if detector.check(ch) {
            SubClickType::DoubleClick
        } else {
            SubClickType::SingleClick
        }
    }

    // ========== 通道状态计算 (用于显示) ==========

    /// 计算通道的显示状态
    /// 关键规则：
    /// - Main 通道：使用当前激活的上下文（比较模式优先）
    /// - SUB 通道：**永远使用 Primary 模式的集合**（不参与自动反转）
    /// - 闪烁：Compare 模式下激活的通道需要闪烁
    pub fn get_channel_display(&self, ch_name: &str) -> ChannelDisplay {
        let primary = *self.primary.read();
        let compare = *self.compare.read();
        let is_sub = ch_name.starts_with("SUB");

        // Idle 状态：全部灰色（UI），但音频全通（has_sound = true）
        if primary == PrimaryMode::None {
            return ChannelDisplay {
                has_sound: true, // ← 修复：Idle = 全通
                marker: None,
                is_blinking: false,
            };
        }

        // === SUB 特殊逻辑 (Group S) ===
        // SUB 永远不参与自动反转，始终使用 Primary 模式的集合
        if is_sub {
            // 检查 User Mute（优先级最高）
            let user_mute = self.user_mute_sub.read();
            if user_mute.contains(ch_name) {
                return ChannelDisplay {
                    has_sound: false,
                    marker: Some(ChannelMarker::Mute),
                    is_blinking: false, // User Mute 不闪烁
                };
            }

            // SUB 使用 Primary 模式的集合（不管是否在 Compare 模式）
            let (sub_context_type, active_set) = match primary {
                PrimaryMode::Solo => (ContextType::Solo, self.solo_set.read()),
                PrimaryMode::Mute => (ContextType::Mute, self.mute_set.read()),
                PrimaryMode::None => unreachable!(),
            };

            let is_in_sub_set = active_set.contains(ch_name);
            let sub_set_has_any = active_set.has_any_sub();
            let main_set_has_any = active_set.has_any_main();

            // 关键逻辑：
            // 1. 如果 Main 和 SUB 组都没有状态 → SUB 灰色
            // 2. 如果只有 Main 有状态 → SUB 豁免权（绿色）
            // 3. 如果 SUB 组有状态 → SUB 组内竞争
            // 4. 如果只有 SUB 有状态（Main 无状态）→ SUB 组内竞争

            if !main_set_has_any && !sub_set_has_any {
                return ChannelDisplay {
                    has_sound: true, // 空集合 = 全通（不处理）
                    marker: None,
                    is_blinking: false,
                };
            }

            let marker = match sub_context_type {
                ContextType::Solo => {
                    if sub_set_has_any {
                        if is_in_sub_set {
                            Some(ChannelMarker::Solo)
                        } else {
                            Some(ChannelMarker::Mute)
                        }
                    } else if main_set_has_any {
                        Some(ChannelMarker::Solo) // 豁免权
                    } else {
                        None
                    }
                }
                ContextType::Mute => {
                    if sub_set_has_any {
                        if is_in_sub_set {
                            Some(ChannelMarker::Mute)
                        } else {
                            Some(ChannelMarker::Solo)
                        }
                    } else if main_set_has_any {
                        Some(ChannelMarker::Solo) // 豁免权
                    } else {
                        None
                    }
                }
            };

            // SUB 不闪烁（因为不参与 Compare 反转）
            return ChannelDisplay {
                has_sound: marker == Some(ChannelMarker::Solo),
                marker,
                is_blinking: false,
            };
        }

        // === Main 通道逻辑 (Group M) ===
        // Main 通道使用当前激活的上下文（比较模式优先）
        let (context_type, active_set, is_compare_mode) = match compare {
            CompareMode::Solo => (ContextType::Solo, self.solo_set.read(), true),
            CompareMode::Mute => (ContextType::Mute, self.mute_set.read(), true),
            CompareMode::None => match primary {
                PrimaryMode::Solo => (ContextType::Solo, self.solo_set.read(), false),
                PrimaryMode::Mute => (ContextType::Mute, self.mute_set.read(), false),
                PrimaryMode::None => unreachable!(),
            },
        };

        let is_in_main_set = active_set.contains(ch_name);
        let main_set_has_any = active_set.has_any_main();

        let marker = match context_type {
            ContextType::Solo => {
                if is_in_main_set {
                    Some(ChannelMarker::Solo)
                } else if main_set_has_any {
                    Some(ChannelMarker::Mute)
                } else {
                    None
                }
            }
            ContextType::Mute => {
                if is_in_main_set {
                    Some(ChannelMarker::Mute)
                } else if main_set_has_any {
                    Some(ChannelMarker::Solo)
                } else {
                    None
                }
            }
        };

        // 闪烁逻辑：只有 Compare 模式中 **被选中的通道** 闪烁
        // Solo Compare: 只有 Solo 集合中的通道闪烁（绿色闪烁）
        // Mute Compare: 只有 Mute 集合中的通道闪烁（红色闪烁）
        // 其他通道（Auto-Mute 或 Auto-Solo）不闪烁
        let is_blinking = is_compare_mode && is_in_main_set;

        ChannelDisplay {
            has_sound: marker != Some(ChannelMarker::Mute),
            marker,
            is_blinking,
        }
    }

    // ========== 动画支持 ==========

    /// 更新闪烁计数器 (每帧调用)
    pub fn tick_blink(&self) {
        self.blink_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取当前是否应该显示 (用于闪烁动画)
    pub fn should_blink_show(&self) -> bool {
        let counter = self.blink_counter.load(Ordering::Relaxed);
        (counter / 15) % 2 == 0
    }

    // ========== OSC 集成方法 ==========

    /// 切换 Solo 模式 (用于 OSC /Monitor/Mode/Solo)
    pub fn toggle_solo_mode(&self) {
        self.on_solo_button_click();
    }

    /// 切换 Mute 模式 (用于 OSC /Monitor/Mode/Mute)
    pub fn toggle_mute_mode(&self) {
        self.on_mute_button_click();
    }

    /// 处理通道点击 (用于 OSC 通道消息)
    pub fn handle_click(&self, ch_name: &str) {
        self.on_channel_click(ch_name);
    }

    /// 检查 Solo 是否激活 (Primary 或 Compare)
    pub fn is_solo_active(&self) -> bool {
        *self.primary.read() == PrimaryMode::Solo || *self.compare.read() == CompareMode::Solo
    }

    /// 检查 Mute 是否激活 (Primary 或 Compare)
    pub fn is_mute_active(&self) -> bool {
        *self.primary.read() == PrimaryMode::Mute || *self.compare.read() == CompareMode::Mute
    }

    /// 获取需要闪烁的通道名称列表 (用于 OSC 闪烁定时器)
    pub fn get_blinking_channels(&self) -> Vec<String> {
        let compare = *self.compare.read();

        match compare {
            CompareMode::Solo => {
                // Solo Compare 模式: 返回 solo_set 中的所有Main通道（SUB不闪烁）
                let solo_set = self.solo_set.read();
                solo_set
                    .iter()
                    .filter(|name| !name.starts_with("SUB"))
                    .cloned()
                    .collect()
            }
            CompareMode::Mute => {
                // Mute Compare 模式: 返回 mute_set 中的所有Main通道（SUB不闪烁）
                let mute_set = self.mute_set.read();
                mute_set
                    .iter()
                    .filter(|name| !name.starts_with("SUB"))
                    .cloned()
                    .collect()
            }
            CompareMode::None => {
                // 无比较模式，无闪烁
                Vec::new()
            }
        }
    }

    /// 检查通道是否应该显示 Solo LED (用于 OSC 反馈)
    ///
    pub fn is_channel_solo(&self, ch_name: &str) -> bool {
        let display = self.get_channel_display(ch_name);
        display.marker == Some(ChannelMarker::Solo)
    }

    /// 检查通道是否应该显示 Mute LED (用于 OSC 反馈)
    pub fn is_channel_muted(&self, ch_name: &str) -> bool {
        let display = self.get_channel_display(ch_name);
        display.marker == Some(ChannelMarker::Mute)
    }
}

/// 当前激活的 Context 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveContext {
    Solo,
    Mute,
}

/// 上下文类型（用于显示逻辑）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextType {
    Solo,
    Mute,
}

/// 通道显示状态
#[derive(Debug, Clone, Copy)]
pub struct ChannelDisplay {
    /// 是否有声音 (true=绿色, false=红色, 无状态时为灰色)
    pub has_sound: bool,
    /// 标记 (S 或 M)
    pub marker: Option<ChannelMarker>,
    /// 是否闪烁 (Compare 模式下激活的通道)
    pub is_blinking: bool,
}

/// 通道标记
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMarker {
    Solo,
    Mute,
}

/// SUB 点击类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubClickType {
    SingleClick,
    DoubleClick,
}

// ========== 实例级使用 ==========
// InteractionManager 现在作为 MonitorControllerMax 的实例字段使用
// 不再使用全局单例，确保多实例隔离

impl InteractionManager {
    // ========== 网络同步方法 (Master-Slave) ==========

    /// 导出当前状态到网络格式 (Master 调用)
    /// 需要传入 master_gain/dim/cut/layout/sub_layout 参数（从 params 读取）
    pub fn to_network_state(
        &self,
        master_gain: f32,
        dim: bool,
        cut: bool,
        layout: i32,
        sub_layout: i32,
    ) -> NetworkInteractionState {
        let primary = match *self.primary.read() {
            PrimaryMode::None => 0,
            PrimaryMode::Solo => 1,
            PrimaryMode::Mute => 2,
        };

        let compare = match *self.compare.read() {
            CompareMode::None => 0,
            CompareMode::Solo => 1,
            CompareMode::Mute => 2,
        };

        let solo_set = self.solo_set.read();
        let mute_set = self.mute_set.read();
        let user_mute_sub = self.user_mute_sub.read();
        let solo_has_memory = *self.solo_has_memory.read();
        let mute_has_memory = *self.mute_has_memory.read();
        let automation_mode = *self.automation_mode.read();

        NetworkInteractionState {
            protocol_version: 0, // M4: with_timestamp() 会设置正确的版本
            primary,
            compare,
            solo_mask: NetworkInteractionState::channel_set_to_mask(&solo_set.channels),
            mute_mask: NetworkInteractionState::channel_set_to_mask(&mute_set.channels),
            user_mute_sub_mask: NetworkInteractionState::sub_set_to_mask(&user_mute_sub),
            master_gain,
            dim,
            cut,
            layout,
            sub_layout,
            solo_has_memory,
            mute_has_memory,
            automation_mode,
            timestamp: 0,
            magic: 0,
        }
        .with_timestamp()
    }

    /// 从网络格式导入状态 (Slave 调用)
    /// H1: 优化为先收集数据，再批量更新，减少中间状态暴露
    pub fn from_network_state(&self, state: &NetworkInteractionState) {
        if !state.is_valid() {
            return;
        }

        // Step 1: 预先解析所有数据（不持有任何锁）
        let new_primary = match state.primary {
            1 => PrimaryMode::Solo,
            2 => PrimaryMode::Mute,
            _ => PrimaryMode::None,
        };
        let new_compare = match state.compare {
            1 => CompareMode::Solo,
            2 => CompareMode::Mute,
            _ => CompareMode::None,
        };
        let new_solo_channels = NetworkInteractionState::mask_to_channel_set(state.solo_mask);
        let new_mute_channels = NetworkInteractionState::mask_to_channel_set(state.mute_mask);
        let new_user_mute_sub = NetworkInteractionState::mask_to_sub_set(state.user_mute_sub_mask);

        // Step 2: 批量更新核心状态（快速获取-释放，最小化锁持有时间）
        *self.primary.write() = new_primary;
        *self.compare.write() = new_compare;
        self.solo_set.write().channels = new_solo_channels;
        self.mute_set.write().channels = new_mute_channels;
        *self.user_mute_sub.write() = new_user_mute_sub;
        *self.solo_has_memory.write() = state.solo_has_memory;
        *self.mute_has_memory.write() = state.mute_has_memory;
        *self.automation_mode.write() = state.automation_mode;

        // Step 3: 更新网络接收值（供 Editor 读取并应用到 params）
        // C10 修复：只在值真正变化时设置，避免重复触发 clear_on_layout_change()
        *self.network_master_gain.write() = Some(state.master_gain);
        *self.network_dim.write() = Some(state.dim);
        *self.network_cut.write() = Some(state.cut);

        // 布局只在首次接收或真正变化时设置
        {
            let current_layout = self.network_layout.read();
            if current_layout.is_none() || *current_layout != Some(state.layout) {
                drop(current_layout);
                *self.network_layout.write() = Some(state.layout);
            }
        }
        {
            let current_sub = self.network_sub_layout.read();
            if current_sub.is_none() || *current_sub != Some(state.sub_layout) {
                drop(current_sub);
                *self.network_sub_layout.write() = Some(state.sub_layout);
            }
        }

        // Step 4: 最后更新 Lock-Free 快照（音频线程读取的是一致的快照）
        self.update_snapshot();
    }

    /// 获取并清除网络接收的主音量（Slave Editor 调用）
    pub fn take_network_master_gain(&self) -> Option<f32> {
        self.network_master_gain.write().take()
    }

    /// 获取并清除网络接收的 Dim 状态（Slave Editor 调用）
    pub fn take_network_dim(&self) -> Option<bool> {
        self.network_dim.write().take()
    }

    /// 获取并清除网络接收的 Cut 状态（Slave Editor 调用）
    pub fn take_network_cut(&self) -> Option<bool> {
        self.network_cut.write().take()
    }

    /// 获取并清除网络接收的布局索引（Slave Editor 调用）
    pub fn take_network_layout(&self) -> Option<i32> {
        self.network_layout.write().take()
    }

    /// 获取并清除网络接收的 SUB 布局索引（Slave Editor 调用）
    pub fn take_network_sub_layout(&self) -> Option<i32> {
        self.network_sub_layout.write().take()
    }

    /// C11 修复：清空所有网络接收的状态
    /// 在心跳超时或断开连接时调用，防止旧数据污染新连接
    pub fn clear_network_state(&self) {
        *self.network_master_gain.write() = None;
        *self.network_dim.write() = None;
        *self.network_cut.write() = None;
        *self.network_layout.write() = None;
        *self.network_sub_layout.write() = None;
    }

    // ========== OSC Hot Reload 方法 ==========

    /// 请求 OSC 重启（携带新配置）
    /// P1 优化：设置 pending 标志，让 process() 可以快速检查
    pub fn request_osc_restart(&self, new_config: AppConfig) {
        *self.osc_restart_config.write() = Some(new_config);
        self.osc_restart_pending.store(true, Ordering::Release);
    }

    /// P1 优化：快速检查是否有 OSC 重启请求（无锁）
    #[inline]
    pub fn has_osc_restart_pending(&self) -> bool {
        self.osc_restart_pending.load(Ordering::Relaxed)
    }

    /// 获取并清除 OSC 重启请求（Lib.rs process 调用）
    /// P1 优化：只在 pending 为 true 时才获取锁
    pub fn take_osc_restart_request(&self) -> Option<AppConfig> {
        if !self.osc_restart_pending.load(Ordering::Relaxed) {
            return None; // 快速路径，无锁
        }
        let config = self.osc_restart_config.write().take();
        if config.is_some() {
            self.osc_restart_pending.store(false, Ordering::Relaxed);
        }
        config
    }

    // ========== Network Hot Reload 方法 ==========

    /// 请求 Network 重启（携带新配置）
    /// P1 优化：设置 pending 标志，让 process() 可以快速检查
    pub fn request_network_restart(&self, new_config: AppConfig) {
        *self.network_restart_config.write() = Some(new_config);
        self.network_restart_pending.store(true, Ordering::Release);
    }

    /// P1 优化：快速检查是否有 Network 重启请求（无锁）
    #[inline]
    pub fn has_network_restart_pending(&self) -> bool {
        self.network_restart_pending.load(Ordering::Relaxed)
    }

    /// 获取并清除 Network 重启请求（Lib.rs process 调用）
    /// P1 优化：只在 pending 为 true 时才获取锁
    pub fn take_network_restart_request(&self) -> Option<AppConfig> {
        if !self.network_restart_pending.load(Ordering::Relaxed) {
            return None; // 快速路径，无锁
        }
        let config = self.network_restart_config.write().take();
        if config.is_some() {
            self.network_restart_pending.store(false, Ordering::Relaxed);
        }
        config
    }

    // ========== Web Hot Reload ==========

    /// 请求 Web 服务器重启（启动/停止）
    pub fn request_web_restart(&self, action: WebRestartAction) {
        *self.web_restart_action.write() = Some(action);
        self.web_restart_pending.store(true, Ordering::Release);
    }

    /// P1 优化：快速检查是否有 Web 重启请求（无锁）
    #[inline]
    pub fn has_web_restart_pending(&self) -> bool {
        self.web_restart_pending.load(Ordering::Relaxed)
    }

    /// 获取并清除 Web 重启请求
    pub fn take_web_restart_request(&self) -> Option<WebRestartAction> {
        if !self.web_restart_pending.load(Ordering::Relaxed) {
            return None;
        }
        let action = self.web_restart_action.write().take();
        if action.is_some() {
            self.web_restart_pending.store(false, Ordering::Relaxed);
        }
        action
    }
}
