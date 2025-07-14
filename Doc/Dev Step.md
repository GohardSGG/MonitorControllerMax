# v4.0主从插件系统实施计划

## 🎯 总体目标

**基于稳定基础架构实现同进程主从插件通信系统**

基于commit 5f04077f51a34e59794a805abe8ea46d5a42cf5c的稳定版本，使用静态全局状态管理器实现同进程内插件间的高效通信：

- **技术基础**：现有语义化状态系统、OSC通信、动态配置等核心功能稳定运行
- **实施原则**：最小侵入性、同进程优化、完全向后兼容
- **技术方案**：静态全局状态管理器 + 直接内存访问 + 零延迟同步

## 📋 实施阶段

### Phase 1: GlobalPluginState核心类实现

#### 1.1 创建GlobalPluginState基础类
**文件**: `Source/GlobalPluginState.h/cpp` (新建)

**核心状态管理器**：
```cpp
class GlobalPluginState {
private:
    // 单例模式 - 线程安全
    static std::unique_ptr<GlobalPluginState> instance;
    static std::mutex instanceMutex;
    
    // 全局状态存储
    std::map<juce::String, bool> globalSoloStates;
    std::map<juce::String, bool> globalMuteStates;
    std::mutex stateMutex;
    
    // 插件实例管理
    MonitorControllerMaxAudioProcessor* masterPlugin = nullptr;
    std::vector<MonitorControllerMaxAudioProcessor*> slavePlugins;
    std::vector<MonitorControllerMaxAudioProcessor*> allPlugins;
    std::mutex pluginsMutex;
    
public:
    // 单例访问
    static GlobalPluginState& getInstance();
    
    // 插件生命周期管理
    void registerPlugin(MonitorControllerMaxAudioProcessor* plugin);
    void unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin);
    
    // Master插件管理
    bool setAsMaster(MonitorControllerMaxAudioProcessor* plugin);
    void removeMaster(MonitorControllerMaxAudioProcessor* plugin);
    bool isMasterPlugin(MonitorControllerMaxAudioProcessor* plugin) const;
    
    // Slave插件管理
    bool addSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    void removeSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    std::vector<MonitorControllerMaxAudioProcessor*> getSlavePlugins() const;
    
    // 状态同步机制
    void setGlobalSoloState(const juce::String& channelName, bool state);
    void setGlobalMuteState(const juce::String& channelName, bool state);
    bool getGlobalSoloState(const juce::String& channelName) const;
    bool getGlobalMuteState(const juce::String& channelName) const;
    
    // 广播机制 - 直接调用，零延迟
    void broadcastStateToSlaves(const juce::String& channelName, const juce::String& action, bool state);
    void syncAllStatesToSlave(MonitorControllerMaxAudioProcessor* slavePlugin);
    
    // 状态查询
    int getSlaveCount() const;
    bool hasMaster() const;
    juce::String getConnectionInfo() const;
    
private:
    GlobalPluginState() = default;
    ~GlobalPluginState() = default;
    
    // 防止复制
    GlobalPluginState(const GlobalPluginState&) = delete;
    GlobalPluginState& operator=(const GlobalPluginState&) = delete;
};
```

**关键实现要点**：
- 线程安全的单例模式，支持多线程DAW环境
- 分离的互斥锁：状态锁定和插件列表锁定
- 直接内存访问，无序列化/反序列化开销
- RAII管理插件生命周期

#### 1.2 GlobalPluginState核心方法实现

**单例模式实现**：
```cpp
std::unique_ptr<GlobalPluginState> GlobalPluginState::instance = nullptr;
std::mutex GlobalPluginState::instanceMutex;

GlobalPluginState& GlobalPluginState::getInstance() {
    std::lock_guard<std::mutex> lock(instanceMutex);
    if (!instance) {
        instance = std::unique_ptr<GlobalPluginState>(new GlobalPluginState());
    }
    return *instance;
}
```

**插件注册管理**：
```cpp
void GlobalPluginState::registerPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
    if (it == allPlugins.end()) {
        allPlugins.push_back(plugin);
        VST3_DBG("Plugin registered to GlobalPluginState, total: " + juce::String(allPlugins.size()));
    }
}

void GlobalPluginState::unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    // 从所有列表中移除
    auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
    if (it != allPlugins.end()) {
        allPlugins.erase(it);
    }
    
    // 如果是Master，清除Master状态
    if (masterPlugin == plugin) {
        masterPlugin = nullptr;
        VST3_DBG("Master plugin unregistered");
    }
    
    // 如果是Slave，从Slave列表移除
    auto slaveIt = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (slaveIt != slavePlugins.end()) {
        slavePlugins.erase(slaveIt);
        VST3_DBG("Slave plugin unregistered");
    }
}
```

