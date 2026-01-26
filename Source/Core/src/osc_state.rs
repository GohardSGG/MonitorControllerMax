use crossbeam::channel::Sender;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config_manager::Layout;
use crate::interaction::InteractionManager;
use mcm_infra::logger::InstanceLogger;
use mcm_protocol::osc_structs::{ChannelLedState, OscOutMessage};

// ==================== 实例级共享状态 ====================

/// OSC 实例共享状态（线程间共享，但实例间隔离）
pub struct OscSharedState {
    /// 当前音频布局的通道数
    pub channel_count: AtomicUsize,
    /// 当前布局的通道名称列表
    pub current_channel_names: RwLock<Vec<String>>,
    /// 之前布局的通道名称列表（用于清空已删除的通道）
    pub prev_channel_names: RwLock<Vec<String>>,
    /// 待激活的 Solo 模式标志
    pub pending_solo: AtomicBool,
    /// 待激活的 Mute 模式标志
    pub pending_mute: AtomicBool,
    /// 当前 Cut 状态（用于 toggle 支持）
    pub current_cut: AtomicBool,
    /// OSC 发送通道
    pub sender_tx: RwLock<Option<Sender<OscOutMessage>>>,
    /// Master Volume (使用 f32 的位表示存储在 AtomicU32 中)
    pub master_volume: AtomicU32,
    /// Dim 状态
    pub dim: AtomicBool,
    /// Cut 接收状态
    pub cut: AtomicBool,
    /// Mono 状态
    pub mono: AtomicBool,
    /// LFE +10dB 状态
    pub lfe_add_10db: AtomicBool,
    /// Low Boost 状态
    pub low_boost: AtomicBool,
    /// High Boost 状态
    pub high_boost: AtomicBool,
    // B1 修复：分离的 pending 标志，追踪每个值的变化
    /// Volume 是否有待处理的变化
    pub volume_pending: AtomicBool,
    /// Dim 是否有待处理的变化
    pub dim_pending: AtomicBool,
    /// Cut 是否有待处理的变化
    pub cut_pending: AtomicBool,
    /// OSC 接收端口是否成功绑定（用于UI显示）
    pub recv_port_bound: AtomicBool,
    /// 实例级日志器（线程安全）
    logger: RwLock<Option<Arc<InstanceLogger>>>,
    /// GUI 重绘请求标志（OSC 参数变化时设置，Editor 检测后清除）
    pub repaint_requested: AtomicBool,
}

impl OscSharedState {
    pub fn new() -> Self {
        Self {
            channel_count: AtomicUsize::new(0),
            current_channel_names: RwLock::new(Vec::new()),
            prev_channel_names: RwLock::new(Vec::new()),
            pending_solo: AtomicBool::new(false),
            pending_mute: AtomicBool::new(false),
            current_cut: AtomicBool::new(false),
            sender_tx: RwLock::new(None),
            master_volume: AtomicU32::new(0),
            dim: AtomicBool::new(false),
            cut: AtomicBool::new(false),
            mono: AtomicBool::new(false),
            lfe_add_10db: AtomicBool::new(false),
            low_boost: AtomicBool::new(false),
            high_boost: AtomicBool::new(false),
            // B1 修复：分离的 pending 标志
            volume_pending: AtomicBool::new(false),
            dim_pending: AtomicBool::new(false),
            cut_pending: AtomicBool::new(false),
            recv_port_bound: AtomicBool::new(false),
            logger: RwLock::new(None),
            repaint_requested: AtomicBool::new(false),
        }
    }

    /// 设置日志器（由 OscManager::init 调用）
    pub fn set_logger(&self, logger: Arc<InstanceLogger>) {
        *self.logger.write() = Some(logger);
    }

    /// 日志辅助方法
    fn log_info(&self, msg: &str) {
        if let Some(ref logger) = *self.logger.read() {
            logger.info("osc", msg);
        }
    }

    fn log_warn(&self, msg: &str) {
        if let Some(ref logger) = *self.logger.read() {
            logger.warn("osc", msg);
        }
    }

    // === 发送方法 ===

    /// 发送 Solo 模式按钮状态
    pub fn send_mode_solo(&self, on: bool) {
        self.send(OscOutMessage::ModeSolo { on });
    }

