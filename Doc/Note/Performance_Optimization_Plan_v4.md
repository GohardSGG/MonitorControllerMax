# MonitorControllerMax 性能优化与 Bug 修复计划 v4

## 任务背景

用户报告两个问题：
1. **性能问题**：Mac 性能吃紧，插件运行时偶尔有爆音
2. **Dim Bug**：按下硬件控制器的 Dim 按钮后，UI 上的音量显示跳到 0

**目标**：
- 极限降低 CPU 占用率，消除音频线程中的任何潜在阻塞
- 修复 Dim 按钮导致音量归零的 Bug

---

## 一、已完成的修复 (v2.5.9)

| # | 问题 | 状态 |
|---|------|------|
| C1-C3 | Editor/OSC/Atomic 修复 | ✅ 已修复 |
| C9 | 时间戳检查 | ✅ 已修复 |
| C10 | 布局同步去重 | ✅ 已修复 |
| C11 | 心跳超时清理 | ✅ 已修复 |
| C12 | RenderSnapshot 内存对齐 | ✅ 已修复 |

---

## 二、关键 Bug 修复 (Critical Bug Fix)

### 🚨 B1: Dim 按钮导致音量归零 (MUST FIX)

**问题描述**：
从日志可见，当用户按下 Dim 按钮时，音量被错误地设置为 0：
```
[00:31:44.065] [OSC] Dim toggle: false -> true
[00:31:44.085] [editor] [OSC Recv] Applied changes: volume=0.000, dim=true, cut=false
```

**根本原因分析**：

1. `OscSharedState` 在 `Osc.rs:76` 初始化 `master_volume` 为 0：
   ```rust
   master_volume: AtomicU32::new(0),  // ← 初始化为 0！
   ```

2. 当 Dim 按钮被按下时，`set_dim()` 设置 `has_pending = true`：
   ```rust
   pub fn set_dim(&self, on: bool) {
       self.dim.store(on, Ordering::Relaxed);
       self.has_pending.store(true, Ordering::Relaxed);  // ← 标记有变化
   }
   ```

3. `get_pending_changes()` 返回**所有三个值**，即使只有 Dim 变了：
   ```rust
   pub fn get_pending_changes(&self) -> Option<(f32, bool, bool)> {
       if !self.has_pending.swap(false, Ordering::Relaxed) { return None; }
       let volume = f32::from_bits(self.master_volume.load(...));  // ← 返回 0.0！
       let dim = self.dim.load(...);
       let cut = self.cut.load(...);
       Some((volume, dim, cut))  // ← 返回 (0.0, true, false)
   }
   ```

4. Editor.rs 应用**所有值**，包括错误的 volume=0：
   ```rust
   if let Some((volume, dim, cut)) = osc_state.get_pending_changes() {
       setter.set_parameter(&params.master_gain, volume);  // ← 设置为 0！
       setter.set_parameter(&params.dim, ...);
       setter.set_parameter(&params.cut, ...);
   }
   ```

**修复方案**：使用分离的 pending 标志追踪每个值的变化

**文件**: `Osc.rs:66-261`, `Editor.rs:151-175`

**修复代码**:

```rust
// Osc.rs - 修改 OscSharedState 结构体
pub struct OscSharedState {
    // 分离的 pending 标志
    pub volume_pending: AtomicBool,
    pub dim_pending: AtomicBool,
    pub cut_pending: AtomicBool,
    // 原有字段保持不变
    pub master_volume: AtomicU32,
    pub dim: AtomicBool,
    pub cut: AtomicBool,
    // 删除 has_pending 字段
}

impl OscSharedState {
    pub fn new() -> Self {
        Self {
            volume_pending: AtomicBool::new(false),
            dim_pending: AtomicBool::new(false),
            cut_pending: AtomicBool::new(false),
            master_volume: AtomicU32::new(0),
            dim: AtomicBool::new(false),
            cut: AtomicBool::new(false),
            // ...
        }
    }

    /// 设置 Master Volume (从 OSC 接收)
    pub fn set_master_volume(&self, value: f32) {
        self.master_volume.store(value.to_bits(), Ordering::Release);
        self.volume_pending.store(true, Ordering::Release);  // ← 只标记 volume
    }

    /// 设置 Dim (从 OSC 接收)
    pub fn set_dim(&self, on: bool) {
        self.dim.store(on, Ordering::Release);
        self.dim_pending.store(true, Ordering::Release);  // ← 只标记 dim
    }

    /// 设置 Cut (从 OSC 接收)
    pub fn set_cut(&self, on: bool) {
        self.cut.store(on, Ordering::Release);
        self.cut_pending.store(true, Ordering::Release);  // ← 只标记 cut
    }

    /// 获取并清除 Volume 变化（返回 Option）
    pub fn take_pending_volume(&self) -> Option<f32> {
        if self.volume_pending.swap(false, Ordering::Acquire) {
            Some(f32::from_bits(self.master_volume.load(Ordering::Acquire)))
        } else {
            None
        }
    }

    /// 获取并清除 Dim 变化（返回 Option）
    pub fn take_pending_dim(&self) -> Option<bool> {
        if self.dim_pending.swap(false, Ordering::Acquire) {
            Some(self.dim.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// 获取并清除 Cut 变化（返回 Option）
    pub fn take_pending_cut(&self) -> Option<bool> {
        if self.cut_pending.swap(false, Ordering::Acquire) {
            Some(self.cut.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// 检查是否有任何待处理的 OSC 变化
    pub fn has_osc_override(&self) -> bool {
        self.volume_pending.load(Ordering::Acquire)
            || self.dim_pending.load(Ordering::Acquire)
            || self.cut_pending.load(Ordering::Acquire)
    }
}
```

