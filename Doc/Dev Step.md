# 语义化OSC架构实施计划

## 🎯 总体目标

**从VST3参数联动架构彻底转向语义化OSC架构**

基于VST3协议根本限制的发现，我们需要完全重构架构：
- **问题根源**：VST3协议铁律 `"No automated parameter must influence another automated parameter!"`
- **解决方案**：语义化内部状态 + OSC外部通信 + 最小VST3参数系统
- **目标**：实现完全功能的专业监听控制器，同时完美外部集成

## 📋 实施阶段

### Phase 1: 核心架构重构

#### 1.1 实现语义化内部状态系统
**文件**: `Source/SemanticChannelState.h/cpp` (新建)

**核心状态管理**：
```cpp
class SemanticChannelState {
private:
    // 语义通道状态存储
    std::map<juce::String, bool> soloStates;    // "L", "R", "C", "LFE", "LR", "RR", ...
    std::map<juce::String, bool> muteStates;    // "L", "R", "C", "LFE", "LR", "RR", ...
    bool globalSoloModeActive = false;
    
    // 状态变化通知
    juce::ListenerList<StateChangeListener> stateChangeListeners;
    
public:
    // 语义化操作接口
    void setSoloState(const juce::String& channelName, bool state);
    void setMuteState(const juce::String& channelName, bool state);
    bool getSoloState(const juce::String& channelName) const;
    bool getMuteState(const juce::String& channelName) const;
    
    // Solo模式联动逻辑
    bool getFinalMuteState(const juce::String& channelName) const;
    void calculateSoloModeLinkage();
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    
    // 初始化和状态管理
    void initializeChannel(const juce::String& channelName);
    void clearAllStates();
    std::vector<juce::String> getActiveChannels() const;
    
    // 状态变化监听
    void addStateChangeListener(StateChangeListener* listener);
    void removeStateChangeListener(StateChangeListener* listener);
    
private:
    void notifyStateChange(const juce::String& channelName, const juce::String& action, bool state);
};
```

**实现要点**：
- 完全脱离VST3参数系统
- 语义通道名固定："L", "R", "C", "LFE", "LR", "RR", "LTF", "RTF", "LTR", "RTR", "SUB_L", "SUB_R", "SUB_M"
- Solo模式自动联动：`getFinalMuteState() = globalSoloModeActive ? !soloStates[channel] : muteStates[channel]`

#### 1.2 实现物理映射系统
**文件**: `Source/PhysicalChannelMapper.h/cpp` (新建)

**映射管理**：
```cpp
class PhysicalChannelMapper {
private:
    std::map<juce::String, int> semanticToPhysical;  // "L" → 1, "R" → 5, etc.
    std::map<int, juce::String> physicalToSemantic;  // 1 → "L", 5 → "R", etc.
    
public:
    // 配置驱动映射更新
    void updateMapping(const Layout& layout);
    void updateFromConfig(const juce::String& speakerLayout, const juce::String& subLayout);
    
    // 映射转换接口
    int getPhysicalPin(const juce::String& semanticName) const;
    juce::String getSemanticName(int physicalPin) const;
    bool hasSemanticChannel(const juce::String& semanticName) const;
    
    // 获取映射信息
    std::vector<juce::String> getActiveSemanticChannels() const;
    std::vector<std::pair<juce::String, int>> getAllMappings() const;
    int getChannelCount() const;
    
private:
    SemanticChannel parseSemanticChannel(const juce::String& name) const;
};
```

**配置集成示例**：
```cpp
void PhysicalChannelMapper::updateMapping(const Layout& layout) {
    semanticToPhysical.clear();
    physicalToSemantic.clear();
    
    // 从Speaker_Config.json解析映射
    for (const auto& channelInfo : layout.channels) {
        juce::String semanticName = channelInfo.name;     // "L", "R", "C"
        int physicalPin = channelInfo.channelIndex;       // 1, 5, 3 (从配置文件)
        
        semanticToPhysical[semanticName] = physicalPin;
        physicalToSemantic[physicalPin] = semanticName;
    }
}
```

#### 1.3 最小化VST3参数系统
**文件**: `Source/PluginProcessor.cpp` (修改)

**移除所有Solo/Mute参数，只保留Gain**：
```cpp
juce::AudioProcessorValueTreeState::ParameterLayout 
MonitorControllerMaxAudioProcessor::createParameterLayout() {
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;
    
    // 只保留Gain参数用于自动化
    for (int i = 1; i <= 26; ++i) {
        params.push_back(std::make_unique<juce::AudioParameterFloat>(
            "GAIN_" + juce::String(i), 
            "Gain " + juce::String(i),
            juce::NormalisableRange<float>(-60.0f, 12.0f, 0.1f, 3.0f), 
            0.0f, "dB"
        ));
    }
    
    // 其他必要的独立参数
    params.push_back(std::make_unique<juce::AudioParameterBool>("BYPASS", "Bypass", false));
    params.push_back(std::make_unique<juce::AudioParameterFloat>("OUTPUT_GAIN", "Output Gain", -12.0f, 12.0f, 0.0f));
    
    return { params.begin(), params.end() };
}
```

