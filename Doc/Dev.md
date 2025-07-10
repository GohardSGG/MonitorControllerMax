# 监听控制器插件开发文档 - 保守式语义化OSC架构

## 📋 项目当前状态 (2025-01-10)

### 🚨 重大架构决策：最小改动的语义化OSC集成

**背景**：经过深入研究VST3协议限制，发现了根本性约束：
- ❌ **VST3铁律**：`"No automated parameter must influence another automated parameter!"`
- ❌ **参数联动在VST3中从协议层面被禁止**
- ❌ **所有尝试的参数间联动都会被宿主阻止**

**解决方案**：**保守式**语义化内部状态 + OSC外部通信架构
**核心原则**：保留现有所有工作逻辑，只替换底层数据源

## 🏗️ 保守式语义化OSC架构

### 设计哲学：最小改动 + 最大保留

**核心理念**：
```
现有逻辑完全保留 + 数据源切换 + OSC通信附加 = 渐进式升级
```

**架构流程**：
```
用户操作 → 语义状态更新(替换VST3参数) → 现有逻辑保留 → OSC状态广播 → 外部设备同步
```

### 🎯 关键设计决策

1. **动态UI创建**：根据配置创建按钮，但保持网格位置系统
2. **现有逻辑完全保留**：所有Solo/Mute复杂逻辑、记忆管理、选择模式保持不变
3. **数据源渐进切换**：从VST3参数逐步切换到语义状态
4. **OSC简单集成**：在现有状态变化处添加OSC发送，不改变架构
5. **音频处理安全**：继续处理最大26通道，未映射通道使用默认值
6. **配置驱动映射**：语义通道动态映射到物理音频pin，但保持配置方法
7. **OSC标准通信**：127.0.0.1:7444，地址格式 `/Monitor/Solo_L/`

## 🏗️ 三层渐进式架构设计

### Layer 1: 语义化内部状态系统（新增，不影响现有系统）
```cpp
class SemanticChannelState {
private:
    // 语义通道状态存储 - 作为VST3参数的替代数据源
    std::map<String, bool> soloStates;  // "L", "R", "C", "LFE", "LR", "RR", ...
    std::map<String, bool> muteStates;  // "L", "R", "C", "LFE", "LR", "RR", ...
    bool globalSoloModeActive = false;
    
public:
    // 语义化操作接口 - 完全兼容现有调用方式
    void setSoloState(const String& channelName, bool state);
    void setMuteState(const String& channelName, bool state);
    bool getSoloState(const String& channelName) const;
    bool getMuteState(const String& channelName) const;
    bool getFinalMuteState(const String& channelName) const;
    
    // **保留现有ParameterLinkageEngine的所有逻辑**
    // 只是把底层数据源从VST3参数换成这个语义状态
    void calculateSoloModeLinkage();
    
    // OSC通信接口（新增，不影响现有功能）
    ListenerList<StateChangeListener> onStateChanged;
};
```

### Layer 2: 配置驱动物理映射系统（增强现有ConfigManager）
```cpp
class PhysicalChannelMapper {
private:
    // 语义名称 ↔ 物理Pin映射（继承现有配置系统）
    std::map<String, int> semanticToPhysical;  // "L" → 1, "R" → 5, etc.
    std::map<int, String> physicalToSemantic;  // 1 → "L", 5 → "R", etc.
    std::map<String, std::pair<int, int>> gridPositions; // "L" → {gridX, gridY}
    
public:
    // **完全兼容现有配置系统**
    void updateMapping(const Layout& layout);
    
    // 映射转换（保持现有调用方式）
    int getPhysicalPin(const String& semanticName) const;
    String getSemanticName(int physicalPin) const;
    
    // 获取当前激活的语义通道列表（用于动态UI创建）
    std::vector<String> getActiveSemanticChannels() const;
    
    // **保留现有网格位置系统**
    std::pair<int, int> getGridPosition(const String& semanticName) const;
    
    // 安全处理：未映射通道返回默认值
    String getSemanticNameSafe(int physicalPin) const;
};
```

### Layer 3: OSC通信系统（纯附加功能）
```cpp
class OSCCommunicator {
private:
    OSCSender sender;
    OSCReceiver receiver;
    const String targetIP = "127.0.0.1";
    const int targetPort = 7444;
    
public:
    void initialize();
    void shutdown();
    
    // **简单集成模式**：在现有状态变化处调用
    void sendSoloState(const String& channelName, bool state) {
        if (!sender.isConnected()) return;
        String address = "/Monitor/Solo_" + channelName + "/";
        sender.send(address, state ? 1.0f : 0.0f);
    }
    
    void sendMuteState(const String& channelName, bool state) {
        if (!sender.isConnected()) return;
        String address = "/Monitor/Mute_" + channelName + "/";
        sender.send(address, state ? 1.0f : 0.0f);
    }
    
    // 状态反馈机制 - 广播所有当前状态
    void broadcastAllStates(const SemanticChannelState& state);
    
    // 接收外部控制（更新内部语义状态）
    void oscMessageReceived(const OSCMessage& message) override;
    
    // **不改变现有状态管理架构**
    bool isConnected() const { return sender.isConnected(); }
};
```