    /// 发送 Mute 模式按钮状态
    pub fn send_mode_mute(&self, on: bool) {
        self.send(OscOutMessage::ModeMute { on });
    }

    /// 发送通道 LED 状态（通过通道名称）
    pub fn send_channel_led_by_name(&self, ch_name: &str, state: ChannelLedState) {
        self.send(OscOutMessage::ChannelLed {
            channel: ch_name.to_string(),
            state,
        });
    }

    /// 发送主音量
    pub fn send_master_volume(&self, value: f32) {
        self.send(OscOutMessage::MasterVolume { value });
    }

    /// 发送 Dim 状态
    pub fn send_dim(&self, on: bool) {
        self.send(OscOutMessage::Dim { on });
    }

    /// 发送 Cut 状态
    pub fn send_cut(&self, on: bool) {
        self.send(OscOutMessage::Cut { on });
    }

    /// 发送 Mono 状态
    pub fn send_mono(&self, on: bool) {
        self.send(OscOutMessage::Mono { on });
    }

    /// 发送 LFE +10dB 状态
    pub fn send_lfe_add_10db(&self, on: bool) {
        self.send(OscOutMessage::LfeAdd10dB { on });
    }

    /// 发送 Low Boost 状态
    pub fn send_low_boost(&self, on: bool) {
        self.send(OscOutMessage::LowBoost { on });
    }

    /// 发送 High Boost 状态
    pub fn send_high_boost(&self, on: bool) {
        self.send(OscOutMessage::HighBoost { on });
    }

    fn send(&self, msg: OscOutMessage) {
        if let Some(tx) = self.sender_tx.read().as_ref() {
            let _ = tx.try_send(msg);
        }
    }

    // === 接收方法 ===

    /// 设置 Master Volume (从 OSC 接收)
    pub fn set_master_volume(&self, value: f32) {
        self.master_volume.store(value.to_bits(), Ordering::Release);
        self.volume_pending.store(true, Ordering::Release);
        self.repaint_requested.store(true, Ordering::Release);
    }

    /// 设置 Dim (从 OSC 接收)
    pub fn set_dim(&self, on: bool) {
        self.dim.store(on, Ordering::Release);
        self.dim_pending.store(true, Ordering::Release);
        self.repaint_requested.store(true, Ordering::Release);
    }

    /// 设置 Cut (从 OSC 接收)
    pub fn set_cut(&self, on: bool) {
        self.cut.store(on, Ordering::Release);
        self.cut_pending.store(true, Ordering::Release);
        self.repaint_requested.store(true, Ordering::Release);
    }

    /// 设置 Mono (从 OSC 接收)
    pub fn set_mono(&self, on: bool) {
        self.mono.store(on, Ordering::Relaxed);
    }

    /// 获取 Mono 状态
    pub fn get_mono(&self) -> bool {
        self.mono.load(Ordering::Relaxed)
    }

    /// 设置 LFE +10dB (从 OSC 接收)
    pub fn set_lfe_add_10db(&self, on: bool) {
        self.lfe_add_10db.store(on, Ordering::Relaxed);
    }

    /// 获取 LFE +10dB 状态
    pub fn get_lfe_add_10db(&self) -> bool {
        self.lfe_add_10db.load(Ordering::Relaxed)
    }

    /// 设置 Low Boost (从 OSC 接收)
    pub fn set_low_boost(&self, on: bool) {
        self.low_boost.store(on, Ordering::Relaxed);
    }

    /// 获取 Low Boost 状态
    pub fn get_low_boost(&self) -> bool {
        self.low_boost.load(Ordering::Relaxed)
    }

    /// 设置 High Boost (从 OSC 接收)
    pub fn set_high_boost(&self, on: bool) {
        self.high_boost.store(on, Ordering::Relaxed);
    }

    /// 获取 High Boost 状态
    pub fn get_high_boost(&self) -> bool {
        self.high_boost.load(Ordering::Relaxed)
    }