### Phase 2: OSC通信系统实现

#### 2.1 实现OSC通信组件
**文件**: `Source/OSCCommunicator.h/cpp` (新建)

**OSC通信系统**：
```cpp
class OSCCommunicator : public juce::OSCReceiver::Listener<juce::OSCReceiver::RealtimeCallback> {
private:
    juce::OSCSender sender;
    juce::OSCReceiver receiver;
    
    // 硬编码配置
    const juce::String targetIP = "127.0.0.1";
    const int targetPort = 7444;
    const int receivePort = 7445;
    
public:
    bool initialize();
    void shutdown();
    
    // 发送状态到外部设备
    void sendSoloState(const juce::String& channelName, bool state);
    void sendMuteState(const juce::String& channelName, bool state);
    
    // 状态反馈机制 - 广播所有当前状态
    void broadcastAllStates(const SemanticChannelState& state);
    
    // 接收外部控制
    void oscMessageReceived(const juce::OSCMessage& message) override;
    
    // 状态查询
    bool isConnected() const;
    
private:
    void handleIncomingOSCMessage(const juce::OSCMessage& message);
    juce::String formatOSCAddress(const juce::String& action, const juce::String& channelName) const;
    std::pair<juce::String, juce::String> parseOSCAddress(const juce::String& address) const;
};
```

**OSC协议实现**：
```cpp
void OSCCommunicator::sendSoloState(const juce::String& channelName, bool state) {
    juce::String address = "/Monitor/Solo_" + channelName + "/";
    sender.send(address, state ? 1.0f : 0.0f);
}

void OSCCommunicator::sendMuteState(const juce::String& channelName, bool state) {
    juce::String address = "/Monitor/Mute_" + channelName + "/";
    sender.send(address, state ? 1.0f : 0.0f);
}

void OSCCommunicator::broadcastAllStates(const SemanticChannelState& state) {
    // 遍历所有可能的语义通道
    const std::vector<juce::String> allChannels = {
        "L", "R", "C", "LFE", "LR", "RR",
        "LTF", "RTF", "LTR", "RTR",
        "SUB_L", "SUB_R", "SUB_M"
    };
    
    for (const auto& channelName : allChannels) {
        // 发送Solo状态
        bool soloState = state.getSoloState(channelName);
        sendSoloState(channelName, soloState);
        
        // 发送Mute状态
        bool muteState = state.getMuteState(channelName);
        sendMuteState(channelName, muteState);
    }
}
```

#### 2.2 状态反馈机制
**触发时机**：
```cpp
// 插件加载时
void MonitorControllerMaxAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    // 初始化完成后广播状态
    if (oscComm.isConnected()) {
        oscComm.broadcastAllStates(semanticState);
    }
}

// 状态改变时
void SemanticChannelState::setSoloState(const juce::String& channelName, bool state) {
    soloStates[channelName] = state;
    globalSoloModeActive = hasAnySoloActive();
    calculateSoloModeLinkage();
    
    // 通知状态变化
    notifyStateChange(channelName, "solo", state);
    
    // 如果Solo模式变化，需要重新广播所有状态
    if (globalSoloModeActive != previousGlobalSoloMode) {
        stateChangeListeners.call([this](StateChangeListener* l) {
            l->onGlobalModeChanged();
        });
    }
}
```

### Phase 3: 音频处理集成

#### 3.1 重构主处理器
**文件**: `Source/PluginProcessor.h/cpp` (重大修改)

**新的主处理器架构**：
```cpp
class MonitorControllerMaxAudioProcessor : public juce::AudioProcessor,
                                         public SemanticChannelState::StateChangeListener {
private:
    SemanticChannelState semanticState;
    PhysicalChannelMapper physicalMapper;
    OSCCommunicator oscComm;
    ConfigManager configManager;
    
    // 最小VST3参数系统 - 只包含Gain
    juce::AudioProcessorValueTreeState apvts;
    
public:
    MonitorControllerMaxAudioProcessor();
    ~MonitorControllerMaxAudioProcessor() override;
    
    // 音频处理 - 核心功能
    void processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages) override;
    
    // 配置管理
    void setCurrentLayout(const juce::String& speaker, const juce::String& sub) override;
    const Layout& getCurrentLayout() const override;
    
    // UI访问接口
    SemanticChannelState& getSemanticState() { return semanticState; }
    PhysicalChannelMapper& getPhysicalMapper() { return physicalMapper; }
    OSCCommunicator& getOSCCommunicator() { return oscComm; }
    
    // 状态变化监听
    void onSemanticStateChanged() override;
    void onGlobalModeChanged() override;
    
private:
    void updatePhysicalMapping();
    void applyGainFromVST3Parameter(juce::AudioBuffer<float>& buffer, int physicalPin);
    
    // 移除所有原有的参数联动相关方法
    // 移除所有Solo/Mute参数相关代码
};
```