## 🎵 音频处理集成（最小改动）

### 主处理器架构（保留现有架构，添加语义化支持）
```cpp
class MonitorControllerProcessor : public AudioProcessor {
private:
    // **新增语义化系统（不影响现有功能）**
    SemanticChannelState semanticState;
    PhysicalChannelMapper physicalMapper;
    OSCCommunicator oscComm;
    
    // **保留现有系统**
    AudioProcessorValueTreeState apvts;  // 继续保留所有VST3参数
    ConfigManager configManager;         // 现有配置管理
    // 暂时保留ParameterLinkageEngine直到完全切换
    
public:
    void processBlock(AudioBuffer<float>& buffer, MidiBuffer&) override {
        // **向下兼容的安全处理**
        for (int physicalPin = 0; physicalPin < buffer.getNumChannels(); ++physicalPin) {
            // 获取语义通道名（如果有映射）
            String semanticName = physicalMapper.getSemanticNameSafe(physicalPin);
            
            // 应用语义状态到物理音频
            if (!semanticName.isEmpty() && semanticState.getFinalMuteState(semanticName)) {
                buffer.clear(physicalPin, 0, buffer.getNumSamples());
            } else {
                // **保留现有增益处理逻辑**
                applyGainFromVST3Parameter(buffer, physicalPin);
            }
        }
    }
    
    // **保留现有接口，添加语义化接口**
    SemanticChannelState& getSemanticState() { return semanticState; }
    PhysicalChannelMapper& getPhysicalMapper() { return physicalMapper; }
    OSCCommunicator& getOSCCommunicator() { return oscComm; }
};
```

## 🎮 UI组件设计（最小改动）

### 动态语义化按钮组件（保留现有交互逻辑）
```cpp
class SemanticSoloButton : public TextButton {
private:
    MonitorControllerProcessor& processor;
    String semanticChannelName;  // "L", "R", "C", etc.
    
public:
    SemanticSoloButton(MonitorControllerProcessor& proc, const String& channelName)
        : processor(proc), semanticChannelName(channelName) 
    {
        setButtonText("Solo " + channelName);
        setClickingTogglesState(true);
    }
    
    void clicked() override {
        bool newState = getToggleState();
        
        // **保留现有复杂逻辑调用**
        // 只是把底层数据源从VST3参数换成语义状态
        processor.getSemanticState().setSoloState(semanticChannelName, newState);
        
        // **OSC通信作为附加功能**
        processor.getOSCCommunicator().sendSoloState(semanticChannelName, newState);
    }
    
    void updateFromSemanticState() {
        bool currentState = processor.getSemanticState().getSoloState(semanticChannelName);
        setToggleState(currentState, dontSendNotification);
        
        // **保留现有颜色和视觉反馈逻辑**
        updateButtonAppearance(currentState);
    }
    
private:
    void updateButtonAppearance(bool state) {
        // 现有的按钮外观逻辑保持不变
        if (state) {
            setColour(TextButton::buttonOnColourId, Colours::green);
        } else {
            setColour(TextButton::buttonOnColourId, Colours::grey);
        }
    }
};
```

## 📡 OSC通信协议（简单集成）

### OSC地址格式
```
发送地址格式：/Monitor/{Action}_{Channel}/
取值范围：1.0f (On) / 0.0f (Off)

示例：
/Monitor/Solo_L/     1.0    // 左声道Solo开启
/Monitor/Mute_R/     0.0    // 右声道Mute关闭
/Monitor/Solo_C/     1.0    // 中置声道Solo开启
/Monitor/Mute_LFE/   1.0    // 低频声道Mute开启
```

