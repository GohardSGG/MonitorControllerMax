# MonitorControllerMax 开发步骤：Bug修复与动态显示功能

## 当前开发优先级

基于Dev.md V7的分析和发现的关键问题，当前开发任务按优先级排序：

### **优先级1：关键Bug修复 (必须先解决)**
1. **Solo主按钮UI状态更新延迟** - 影响用户体验的关键问题
2. **Solo撤销时Mute状态恢复失败** - 破坏用户配置的严重问题

### **优先级2：阶段1功能实现**
3. **动态插件输入输出针脚名** - 随配置切换动态更新I/O通道名称

### **优先级3：未来阶段**
4. **动态VST3插件参数名** - DAW参数列表显示优化

## 阶段A：Solo功能Bug修复

### A.1 分析Solo主按钮UI更新问题

#### A.1.1 代码审查 (PluginEditor.cpp)
**目标:** 找到Solo主按钮点击处理和UI更新的代码位置

**分析要点:**
- 检查Solo主按钮的onClick事件处理器
- 找到`updateChannelButtonStates`函数的调用时机
- 确认主按钮状态计算逻辑

**预期发现:**
- Solo主按钮点击后可能没有立即调用UI更新
- 或者`updateChannelButtonStates`中主按钮的状态计算有误

#### A.1.2 修复Solo主按钮UI更新延迟
**问题定位后的修复方案:**

**方案1: 添加即时UI更新**
```cpp
// 在Solo主按钮onClick处理器中添加
globalSoloButton.onClick = [this]()
{
    // 现有的Solo切换逻辑...
    
    // 立即更新UI状态 - 新增这行
    updateChannelButtonStates();
};
```

**方案2: 修复updateChannelButtonStates中的逻辑**
- 检查主按钮状态计算是否正确
- 确保聚合状态(`anySoloEngaged`)计算准确

### A.2 分析Solo撤销时Mute状态恢复问题

#### A.2.1 代码审查 (PluginEditor.cpp)
**目标:** 找到"独奏前状态"缓存相关的代码

**分析要点:**
- 检查`preSoloMuteStates`的声明和使用
- 找到状态缓存的保存时机
- 找到状态恢复的触发条件和执行逻辑

**预期发现:**
- 状态缓存可能没有正确保存
- 或者恢复逻辑的触发条件有问题
- 或者恢复时没有正确设置参数值

#### A.2.2 修复Solo撤销时的状态恢复
**根据代码审查结果实施修复:**

**修复要点:**
- 确保在激活第一个solo时正确缓存所有主声道的mute状态
- 确保在撤销最后一个solo时正确恢复缓存的状态
- 确保状态恢复后立即更新UI显示

**可能的修复代码框架:**
```cpp
// 在handleSoloButtonClick或类似函数中
void MonitorControllerMaxAudioProcessorEditor::handleSoloButtonClick(int channelIndex)
{
    bool wasAnySoloActive = /* 计算动作前的solo状态 */;
    
    // 切换被点击按钮的Solo参数
    auto* soloParam = /* 获取对应的solo参数 */;
    soloParam->setValueNotifyingHost(/* 新值 */);
    
    bool isAnySoloNowActive = /* 计算动作后的solo状态 */;
    
    // 状态变化处理
    if (!wasAnySoloActive && isAnySoloNowActive)
    {
        // 进入solo模式：缓存当前mute状态
        saveMuteStatesBeforeSolo();
    }
    else if (wasAnySoloActive && !isAnySoloNowActive)
    {
        // 退出solo模式：恢复缓存的mute状态
        restoreMuteStatesAfterSolo();
    }
    
    // 应用solo联动逻辑
    if (isAnySoloNowActive)
    {
        applySoloMuteLogic();
    }
    
    // 立即更新UI
    updateChannelButtonStates();
}
```

### A.3 验证Solo功能修复

#### A.3.1 编译和基础测试
- 使用快速Debug编译验证修复
- 测试Solo主按钮点击后的即时UI响应
- 测试Solo撤销后Mute状态的正确恢复

#### A.3.2 完整功能测试
- 测试单通道solo激活/撤销
- 测试多通道solo激活/撤销
- 测试混合mute+solo场景
- 验证UI状态显示与实际参数状态一致

#### A.3.3 Git提交修复版本
**提交信息示例:**
```
修复: Solo功能的关键UI和逻辑问题

- 修复Solo主按钮UI状态更新延迟问题
- 修复Solo撤销时Mute状态恢复失败问题
- 确保UI状态与参数状态实时同步

相关: Dev Step.md 阶段A
```

## 阶段B：动态I/O针脚名实现

### B.1 修复错误的函数签名

#### B.1.1 移除错误的getParameterName声明和实现 (PluginProcessor.h/cpp)
**发现的问题:**
- 当前代码使用了错误的`getParameterName(int parameterIndex, int maximumStringLength)`签名
- 这不是JUCE AudioProcessor的正确虚函数

