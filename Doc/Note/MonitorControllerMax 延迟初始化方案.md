# MonitorControllerMax 延迟初始化方案

## 问题根源

### DAW 初始化时序问题

```
default()      → 创建实例，role = Standalone（默认值）
initialize()   → 读取 role，此时仍是默认值！
[DAW 恢复参数] → role 被恢复为保存的值（如 Slave）
reset()        → 此时 role 才正确
process()      → 正常运行
```

**日志证据**（来自 MCM_2025-12-10_17-23-22_108CEF0B.log）：

- 17:23:22.897: `[OSC] Initialized for Standalone mode` ← 错误！应该是 Slave
- 17:23:22.898: `[OSC] Disabled for Slave mode` ← 第二次 init 时参数已恢复

### 结论

`initialize()` 时读取的 `params.role.value()` 总是默认值，因为 DAW 在 `initialize()` 返回后才恢复保存的参数。

---

## 解决方案：延迟初始化

**核心思想**：不在 `initialize()` 中初始化 OSC/Network，延迟到 `reset()` 执行。

### 新的初始化时序

```
default()      → 创建实例，不做任何初始化
initialize()   → 只记录 audio layout 信息，标记需要延迟初始化
[DAW 恢复参数]
reset()        → 读取正确的 role，执行延迟初始化 ✓
process()      → 安全网：如果 reset 未调用，在这里初始化
```

---

## 实施步骤

### Step 1: 修改 MonitorControllerMax 结构体

**文件**: `Source/Plugin/Src/Lib.rs` (L35-51)

 

添加两个新字段：

```rust
pub struct MonitorControllerMax {
    // ... 现有字段 ...

    /// 是否需要延迟初始化（在 reset/process 中执行）
    needs_deferred_init: bool,
    /// 是否已完成延迟初始化
    deferred_init_done: bool,
}
```

### Step 2: 修改 default()

**文件**: `Source/Plugin/Src/Lib.rs` (L53-78)

 

初始化新字段：

```rust
Self {
    // ... 现有字段 ...
    needs_deferred_init: false,
    deferred_init_done: false,
}
```

### Step 3: 修改 initialize() - 不再初始化 OSC/Network

**文件**: `Source/Plugin/Src/Lib.rs` (L114-180)

 

删除所有 `self.network.init_*()` 和 `self.osc.init()` 调用，改为：

```rust
fn initialize(&mut self, audio_io_layout: &AudioIOLayout, ...) -> bool {
    // ... 保留 audio layout 日志 ...

    // 只记录 output_channels
    self.output_channels = output_channels as usize;

    // 注册实例
    Registry::GlobalRegistry::register_instance();

    // ★ 关键：标记需要延迟初始化，不在这里初始化
    self.needs_deferred_init = true;
    self.deferred_init_done = false;
    self.last_role = None;  // 清空，等待真正初始化时设置

    mcm_info!("[Init] Deferred initialization scheduled");
    true
}
```

### Step 4: 新增 perform_deferred_init() 方法

**文件**: `Source/Plugin/Src/Lib.rs` (新增方法)

```rust
impl MonitorControllerMax {
    /// 执行延迟初始化（在参数恢复后调用）
    fn perform_deferred_init(&mut self) {
        if self.deferred_init_done {
            return;  // 已完成，跳过
        }

        let role = self.params.role.value();  // ✓ 此时参数已正确
        mcm_info!("[DeferredInit] Role = {:?}, output_channels = {}", role, self.output_channels);

        match role {
            Params::PluginRole::Master => {
                self.network.init_master(9123);
                let (vol, dim, cut) = (
                    self.params.master_gain.value(),
                    self.params.dim.value(),
                    self.params.cut.value()
                );
                self.osc.init(self.output_channels, vol, dim, cut, role,
                             self.interaction.clone(), &self.instance_id);
                mcm_info!("[DeferredInit] Master: Network + OSC initialized");
                self.ui_log.log(ui_log::LogLevel::Info, "Role: Master - OSC enabled");
            }
            Params::PluginRole::Slave => {
                self.network.init_slave("127.0.0.1", 9123);
                // Slave 永远不初始化 OSC
                mcm_info!("[DeferredInit] Slave: Network only, OSC disabled");
                self.ui_log.log(ui_log::LogLevel::Info, "Role: Slave - No OSC");
            }
            Params::PluginRole::Standalone => {
                let (vol, dim, cut) = (
                    self.params.master_gain.value(),
                    self.params.dim.value(),
                    self.params.cut.value()
                );
                self.osc.init(self.output_channels, vol, dim, cut, role,
                             self.interaction.clone(), &self.instance_id);
                mcm_info!("[DeferredInit] Standalone: OSC initialized");
                self.ui_log.log(ui_log::LogLevel::Info, "Role: Standalone - OSC enabled");
            }
        }

        self.last_role = Some(role);
        self.deferred_init_done = true;
        self.needs_deferred_init = false;
    }
}
```

