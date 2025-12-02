# 

## 问题根因

### Bug 现象

Solo L → 取消 Solo → 所有 VST3 参数变为 Off

### 根本原因

**Interaction.rs 第 553-558 行**：

```rust
// Idle 状态返回 has_sound: false（错误！）
if primary == PrimaryMode::None {
    return ChannelDisplay {
        has_sound: false,  // ← BUG! 应该是 true
        marker: None,
        is_blinking: false,
    };
}
```

**语义错误**：

- Idle 状态的 **UI 显示**应该是灰色（无标记）✓
- Idle 状态的 **音频行为**应该是全通（has_sound = true）✗

---

## 修复方案

### 1. 修复 `get_channel_display()` 的 Idle 返回值

**文件**：`Interaction.rs` 第 553-558 行

```rust
// 修改前
if primary == PrimaryMode::None {
    return ChannelDisplay {
        has_sound: false,  // ← 错误
        marker: None,
        is_blinking: false,
    };
}

// 修改后
if primary == PrimaryMode::None {
    return ChannelDisplay {
        has_sound: true,   // ← Idle = 全通
        marker: None,
        is_blinking: false,
    };
}
```

---

## 调试日志系统

### 设计原则

1. **单行精简**：一行描述一个完整操作
2. **批量压缩**：通道状态用位掩码/列表表示，不逐行输出
3. **关键节点**：只记录状态变化点

### 日志格式规范

#### 全局按钮操作

```
[SM] SOLO: (None,None)->(Solo,None) set=0x0
[SM] MUTE: (Solo,None)->(Solo,Mute) set=0x3
[SM] EXIT: (Solo,Mute)->(None,None) cleared
```

- `SM` = State Machine
- 状态用 `(primary,compare)` 表示
- `set=0x...` 显示当前集合的位掩码

#### 通道点击

```
[CH] Main0 click: solo_set 0x0->0x1, sync=[On,Off,Off,Off,Off,Off]
[CH] Sub6 dblclick: user_mute 0x0->0x40
```

- 显示操作类型、集合变化、同步结果摘要

#### VST3 同步

```
[SYNC] 6ch: mask=0b000001 (L=On, others=Off)
[SYNC] 8ch: mask=0b11111100 (1-2=On, 3-8=Off)
```

- 显示总通道数、位掩码、人类可读摘要

#### 布局切换

```
[LAYOUT] 2.0->7.1 (2ch->6ch), sync triggered
```

#### 自动化模式

```
[AUTO] Enter: cleared all state, params unchanged
[AUTO] Exit: idle state, sync all=On
```

### 实现位置

|位置|日志内容|
|---|---|
|`on_solo_button_click()`|状态转换 + 集合变化|
|`on_mute_button_click()`|状态转换 + 集合变化|
|`on_channel_click()`|通道操作 + 集合变化|
|`sync_all_channel_params()`|同步结果摘要|
|布局切换处检测|布局变化|
|自动化模式切换|模式变化|

---

## 边界情况完整清单

### 测试场景矩阵

|#|场景|预期 VST3 状态|
|---|---|---|
|1|加载 → Idle|全 On|
|2|Idle → SOLO → Idle|全 On|
|3|SOLO → 点击 L → Idle|全 On|
|4|SOLO → 点击 L → 点击 R → Idle|全 On|
|5|MUTE → 点击 L → Idle|全 On|
|6|SOLO L → 切换布局 2.0→7.1|L=On, 其他=Off|
|7|Solo L → Compare(MUTE) → 退出|全 On|
|8|启用自动化 → 退出自动化|全 On|

### 关键验证点

1. **Idle 状态恒等于全通**
    
    - `has_sound: true` for all channels
    - VST3 Enable 全 On
2. **Solo 模式下的互斥**
    
    - 被 Solo 的通道 = On
    - 其他通道 = Off
3. **Mute 模式下的互斥**
    
    - 被 Mute 的通道 = Off
    - 其他通道 = On
4. **布局切换保持状态**
    
    - InteractionManager 状态不变
    - VST3 参数根据新布局重新计算

---

## 实现步骤

### Step 1: 修复 Idle 状态返回值

**文件**：`Interaction.rs`

- 将第 555 行 `has_sound: false` 改为 `has_sound: true`

### Step 2: 添加调试日志宏

**文件**：新建 `debug_log.rs` 或直接在相关文件添加

```rust
/// 生成通道状态摘要字符串
fn channel_mask_summary(layout_channels: usize, mask: u32) -> String {
    let on_count = mask.count_ones();
    let off_count = layout_channels as u32 - on_count;
    format!("{}ch: {}on/{}off mask=0x{:x}", layout_channels, on_count, off_count, mask)
}
```

### Step 3: 在关键位置添加日志

#### Interaction.rs

```rust
// on_solo_button_click() 末尾
mcm_info!("[SM] SOLO: {:?}->{:?} solo=0x{:x} mute=0x{:x}",
    (old_primary, old_compare),
    (*self.primary.read(), *self.compare.read()),
    self.solo_set.read().main,
    self.mute_set.read().main);
```

#### Editor.rs - sync_all_channel_params()

```rust
fn sync_all_channel_params(...) {
    // ... 同步逻辑 ...

    // 生成摘要
    let mut on_mask: u32 = 0;
    for i in 0..layout.total_channels {
        let display = interaction.get_channel_display(i, is_sub);
        if display.has_sound { on_mask |= 1 << i; }
        // ... 同步 ...
    }
    mcm_info!("[SYNC] {}ch on_mask=0x{:x}", layout.total_channels, on_mask);
}
```

### Step 4: 验证所有边界场景

按照上面的测试场景矩阵逐一验证。

---

## 文件修改清单

|文件|修改内容|
|---|---|
|`Interaction.rs`|1. 修复 Idle 返回值<br>2. 添加状态变化日志|
|`Editor.rs`|1. sync 函数添加摘要日志<br>2. 布局切换添加日志|

---

## 预期结果

修复后的行为：

1. **Idle 状态**：所有通道 `has_sound=true`，VST3 全 On
2. **进入 Solo/Mute**：按状态机逻辑计算 `has_sound`
3. **退出到 Idle**：恢复全 On
4. **布局切换**：保持状态，重新计算新布局的通道

日志输出示例：

```
[SM] SOLO: (None,None)->(Solo,None) solo=0x0 mute=0x0
[CH] Main0 click: solo_set 0x0->0x1
[SYNC] 2ch on_mask=0x1
[SM] SOLO: (Solo,None)->(None,None) cleared
[SYNC] 2ch on_mask=0x3
```