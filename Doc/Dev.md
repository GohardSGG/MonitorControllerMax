# 监听控制器插件开发文档 - 大一统架构设计

## 📋 项目当前状态 (2025-01-09)

### ✅ 已完成的核心功能
1. **参数驱动架构** - 完全重构为参数驱动的纯函数式架构
2. **Solo/Mute联动机制** - 实现了与JSFX完全一致的联动逻辑
3. **状态记忆系统** - Solo进入/退出时的Mute状态保存和恢复
4. **UI实时同步** - 30Hz定时器确保UI与参数完全同步
5. **通道按钮逻辑** - 修复了操作逻辑，基于选择模式状态
6. **颜色配置系统** - 使用正确的customLookAndFeel配色方案
7. **VST3调试系统** - 完整的日志记录系统便于开发调试
8. **选择模式架构** - 实现完整的选择模式状态管理系统

### 🚀 最新突破：大一统架构
**双重状态判断系统**：
- 参数激活状态 + 选择模式状态 = 完整的按钮状态控制
- 主按钮激活显示 = 实际参数激活 OR 选择模式等待
- 不自动激活任何通道，完全按照架构文档实现
- 统一的参数驱动逻辑，无状态机复杂度

### ⚠️ 需要验证的功能
1. **选择模式UI反馈** - 确保选择模式下按钮正确显示激活状态
2. **VST3宿主参数同步** - 在REAPER中测试参数窗口与UI的双向同步
3. **Master-Slave通信** - 多实例间的状态同步
4. **完整功能测试** - 对比JSFX版本确保功能完全一致

## 🏗️ 大一统架构设计

### 设计哲学：双重状态判断系统

**核心理念：参数系统 + 选择模式状态 = 完整状态控制**

```
用户操作 → 选择模式变化 → 参数变化 → 自动联动计算 → UI自动同步
```

### 核心状态系统

**双重状态判断**：
- `hasAnySoloActive()` - 检查是否有通道被Solo
- `hasAnyMuteActive()` - 检查是否有通道被Mute
- `isInSoloSelectionMode()` - 等待用户点击通道的Solo选择状态
- `isInMuteSelectionMode()` - 等待用户点击通道的Mute选择状态

**主按钮激活显示**：
- Solo按钮激活（绿色）= `hasAnySoloActive() || isInSoloSelectionMode()`
- Mute按钮激活（红色）= `hasAnyMuteActive() || isInMuteSelectionMode()`

### 分层架构

```
┌─────────────────────────────────────────┐
│                UI层 (前端)                 │
│        - 双重状态显示：参数+选择模式          │
│        - 30Hz定时器更新确保同步             │
│        - 选择模式视觉反馈                  │
└─────────────────────────────────────────┘
                    ↑ 读取双重状态
┌─────────────────────────────────────────┐
│            选择模式管理层                   │
│        - 选择模式状态跟踪                  │
│        - 主按钮点击逻辑                   │
│        - 模式切换和清除                   │
└─────────────────────────────────────────┘
                    ↑ 状态变化
┌─────────────────────────────────────────┐
│            参数联动层 (核心引擎)             │
│        - ParameterLinkageEngine          │
│        - 自动Solo/Mute联动计算             │
│        - 记忆管理和状态恢复                │
└─────────────────────────────────────────┘
                    ↑ 参数变化
┌─────────────────────────────────────────┐
│              输入层 (后端)                 │
│        - UI点击 → 选择模式/参数变化         │
│        - 宿主参数 → 参数变化               │
│        - 主从通信 → 参数变化               │
└─────────────────────────────────────────┘
```

## 🔧 核心组件实现

### 1. 双重状态判断系统 (MonitorControllerMaxAudioProcessor)

**主按钮状态判断**：
```cpp
// Solo按钮激活状态 = 有通道被Solo OR 处于Solo选择模式
bool isSoloButtonActive() const {
    return hasAnySoloActive() || isInSoloSelectionMode();
}

// Mute按钮激活状态 = 有通道被Mute OR 处于Mute选择模式  
bool isMuteButtonActive() const {
    return hasAnyMuteActive() || isInMuteSelectionMode();
}
```

