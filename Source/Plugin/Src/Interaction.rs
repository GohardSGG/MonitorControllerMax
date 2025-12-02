//! InteractionManager - 交互状态机
//!
//! 实现 v4.0 规范的核心交互逻辑：
//! - 主模式: SoloActive (常亮绿), MuteActive (常亮红)
//! - 比较模式: 在主模式基础上叠加另一个模式 (闪烁)
//! - 通道操作: 始终操作当前激活的 Context (闪烁的那个优先)

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

/// 通道集合 - 使用位图存储
#[derive(Debug, Clone, Copy, Default)]
pub struct ChannelSet {
    /// 主声道位图 (bit i = 1 表示通道 i 在集合中)
    pub main: u32,
    /// SUB 声道位图
    pub sub: u32,
}

impl ChannelSet {
    pub fn new() -> Self {
        Self { main: 0, sub: 0 }
    }

    pub fn clear(&mut self) {
        self.main = 0;
        self.sub = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.main == 0 && self.sub == 0
    }

    /// 切换主声道
    pub fn toggle_main(&mut self, ch: usize) {
        if ch < 32 {
            self.main ^= 1 << ch;
        }
    }

    /// 检查主声道是否在集合中
    pub fn contains_main(&self, ch: usize) -> bool {
        if ch < 32 {
            (self.main >> ch) & 1 == 1
        } else {
            false
        }
    }

    /// 切换 SUB 声道
    pub fn toggle_sub(&mut self, ch: usize) {
        if ch < 32 {
            self.sub ^= 1 << ch;
        }
    }

    /// 检查 SUB 是否在集合中
    pub fn contains_sub(&self, ch: usize) -> bool {
        if ch < 32 {
            (self.sub >> ch) & 1 == 1
        } else {
            false
        }
    }

    /// 检查通道是否在集合中
    pub fn contains(&self, ch: usize, is_sub: bool) -> bool {
        if is_sub {
            self.contains_sub(ch)
        } else {
            self.contains_main(ch)
        }
    }

    /// 切换通道
    pub fn toggle(&mut self, ch: usize, is_sub: bool) {
        if is_sub {
            self.toggle_sub(ch);
        } else {
            self.toggle_main(ch);
        }
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

    /// User Mute SUB (SUB 双击/长按强制静音)
    user_mute_sub: RwLock<u32>,

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
            user_mute_sub: RwLock::new(0),
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
        *self.solo_set.read()
    }

    /// 获取 Mute 集合
    pub fn get_mute_set(&self) -> ChannelSet {
        *self.mute_set.read()
    }

    /// 检查通道是否在 Solo 集合中
    pub fn is_in_solo_set(&self, ch: usize, is_sub: bool) -> bool {
        self.solo_set.read().contains(ch, is_sub)
    }

    /// 检查通道是否在 Mute 集合中
    pub fn is_in_mute_set(&self, ch: usize, is_sub: bool) -> bool {
        self.mute_set.read().contains(ch, is_sub)
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
        *self.user_mute_sub.write() = 0;
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
        // 注意：这里只拷贝 main，sub 保持为 0 (SUB 不参与自动反转)
        ChannelSet {
            main: source.main,   // 拷贝，不是位反转
            sub: 0,              // SUB 不参与反转，保持空
        }
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
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitSoloCompare,
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitAll,
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
                *self.user_mute_sub.write() = 0;
            }
            Action::EnterSoloCompare => {
                // 从 Mute Active 进入 Solo Compare
                // 关键：自动反转逻辑（拷贝 mute_set 到 solo_set）
                let has_memory = *self.solo_has_memory.read();

                if !has_memory {
                    // 如果 Solo 没有记忆，执行自动反转（拷贝）
                    let mute_set = *self.mute_set.read();
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
                *self.user_mute_sub.write() = 0;
            }
            Action::None => {}
        }

