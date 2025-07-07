# MonitorControllerMax 开发步骤：Bug修复与动态显示功能

## 当前开发优先级

基于Dev.md V7的分析和发现的关键问题，当前开发任务按优先级排序：

### **优先级2：阶段1功能实现**
3. **动态插件输入输出针脚名** - 随配置切换动态更新I/O通道名称

### **优先级3：未来阶段**
4. **动态VST3插件参数名** - DAW参数列表显示优化

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

## 新发现的关键问题 (需要立即修复)

**基于实际测试发现的问题：**

### 问题2: 手动切换配置后I/O针脚名不更新
**现象:**
- 即使手动切换到5.1配置，REAPER的I/O矩阵针脚名依然显示"Input 1, Input 2"
- 说明`updateHostDisplay()`调用无效或REAPER没有响应

**原因分析:**
- `updateHostDisplay()`可能在错误的线程中调用
- 或者REAPER需要特殊的通知机制来刷新I/O针脚名


## 修复计划

### 阶段C: 新问题修复
2. **C.2** 修复手动切换配置后I/O针脚名不更新

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
- **新增:** 测试DAW轨道通道数变化场景
- **新增:** 验证I/O针脚名在不同DAW中的更新效果