**选择模式状态管理**：
```cpp
// 选择模式状态标志
std::atomic<bool> pendingSoloSelection{false};
std::atomic<bool> pendingMuteSelection{false};

// 选择模式判断
bool isInSoloSelectionMode() const;
bool isInMuteSelectionMode() const;
```

### 2. 主按钮功能逻辑

**Solo主按钮点击**：
```cpp
void handleSoloButtonClick() {
    if (hasAnySoloActive()) {
        // 有Solo参数激活 → 清除所有Solo + 清除选择模式
        clearAllSoloParameters();
        pendingSoloSelection.store(false);
    } else {
        // 无Solo参数激活 → 进入Solo选择模式（不激活任何通道）
        pendingSoloSelection.store(true);
        pendingMuteSelection.store(false);  // 切换模式
    }
}
```

**Mute主按钮点击**：
```cpp
void handleMuteButtonClick() {
    if (hasAnySoloActive()) return;  // Solo优先原则
    
    if (hasAnyMuteActive()) {
        // 有Mute参数激活 → 清除所有Mute + 清除选择模式
        clearAllMuteParameters();
        pendingMuteSelection.store(false);
    } else {
        // 无Mute参数激活 → 进入Mute选择模式（不激活任何通道）
        pendingMuteSelection.store(true);
        pendingSoloSelection.store(false);  // 切换模式
    }
}
```

### 3. 通道点击逻辑

**基于双重状态的通道操作**：
```cpp
void handleChannelClick(int channelIndex) {
    bool inSoloSelection = isInSoloSelectionMode();
    bool inMuteSelection = isInMuteSelectionMode();
    
    if (inSoloSelection) {
        // Solo选择模式 → 操作Solo参数
        toggleSoloParameter(channelIndex);
        pendingSoloSelection.store(false);  // 清除选择模式
    } else if (inMuteSelection) {
        // Mute选择模式 → 操作Mute参数
        toggleMuteParameter(channelIndex);
        pendingMuteSelection.store(false);  // 清除选择模式
    } else {
        // 初始状态 → 无效果
        VST3_DBG("Channel clicked in Initial state - no effect");
    }
}
```

### 4. ParameterLinkageEngine (核心引擎)

**主要功能**:
- 模仿JSFX的slider联动逻辑
- Solo状态变化检测和联动处理
- Mute状态记忆的保存和恢复
- 防止递归调用的保护机制

**关键方法**:
```cpp
void handleParameterChange(const String& paramID, float value);
bool hasAnySoloActive() const;
bool hasAnyMuteActive() const;
void saveCurrentMuteMemory();
void restoreMuteMemory();
void applyAutoMuteForSolo();  // Solo激活时的Mute联动
void clearAllSoloParameters();
void clearAllMuteParameters();
```

### 5. UI更新系统 (PluginEditor)

**实现方式**:
- 30Hz定时器 (`timerCallback()`) 确保UI实时更新
- UI状态完全从双重状态计算得出
- 选择模式下的视觉反馈

**主要方法**:
```cpp
void updateChannelButtonStates();  // 基于参数值更新所有按钮状态
void updateMainButtonStates();     // 基于双重状态更新主按钮显示
void timerCallback() override;     // 30Hz定时器确保同步
```

## 🎯 关键技术细节

### 1. 双重状态系统的优势

**选择模式不依赖参数激活**：
- 用户点击主按钮后，立即进入选择模式
- 按钮显示激活状态，但不激活任何通道参数
- 等待用户点击通道后才激活对应参数

**统一的按钮状态控制**：
- 实际参数激活 OR 选择模式等待 = 按钮激活显示
- 消除了"选择状态≠激活状态"的设计复杂度
- 用户看到的就是系统实际的状态

### 2. 参数联动机制

模仿JSFX的核心逻辑:
```cpp
// JSFX原理: slider11 = slider31 ? 0 : 1
// JUCE实现: 
void applyAutoMuteForSolo() {
    for (int i = 0; i < 26; ++i) {
        bool isSolo = getSoloParameter(i) > 0.5f;
        float newMuteValue = isSolo ? 0.0f : 1.0f;
        setMuteParameter(i, newMuteValue);
    }
}
```

### 3. 参数保护机制

**Solo模式下的Mute参数强制保护**:
```cpp
// 在parameterChanged中实现
if (paramID.startsWith("MUTE_") && hasAnySoloActive()) {
    // Solo模式下强制恢复Mute参数到联动计算值
    restoreAutoMuteValue(paramID);
    return;
}
```