**Editor.rs 修改**:
```rust
// 替换原来的 get_pending_changes() 调用
// 分别处理每个参数，只在有变化时更新

if let Some(volume) = osc_state_clone.take_pending_volume() {
    setter.begin_set_parameter(&params.master_gain);
    setter.set_parameter(&params.master_gain, volume);
    setter.end_set_parameter(&params.master_gain);
    logger_clone.info("editor", &format!("[OSC Recv] Volume: {:.3}", volume));
}

if let Some(dim) = osc_state_clone.take_pending_dim() {
    setter.begin_set_parameter(&params.dim);
    setter.set_parameter(&params.dim, dim);
    setter.end_set_parameter(&params.dim);
    osc_state_clone.send_dim(dim);
    logger_clone.info("editor", &format!("[OSC Recv] Dim: {}", dim));
}

if let Some(cut) = osc_state_clone.take_pending_cut() {
    setter.begin_set_parameter(&params.cut);
    setter.set_parameter(&params.cut, cut);
    setter.end_set_parameter(&params.cut);
    osc_state_clone.sync_cut_state(cut);
    osc_state_clone.send_cut(cut);
    logger_clone.info("editor", &format!("[OSC Recv] Cut: {}", cut));
}
```

---

## 三、性能优化 (Performance Optimization)

### 🔴 关键性能问题 (Critical Performance)

| # | 问题 | 位置 | 影响 | 预期收益 |
|---|------|------|------|---------|
| **P1** | process() 中 RwLock 检查 | Lib.rs:214-289 | 每 Block 获取锁 | -50% 锁调用 |
| **P2** | Tokio Runtime 过重 | Network.rs:68,174 | 多线程开销 | -30% CPU |
| **P3** | update_snapshot() HashSet 克隆 | Interaction.rs:415-450 | 内存分配 | -80% 分配 |
| **P4** | 原子操作 Ordering 错误 | Osc.rs:168-184 | 丢失更新风险 | 正确性修复 |

### 🟡 中等性能问题 (Medium)

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| **P5** | 100ms Role 轮询 | Network.rs:92, Osc.rs:626 | CPU 唤醒 |
| **P6** | to_network_state 8个锁 | Interaction.rs:1204 | 锁竞争 |
| **P7** | get_channel_display 多锁 | Interaction.rs:932 | UI 卡顿 |
| **P8** | OSC 线程过多 | Osc.rs | 5线程/实例 |

### 🟢 优化机会 (Optimization)

| # | 问题 | 影响 |
|---|------|------|
| **P9** | 无 SIMD 优化 | 吞吐量可提升 4x |
| **P10** | 缓存不友好访问 | L1 命中率低 |

---

## 四、优化方案详情

### Phase 1: 快速修复 (立即见效)

#### P1: process() 快速路径 - 避免 99% 的 RwLock

**文件**: `Lib.rs:214-289`

**问题**: 每个音频 Block 都调用 `take_osc_restart_request()` 和 `take_network_restart_request()`，即使没有重启请求也获取 RwLock。

**修复**:
```rust
// Interaction.rs - 添加快速检查标志
pub struct InteractionManager {
    // 新增
    osc_restart_pending: AtomicBool,
    network_restart_pending: AtomicBool,
    // 保持原有
    osc_restart_config: RwLock<Option<AppConfig>>,
    network_restart_config: RwLock<Option<AppConfig>>,
}

// 快速检查（无锁）
#[inline]
pub fn has_osc_restart_pending(&self) -> bool {
    self.osc_restart_pending.load(Ordering::Relaxed)
}

// 仅在需要时获取锁
pub fn take_osc_restart_request(&self) -> Option<AppConfig> {
    if !self.osc_restart_pending.load(Ordering::Relaxed) {
        return None;  // 快速路径，无锁
    }
    let config = self.osc_restart_config.write().take();
    if config.is_some() {
        self.osc_restart_pending.store(false, Ordering::Relaxed);
    }
    config
}

// 请求时设置标志
pub fn request_osc_restart(&self, config: AppConfig) {
    *self.osc_restart_config.write() = Some(config);
    self.osc_restart_pending.store(true, Ordering::Release);
}
```