**新的processBlock实现**：
```cpp
void MonitorControllerMaxAudioProcessor::processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages) {
    juce::ScopedNoDenormals noDenormals;
    
    int totalNumChannels = buffer.getNumChannels();
    
    // 遍历所有物理输出通道
    for (int physicalPin = 0; physicalPin < totalNumChannels; ++physicalPin) {
        // 获取对应的语义通道名
        juce::String semanticName = physicalMapper.getSemanticName(physicalPin);
        
        // 应用语义状态到物理音频
        if (!semanticName.isEmpty() && semanticState.getFinalMuteState(semanticName)) {
            // 该语义通道被mute - 清除音频
            buffer.clear(physicalPin, 0, buffer.getNumSamples());
        } else {
            // 应用Gain（来自VST3参数系统）
            applyGainFromVST3Parameter(buffer, physicalPin);
        }
    }
}
```

### Phase 4: UI重构

#### 4.1 语义化UI组件
**文件**: `Source/PluginEditor.h/cpp` (重大修改)

**语义化按钮组件**：
```cpp
class SemanticSoloButton : public juce::TextButton {
private:
    MonitorControllerMaxAudioProcessor& processor;
    juce::String semanticChannelName;
    
public:
    SemanticSoloButton(MonitorControllerMaxAudioProcessor& proc, const juce::String& channelName)
        : processor(proc), semanticChannelName(channelName) 
    {
        setButtonText("Solo " + channelName);
        setClickingTogglesState(true);
    }
    
    void clicked() override {
        bool newState = getToggleState();
        
        // 直接操作语义状态 - 完全绕过VST3参数系统
        processor.getSemanticState().setSoloState(semanticChannelName, newState);
    }
    
    void updateFromSemanticState() {
        bool currentState = processor.getSemanticState().getSoloState(semanticChannelName);
        setToggleState(currentState, juce::dontSendNotification);
        
        // 更新颜色显示
        if (currentState) {
            setColour(juce::TextButton::buttonOnColourId, juce::Colours::green);
        }
    }
};

class SemanticMuteButton : public juce::TextButton {
    // 类似实现，使用红色显示
};
```

#### 4.2 动态UI布局
**配置驱动的UI更新**：
```cpp
void MonitorControllerMaxAudioProcessorEditor::updateLayoutFromConfig() {
    // 清除现有按钮
    clearExistingChannelButtons();
    
    // 获取当前配置的语义通道列表
    auto activeChannels = audioProcessor.getPhysicalMapper().getActiveSemanticChannels();
    
    // 为每个语义通道创建按钮
    for (const auto& channelName : activeChannels) {
        auto soloButton = std::make_unique<SemanticSoloButton>(audioProcessor, channelName);
        auto muteButton = std::make_unique<SemanticMuteButton>(audioProcessor, channelName);
        
        // 添加到UI布局
        addChannelButtonPair(channelName, std::move(soloButton), std::move(muteButton));
    }
    
    // 重新布局UI
    updateChannelGridLayout();
    resized();
}
```

#### 4.3 实时状态更新
**替换参数驱动为状态驱动**：
```cpp
void MonitorControllerMaxAudioProcessorEditor::timerCallback() {
    // 不再监听VST3参数变化，直接从语义状态更新UI
    updateAllChannelButtonsFromSemanticState();
    updateMainButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateAllChannelButtonsFromSemanticState() {
    for (auto& [channelName, buttonPair] : channelButtons) {
        buttonPair.soloButton->updateFromSemanticState();
        buttonPair.muteButton->updateFromSemanticState();
    }
}
```

### Phase 5: 集成和配置系统

#### 5.1 配置系统集成
**配置切换时的完整更新**：
```cpp
void MonitorControllerMaxAudioProcessor::setCurrentLayout(const juce::String& speaker, const juce::String& sub) {
    // 更新配置
    Layout newLayout = configManager.getLayout(speaker, sub);
    currentLayout = newLayout;
    
    // 更新物理映射
    physicalMapper.updateMapping(newLayout);
    
    // 重新初始化语义状态
    semanticState.clearAllStates();
    for (const auto& channelInfo : newLayout.channels) {
        semanticState.initializeChannel(channelInfo.name);
    }
    
    // 更新UI显示
    if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
        editor->updateLayoutFromConfig();
    }
    
    // 广播新状态给外部设备
    if (oscComm.isConnected()) {
        oscComm.broadcastAllStates(semanticState);
    }
}
```

