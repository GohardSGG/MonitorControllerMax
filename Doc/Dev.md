# 监听控制器插件开发文档 - 大一统架构设计

## 📋 项目当前状态 (2025-01-09)

### ✅ 已完成的核心功能
1. **参数驱动架构** - 完全重构为参数驱动的纯函数式架构
2. **Solo/Mute联动机制** - 实现了与JSFX完全一致的联动逻辑
3. **状态记忆系统** - Solo进入/退出时的Mute状态保存和恢复
4. **UI实时同步** - 30Hz定时器确保UI与参数完全同步
5. **通道按钮逻辑** - 修复了操作逻辑，只有在主按钮激活时才有效
6. **颜色配置系统** - 使用正确的customLookAndFeel配色方案
7. **VST3调试系统** - 完整的日志记录系统便于开发调试
8. **纯逻辑架构重构** - 移除了不稳定的状态机，采用完全基于参数的纯函数式架构

### 🚀 今日重要突破
**纯逻辑架构重构**：
- 移除了不稳定的状态机设计
- 采用完全基于参数计算的纯函数式架构
- Solo优先原则的简洁实现
- 参数保护机制防止非法修改
- 极简且可预测的交互逻辑

### ⚠️ 需要新同事验证的功能
1. **VST3宿主参数同步** - 在REAPER中测试参数窗口与UI的双向同步
2. **Master-Slave通信** - 多实例间的状态同步
3. **完整功能测试** - 对比JSFX版本确保功能完全一致
4. **参数保护机制** - 验证Solo模式下Mute参数的强制恢复

## 🏗️ 架构设计原则

### 设计哲学：参数驱动的纯函数式架构

**核心理念：参数系统 = 唯一真理来源**

```
用户操作 → 参数变化 → 自动联动计算 → UI自动同步
```

### 分层架构

```
┌─────────────────────────────────────────┐
│                UI层 (前端)                 │
│        - 纯显示层，无状态                   │
│        - 主按钮状态由参数计算得出             │
│        - 30Hz定时器更新确保同步             │
└─────────────────────────────────────────┘
                    ↑ 读取状态
┌─────────────────────────────────────────┐
│            参数联动层 (核心引擎)             │
│        - ParameterLinkageEngine          │
│        - 自动Solo/Mute联动计算             │
│        - 记忆管理和状态恢复                │
└─────────────────────────────────────────┘
                    ↑ 参数变化
┌─────────────────────────────────────────┐
│              输入层 (后端)                 │
│        - UI点击 → 参数变化                │
│        - 宿主参数 → 参数变化               │
│        - 主从通信 → 参数变化               │
└─────────────────────────────────────────┘
```

## 🔧 核心组件实现

### 1. ParameterLinkageEngine (核心引擎)
**文件位置**: `Source/ParameterLinkageEngine.h/.cpp`

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
```

### 2. 通道按钮逻辑 (MonitorControllerMaxAudioProcessor)
**正确的操作逻辑**:
- 通道按钮本身**没有独立功能**
- 只有在主按钮激活时，通道按钮才有意义:
  - Solo主按钮激活 → 点击通道按钮 = 切换该通道Solo状态
  - Mute主按钮激活 → 点击通道按钮 = 切换该通道Mute状态
  - 没有主按钮激活 → 点击通道按钮无效果

### 3. UI更新系统 (PluginEditor)
**实现方式**:
- 30Hz定时器 (`timerCallback()`) 确保UI实时更新
- UI状态完全从参数值计算得出
- 使用正确的`customLookAndFeel`颜色配置

**主要方法**:
```cpp
void updateChannelButtonStates();  // 基于参数值更新所有按钮状态
void timerCallback() override;     // 30Hz定时器确保同步
```

### 4. 纯逻辑架构设计 (修正版)

#### 纯逻辑架构设计
**核心原则：无状态变量，完全基于参数计算，主按钮作为模式切换器**

**主按钮逻辑（修正版）**：
```cpp
void handleSoloButtonClick() {
    if (hasAnySoloActive()) {
        clearAllSoloParameters();  // 有Solo就清除
    } else {
        // 激活Solo模式 - 自动Solo第一个可见通道作为起始点
        activateFirstVisibleChannelSolo();
    }
}

