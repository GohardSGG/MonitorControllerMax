# MonitorControllerMax: 专业监听控制器插件架构设计

## 项目概述

MonitorControllerMax是一个专业级的多声道监听控制器插件，设计目标是实现与现有`Monitor Controllor 7.1.4.jsfx`完全相同的功能，但通过JUCE框架提供更好的跨平台兼容性和现代化的用户界面。

### 核心设计理念

1. **完全兼容JSFX行为**：实现与JSFX版本完全相同的Solo/Mute逻辑和状态管理
2. **双向参数同步**：UI操作和VST3参数变化能够完美双向同步
3. **状态记忆功能**：Solo操作前后的Mute状态完整保存和恢复
4. **多声道支持**：支持最大26个通道的灵活配置
5. **主从架构**：支持主从插件实例协同工作

## 当前开发状态

### ✅ 已完成的核心功能

#### 1. 强大的状态机系统 (StateManager)
- **统一状态管理**：替代原有的分散状态逻辑
- **记忆保护机制**：防止空状态覆盖有效的Mute记忆
- **完整状态转换**：Normal → MuteSelecting → MuteActive → SoloSelecting → SoloMuteActive
- **UI同步回调**：状态变化时自动触发UI更新

#### 2. VST3调试系统 (DebugLogger)
- **双重输出**：同时输出到控制台和文件 (`%TEMP%\MonitorControllerMax_Debug.log`)
- **时间戳记录**：精确的毫秒级时间戳
- **VST3兼容**：解决VST3插件调试输出问题
- **自动初始化**：插件加载时自动创建日志系统

#### 3. UI与状态同步
- **正确的AutoMute显示**：Solo模式下AutoMute通道显示为红色非激活按钮
- **实时状态反馈**：状态机变化立即反映在UI上
- **颜色逻辑**：Solo=绿色激活，ManualMute=红色激活，AutoMute=红色非激活

#### 4. Mute记忆系统
- **自动保存**：进入Solo模式时自动保存当前Mute状态
- **完整恢复**：退出Solo模式时恢复到原始Mute配置
- **保护机制**：防止状态转换过程中记忆被错误覆盖
- **持久化存储**：记忆状态保存到文件系统

### 🔄 当前核心问题：双向参数同步缺失

#### 问题描述
- **UI → 参数**：正常工作 ✅
- **参数 → UI/状态机**：不工作 ❌

当在REAPER中直接操作VST3参数时（如图中的Mute/Solo参数），状态机和UI不会更新，导致：
1. 参数窗口显示的状态与UI不一致
2. 状态机逻辑不被触发
3. 缺少Solo/Mute互斥检查
4. 记忆功能无法正常工作

## 目标架构：完整的双向同步系统

### 核心设计模式：JSFX等价实现

#### 1. 参数监听机制
```cpp
// 类似JSFX的@slider区块功能
void parameterChanged(const juce::String& parameterID, float newValue) {
    // 参数变化时同步到状态机
    if (parameterID.startsWith("SOLO_")) {
        handleSoloParameterChange(channelIndex, newValue > 0.5f);
    } else if (parameterID.startsWith("MUTE_")) {
        handleMuteParameterChange(channelIndex, newValue > 0.5f);
    }
}
```

#### 2. 状态机到参数的回写
```cpp
// 状态机变化时回写到参数
void StateManager::setChannelState(int channelIndex, ChannelState newState) {
    // 更新内部状态
    channelStates[channelIndex] = newState;
    
    // 回写到VST3参数（触发宿主更新）
    if (parameterUpdateCallback) {
        parameterUpdateCallback(channelIndex, newState);
    }
}
```

#### 3. Solo/Mute互斥逻辑
```cpp
// 确保同一通道不能同时Solo和Mute
void handleSoloParameterChange(int channelIndex, bool enabled) {
    if (enabled) {
        // 激活Solo时自动清除Mute
        clearChannelMute(channelIndex);
        setChannelSolo(channelIndex);
    } else {
        clearChannelSolo(channelIndex);
    }
}
```

#### 4. 记忆状态管理
```cpp
// 类似JSFX的user_mute_*变量机制
class MuteMemoryManager {
    void saveCurrentMuteState();     // 进入Solo时保存
    void restoreMuteState();         // 退出Solo时恢复
    void clearMemory();              // 清空记忆
};
```

### 目标行为规范

#### Solo操作流程
1. **参数/UI触发Solo**
   - 检测到Solo参数变化或UI点击
   - 如果是首个Solo通道：保存当前所有Mute状态到记忆
   - 设置该通道为Solo状态
   - 其他通道自动设置为AutoMute状态
   - 同步更新所有相关参数和UI

2. **Solo状态下的操作**
   - 可以激活多个Solo通道
   - Solo通道之间不会相互AutoMute
   - 可以在Solo状态下手动Mute某些Solo通道

3. **退出Solo模式**
   - 清除所有Solo状态
   - 恢复记忆中的Mute状态到参数
   - 触发UI更新
   - 清除AutoMute状态

#### Mute操作流程
1. **独立Mute操作**
   - 在Normal状态下：正常Mute/Unmute
   - 在Solo状态下：暂存Mute变化到记忆中

2. **Mute记忆管理**
   - 自动保存：进入Solo前的状态
   - 动态更新：Solo期间的Mute操作更新记忆
   - 完整恢复：退出Solo时恢复完整状态

### 实现优先级

#### 🔥 高优先级（立即实现）
1. **修复parameterChanged方法**：实现参数到状态机的同步
2. **完善状态机回写**：状态变化时更新VST3参数
3. **Solo/Mute互斥检查**：确保状态一致性
4. **双向同步测试**：验证UI和参数窗口的一致性

#### 🔶 中优先级（后续优化）
1. **性能优化**：减少不必要的参数更新
2. **状态验证**：添加状态一致性检查
3. **错误处理**：异常状态的恢复机制

#### 🔷 低优先级（未来扩展）
1. **主从模式实现**：多实例协同工作
2. **配置系统增强**：更灵活的通道配置
3. **自动化支持**：DAW自动化兼容性

## 技术架构

### 核心组件
1. **StateManager**：统一状态管理和转换
2. **MuteMemoryManager**：Mute状态记忆和恢复
3. **ParameterSyncManager**：双向参数同步（待实现）
4. **DebugLogger**：VST3调试支持

### 数据流
```
VST3参数 ↔ StateManager ↔ UI组件
    ↕                ↕
记忆管理        颜色/状态逻辑
```

### 关键设计原则
1. **单一真理来源**：StateManager作为状态的唯一权威
2. **事件驱动**：所有状态变化通过事件/回调传播
3. **原子操作**：状态变化操作具有原子性
4. **向后兼容**：保持与现有JSFX行为的完全兼容性

## 开发指导原则

1. **优先修复双向同步**：这是当前最关键的问题
2. **保持JSFX兼容性**：所有行为必须与JSFX版本完全一致
3. **充分测试**：每个状态转换都要在UI和参数两个层面验证
4. **清晰的调试信息**：利用VST3调试系统记录所有状态变化
5. **渐进式开发**：小步快跑，每个功能点完成后立即测试和提交