#### 5.2 状态保存和恢复
**VST3状态管理**：
```cpp
void MonitorControllerMaxAudioProcessor::getStateInformation(juce::MemoryBlock& destData) {
    // 只保存VST3参数（Gain等）
    auto apvtsState = apvts.copyState();
    
    // 保存语义状态
    auto semanticStateXml = std::make_unique<juce::XmlElement>("SemanticState");
    
    auto activeChannels = physicalMapper.getActiveSemanticChannels();
    for (const auto& channelName : activeChannels) {
        auto channelXml = semanticStateXml->createNewChildElement("Channel");
        channelXml->setAttribute("name", channelName);
        channelXml->setAttribute("solo", semanticState.getSoloState(channelName));
        channelXml->setAttribute("mute", semanticState.getMuteState(channelName));
    }
    
    // 保存当前配置
    semanticStateXml->setAttribute("speakerLayout", currentLayout.speakerName);
    semanticStateXml->setAttribute("subLayout", currentLayout.subName);
    
    // 合并状态
    auto completeState = apvtsState.createCopy();
    completeState.appendChild(juce::ValueTree::fromXml(*semanticStateXml), nullptr);
    
    auto xml = completeState.createXml();
    copyXmlToBinary(*xml, destData);
}
```

### Phase 6: 测试和验证

#### 6.1 功能测试
**测试场景**：
```
1. 基本Solo/Mute功能
   - 单通道Solo → 其他通道Auto-Mute
   - Solo模式下的联动逻辑
   - Mute功能的独立工作

2. 配置切换测试
   - 5.1 → 7.1.4 配置切换
   - 物理映射正确更新
   - UI按钮正确重建

3. OSC通信测试
   - 状态变化时的OSC发送
   - 外部OSC控制接收
   - 状态反馈机制

4. VST3兼容性测试
   - 插件加载/卸载
   - 状态保存/恢复
   - 宿主自动化（仅Gain参数）
```

#### 6.2 外部集成测试
**OSC测试工具**：
```bash
# 发送OSC命令测试
oscsend 127.0.0.1 7444 /Monitor/Solo_L/ f 1.0
oscsend 127.0.0.1 7444 /Monitor/Mute_R/ f 0.0

# 监听OSC反馈
oscdump 7444
```

## 🔧 实施优先级

### 高优先级（立即执行）：
1. **Phase 1** - 实现核心语义化架构
2. **Phase 3.1** - 重构音频处理逻辑
3. **移除参数联动系统** - 清理所有旧代码

### 中优先级：
4. **Phase 2** - 实现OSC通信系统
5. **Phase 4** - 重构UI为语义化组件

### 低优先级：
6. **Phase 5** - 完善配置系统集成
7. **Phase 6** - 全面测试和优化

## 📊 实施进度追踪

### 当前阶段：Phase 1 - 核心架构重构 ✅ 已完成

**已创建的新文件**：
- ✅ `Source/SemanticChannelState.h/cpp` - 语义状态管理核心
- ✅ `Source/PhysicalChannelMapper.h/cpp` - 物理通道映射系统
- ✅ `Source/SemanticChannelButton.h/cpp` - 动态语义按钮组件
- ⏸️ `Source/OSCCommunicator.h/cpp` - 暂时跳过，专注语义状态系统

**已重构的现有文件**：
- ✅ `Source/PluginProcessor.h/cpp` - 集成语义状态系统，保留现有逻辑
- ✅ `Source/PluginEditor.h/cpp` - 添加动态语义按钮支持
- ⏸️ `MonitorControllerMax.jucer` - 用户手动添加新源文件

**暂时保留的文件**：
- ⏸️ `Source/ParameterLinkageEngine.h/cpp` - 暂时保留作为备用
- ⏸️ 其他参数联动相关代码 - 逐步迁移

## 🎯 成功标准

**架构目标达成**：
- ✅ 完全绕过VST3参数联动限制
- ✅ 语义通道命名保持一致性
- ✅ OSC外部集成完全功能
- ✅ 配置切换不影响控制协议

**功能验证标准**：
- ✅ Solo/Mute联动逻辑完全正确
- ✅ 外部OSC控制双向通信正常
- ✅ 状态反馈机制实时同步
- ✅ VST3基本功能保持兼容

## 🔥 架构突破意义

**这个新架构代表了从VST3限制到完全自由的根本性突破！**

- **技术突破** - 彻底解决VST3协议限制
- **架构优势** - 语义化一致性和完美外部集成
- **专业标准** - 达到专业监听控制器的工业级要求
- **未来扩展** - 为更复杂功能奠定坚实基础

**这就是现代专业音频插件的正确发展方向！** 🎵