        // 记录状态变化（在所有操作完成后）
        let new_primary = *self.primary.read();
        let new_compare = *self.compare.read();
        let solo_mask = self.solo_set.read().main;
        let mute_mask = self.mute_set.read().main;
        mcm_info!("[SM] SOLO: ({:?},{:?})->({:?},{:?}) solo=0x{:x} mute=0x{:x}",
            current_primary, current_compare, new_primary, new_compare, solo_mask, mute_mask);
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
            (PrimaryMode::Solo, CompareMode::Mute) => Action::ExitMuteCompare,
            (PrimaryMode::Mute, CompareMode::Solo) => Action::ExitAll,
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
                *self.user_mute_sub.write() = 0;
            }
            Action::EnterMuteCompare => {
                // 从 Solo Active 进入 Mute Compare
                // 关键：自动反转逻辑（拷贝 solo_set 到 mute_set）
                let has_memory = *self.mute_has_memory.read();

                if !has_memory {
                    // 如果 Mute 没有记忆，执行自动反转（拷贝）
                    let solo_set = *self.solo_set.read();
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
                *self.user_mute_sub.write() = 0;
            }
            Action::None => {}
        }

        // 记录状态变化（在所有操作完成后）
        let new_primary = *self.primary.read();
        let new_compare = *self.compare.read();
        let solo_mask = self.solo_set.read().main;
        let mute_mask = self.mute_set.read().main;
        mcm_info!("[SM] MUTE: ({:?},{:?})->({:?},{:?}) solo=0x{:x} mute=0x{:x}",
            current_primary, current_compare, new_primary, new_compare, solo_mask, mute_mask);
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
    pub fn on_channel_click(&self, ch: usize, is_sub: bool) -> bool {
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
                self.solo_set.write().toggle(ch, is_sub);

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
                self.mute_set.write().toggle(ch, is_sub);

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
            let solo_mask = self.solo_set.read().main;
            let mute_mask = self.mute_set.read().main;
            let ch_type = if is_sub { "SUB" } else { "Main" };
            mcm_info!("[CH] {}{} click: solo=0x{:x} mute=0x{:x}",
                ch_type, ch, solo_mask, mute_mask);
        }

        result
    }

    /// SUB 双击 - User Mute (强制静音，优先级最高)
    pub fn on_sub_double_click(&self, ch: usize) -> bool {
        if ch < 32 {
            let old_mask = *self.user_mute_sub.read();
            let mut user_mute = self.user_mute_sub.write();
            *user_mute ^= 1 << ch;  // 切换位
            let new_mask = *user_mute;
            mcm_info!("[CH] SUB{} dblclick: user_mute 0x{:x}->0x{:x}", ch, old_mask, new_mask);
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
    pub fn get_channel_display(&self, ch: usize, is_sub: bool) -> ChannelDisplay {
        let primary = *self.primary.read();
        let compare = *self.compare.read();

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
            let user_mute = *self.user_mute_sub.read();
            if (user_mute >> ch) & 1 == 1 {
                return ChannelDisplay {
                    has_sound: false,
                    marker: Some(ChannelMarker::Mute),
                    is_blinking: false,  // User Mute 不闪烁
                };
            }

            // SUB 使用 Primary 模式的集合（不管是否在 Compare 模式）
            let (sub_context_type, sub_set, main_set) = match primary {
                PrimaryMode::Solo => {
                    let solo = self.solo_set.read();
                    (ContextType::Solo, solo.sub, solo.main)
                }
                PrimaryMode::Mute => {
                    let mute = self.mute_set.read();
                    (ContextType::Mute, mute.sub, mute.main)
                }
                PrimaryMode::None => unreachable!(),
            };

            let is_in_sub_set = (sub_set >> ch) & 1 == 1;
            let sub_set_has_any = sub_set != 0;
            let main_set_has_any = main_set != 0;

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

        let is_in_main_set = active_set.contains_main(ch);
        let main_set_has_any = active_set.main != 0;

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
    pub fn handle_click(&self, ch: usize) {
        self.on_channel_click(ch, false);
    }

    /// 检查 Solo 是否激活 (Primary 或 Compare)
    pub fn is_solo_active(&self) -> bool {
        *self.primary.read() == PrimaryMode::Solo || *self.compare.read() == CompareMode::Solo
    }

    /// 检查 Mute 是否激活 (Primary 或 Compare)
    pub fn is_mute_active(&self) -> bool {
        *self.primary.read() == PrimaryMode::Mute || *self.compare.read() == CompareMode::Mute
    }

    /// 获取需要闪烁的通道索引列表 (用于 OSC 闪烁定时器)
    pub fn get_blinking_channels(&self) -> Vec<usize> {
        let compare = *self.compare.read();

        match compare {
            CompareMode::Solo => {
                // Solo Compare 模式: 返回 solo_set 中的所有通道
                let solo_set = self.solo_set.read();
                (0..32)
                    .filter(|&ch| (solo_set.main >> ch) & 1 == 1)
                    .collect()
            }
            CompareMode::Mute => {
                // Mute Compare 模式: 返回 mute_set 中的所有通道
                let mute_set = self.mute_set.read();
                (0..32)
                    .filter(|&ch| (mute_set.main >> ch) & 1 == 1)
                    .collect()
            }
            CompareMode::None => {
                // 无比较模式，无闪烁
                Vec::new()
            }
        }
    }

    /// 检查通道是否应该显示 Solo LED (用于 OSC 反馈)
    pub fn is_channel_solo(&self, ch: usize) -> bool {
        let display = self.get_channel_display(ch, false);
        display.marker == Some(ChannelMarker::Solo)
    }

    /// 检查通道是否应该显示 Mute LED (用于 OSC 反馈)
    pub fn is_channel_muted(&self, ch: usize) -> bool {
        let display = self.get_channel_display(ch, false);
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