    /// 检查是否有待处理的 OSC 变化（不清除标志）
    #[allow(dead_code)]
    #[inline(always)]
    pub fn has_osc_override(&self) -> bool {
        self.volume_pending.load(Ordering::Relaxed)
            || self.dim_pending.load(Ordering::Relaxed)
            || self.cut_pending.load(Ordering::Relaxed)
    }

    /// P9 优化：获取 OSC 覆盖值快照（合并多次原子操作）
    #[inline(always)]
    pub fn get_override_snapshot(&self) -> Option<(f32, bool, bool)> {
        // 快速路径
        let any_pending = self.volume_pending.load(Ordering::Relaxed)
            || self.dim_pending.load(Ordering::Relaxed)
            || self.cut_pending.load(Ordering::Relaxed);

        if any_pending {
            // 慢路径
            Some((
                f32::from_bits(self.master_volume.load(Ordering::Acquire)),
                self.dim.load(Ordering::Acquire),
                self.cut.load(Ordering::Acquire),
            ))
        } else {
            None
        }
    }

    /// 获取并清除 GUI 重绘请求标志
    #[inline(always)]
    pub fn take_repaint_request(&self) -> bool {
        self.repaint_requested.swap(false, Ordering::Acquire)
    }

    /// B1 修复：获取并清除 Volume 变化（返回 Option）
    pub fn take_pending_volume(&self) -> Option<f32> {
        if self.volume_pending.swap(false, Ordering::Acquire) {
            Some(f32::from_bits(self.master_volume.load(Ordering::Acquire)))
        } else {
            None
        }
    }

    /// B1 修复：获取并清除 Dim 变化（返回 Option）
    pub fn take_pending_dim(&self) -> Option<bool> {
        if self.dim_pending.swap(false, Ordering::Acquire) {
            Some(self.dim.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// B1 修复：获取并清除 Cut 变化（返回 Option）
    pub fn take_pending_cut(&self) -> Option<bool> {
        if self.cut_pending.swap(false, Ordering::Acquire) {
            Some(self.cut.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// 同步 Cut 状态
    pub fn sync_cut_state(&self, cut: bool) {
        self.current_cut.store(cut, Ordering::Relaxed);
    }

    /// 更新布局通道信息
    pub fn update_layout_channels(&self, layout: &Layout) {
        let mut prev = self.prev_channel_names.write();
        let mut curr = self.current_channel_names.write();

        // 保存旧列表
        *prev = curr.clone();

        // 从 layout 构建新列表
        let mut names = Vec::new();
        for ch in &layout.main_channels {
            names.push(ch.name.clone());
        }
        for ch in &layout.sub_channels {
            names.push(ch.name.clone());
        }

        self.log_info(&format!(
            "[OSC] Layout channels updated: {} → {} channels",
            prev.len(),
            names.len()
        ));

        *curr = names;
        self.channel_count
            .store(layout.total_channels, Ordering::Relaxed);
    }

    /// 广播所有通道的 LED 状态
    pub fn broadcast_channel_states(&self, interaction: &InteractionManager) {
        let curr = self.current_channel_names.read();
        let prev = self.prev_channel_names.read();

        if curr.is_empty() {
            self.log_warn("[OSC] Channel names not initialized, skipping broadcast");
            return;
        }

        self.log_info(&format!(
            "[OSC] Broadcasting LED states for {} channels...",
            curr.len()
        ));

        // 广播当前布局的所有通道状态
        for name in curr.iter() {
            let state = if interaction.is_channel_solo(name) {
                ChannelLedState::Solo
            } else if interaction.is_channel_muted(name) {
                ChannelLedState::Mute
            } else {
                ChannelLedState::Off
            };

            self.send_channel_led_by_name(name, state);
        }

        // 清空已删除的通道
        let curr_set: HashSet<_> = curr.iter().collect();
        for name in prev.iter() {
            if !curr_set.contains(name) {
                self.log_info(&format!("[OSC] Clearing removed channel: {}", name));
                self.send_channel_led_by_name(name, ChannelLedState::Off);
            }
        }

        self.log_info(&format!(
            "[OSC] Broadcast complete (cleared {} removed channels)",
            prev.iter().filter(|n| !curr_set.contains(n)).count()
        ));
    }
}

impl Default for OscSharedState {
    fn default() -> Self {
        Self::new()
    }
}