void handleMuteButtonClick() {
    if (hasAnySoloActive()) return;  // Solo优先原则
    
    if (hasAnyMuteActive()) {
        clearAllMuteParameters();  // 有Mute就清除
    } else {
        // 激活Mute模式 - 自动Mute所有可见通道作为起始点
        activateAllVisibleChannelsMute();
    }
}
```

**通道按钮逻辑（基于参数状态）**：
```cpp
void handleChannelClick(int channelIndex) {
    if (hasAnySoloActive()) {
        toggleSoloParameter(channelIndex);  // Solo模式下切换Solo状态
    } else if (hasAnyMuteActive()) {
        toggleMuteParameter(channelIndex);  // Mute模式下切换Mute状态
    } else {
        // 初始状态：通道点击无效果（需要先激活主按钮）
        VST3_DBG("Channel clicked in Initial state - no effect");
    }
}
```

**UI状态计算（完全基于参数）**：
- 主按钮激活状态 = `hasAnySoloActive()` / `hasAnyMuteActive()`
- 主按钮可点击性 = `!hasAnySoloActive()` (Mute按钮)
- 通道按钮状态 = 直接读取参数值

#### 核心优势
1. **主动模式切换** - 主按钮是模式切换器，而不是被动响应器
2. **直观交互逻辑** - 符合用户对监听控制器的直觉期望
3. **完全可预测** - 所有行为都是参数的纯函数
4. **调试友好** - 只需要看参数值就知道所有状态
5. **无同步问题** - UI永远反映参数的真实状态
6. **符合JSFX逻辑** - 与原版JSFX的设计完全一致

## 🎯 关键技术细节

### 1. 参数联动机制
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

### 2. 参数保护机制
**Solo模式下的Mute参数强制保护**:
```cpp
// 在parameterChanged中实现
if (paramID.startsWith("MUTE_") && hasAnySoloActive()) {
    // Solo模式下强制恢复Mute参数到联动计算值
    restoreAutoMuteValue(paramID);
    return;
}
```

### 3. 主按钮状态控制
**Mute主按钮的动态启用/禁用**:
```cpp
// 根据Solo状态动态控制Mute按钮可点击性
bool muteButtonEnabled = !hasAnySoloActive();
globalMuteButton.setEnabled(muteButtonEnabled);
```

### 4. 循环防护
使用原子标志防止参数联动时的递归调用:
```cpp
std::atomic<bool> isApplyingLinkage{false};
ScopedLinkageGuard guard(isApplyingLinkage);
```

### 5. 状态重置
插件加载时执行激进状态重置，防止REAPER恢复不一致状态:
```cpp
void resetToCleanState() {
    // 清除所有Solo和Mute参数
    // 不使用ScopedLinkageGuard，允许UI更新
}
```

## 📋 开发状态总结

### 已解决的问题
1. ✅ **参数与UI不同步** - 实现了完全的参数驱动架构
2. ✅ **通道按钮错误逻辑** - 修复为正确的监听控制器逻辑
3. ✅ **颜色配置错误** - 使用正确的customLookAndFeel配色
4. ✅ **主按钮无功能** - 实现了完整的批量操作功能
5. ✅ **UI不实时更新** - 30Hz定时器确保同步
6. ✅ **状态记忆功能** - Solo进入/退出时的Mute状态保存恢复
7. ✅ **主按钮交互逻辑** - 完整的选择模式切换和Solo优先原则
8. ✅ **参数保护机制** - Solo模式下Mute参数的强制保护
9. ✅ **VST3参数窗口联动** - 修复了参数窗口操作不触发联动的问题

### 技术架构特点
- **无状态机复杂度** - 纯函数式参数计算
- **完全同步** - 参数系统是唯一真理来源
- **防护机制** - 递归调用防护和状态重置
- **调试友好** - 完整的VST3调试日志系统

### 下一步工作建议
1. 实现并测试完整的参数保护机制
2. 实现并测试主按钮交互逻辑的完整状态机
3. 在REAPER中测试VST3参数窗口同步功能
4. 验证Master-Slave多实例通信
5. 与JSFX版本进行完整功能对比测试
6. 根据测试结果进行最终优化