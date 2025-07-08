# 大一统架构重构实施计划

## 🎯 总体目标

**完全重构为参数驱动的纯函数式架构，实现与JSFX版本完全一致的功能**

基于对`Monitor Controllor 7.1.4.jsfx`的深度分析，采用参数系统作为唯一真理来源，彻底解决UI和参数脱节问题。

## 📋 实施阶段

### Phase 1: 核心引擎重构 (参数联动系统)

#### 1.1 移除现有复杂状态机
```cpp
// 需要完全移除或简化的组件
- StateManager (复杂状态机)
- 独立的UI状态管理
- 复杂的回调机制
```

#### 1.2 实现参数联动引擎
**文件**: `Source/ParameterLinkageEngine.h/cpp`
```cpp
class ParameterLinkageEngine {
public:
    explicit ParameterLinkageEngine(AudioProcessorValueTreeState& apvts);
    
    // 核心联动逻辑 - 模仿JSFX
    void handleParameterChange(const String& paramID, float value);
    
private:
    AudioProcessorValueTreeState& parameters;
    
    // Solo/Mute状态检测 (模仿JSFX的Current_Solo_Active)
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    
    // 联动计算 (模仿JSFX的联动逻辑)
    void applyAutoMuteForSolo();    // slider11 = slider31 ? 0 : 1
    void restoreMuteMemory();       // 恢复user_mute记忆
    
    // 记忆管理 (模仿JSFX的user_mute_xxx)
    void saveCurrentMuteMemory();
    std::map<int, float> muteMemory;
    
    // 状态追踪 (模仿JSFX的Pre_Solo_Active)
    bool previousSoloActive = false;
    
    // 循环防护
    std::atomic<bool> isApplyingLinkage{false};
};
```

**关键实现：**
```cpp
void ParameterLinkageEngine::handleParameterChange(const String& paramID, float value) {
    if (isApplyingLinkage) return;  // 防止递归
    
    // 检测Solo状态变化 (模仿JSFX)
    bool currentSoloActive = hasAnySoloActive();
    
    if (currentSoloActive != previousSoloActive) {
        ScopedValueSetter guard(isApplyingLinkage, true);
        
        if (currentSoloActive) {
            // 进入Solo模式 (模仿JSFX进入逻辑)
            saveCurrentMuteMemory();    // user_mute_L = slider11
            applyAutoMuteForSolo();     // slider11 = slider31 ? 0 : 1
        } else {
            // 退出Solo模式 (模仿JSFX退出逻辑)
            restoreMuteMemory();        // slider11 = user_mute_L
        }
        
        previousSoloActive = currentSoloActive;
    }
}
```

#### 1.3 核心联动逻辑
```cpp
void ParameterLinkageEngine::applyAutoMuteForSolo() {
    // 模仿JSFX: slider11 = slider31 ? 0 : 1
    for (int i = 0; i < 26; ++i) {
        auto soloParamId = "SOLO_" + String(i + 1);
        auto muteParamId = "MUTE_" + String(i + 1);
        
        auto* soloParam = parameters.getParameter(soloParamId);
        auto* muteParam = parameters.getParameter(muteParamId);
        
        if (soloParam && muteParam) {
            // Solo的通道 = 不Mute，非Solo的通道 = Mute
            float newMuteValue = soloParam->getValue() > 0.5f ? 0.0f : 1.0f;
            muteParam->setValueNotifyingHost(newMuteValue);
        }
    }
}
```

### Phase 2: UI系统重构 (纯显示层)

#### 2.1 实现UI状态计算器
**文件**: `Source/UIStateCalculator.h/cpp`
```cpp
class UIStateCalculator {
public:
    explicit UIStateCalculator(const AudioProcessorValueTreeState& apvts);
    
    // 主按钮状态计算 - 完全由参数推导
    bool shouldSoloButtonBeActive() const;
    bool shouldMuteButtonBeActive() const;
    
    // 通道按钮状态和颜色
    bool shouldChannelBeActive(int channelIndex) const;
    Colour getChannelColour(int channelIndex) const;
    
private:
    const AudioProcessorValueTreeState& parameters;
    
    // 纯函数，无状态存储
    bool isChannelSolo(int channelIndex) const;
    bool isChannelMute(int channelIndex) const;
};
```

**关键实现：**
```cpp
bool UIStateCalculator::shouldSoloButtonBeActive() const {
    // 任何Solo激活 → Solo主按钮激活
    for (int i = 0; i < 26; ++i) {
        if (isChannelSolo(i)) return true;
    }
    return false;
}

bool UIStateCalculator::shouldMuteButtonBeActive() const {
    // Solo优先级高：有Solo时不显示Mute主按钮激活
    if (shouldSoloButtonBeActive()) return false;
    
    // 任何Mute激活 → Mute主按钮激活
    for (int i = 0; i < 26; ++i) {
        if (isChannelMute(i)) return true;
    }
    return false;
}
```

#### 2.2 重构UI管理器
**修改**: `PluginEditor.h/cpp`
```cpp
class MonitorControllerMaxAudioProcessorEditor {
private:
    std::unique_ptr<UIStateCalculator> uiCalculator;
    
    // 移除所有UI独立状态
    // 移除复杂的状态管理逻辑
    
public:
    // 简化的UI更新 - 纯显示
    void updateFromParameters();
    
private:
    void updateSoloButton();
    void updateMuteButton();
    void updateChannelButtons();
};
```