**状态同步和广播**：
```cpp
void GlobalPluginState::broadcastStateToSlaves(const juce::String& channelName, const juce::String& action, bool state) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    for (auto* slave : slavePlugins) {
        if (slave != nullptr) {
            // 直接调用Slave的状态接收方法 - 零延迟
            slave->receiveMasterState(channelName, action, state);
        }
    }
    
    VST3_DBG("Broadcast to " + juce::String(slavePlugins.size()) + " slaves: " + action + " " + channelName);
}
```

### Phase 2: 角色管理系统集成

#### 2.1 PluginProcessor角色管理扩展
**文件**: `Source/PluginProcessor.h/cpp` (扩展现有文件)

**角色定义和管理**：
```cpp
enum class PluginRole {
    Standalone = 0,  // 默认独立模式
    Master = 1,      // 主控制模式
    Slave = 2        // 从属显示模式
};

class MonitorControllerMaxAudioProcessor : public SemanticChannelState::StateChangeListener {
private:
    // 新增成员变量
    PluginRole currentRole = PluginRole::Standalone;
    bool isRegisteredToGlobalState = false;
    bool suppressStateChange = false;  // 防止循环回调
    
public:
    // 角色管理接口
    void switchToStandalone();
    void switchToMaster();
    void switchToSlave();
    PluginRole getCurrentRole() const { return currentRole; }
    
    // 状态同步接口（供GlobalPluginState调用）
    void receiveMasterState(const juce::String& channelName, const juce::String& action, bool state);
    void notifyMasterStatusChanged();
    
    // 连接状态查询
    bool isMasterWithSlaves() const;
    bool isSlaveConnected() const;
    int getConnectedSlaveCount() const;
    juce::String getConnectionStatusText() const;
    
private:
    void registerToGlobalState();
    void unregisterFromGlobalState();
    void handleRoleTransition(PluginRole newRole);
    void updateUIFromRole();
};
```

**角色切换实现**：
```cpp
void MonitorControllerMaxAudioProcessor::switchToMaster() {
    if (currentRole == PluginRole::Master) return;
    
    auto& globalState = GlobalPluginState::getInstance();
    
    if (globalState.setAsMaster(this)) {
        handleRoleTransition(PluginRole::Master);
        VST3_DBG("Successfully switched to Master mode");
        
        // 同步当前状态到所有Slave
        auto activeChannels = physicalMapper.getActiveSemanticChannels();
        for (const auto& channelName : activeChannels) {
            bool soloState = semanticState.getSoloState(channelName);
            bool muteState = semanticState.getMuteState(channelName);
            
            globalState.setGlobalSoloState(channelName, soloState);
            globalState.setGlobalMuteState(channelName, muteState);
            globalState.broadcastStateToSlaves(channelName, "solo", soloState);
            globalState.broadcastStateToSlaves(channelName, "mute", muteState);
        }
    } else {
        VST3_DBG("Failed to switch to Master - another Master exists");
        // 保持当前角色不变
    }
}

void MonitorControllerMaxAudioProcessor::switchToSlave() {
    auto& globalState = GlobalPluginState::getInstance();
    
    if (currentRole == PluginRole::Master) {
        globalState.removeMaster(this);
    }
    
    if (globalState.addSlavePlugin(this)) {
        handleRoleTransition(PluginRole::Slave);
        
        // 同步Master状态到本地
        globalState.syncAllStatesToSlave(this);
        VST3_DBG("Successfully switched to Slave mode");
    } else {
        VST3_DBG("Failed to switch to Slave - no Master available");
        switchToStandalone();
    }
}
```

#### 2.2 状态同步逻辑实现

**Master状态广播集成**：
```cpp
void MonitorControllerMaxAudioProcessor::onSemanticStateChanged(
    const juce::String& channelName, const juce::String& action, bool state) {
    
    // 防止循环回调
    if (suppressStateChange) return;
    
    // 现有OSC通信（保持不变）
    if (currentRole != PluginRole::Slave) {
        // 只有非Slave角色才发送OSC消息
        if (action == "solo") {
            oscCommunicator.sendSoloState(channelName, state);
        } else if (action == "mute") {
            oscCommunicator.sendMuteState(channelName, state);
        }
    }
    
    // 新增主从同步（最小侵入）
    if (currentRole == PluginRole::Master) {
        auto& globalState = GlobalPluginState::getInstance();
        
        // 更新全局状态
        if (action == "solo") {
            globalState.setGlobalSoloState(channelName, state);
        } else if (action == "mute") {
            globalState.setGlobalMuteState(channelName, state);
        }
        
        // 广播给所有Slave
        globalState.broadcastStateToSlaves(channelName, action, state);
    }
}
```