### 4. Solo优先原则

**Mute主按钮的动态启用/禁用**:
```cpp
// 根据Solo状态动态控制Mute按钮可点击性
bool muteButtonEnabled = !hasAnySoloActive();
globalMuteButton.setEnabled(muteButtonEnabled);
```

### 5. 选择模式状态管理

**选择模式切换逻辑**：
```cpp
// 从Solo选择模式切换到Mute选择模式
void switchToMuteSelection() {
    pendingSoloSelection.store(false);
    pendingMuteSelection.store(true);
}

// 清除所有选择模式
void clearAllSelectionModes() {
    pendingSoloSelection.store(false);
    pendingMuteSelection.store(false);
}
```

## 🎮 典型操作场景

### 场景1：初始状态
- **状态**：无参数激活，无选择模式
- **UI显示**：Solo按钮非激活，Mute按钮非激活
- **点击Solo主按钮**：进入Solo选择模式 → Solo按钮变绿色激活
- **点击Mute主按钮**：进入Mute选择模式 → Mute按钮变红色激活

### 场景2：Solo选择模式
- **状态**：Solo选择模式激活，无参数激活
- **UI显示**：Solo按钮激活（绿色），Mute按钮非激活
- **点击通道1**：激活SOLO_1 + 清除选择模式 → 进入实际Solo状态
- **点击Mute主按钮**：切换到Mute选择模式

### 场景3：实际Solo激活
- **状态**：有Solo参数激活，无选择模式
- **UI显示**：Solo按钮激活（绿色），Mute按钮激活（红色，Auto-Mute）
- **点击Solo主按钮**：清除所有Solo → 恢复记忆 → 回到对应状态
- **点击Mute主按钮**：无效果（Solo优先原则）

### 场景4：选择模式切换
- **从Solo选择模式**：点击Mute主按钮 → 切换到Mute选择模式
- **从Mute选择模式**：点击Solo主按钮 → 切换到Solo选择模式
- **条件**：只有在没有实际参数激活时才能切换

## 📋 开发状态总结

### 已解决的问题
1. ✅ **选择模式显示问题** - 实现双重状态判断系统
2. ✅ **主按钮激活逻辑** - 选择模式 OR 参数激活 = 按钮激活
3. ✅ **不自动激活通道** - 选择模式纯粹等待用户操作
4. ✅ **参数与UI同步** - 完全的参数驱动架构
5. ✅ **Solo优先原则** - Mute按钮在Solo模式下完全失效
6. ✅ **状态记忆功能** - Solo进入/退出时的Mute状态保存恢复
7. ✅ **参数保护机制** - Solo模式下Mute参数的强制保护
8. ✅ **VST3调试系统** - 完整的日志记录便于开发调试

### 技术架构特点
- **双重状态系统** - 参数激活 + 选择模式的完整状态控制
- **不自动激活** - 严格按照架构文档，选择模式等待用户操作
- **统一触发点** - 所有逻辑变更都通过parameterChanged触发
- **完全同步** - 参数系统 + 选择模式 = 完整状态控制
- **防护机制** - 递归调用防护和状态重置
- **调试友好** - 完整的VST3调试日志系统

### 下一步工作
1. 实现并测试完整的选择模式状态管理
2. 验证双重状态系统的UI反馈正确性
3. 在REAPER中测试VST3参数窗口同步功能
4. 验证Master-Slave多实例通信
5. 与JSFX版本进行完整功能对比测试
6. 根据测试结果进行最终优化

## 🔧 实现要点

### 关键设计原则
1. **选择模式独立跟踪** - 不依赖参数激活状态
2. **双重按钮状态** - 实际激活 OR 选择模式 = 按钮激活显示
3. **Solo绝对优先** - Solo存在时Mute主按钮完全失效
4. **统一记忆管理** - 只在parameterChanged中处理
5. **不自动激活通道** - 选择模式纯粹等待用户操作

### 实现细节
- 使用原子标志跟踪选择模式状态
- 主按钮点击逻辑简化为状态切换
- 通道点击后自动清除选择模式
- UI更新基于双重状态计算
- 保持与架构文档的完全一致性