---

#### P2: Tokio Runtime 轻量化

**文件**: `Network.rs:68, 174`

**问题**: 使用 `Runtime::new()` 创建多线程 Runtime，但网络线程只需要单线程。

**修复**:
```rust
// 修改前
let rt = Runtime::new()?;

// 修改后 - 单线程 Runtime
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;
```

**收益**:
- 内存减少 ~2-3MB/实例
- 启动延迟 -50-100ms
- CPU 开销 -30%

---

#### P4: 修复原子操作 Ordering

**文件**: `Osc.rs:168-261`

**问题**: 使用 `Relaxed` ordering，OSC 线程写入的值可能不被音频线程看到。

**修复**: 写入方使用 `Ordering::Release`，读取方使用 `Ordering::Acquire`

---

### Phase 2: 锁竞争优化

#### P3: 消除 update_snapshot() 中的 HashSet 克隆

**文件**: `Interaction.rs:415-450`

**问题**: 每次快照更新都克隆 HashSet，造成内存分配。

**方案**: 原地计算掩码（推荐）

---

#### P6: 优化 to_network_state() 的 8 个锁

**文件**: `Interaction.rs:1204-1242`

**问题**: 顺序获取 8 个 RwLock，造成严重锁竞争。

**修复**: 合并读取

---

## 五、优先级排序

### 🚨 最高优先级 - Bug 修复 (MUST FIX)

| # | 修改 | 文件 | 状态 |
|---|------|------|------|
| **B1** | Dim 音量归零 Bug | Osc.rs, Editor.rs | 待修复 |

### P0 - 性能关键 (预计收益最大)

| # | 修改 | 文件 | 预期 CPU 降低 |
|---|------|------|-------------|
| **P1** | process() 快速路径 | Lib.rs, Interaction.rs | 5-10% |
| **P2** | Tokio 单线程 Runtime | Network.rs | 10-15% |
| **P4** | Ordering 修复 | Osc.rs | 正确性 |

### P1 - 推荐实现

| # | 修改 | 文件 | 预期收益 |
|---|------|------|---------|
| **P3** | 消除 HashSet 克隆 | Interaction.rs | 减少分配 |
| **P6** | 合并锁读取 | Interaction.rs | 减少竞争 |

---

## 六、关键文件修改清单

| 文件 | 修改内容 |
|------|---------|
| `Osc.rs` | B1 分离 pending 标志, P4 Ordering 修复 |
| `Editor.rs` | B1 分别处理 volume/dim/cut 变化 |
| `Interaction.rs` | P1 快速路径标志, P3 消除克隆, P6 合并锁 |
| `Lib.rs` | P1 快速路径检查 |
| `Network.rs` | P2 单线程 Runtime |

---

## 七、预期总体收益

| 指标 | 当前 | 优化后 | 改善 |
|------|------|--------|------|
| 音频线程锁调用 | 每 Block 2次 | 接近 0 | -99% |
| 内存分配/快照更新 | 3次 HashSet 克隆 | 0 | -100% |
| Tokio Runtime 开销 | 多线程 | 单线程 | -30% |
| 原子操作正确性 | Relaxed (有风险) | Acquire/Release | 正确 |

---

## 八、测试验证

### Bug 修复测试
1. 启动 DAW，加载 MonitorControllerMax
2. 通过硬件控制器按下 Dim 按钮
3. 确认 UI 音量显示**不变化**
4. 确认只有 Dim 状态改变
5. 手动调整音量旋钮，确认音量正常响应

### 性能测试
1. 在 Mac M1/M2 上运行 DAW
2. 加载 10 个 MonitorControllerMax 实例
3. 播放 48kHz 7.1.4 音频
4. 监控 CPU 占用率
5. 确认无爆音

---

## 九、硬件控制器分析 (MonitorOSCPlugin)

**代码位置**: `c:\Code\LogiPluginSdkTools\MonitorOSCPlugin\src`

### 问题确认

硬件控制器代码**没有问题**，问题在 Rust 后端的 `OscSharedState`：
- 硬件只发送 Dim toggle 请求
- 后端错误地将未初始化的 volume=0 一起返回
- 修复应该在 Rust 端，不需要修改硬件控制器代码

---

## 十、实施步骤

### 第一阶段：修复 Dim Bug (B1)
1. 修改 `Osc.rs` - 添加分离的 pending 标志
2. 修改 `Editor.rs` - 分别处理每个参数变化
3. 测试验证

### 第二阶段：性能优化 (P1-P4)
1. 修改 `Interaction.rs` - 添加快速路径标志
2. 修改 `Lib.rs` - 使用快速路径检查
3. 修改 `Network.rs` - 切换到单线程 Runtime
4. 性能测试验证

### 第三阶段：可选优化 (P5-P6)
根据测试结果决定是否实施

---

**文档生成时间**: 2025-12-15
**版本**: v4