**修复动作:**
```cpp
// PluginProcessor.h - 移除错误的声明
// juce::String getParameterName(int parameterIndex, int maximumStringLength) override;
// juce::String getParameterLabel(int parameterIndex) const override;

// PluginProcessor.cpp - 删除对应的实现函数
```

#### B.1.2 添加正确的I/O通道名函数声明 (PluginProcessor.h)
**新增声明:**
```cpp
// 动态I/O通道名函数 - 根据当前布局提供有意义的通道名称
const String getInputChannelName(int channelIndex) const override;
const String getOutputChannelName(int channelIndex) const override;
```

### B.2 实现动态I/O通道名功能

#### B.2.1 实现getInputChannelName函数 (PluginProcessor.cpp)
```cpp
// 动态获取输入通道名称，根据当前音箱布局映射物理通道到逻辑声道名
// channelIndex: 物理通道索引（从0开始）
// 返回: 对应的声道名称（如"LFE"）或默认名称
const String MonitorControllerMaxAudioProcessor::getInputChannelName(int channelIndex) const
{
    // 遍历当前布局中的所有通道配置
    for (const auto& chanInfo : currentLayout.channels)
    {
        // 检查物理通道索引是否匹配布局中的通道索引
        if (chanInfo.channelIndex == channelIndex)
        {
            return chanInfo.name;  // 返回配置文件中定义的声道名称
        }
    }
    
    // 未找到映射时返回默认通道名称
    return "Channel " + String(channelIndex + 1);
}
```

#### B.2.2 实现getOutputChannelName函数 (PluginProcessor.cpp)
```cpp
// 动态获取输出通道名称，与输入通道使用相同的映射逻辑
// channelIndex: 物理通道索引（从0开始）
// 返回: 对应的声道名称（如"LFE"）或默认名称
const String MonitorControllerMaxAudioProcessor::getOutputChannelName(int channelIndex) const
{
    // 输出通道名称与输入相同，复用输入通道名称逻辑
    return getInputChannelName(channelIndex);
}
```

### B.3 添加宿主通知机制

#### B.3.1 修改setCurrentLayout函数 (PluginProcessor.cpp)
**添加updateHostDisplay调用:**
```cpp
void MonitorControllerMaxAudioProcessor::setCurrentLayout(const juce::String& speaker, const juce::String& sub)
{
    // 现有的布局设置代码...
    currentLayout = configManager.getLayoutFor(speaker, sub);
    
    // 通知DAW刷新I/O通道名称显示 - 新增这行
    updateHostDisplay();
}
```

### B.4 编译和测试I/O针脚名功能

#### B.4.1 快速Debug编译验证
- 编译验证新函数没有语法错误
- 确保函数能够正确返回通道名称

#### B.4.2 DAW集成测试
- 在DAW中打开I/O连接矩阵/路由界面
- 切换插件布局配置（如从2.0切换到7.1.4）
- 验证I/O针脚名称是否正确更新
- 测试未映射通道的默认名称显示

#### B.4.3 动态响应测试
- 测试DAW轨道大小变化时的响应
- 验证插件总线大小变化时的适配
- 确认配置切换时的实时更新

#### B.4.4 Git提交I/O针脚名功能
**提交信息示例:**
```
功能: 实现动态I/O针脚名功能

- 添加getInputChannelName和getOutputChannelName函数
- 实现基于当前布局的物理通道到逻辑声道名映射
- 添加布局切换时的宿主通知机制
- 支持随DAW轨道/总线大小变化动态更新

相关: Dev Step.md 阶段B
```

## 测试验证和质量保证

### 最终集成测试
1. **功能完整性测试:**
   - Solo功能的完整工作流程
   - I/O针脚名在各种布局下的正确显示
   - 布局切换的响应性测试

2. **兼容性测试:**
   - 在多个DAW中测试（Reaper, Cubase等）
   - 不同音频接口通道数配置测试
   - 各种音箱布局配置测试

3. **性能测试:**
   - 确保I/O名称查找不影响实时音频性能
   - UI更新响应性测试

### 最终Git提交
**完整功能提交:**
```
完成: Bug修复和I/O针脚名功能实现

- 修复Solo主按钮UI更新延迟问题
- 修复Solo撤销时Mute状态恢复问题
- 实现动态I/O针脚名功能
- 支持配置切换时的实时更新
- 所有功能测试通过

相关: Dev.md 阶段1完成
```

## 开发注意事项

### Git工作流程
- 每个子步骤完成后立即提交
- 确保每个提交都是可编译的稳定状态
- 使用描述性的中文提交信息

### 代码质量
- 所有新增代码必须包含中文注释
- 遵循现有代码风格和命名约定
- 确保线程安全和实时音频性能

### 调试策略
- 优先使用Debug独立程序进行测试
- 重点关注Solo功能的边界情况
- 验证各种布局切换场景