### Step 5: 修改 reset() - 执行延迟初始化

**文件**: `Source/Plugin/Src/Lib.rs` (L182-199)

```rust
fn reset(&mut self) {
    logger::set_current_instance(&self.instance_id);
    mcm_info!("Plugin reset() called");

    // ★ 执行延迟初始化（此时参数已被 DAW 恢复）
    if self.needs_deferred_init {
        self.perform_deferred_init();
    }

    // 原有的广播逻辑
    let role = self.params.role.value();
    if role != Params::PluginRole::Slave {
        let channel_count = CURRENT_CHANNEL_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let (vol, dim, cut) = (
            self.params.master_gain.value(),
            self.params.dim.value(),
            self.params.cut.value()
        );
        mcm_info!("[Reset] Broadcasting state: vol={:.4}, dim={}, cut={}", vol, dim, cut);
        self.osc.broadcast_state(channel_count, vol, dim, cut);
    }
}
```

### Step 6: 修改 process() - 添加安全网

**文件**: `Source/Plugin/Src/Lib.rs` (L201-255)

 

在运行时 Role 切换检测之前添加：

```rust
fn process(&mut self, buffer: &mut Buffer, ...) -> ProcessStatus {
    logger::set_current_instance(&self.instance_id);

    // ★ 安全网：如果 reset() 未被调用，在这里执行延迟初始化
    if self.needs_deferred_init && !self.deferred_init_done {
        mcm_info!("[Process] Executing deferred init (reset was skipped)");
        self.perform_deferred_init();
    }

    // ... 原有的运行时 Role 切换检测逻辑保持不变 ...
}
```

---

## 需要修改的文件汇总

|文件|修改内容|
|---|---|
|`Lib.rs` L35-51|添加 `needs_deferred_init`, `deferred_init_done` 字段|
|`Lib.rs` L66-76|在 `default()` 中初始化新字段|
|`Lib.rs` L114-180|删除 `initialize()` 中的 OSC/Network 初始化代码|
|`Lib.rs` (新增)|添加 `perform_deferred_init()` 方法|
|`Lib.rs` L182-199|在 `reset()` 中调用延迟初始化|
|`Lib.rs` L201-255|在 `process()` 中添加安全网检查|

---

## 测试验证

### 测试场景 1: 加载包含 Slave 的项目

1. 创建项目，设置实例 A = Master，实例 B = Slave
2. 保存项目，关闭 DAW
3. 重新打开项目

**预期日志**：

```
[Plugin] Creating new instance              // default()
Plugin initialize() called                   // initialize()
[Init] Deferred initialization scheduled     // 不再有 "Initialized for Standalone"
Plugin reset() called                        // reset()
[DeferredInit] Role = Slave, ...            // ✓ 正确识别为 Slave
[DeferredInit] Slave: Network only, OSC disabled
```

### 测试场景 2: 运行时切换 Role

1. 打开插件（默认 Standalone）
2. 切换 Role 为 Slave
3. 切换 Role 为 Master

**预期**: OSC 正确关闭/重新初始化

### 测试场景 3: 多实例 Master + Slave

1. 实例 A 设为 Master
2. 实例 B 设为 Slave
3. 验证 Slave UI 显示 "Connected to Master"
4. 验证只有 Master 响应 OSC 硬件控制

---

## 方案优势

1. **彻底解决时序问题**: 延迟到 `reset()` 确保参数已恢复
2. **最小化改动**: 只修改 `Lib.rs`，不改变 OSC/Network 模块
3. **向后兼容**: 不改变插件对外接口
4. **双重保险**: `process()` 安全网确保即使 DAW 跳过 `reset()` 也能正常工作
5. **清晰的日志**: `[DeferredInit]` 前缀便于调试验证