**Slave状态接收实现**：
```cpp
void MonitorControllerMaxAudioProcessor::receiveMasterState(
    const juce::String& channelName, const juce::String& action, bool state) {
    
    if (currentRole != PluginRole::Slave) return;
    
    // 防止循环回调
    suppressStateChange = true;
    
    try {
        // 应用Master状态到本地语义状态
        if (action == "solo") {
            semanticState.setSoloState(channelName, state);
        } else if (action == "mute") {
            semanticState.setMuteState(channelName, state);
        }
        
        VST3_DBG("Slave received Master state: " + action + " " + channelName + " = " + (state ? "true" : "false"));
        
        // 异步通知UI更新
        juce::MessageManager::callAsync([this]() {
            if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
                editor->updateFromSemanticState();
            }
        });
        
    } catch (const std::exception& e) {
        VST3_DBG("Error receiving Master state: " + juce::String(e.what()));
    }
    
    // 重新启用回调
    suppressStateChange = false;
}
```

### Phase 3: UI集成适配

#### 3.1 角色选择UI组件
**文件**: `Source/PluginEditor.h/cpp` (扩展现有UI)

**UI组件声明**：
```cpp
class MonitorControllerMaxAudioProcessorEditor : public juce::AudioProcessorEditor,
                                               public juce::Timer {
private:
    // 现有UI组件 ...
    
    // 新增角色管理UI组件
    juce::ComboBox roleSelector;
    juce::Label roleLabel;
    juce::Label connectionStatusLabel;
    std::unique_ptr<juce::Component> slaveOverlay;
    
    // UI状态
    bool isUILockedForSlave = false;
    
public:
    // 新增方法
    void setupRoleManagementUI();
    void updateUIForRole();
    void updateConnectionStatus();
    void enableAllControls(bool enabled);
    void updateFromSemanticState();
    
private:
    void onRoleSelectionChanged();
    void createSlaveOverlay();
    void removeSlaveOverlay();
    void layoutRoleManagementComponents();
};
```

**角色选择器实现**：
```cpp
void MonitorControllerMaxAudioProcessorEditor::setupRoleManagementUI() {
    // 角色标签
    roleLabel.setText("Role:", juce::dontSendNotification);
    roleLabel.setJustificationType(juce::Justification::centredRight);
    addAndMakeVisible(roleLabel);
    
    // 角色选择器
    roleSelector.addItem("Standalone", static_cast<int>(PluginRole::Standalone) + 1);
    roleSelector.addItem("Master", static_cast<int>(PluginRole::Master) + 1);
    roleSelector.addItem("Slave", static_cast<int>(PluginRole::Slave) + 1);
    
    roleSelector.setSelectedId(static_cast<int>(audioProcessor.getCurrentRole()) + 1, juce::dontSendNotification);
    roleSelector.onChange = [this] { onRoleSelectionChanged(); };
    addAndMakeVisible(roleSelector);
    
    // 连接状态标签
    connectionStatusLabel.setText("Standalone", juce::dontSendNotification);
    connectionStatusLabel.setJustificationType(juce::Justification::centredLeft);
    addAndMakeVisible(connectionStatusLabel);
    
    // 初始UI状态
    updateUIForRole();
}

void MonitorControllerMaxAudioProcessorEditor::onRoleSelectionChanged() {
    int selectedId = roleSelector.getSelectedId();
    PluginRole selectedRole = static_cast<PluginRole>(selectedId - 1);
    
    switch (selectedRole) {
        case PluginRole::Standalone:
            audioProcessor.switchToStandalone();
            break;
        case PluginRole::Master:
            audioProcessor.switchToMaster();
            break;
        case PluginRole::Slave:
            audioProcessor.switchToSlave();
            break;
    }
    
    // 确保选择器反映实际状态（切换可能失败）
    roleSelector.setSelectedId(static_cast<int>(audioProcessor.getCurrentRole()) + 1, juce::dontSendNotification);
    updateUIForRole();
}
```

#### 3.2 UI状态控制机制

