# 监听控制器插件开发文档 - 大一统架构设计

## 架构设计原则

### 🎯 设计哲学：参数驱动的纯函数式架构

基于对 `Monitor Controllor 7.1.4.jsfx` 的深度分析，我们采用完全不同的架构方案：

**核心理念：参数系统 = 唯一真理来源**

```
用户操作 → 参数变化 → 自动联动计算 → UI自动同步
```

## 🏗️ 新架构设计

### 1. 分层架构

```
┌─────────────────────────────────────────┐
│                UI层 (前端)                 │
│        - 纯显示层，无状态                   │
│        - 主按钮状态由参数计算得出             │
│        - 所有UI状态从参数读取               │
└─────────────────────────────────────────┘
                    ↑ 读取状态
┌─────────────────────────────────────────┐
│            参数联动层 (核心引擎)             │
│        - JUCE AudioProcessor参数系统      │
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

### 2. 核心设计模式：模仿JSFX

**JSFX模式分析：**
```javascript
// 检测Solo状态
Current_Solo_Active = slider31||slider32||...||slider46;

// 状态变化时的联动
(Current_Solo_Active != Pre_Solo_Active) ? (
    Current_Solo_Active ? (
        // 进入Solo：保存Mute记忆 + 自动设置联动Mute
        user_mute_L = slider11;  // 保存记忆
        ...
        slider11 = slider31 ? 0 : 1;  // 联动计算
        ...
    ) : (
        // 退出Solo：恢复Mute记忆
        slider11 = user_mute_L;
        ...
    )
);
```

**JUCE对应实现：**
```cpp
void parameterChanged(const String& parameterID, float newValue) {
    // 1. 检测Solo状态变化
    bool currentSoloActive = hasAnySoloActive();
    
    // 2. Solo状态变化时的联动处理
    if (currentSoloActive != previousSoloActive) {
        if (currentSoloActive) {
            saveCurrentMuteMemory();    // 保存记忆
            applyAutoMuteForSolo();     // 应用联动
        } else {
            restoreMuteMemory();        // 恢复记忆
        }
        previousSoloActive = currentSoloActive;
    }
    
    // 3. 主从通信
    if (isMaster) sendStateToSlaves();
}
```

### 3. 关键组件重新设计

#### 3.1 参数联动引擎 (CoreEngine)
```cpp
class ParameterLinkageEngine {
public:
    // 核心联动逻辑 - 模仿JSFX
    void handleParameterChange(const String& paramID, float value);
    
private:
    // Solo/Mute状态检测
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    
    // 联动计算
    void applyAutoMuteForSolo();    // Solo激活时的Mute联动
    void restoreMuteMemory();       // Solo关闭时的记忆恢复
    
    // 记忆管理
    void saveCurrentMuteMemory();
    std::map<int, float> muteMemory;
    
    // 状态追踪
    bool previousSoloActive = false;
    bool previousMuteActive = false;
};
```

#### 3.2 UI状态计算器 (UICalculator)
```cpp
class UIStateCalculator {
public:
    // 主按钮状态计算 - 完全由参数推导
    bool shouldSoloButtonBeActive() const;
    bool shouldMuteButtonBeActive() const;
    
    // 通道按钮状态计算
    ChannelDisplayState getChannelDisplayState(int channel) const;
    
private:
    // 纯函数计算，无状态存储
    const AudioProcessorValueTreeState& parameters;
};
```

#### 3.3 简化的UI管理器
```cpp
class SimpleUIManager {
public:
    // 纯显示更新，无状态管理
    void updateFromParameters();
    
private:
    // UI只读取参数，不维护状态
    void updateSoloButton();
    void updateMuteButton(); 
    void updateChannelButtons();
};
```

### 4. 数据流设计

#### 4.1 正向数据流（用户操作）
```
UI点击 → 参数变化 → 联动引擎计算 → 其他参数自动更新 → UI自动同步
```

#### 4.2 联动逻辑核心
```cpp
// Solo联动逻辑 (模仿JSFX的slider11 = slider31 ? 0 : 1)
for (int i = 0; i < 26; ++i) {
    auto soloParam = getSoloParameter(i);
    auto muteParam = getMuteParameter(i);
    
    if (hasAnySoloActive()) {
        // Solo模式：Solo的通道不Mute，非Solo的通道Mute
        float newMuteValue = soloParam->getValue() > 0.5f ? 0.0f : 1.0f;
        muteParam->setValueNotifyingHost(newMuteValue);
    }
}
```

#### 4.3 主按钮状态推导
```cpp
// 主按钮状态完全由参数推导，无需独立状态
bool shouldSoloButtonBeActive() const {
    return hasAnySoloActive();  // 任何Solo激活 → Solo按钮激活
}

bool shouldMuteButtonBeActive() const {
    return !hasAnySoloActive() && hasAnyMuteActive();  // Solo优先级高
}
```

## 🎯 实现优势

### 1. 完全一致性
- **UI ↔ 参数 100%同步**：UI状态完全由参数计算，不可能不一致
- **前后端统一**：参数系统是唯一真理来源

### 2. 简化逻辑
- **无复杂状态机**：模仿JSFX的简单联动计算
- **纯函数式**：状态计算都是纯函数，可预测、可测试

### 3. 强联动
- **滑块 ↔ 主按钮**：主按钮状态由滑块状态推导
- **Solo ↔ Mute**：Solo优先级高，自动联动Mute

### 4. 记忆功能
- **Solo记忆**：进入Solo时保存Mute状态，退出时恢复
- **跨会话持久化**：与现有记忆系统兼容

## 🔧 关键实现细节

### 1. 循环防护
```cpp
std::atomic<bool> isApplyingLinkage{false};

void handleParameterChange(const String& paramID, float value) {
    if (isApplyingLinkage) return;  // 防止联动时的递归
    
    ScopedValueSetter guard(isApplyingLinkage, true);
    applyLinkageLogic();
}
```

### 2. 高效更新
```cpp
// 批量参数更新，减少回调
void applyAutoMuteForSolo() {
    for (int i = 0; i < 26; ++i) {
        if (needsUpdate(i)) {
            updateParameterSilently(i);  // 不触发回调
        }
    }
    notifyHostOfParameterChanges();  // 统一通知
}
```

### 3. UI响应
```cpp
// UI定时器更新，确保同步
void timerCallback() override {
    if (parametersChanged) {
        updateFromParameters();
        parametersChanged = false;
    }
}
```

## 📋 实现检查清单

### Phase 1: 核心联动引擎
- [ ] 实现ParameterLinkageEngine
- [ ] Solo/Mute联动逻辑 (模仿JSFX)
- [ ] 记忆保存和恢复
- [ ] 循环防护机制

### Phase 2: UI重构
- [ ] 实现UIStateCalculator
- [ ] 主按钮状态推导
- [ ] 通道按钮状态计算
- [ ] 移除UI独立状态管理

### Phase 3: 集成测试
- [ ] 参数 ↔ UI同步测试
- [ ] Solo/Mute联动测试
- [ ] 记忆功能测试
- [ ] 主从通信测试

## 🎯 最终目标

**实现与JSFX完全一致的功能，同时保持UI和参数的完美同步。**

- ✅ Solo优先级高于Mute
- ✅ 任何Solo激活 → 其他通道自动Mute
- ✅ Solo关闭 → 恢复原始Mute状态  
- ✅ 主按钮反映滑块状态
- ✅ 参数窗口 ↔ UI完全同步
- ✅ 记忆功能保持不变

**这个架构彻底解决了前后端脱节问题，实现真正的大一统方案。**