### 状态反馈机制（在现有逻辑上添加）
```cpp
void OSCCommunicator::broadcastAllStates(const SemanticChannelState& state) {
    if (!isConnected()) return;
    
    // 遍历当前配置的活跃语义通道
    auto activeChannels = physicalMapper.getActiveSemanticChannels();
    for (const String& channelName : activeChannels) {
        // 发送Solo状态
        bool soloState = state.getSoloState(channelName);
        sendSoloState(channelName, soloState);
        
        // 发送Mute状态
        bool muteState = state.getMuteState(channelName);
        sendMuteState(channelName, muteState);
    }
}

// **简单集成触发时机**
void SemanticChannelState::setSoloState(const String& channelName, bool state) {
    soloStates[channelName] = state;
    
    // **保留现有的复杂Solo逻辑**
    calculateSoloModeLinkage(); // 现有方法保持不变
    
    // **添加OSC通信（不影响现有逻辑）**
    onStateChanged.call([this, channelName, state](StateChangeListener* l) {
        l->onSoloStateChanged(channelName, state);
    });
}
```

## 🔧 配置系统集成（增强现有系统）

### 动态映射更新（保留现有接口）
```cpp
void MonitorControllerProcessor::setCurrentLayout(const String& speaker, const String& sub) {
    // **保留现有配置系统调用**
    Layout newLayout = configManager.getLayout(speaker, sub);
    currentLayout = newLayout;
    
    // **添加物理映射更新**
    physicalMapper.updateMapping(newLayout);
    
    // **保留现有UI更新逻辑**
    updateUIChannelList(newLayout);
    
    // **添加OSC状态广播（不影响现有功能）**
    if (oscComm.isConnected()) {
        oscComm.broadcastAllStates(semanticState);
    }
}

// 示例映射更新（兼容现有配置格式）
void PhysicalChannelMapper::updateMapping(const Layout& layout) {
    semanticToPhysical.clear();
    physicalToSemantic.clear();
    gridPositions.clear();
    
    // **完全兼容现有配置文件格式**
    for (const auto& channelInfo : layout.channels) {
        String semanticName = channelInfo.name;     // "L", "R", "C"
        int physicalPin = channelInfo.channelIndex; // 1, 5, 3
        
        semanticToPhysical[semanticName] = physicalPin;
        physicalToSemantic[physicalPin] = semanticName;
        
        // **保留网格位置信息**
        gridPositions[semanticName] = {channelInfo.gridX, channelInfo.gridY};
    }
}
```

## 🎯 架构优势

### ✅ 完全绕过VST3限制
- 内部状态不是VST3参数，可以任意联动
- Solo/Mute逻辑完全在内部实现
- 无需担心宿主参数面板同步问题

### ✅ 最小改动风险
- **保留所有现有工作逻辑**
- **保留现有UI交互体验**
- **保留现有配置系统**
- **保留现有复杂状态管理**
- **渐进式数据源切换**

### ✅ 语义一致性
- Solo_L永远表示左声道，不管物理pin是几
- 配置切换不影响OSC控制协议
- 外部设备控制协议统一稳定

### ✅ 完美外部集成
- OSC协议提供完整的双向通信
- 状态反馈确保外部设备同步
- 标准化地址格式便于集成

### ✅ 保持VST3兼容
- 继续保留所有VST3参数
- 不会触发参数联动冲突
- 宿主可以正常保存/加载插件

### ✅ 音频处理安全
- 继续处理最大26通道
- 未映射通道使用安全默认值
- 向下兼容：少配置不影响多输入

## 📋 实现计划（保守渐进式）

### 第一阶段：核心架构实现（不影响现有功能）
1. 实现SemanticChannelState类
2. 实现PhysicalChannelMapper类
3. 集成到主处理器processBlock
4. **保留所有VST3参数，暂不移除**

### 第二阶段：UI数据源切换（最小改动）
1. 修改UI按钮为动态创建
2. 保留现有按钮交互逻辑
3. 切换按钮数据源：VST3参数 → 语义状态
4. 保留现有颜色、布局、网格位置系统

### 第三阶段：OSC通信实现（附加功能）
1. 实现OSCCommunicator类
2. 集成OSC发送/接收功能
3. 在现有状态变化处添加OSC调用
4. 测试OSC通信协议

### 第四阶段：渐进式测试和优化
1. 测试不同配置下的物理映射
2. 验证OSC外部控制功能
3. 测试状态反馈机制
4. 多配置切换测试
5. **最后阶段考虑移除VST3 Solo/Mute参数**

## 🔥 关键突破

**这个保守式架构彻底解决了VST3参数联动限制，同时保持最小风险！**

- **不再对抗VST3协议** - 拥抱约束而不是对抗
- **完全的控制权** - 内部状态完全自主控制
- **最小改动风险** - 保留所有现有工作逻辑
- **渐进式升级** - 可以逐步切换，随时回滚
- **标准化通信** - OSC协议提供工业级外部集成
- **语义化一致性** - 控制协议不受配置影响

**这就是专业监听控制器的正确渐进式升级路径！** 🎵