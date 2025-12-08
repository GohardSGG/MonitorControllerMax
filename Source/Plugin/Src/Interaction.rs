//! InteractionManager - 交互状态机
//!
//! 实现 v4.0 规范的核心交互逻辑：
//! - 主模式: SoloActive (常亮绿), MuteActive (常亮红)
//! - 比较模式: 在主模式基础上叠加另一个模式 (闪烁)
//! - 通道操作: 始终操作当前激活的 Context (闪烁的那个优先)

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::mcm_info;

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
}

impl InteractionManager {
    pub fn new() -> Self {
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
        }
    }

    // ========== 状态查询 ==========

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
    pub fn get_solo_set(&self) -> ChannelSet {
        self.solo_set.read().clone()
    }

    /// 获取 Mute 集合
    pub fn get_mute_set(&self) -> ChannelSet {
        self.mute_set.read().clone()
    }

    /// 检查通道是否在 Solo 集合中
    pub fn is_in_solo_set(&self, ch_name: &str) -> bool {
        self.solo_set.read().contains(ch_name)
    }

    /// 检查通道是否在 Mute 集合中
    pub fn is_in_mute_set(&self, ch_name: &str) -> bool {
        self.mute_set.read().contains(ch_name)
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
    }

    /// 退出自动化模式（保持清空状态）
    pub fn exit_automation_mode(&self) {
        *self.automation_mode.write() = false;
        // 不恢复任何状态，保持 Idle
    }

    // ========== 辅助函数 ==========

    /// 拷贝集合到比较模式 (只拷贝 Main 通道，SUB 不参与)
    /// 逻辑：被 Solo 的通道 -> 变成被 Mute 的通道（相同的通道集合）
    fn copy_set(&self, source: &ChannelSet) -> ChannelSet {
        // 只拷贝 Main 通道（过滤掉 SUB）
        let channels: HashSet<String> = source.channels.iter()
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
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitSoloCompare,  // 点击 Compare 按钮，只退出 Compare
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitAll,  // 点击 Primary 按钮，完全退出
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
        mcm_info!("[SM] SOLO: ({:?},{:?})->({:?},{:?}) solo_count={} mute_count={}",
            current_primary, current_compare, new_primary, new_compare, solo_count, mute_count);
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
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitMuteCompare,  // 点击 Compare 按钮，只退出 Compare
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitAll,  // 点击 Primary 按钮，完全退出
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
        mcm_info!("[SM] MUTE: ({:?},{:?})->({:?},{:?}) solo_count={} mute_count={}",
            current_primary, current_compare, new_primary, new_compare, solo_count, mute_count);
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
            let solo_count = self.solo_set.read().iter().filter(|n| !n.starts_with("SUB")).count();
            let mute_count = self.mute_set.read().iter().filter(|n| !n.starts_with("SUB")).count();
            mcm_info!("[CH] {} click: solo_count={} mute_count={}",
                ch_name, solo_count, mute_count);
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
    }

    /// 设置通道声音状态（语义层，用于 Group_Dial）
    /// has_sound: true = 有声音, false = 没声音
    /// 根据当前 ActiveContext 正确解释语义
    pub fn set_channel_sound(&self, ch_name: &str, has_sound: bool) {
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

        // 检查集合是否变空，自动退出模式
        self.check_and_exit_empty_mode();
    }

    /// 检查集合是否变空，自动退出对应模式
    fn check_and_exit_empty_mode(&self) {
        let solo_empty = self.solo_set.read().is_empty();
        let mute_empty = self.mute_set.read().is_empty();

        // 检查 Solo 模式
        if solo_empty && *self.primary.read() == PrimaryMode::Solo {
            *self.primary.write() = PrimaryMode::None;
            mcm_info!("[Mode] Solo set empty, auto-exiting Solo mode");
        }

        // 检查 Mute 模式
        if mute_empty && *self.primary.read() == PrimaryMode::Mute {
            *self.primary.write() = PrimaryMode::None;
            mcm_info!("[Mode] Mute set empty, auto-exiting Mute mode");
        }
    }

    /// SUB 双击 - User Mute (强制静音，优先级最高)
    pub fn on_sub_double_click(&self, ch_name: &str) -> bool {
        if ch_name.starts_with("SUB") {
            let mut user_mute = self.user_mute_sub.write();
            let was_muted = user_mute.contains(ch_name);

            if was_muted {
                user_mute.remove(ch_name);
                mcm_info!("[CH] {} dblclick: user_mute removed", ch_name);
            } else {
                user_mute.insert(ch_name.to_string());
                mcm_info!("[CH] {} dblclick: user_mute added", ch_name);
            }
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
                has_sound: true,   // ← 修复：Idle = 全通
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
                    is_blinking: false,  // User Mute 不闪烁
                };
            }

            // SUB 使用 Primary 模式的集合（不管是否在 Compare 模式）
            let (sub_context_type, active_set) = match primary {
                PrimaryMode::Solo => {
                    (ContextType::Solo, self.solo_set.read())
                }
                PrimaryMode::Mute => {
                    (ContextType::Mute, self.mute_set.read())
                }
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
                    has_sound: false,
                    marker: None,
                    is_blinking: false,
                };
            }

            let marker = match sub_context_type {
                ContextType::Solo => {
                    if sub_set_has_any {
                        if is_in_sub_set { Some(ChannelMarker::Solo) }
                        else { Some(ChannelMarker::Mute) }
                    } else if main_set_has_any {
                        Some(ChannelMarker::Solo)  // 豁免权
                    } else {
                        None
                    }
                }
                ContextType::Mute => {
                    if sub_set_has_any {
                        if is_in_sub_set { Some(ChannelMarker::Mute) }
                        else { Some(ChannelMarker::Solo) }
                    } else if main_set_has_any {
                        Some(ChannelMarker::Solo)  // 豁免权
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
            CompareMode::Solo => {
                (ContextType::Solo, self.solo_set.read(), true)
            }
            CompareMode::Mute => {
                (ContextType::Mute, self.mute_set.read(), true)
            }
            CompareMode::None => {
                match primary {
                    PrimaryMode::Solo => (ContextType::Solo, self.solo_set.read(), false),
                    PrimaryMode::Mute => (ContextType::Mute, self.mute_set.read(), false),
                    PrimaryMode::None => unreachable!(),
                }
            }
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
            has_sound: marker == Some(ChannelMarker::Solo),
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
                solo_set.iter()
                    .filter(|name| !name.starts_with("SUB"))
                    .cloned()
                    .collect()
            }
            CompareMode::Mute => {
                // Mute Compare 模式: 返回 mute_set 中的所有Main通道（SUB不闪烁）
                let mute_set = self.mute_set.read();
                mute_set.iter()
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

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 全局单例 ==========
use lazy_static::lazy_static;

lazy_static! {
    /// 全局交互管理器 (线程安全)
    pub static ref INTERACTION: InteractionManager = InteractionManager::new();
}

/// 获取全局交互管理器 (已弃用,使用 INTERACTION 替代)
#[deprecated(note = "Use INTERACTION static instead")]
pub fn get_interaction_manager() -> &'static InteractionManager {
    &INTERACTION
}