**角色UI适配**：
```cpp
void MonitorControllerMaxAudioProcessorEditor::updateUIForRole() {
    auto role = audioProcessor.getCurrentRole();
    
    switch (role) {
        case PluginRole::Standalone:
            enableAllControls(true);
            removeSlaveOverlay();
            connectionStatusLabel.setText("Standalone", juce::dontSendNotification);
            roleSelector.setEnabled(true);
            isUILockedForSlave = false;
            break;
            
        case PluginRole::Master:
            enableAllControls(true);
            removeSlaveOverlay();
            updateConnectionStatus();
            roleSelector.setEnabled(true);
            isUILockedForSlave = false;
            break;
            
        case PluginRole::Slave:
            enableAllControls(false);
            createSlaveOverlay();
            connectionStatusLabel.setText("Slave (syncing with Master)", juce::dontSendNotification);
            roleSelector.setEnabled(false);  // Slave不能切换角色
            isUILockedForSlave = true;
            break;
    }
    
    repaint();
}

void MonitorControllerMaxAudioProcessorEditor::createSlaveOverlay() {
    if (slaveOverlay != nullptr) return;
    
    slaveOverlay = std::make_unique<juce::Component>();
    slaveOverlay->setBounds(getLocalBounds());
    slaveOverlay->setAlpha(0.5f);
    slaveOverlay->setInterceptsMouseClicks(true, true);
    
    // 添加到最顶层
    addAndMakeVisible(*slaveOverlay);
    slaveOverlay->toFront(false);
}

void MonitorControllerMaxAudioProcessorEditor::removeSlaveOverlay() {
    if (slaveOverlay != nullptr) {
        slaveOverlay.reset();
    }
}
```

**连接状态更新**：
```cpp
void MonitorControllerMaxAudioProcessorEditor::updateConnectionStatus() {
    juce::String statusText = audioProcessor.getConnectionStatusText();
    connectionStatusLabel.setText(statusText, juce::dontSendNotification);
}

void MonitorControllerMaxAudioProcessorEditor::timerCallback() {
    // 现有计时器逻辑 ...
    
    // 新增连接状态更新
    if (audioProcessor.getCurrentRole() == PluginRole::Master) {
        updateConnectionStatus();
    }
    
    // 如果不是Slave模式，正常更新UI
    if (!isUILockedForSlave) {
        updateAllChannelButtonsFromSemanticState();
    }
}
```

### Phase 4: 集成测试和验证

#### 4.1 构造/析构函数集成
**文件**: `Source/PluginProcessor.cpp` (修改现有构造/析构函数)

**构造函数注册**：
```cpp
MonitorControllerMaxAudioProcessor::MonitorControllerMaxAudioProcessor()
    : // 现有初始化列表 ...
{
    // 现有初始化代码 ...
    
    // 新增：注册到GlobalPluginState
    registerToGlobalState();
    
    VST3_DBG("Plugin initialized and registered to GlobalPluginState");
}

void MonitorControllerMaxAudioProcessor::registerToGlobalState() {
    if (!isRegisteredToGlobalState) {
        GlobalPluginState::getInstance().registerPlugin(this);
        isRegisteredToGlobalState = true;
    }
}
```

**析构函数注销**：
```cpp
MonitorControllerMaxAudioProcessor::~MonitorControllerMaxAudioProcessor() {
    VST3_DBG("Plugin destructor - cleaning up GlobalPluginState registration");
    
    // 先注销GlobalPluginState
    unregisterFromGlobalState();
    
    // 现有清理代码 ...
}

void MonitorControllerMaxAudioProcessor::unregisterFromGlobalState() {
    if (isRegisteredToGlobalState) {
        GlobalPluginState::getInstance().unregisterPlugin(this);
        isRegisteredToGlobalState = false;
    }
}
```

#### 4.2 功能完整性测试计划

**测试场景覆盖**：
```
1. 基础角色切换测试
   - Standalone → Master: 成功切换，UI更新正确
   - Master → Slave: 成功切换，UI锁定，状态同步
   - Slave → Standalone: 成功切换，UI解锁
   - 多次角色切换无内存泄漏

2. 多实例Master冲突测试
   - 第一个插件切换Master: 成功
   - 第二个插件尝试切换Master: 失败，保持原角色
   - 第一个Master关闭: 第二个插件可成功切换Master

3. 状态同步测试
   - Master操作Solo按钮 → 所有Slave实时同步显示
   - Master操作Mute按钮 → 所有Slave实时同步显示
   - 多Slave并发连接 → 状态同步正确
   - Slave UI完全锁定 → 无法操作任何控件

4. 生命周期测试
   - 插件加载/卸载 → GlobalPluginState正确注册/注销
   - Master插件关闭 → Slave插件自动切换Standalone
   - 多实例并发加载/卸载 → 无崩溃，无内存泄漏

5. 性能测试
   - 状态同步延迟 < 1ms
   - CPU占用增量 < 2%
   - 内存占用增量 < 1MB
```