**核心更新逻辑：**
```cpp
void MonitorControllerMaxAudioProcessorEditor::updateFromParameters() {
    // 主按钮状态完全由参数推导
    globalSoloButton.setToggleState(uiCalculator->shouldSoloButtonBeActive(), dontSendNotification);
    globalMuteButton.setToggleState(uiCalculator->shouldMuteButtonBeActive(), dontSendNotification);
    
    // 通道按钮状态完全由参数推导
    for (auto& [channelIndex, button] : channelButtons) {
        bool isActive = uiCalculator->shouldChannelBeActive(channelIndex);
        Colour colour = uiCalculator->getChannelColour(channelIndex);
        
        button->setToggleState(isActive, dontSendNotification);
        button->setColour(TextButton::buttonOnColourId, colour);
    }
}
```

### Phase 3: PluginProcessor集成

#### 3.1 重构parameterChanged
**修改**: `PluginProcessor.cpp`
```cpp
void MonitorControllerMaxAudioProcessor::parameterChanged(const String& parameterID, float newValue) {
    // 1. 参数联动处理 (核心逻辑)
    if (linkageEngine) {
        linkageEngine->handleParameterChange(parameterID, newValue);
    }
    
    // 2. UI更新通知
    if (editor) {
        editor->parametersChanged();  // 触发UI更新
    }
    
    // 3. 主从通信 (保持现有逻辑)
    if (getRole() == Role::master && (parameterID.startsWith("MUTE_") || parameterID.startsWith("SOLO_"))) {
        sendStateToSlaves();
    }
}
```

#### 3.2 移除复杂回调
```cpp
// 移除这些复杂的接口
- onParameterUpdate()
- onUIUpdate()
- StateManager相关回调
```

#### 3.3 主按钮功能保留与简化
**重要说明：Solo和Mute主按钮仍然是功能按钮，可以点击！**

```cpp
// Solo主按钮：批量Solo控制
void handleSoloButtonClick() {
    bool currentlyActive = hasAnySoloActive();
    
    if (currentlyActive) {
        // 当前有Solo激活 → 清除所有Solo参数
        clearAllSoloParameters();
    } else {
        // 当前无Solo → 进入Solo选择模式
        // 可以通过UI视觉提示用户现在可以点击通道进行Solo
        // 或者实现其他Solo批量操作逻辑
    }
}

// Mute主按钮：批量Mute控制
void handleMuteButtonClick() {
    bool currentlyActive = hasAnyMuteActive();
    
    if (currentlyActive) {
        // 当前有Mute激活 → 清除所有Mute参数
        clearAllMuteParameters();
    } else {
        // 当前无Mute → 进入Mute选择模式
        // 可以通过UI视觉提示用户现在可以点击通道进行Mute
    }
}

// 核心特性：主按钮状态由参数推导，但功能仍然存在
bool shouldSoloButtonBeActive() {
    return hasAnySoloActive();  // 参数驱动状态显示
}

bool shouldMuteButtonBeActive() {
    return !hasAnySoloActive() && hasAnyMuteActive();  // Solo优先级高
}
```

**主按钮的双重特性：**
1. **状态显示**：按钮的激活状态由参数自动推导
2. **功能操作**：按钮仍然可以点击，执行批量操作

### Phase 4: 测试验证

#### 4.1 核心功能测试
```
✅ Solo一个通道 → 其他通道自动Mute
✅ Solo多个通道 → 非Solo通道自动Mute  
✅ 取消所有Solo → 恢复原始Mute状态
✅ 主按钮状态反映滑块状态
✅ 参数窗口操作 → UI立即同步
✅ UI操作 → 参数窗口立即同步
```

#### 4.2 边界情况测试
```
✅ 同时操作UI和参数窗口
✅ 快速连续操作
✅ 主从实例同步
✅ 记忆功能跨会话
✅ 不同音箱布局切换
```

## 🔧 实施细节

### 关键设计原则
1. **参数 = 唯一真理来源**：所有状态都存储在JUCE参数中
2. **UI = 纯显示层**：UI只读取参数，不维护独立状态
3. **联动 = 参数计算**：所有联动都是参数之间的自动计算
4. **JSFX对等**：功能逻辑完全模仿JSFX版本

### 循环防护策略
```cpp
// 使用原子标志防止联动时的递归
std::atomic<bool> isApplyingLinkage{false};

// 使用RAII确保标志正确重置
class ScopedValueSetter {
    std::atomic<bool>& flag;
public:
    ScopedValueSetter(std::atomic<bool>& f, bool value) : flag(f) { 
        flag.store(value); 
    }
    ~ScopedValueSetter() { 
        flag.store(false); 
    }
};
```

### 性能优化
```cpp
// 批量参数更新，减少UI刷新
void applyAutoMuteForSolo() {
    beginParameterChangeGesture();
    for (int i = 0; i < 26; ++i) {
        updateParameterIfNeeded(i);
    }
    endParameterChangeGesture();
}
```

## 📊 进度追踪

### 当前状态：Phase 0 - 准备阶段
- [x] 分析JSFX设计模式
- [x] 设计新架构
- [x] 更新开发文档
- [ ] 开始实施

### 下一步行动
1. **立即开始**: 实现ParameterLinkageEngine
2. **核心目标**: 实现Solo → Mute自动联动
3. **验证标准**: 参数窗口和UI完全同步

## 🎯 成功标准

**最终效果应该达到：**
- 在参数窗口拖动Solo 1 → 其他通道的Mute参数自动变为On，UI同步变红
- 在UI点击Solo L → 参数窗口Solo 1变为On，其他Mute参数自动变为On
- 取消Solo → 恢复原始Mute状态，参数和UI完全同步
- 主按钮状态完全反映滑块状态，无任何脱节

**这个架构将彻底解决前后端脱节问题，实现真正的大一统方案！**