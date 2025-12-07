# OSC 双向通信架构方案 v6 - 逻辑统一

## 核心问题：语义不匹配

### C# 端发送逻辑（Channel_Button_Base.cs:40-42）

```csharp
var currentState = OSCStateManager.Instance.GetState(this.ChannelAddress);
var newVal = currentState == 2f ? 0f : 2f;  // Solo→发0, 非Solo→发2
MonitorOSCPlugin.SendOSCMessage(this.ChannelAddress, newVal);
```

**C# 发送的是「目标状态」**（0=Off, 1=Mute, 2=Solo）

### VST 端处理逻辑（Osc.rs:597-601）

```rust
if value > 0.5 {
    INTERACTION.handle_click(ch_idx);  // toggle 操作
    Self::broadcast_channel_states();
}
```

**VST 只响应 value > 0.5，执行的是「toggle」**

### 问题场景

L 通道已 Solo 时，用户点击硬件 L 按钮：

1. C# 缓存：L = 2（Solo）
2. C# 计算：`newVal = 0f`（想取消 Solo）
3. C# 发送：`/Monitor/Channel/L = 0.0`
4. **VST 收到 0.0 ≤ 0.5，被忽略！**
5. 状态未改变

**这就是硬件无法取消 Solo 的根因！**

---

## 解决方案：统一为「点击事件」模式

### 设计原则

**C# 端只发送「点击事件」，不计算目标状态**

- 始终发送 `1.0` 表示"用户点击了这个按钮"
- VST 端负责计算状态变化（toggle）
- VST 广播新状态回 C# 端更新 LED

### 通信流程

```
┌─────────────────┐                    ┌─────────────────┐
│  C# (硬件)      │                    │  VST (逻辑核心) │
├─────────────────┤                    ├─────────────────┤
│ 用户点击 L      │                    │                 │
│ → 发送 1.0      │ ──────────────────→│ 收到点击事件    │
│   (点击事件)    │                    │ → toggle(L)     │
│                 │                    │ → 广播所有状态  │
│ 收到 L=0/1/2    │ ←──────────────────│ ← 发送新状态    │
│ → 更新 LED      │                    │                 │
└─────────────────┘                    └─────────────────┘
```

**VST 是状态的唯一权威来源**

---

## C# 端修改

### Channel_Button_Base.cs（关键修改）

```csharp
protected override void RunCommand(string actionParameter)
{
    // 不再读取本地状态，直接发送点击事件
    // VST 会处理 toggle 逻辑并广播新状态回来
    MonitorOSCPlugin.SendOSCMessage(this.ChannelAddress, 1f);  // 1.0 = 点击事件
}
```

### Group_Dial_Base.cs

旋钮逻辑可以保持不变（发送 0/2），或也改为点击事件模式。 建议：旋钮的 Solo/Mute 切换也改为发送点击事件，让 VST 统一处理。

---

## VST 端修改

### Osc.rs handle_channel_click（无需修改）

现有逻辑已正确：

```rust
if value > 0.5 {  // 1.0 会触发
    INTERACTION.handle_click(ch_idx);  // toggle
    Self::broadcast_channel_states();  // 广播新状态
}
```

### 延迟优化（已完成）

发送线程已优化为阻塞等待 + 批量处理。

---

## 修改文件清单

### C# 端

1. **Channel_Button_Base.cs** - RunCommand 改为只发送 1.0
2. **Group_Dial_Base.cs** - （可选）统一为点击事件模式

### VST 端

- ✅ 无需修改，现有逻辑正确

---

## 验证清单

- [ ]  C# 端编译成功
- [ ]  Solo L → 硬件点击 L → 所有通道回归 Off
- [ ]  Solo L → VST GUI 点击 L → 硬件 LED 全部更新
- [ ]  Mute 模式双向同步
- [ ]  快速连续点击状态正确
- [ ]  布局切换后状态正确