#### 4.3 错误处理和边界条件

**边界条件处理**：
```cpp
// GlobalPluginState中的安全检查
void GlobalPluginState::broadcastStateToSlaves(const juce::String& channelName, const juce::String& action, bool state) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    // 安全检查：移除无效指针
    slavePlugins.erase(
        std::remove_if(slavePlugins.begin(), slavePlugins.end(),
            [](MonitorControllerMaxAudioProcessor* plugin) {
                return plugin == nullptr;
            }),
        slavePlugins.end()
    );
    
    for (auto* slave : slavePlugins) {
        try {
            slave->receiveMasterState(channelName, action, state);
        } catch (const std::exception& e) {
            VST3_DBG("Error broadcasting to slave: " + juce::String(e.what()));
        }
    }
}
```

## 🔧 实施优先级

### 高优先级（立即执行）：
1. **Phase 1.1** - 创建GlobalPluginState基础类
2. **Phase 1.2** - 实现核心方法和单例模式
3. **Phase 2.1** - 集成PluginProcessor角色管理

### 中优先级：
4. **Phase 2.2** - 实现状态同步逻辑
5. **Phase 3.1** - 添加角色选择UI
6. **Phase 3.2** - 实现UI状态控制

### 低优先级：
7. **Phase 4.1** - 构造/析构函数集成
8. **Phase 4.2** - 全面测试和验证

## 📊 实施进度追踪

### ⚠️ **Phase 1 - 核心GlobalPluginState类** - 待实施

**计划创建的新文件**：
- 🔜 `Source/GlobalPluginState.h/cpp` - 静态全局状态管理器

**核心功能实现**：
- 🔜 线程安全单例模式
- 🔜 插件实例注册/注销机制
- 🔜 Master/Slave角色管理
- 🔜 状态存储和广播机制

### 🔜 **Phase 2 - 角色管理集成** - 待实施

**计划修改的现有文件**：
- 🔜 `Source/PluginProcessor.h/cpp` - 添加角色管理方法
- 🔜 集成状态变化回调
- 🔜 实现Master/Slave状态同步

### 🔜 **Phase 3 - UI集成适配** - 待实施

**UI功能扩展**：
- 🔜 角色选择下拉框 (PluginEditor)
- 🔜 连接状态显示标签
- 🔜 Slave模式UI锁定机制
- 🔜 实时状态更新响应

## 🎯 成功标准验证

### ✅ **架构目标**
- 🎯 **同进程优化** - 使用静态全局状态，专为DAW环境设计
- 🎯 **最小侵入性** - 现有语义状态系统完全保持不变
- 🎯 **零依赖** - 无需网络、端口、序列化，纯内存操作
- 🎯 **线程安全** - 多实例并发稳定运行

### 🔜 **功能验证标准**
- 🔜 **角色切换流畅** - 三种角色无缝切换，Master冲突正确处理
- 🔜 **状态同步实时** - Master操作立即同步到Slave (< 1ms)
- 🔜 **UI响应正确** - Slave UI正确锁定，连接状态准确显示
- 🔜 **生命周期健壮** - 插件加载/卸载正确处理，无内存泄漏

### 🔜 **集成兼容性验证**
- 🔜 **现有功能保持** - Solo/Mute逻辑、OSC通信、配置系统完全不变
- 🔜 **性能影响最小** - CPU/内存占用增量 < 2%
- 🔜 **编译稳定性** - Debug/Release编译成功，无警告错误

## 🏆 **v4.0架构优势**

**这个新架构具有以下关键优势：**

- **同进程专优** - 针对DAW同进程插件环境专门设计
- **零延迟通信** - 直接内存访问，纳秒级状态同步
- **线程安全** - 完整的互斥锁保护，支持多线程DAW
- **最小开销** - 无网络、无序列化，最小的性能影响
- **稳定可靠** - 基于经过验证的稳定版本构建
- **简单维护** - 清晰的角色分工，直观的实现逻辑

**这标志着使用最适合DAW环境的技术方案，实现高效可靠的专业级主从插件通信系统！** 🎵🎉