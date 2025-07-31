/*
  ==============================================================================

    StateManager.cpp
    Created: 2025-07-30
    Author:  GohardSGG & Claude Code

    状态管理器实现 - 所有业务逻辑的中心

  ==============================================================================
*/

#include "StateManager.h"
#include "RenderState.h"
#include "PluginProcessor.h"
#include "GlobalPluginState.h"

//==============================================================================
StateManager::StateManager(MonitorControllerMaxAudioProcessor& processor)
    : processor(processor)
{
    // 初始化双缓冲渲染状态
    renderStateA = std::make_unique<RenderState>();
    renderStateB = std::make_unique<RenderState>();
    
    // 初始设置A为活跃状态
    activeRenderState.store(renderStateA.get());
    
    // 初始化OSC通信器
    oscComm = std::make_unique<OSCCommunicator>();
    
    // 注册为语义状态监听器
    processor.getSemanticState().addStateChangeListener(this);
    
    // 从AudioProcessorValueTreeState同步初始状态
    syncFromValueTreeState();
    
    // 计算初始渲染状态
    recalculateRenderState(renderStateA.get());
    
    VST3_DBG("StateManager initialized with double-buffered render states");
}

//==============================================================================
StateManager::~StateManager()
{
    // 移除监听器
    processor.getSemanticState().removeStateChangeListener(this);
}

//==============================================================================
// 用户接口实现（消息线程）
void StateManager::setSoloState(const juce::String& channelName, bool soloState)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.soloStates[channelName] = soloState;
    }
    
    // 更新语义状态系统
    processor.getSemanticState().setSoloState(channelName, soloState);
    
    // 通知状态变化
    notifyStateChange(channelName, "solo", soloState);
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setMuteState(const juce::String& channelName, bool muteState)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.muteStates[channelName] = muteState;
    }
    
    // 更新语义状态系统
    processor.getSemanticState().setMuteState(channelName, muteState);
    
    // 通知状态变化
    notifyStateChange(channelName, "mute", muteState);
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setChannelGain(const juce::String& channelName, float gainDb)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        // 将dB转换为线性增益
        state.gainStates[channelName] = juce::Decibels::decibelsToGain(gainDb);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setMasterGain(float gainPercent)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.masterGainPercent = juce::jlimit(0.0f, 100.0f, gainPercent);
    }
    
    // 更新音频处理器参数
    if (auto* param = processor.apvts.getParameter("masterGain")) {
        param->setValueNotifyingHost(state.masterGainPercent / 100.0f);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setDimActive(bool active)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.dimActive = active;
    }
    
    // 通知总线处理器
    processor.masterBusProcessor.setDimActive(active);
    
    // 发送OSC更新（如果是Master或Standalone）
    if (state.currentRole != PluginRole::Slave) {
        oscComm->sendMasterDim(active);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setLowBoostActive(bool active)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.lowBoostActive = active;
    }
    
    // 通知总线处理器
    processor.masterBusProcessor.setLowBoostActive(active);
    
    // 发送OSC更新（如果是Master或Standalone）
    if (state.currentRole != PluginRole::Slave) {
        oscComm->sendMasterLowBoost(active);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setMasterMuteActive(bool active)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.masterMuteActive = active;
    }
    
    // 通知总线处理器
    processor.masterBusProcessor.setMasterMuteActive(active);
    
    // 发送OSC更新（如果是Master或Standalone）
    if (state.currentRole != PluginRole::Slave) {
        oscComm->sendMasterMute(active);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setMonoActive(bool active)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.monoActive = active;
    }
    
    // 通知总线处理器
    processor.masterBusProcessor.setMonoActive(active);
    
    // 发送OSC更新（如果是Master或Standalone）
    if (state.currentRole != PluginRole::Slave) {
        oscComm->sendMasterMono(active);
    }
    
    // 如果是Master，广播到Slave
    if (state.currentRole == PluginRole::Master) {
        GlobalPluginState::getInstance().broadcastMonoStateToSlaves(active);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
void StateManager::setCurrentLayout(const juce::String& speakerLayout, const juce::String& subLayout)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        
        // 从配置管理器获取新布局
        state.currentLayout = processor.configManager.getLayoutFor(speakerLayout, subLayout);
    }
    
    // 更新布局映射
    updateLayoutMapping(state.currentLayout);
    
    // 重新计算渲染状态
    auto* inactiveState = (activeRenderState.load() == renderStateA.get()) ? 
                          renderStateB.get() : renderStateA.get();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
// 状态查询实现
bool StateManager::getSoloState(const juce::String& channelName) const
{
    std::lock_guard<std::mutex> lock(stateMutex);
    auto it = state.soloStates.find(channelName);
    return it != state.soloStates.end() ? it->second : false;
}

bool StateManager::getMuteState(const juce::String& channelName) const
{
    std::lock_guard<std::mutex> lock(stateMutex);
    auto it = state.muteStates.find(channelName);
    return it != state.muteStates.end() ? it->second : false;
}

float StateManager::getMasterGain() const
{
    std::lock_guard<std::mutex> lock(stateMutex);
    return state.masterGainPercent;
}

bool StateManager::isDimActive() const
{
    std::lock_guard<std::mutex> lock(stateMutex);
    return state.dimActive;
}

bool StateManager::isMonoActive() const
{
    std::lock_guard<std::mutex> lock(stateMutex);
    return state.monoActive;
}

//==============================================================================
// Master-Slave通信实现
void StateManager::setPluginRole(PluginRole role)
{
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        state.currentRole = role;
    }
    
    // 更新OSC系统的角色
    // TODO: oscComm->setRole(role);
    
    // 处理角色转换
    switch (role) {
        case PluginRole::Master:
            // 开始广播状态到Slave
            broadcastToSlaves();
            break;
            
        case PluginRole::Slave:
            // 停止OSC发送
            // TODO: oscComm->stopSending();
            break;
            
        case PluginRole::Standalone:
            // 恢复独立运行
            // TODO: oscComm->startSending();
            break;
    }
}

//==============================================================================
void StateManager::receiveMasterState(const juce::String& channelName, const juce::String& action, bool newState)
{
    // 只有Slave才处理Master状态
    if (state.currentRole != PluginRole::Slave) return;
    
    if (action == "solo") {
        setSoloState(channelName, newState);
    }
    else if (action == "mute") {
        setMuteState(channelName, newState);
    }
}

//==============================================================================
void StateManager::broadcastToSlaves()
{
    if (state.currentRole != PluginRole::Master) return;
    
    std::lock_guard<std::mutex> lock(stateMutex);
    
    // 广播所有Solo状态
    for (const auto& [channelName, soloState] : state.soloStates) {
        GlobalPluginState::getInstance().broadcastStateToSlaves(channelName, "solo", soloState);
    }
    
    // 广播所有Mute状态
    for (const auto& [channelName, muteState] : state.muteStates) {
        GlobalPluginState::getInstance().broadcastStateToSlaves(channelName, "mute", muteState);
    }
    
    // 广播Mono状态
    GlobalPluginState::getInstance().broadcastMonoStateToSlaves(state.monoActive);
}

//==============================================================================
// OSC控制实现
void StateManager::handleOSCMessage(const juce::String& address, float value)
{
    // 解析OSC地址
    if (address.startsWith("/Monitor/")) {
        auto parts = juce::StringArray::fromTokens(address, "/", "");
        if (parts.size() >= 4) {
            const auto& channelName = parts[2];
            const auto& action = parts[3];
            
            if (action == "solo") {
                setSoloState(channelName, value > 0.5f);
            }
            else if (action == "mute") {
                setMuteState(channelName, value > 0.5f);
            }
        }
        else if (parts.size() >= 3 && parts[1] == "Master") {
            const auto& control = parts[2];
            
            if (control == "Volume") {
                setMasterGain(value * 100.0f);
            }
            else if (control == "Dim") {
                setDimActive(value > 0.5f);
            }
            else if (control == "LowBoost") {
                setLowBoostActive(value > 0.5f);
            }
            else if (control == "Mute") {
                setMasterMuteActive(value > 0.5f);
            }
            else if (control == "Mono") {
                setMonoActive(value > 0.5f);
            }
        }
    }
}

//==============================================================================
void StateManager::sendOSCUpdate(const juce::String& channelName, const juce::String& action, bool actionState)
{
    if (state.currentRole == PluginRole::Slave) return;  // Slave不发送OSC
    
    if (action == "solo") {
        oscComm->sendSoloState(channelName, actionState);
    }
    else if (action == "mute") {
        oscComm->sendMuteState(channelName, actionState);
    }
}

//==============================================================================
// 实时渲染接口实现
RenderState* StateManager::beginStateUpdate()
{
    // 返回非活跃的缓冲区用于更新
    return (activeRenderState.load() == renderStateA.get()) ? 
            renderStateB.get() : renderStateA.get();
}

void StateManager::commitStateUpdate()
{
    // 原子切换活跃缓冲区
    auto* current = activeRenderState.load();
    auto* next = (current == renderStateA.get()) ? renderStateB.get() : renderStateA.get();
    
    // 增加版本号
    next->version++;
    
    // 原子切换
    activeRenderState.store(next);
}

const RenderState* StateManager::getCurrentRenderState() const
{
    return activeRenderState.load();
}

//==============================================================================
// AudioProcessorValueTreeState::Listener 接口实现
void StateManager::parameterChanged(const juce::String& parameterID, float newValue)
{
    // 处理参数变化
    if (parameterID == "masterGain") {
        setMasterGain(newValue * 100.0f);
    }
    else if (parameterID.startsWith("mute")) {
        // 提取通道索引
        auto channelStr = parameterID.substring(4);
        int channelIndex = channelStr.getIntValue() - 1;
        
        // 获取语义通道名
        const auto& layout = state.currentLayout;
        if (channelIndex >= 0 && channelIndex < layout.channels.size()) {
            const auto& channelName = layout.channels[channelIndex].name;
            setMuteState(channelName, newValue > 0.5f);
        }
    }
    else if (parameterID.startsWith("solo")) {
        // 提取通道索引
        auto channelStr = parameterID.substring(4);
        int channelIndex = channelStr.getIntValue() - 1;
        
        // 获取语义通道名
        const auto& layout = state.currentLayout;
        if (channelIndex >= 0 && channelIndex < layout.channels.size()) {
            const auto& channelName = layout.channels[channelIndex].name;
            setSoloState(channelName, newValue > 0.5f);
        }
    }
    else if (parameterID.startsWith("GAIN_")) {
        // 处理通道增益参数
        auto channelStr = parameterID.substring(5);
        int channelIndex = channelStr.getIntValue() - 1;
        
        // 获取语义通道名
        const auto& layout = state.currentLayout;
        if (channelIndex >= 0 && channelIndex < layout.channels.size()) {
            const auto& channelName = layout.channels[channelIndex].name;
            setChannelGain(channelName, newValue);  // newValue已经是dB值
        }
    }
}

//==============================================================================
// SemanticChannelState::StateChangeListener 接口实现
void StateManager::onSoloStateChanged(const juce::String& channelName, bool soloState)
{
    // 语义状态已更改，更新内部状态并重新计算
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        this->state.soloStates[channelName] = soloState;
    }
    
    // 发送OSC更新
    sendOSCUpdate(channelName, "solo", soloState);
    
    // 如果是Master，广播到Slave
    if (this->state.currentRole == PluginRole::Master) {
        GlobalPluginState::getInstance().broadcastStateToSlaves(channelName, "solo", soloState);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = beginStateUpdate();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

void StateManager::onMuteStateChanged(const juce::String& channelName, bool muteState)
{
    // 语义状态已更改，更新内部状态并重新计算
    {
        std::lock_guard<std::mutex> lock(stateMutex);
        this->state.muteStates[channelName] = muteState;
    }
    
    // 发送OSC更新
    sendOSCUpdate(channelName, "mute", muteState);
    
    // 如果是Master，广播到Slave
    if (this->state.currentRole == PluginRole::Master) {
        GlobalPluginState::getInstance().broadcastStateToSlaves(channelName, "mute", muteState);
    }
    
    // 重新计算渲染状态
    auto* inactiveState = beginStateUpdate();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

void StateManager::onGlobalModeChanged()
{
    // 全局模式改变，重新计算所有状态
    auto* inactiveState = beginStateUpdate();
    recalculateRenderState(inactiveState);
    commitStateUpdate();
}

//==============================================================================
// 内部方法实现
void StateManager::recalculateRenderState(RenderState* targetState)
{
    std::lock_guard<std::mutex> lock(stateMutex);
    
    // 清空目标状态
    for (int i = 0; i < RenderState::MAX_CHANNELS; ++i) {
        targetState->channels[i] = { 1.0f, 1.0f, false, false, {0, 0} };
    }
    
    // 设置Master总线状态
    targetState->master.masterMuteActive = state.masterMuteActive;
    targetState->master.monoEffectActive = state.monoActive;
    targetState->master.monoChannelCount = 0;
    
    // 应用复杂的Solo逻辑
    applyComplexSoloLogic(targetState);
    
    // 计算每个通道的最终增益
    const float masterGainFactor = state.masterGainPercent * 0.01f;
    const float dimFactor = state.dimActive ? 0.16f : 1.0f;
    
    for (const auto& channelInfo : state.currentLayout.channels) {
        const int physicalChannel = channelInfo.channelIndex;
        if (physicalChannel < 0 || physicalChannel >= RenderState::MAX_CHANNELS) continue;
        
        auto& chData = targetState->channels[physicalChannel];
        
        // 计算通道增益
        float channelGain = 1.0f;
        auto gainIt = state.gainStates.find(channelInfo.name);
        if (gainIt != state.gainStates.end()) {
            channelGain = gainIt->second;
        }
        
        // 应用所有增益因子
        chData.targetGain = channelGain * masterGainFactor * dimFactor;
        
        // 设置Mono通道
        if (state.monoActive && !channelInfo.name.contains("SUB") && !channelInfo.name.contains("LFE")) {
            chData.isMonoChannel = true;
            if (targetState->master.monoChannelCount < RenderState::MAX_CHANNELS) {
                targetState->master.monoChannelIndices[targetState->master.monoChannelCount++] = 
                    static_cast<uint8_t>(physicalChannel);
            }
        }
    }
}

//==============================================================================
void StateManager::applyComplexSoloLogic(RenderState* targetState)
{
    // 实现与原版相同的复杂Solo逻辑（来自SemanticChannelState::getFinalMuteState）
    bool hasAnySolo = false;
    bool hasNonSUBSolo = false;
    bool hasSUBSolo = false;
    std::set<juce::String> soloChannels;
    
    // 收集所有Solo的通道并分类
    for (const auto& [channelName, isSolo] : state.soloStates) {
        if (isSolo) {
            hasAnySolo = true;
            soloChannels.insert(channelName);
            
            if (channelName.contains("SUB")) {
                hasSUBSolo = true;
            } else {
                hasNonSUBSolo = true;
            }
        }
    }
    
    if (!hasAnySolo) {
        // 无Solo模式：只应用用户的Mute状态
        for (const auto& channelInfo : state.currentLayout.channels) {
            const int physicalChannel = channelInfo.channelIndex;
            if (physicalChannel < 0 || physicalChannel >= RenderState::MAX_CHANNELS) continue;
            
            auto muteIt = state.muteStates.find(channelInfo.name);
            if (muteIt != state.muteStates.end() && muteIt->second) {
                targetState->channels[physicalChannel].shouldMute = true;
            }
        }
    }
    else {
        // Solo模式激活：应用复杂的Solo逻辑
        for (const auto& channelInfo : state.currentLayout.channels) {
            const int physicalChannel = channelInfo.channelIndex;
            if (physicalChannel < 0 || physicalChannel >= RenderState::MAX_CHANNELS) continue;
            
            const bool isSolo = soloChannels.count(channelInfo.name) > 0;
            const bool isSUB = channelInfo.name.contains("SUB");
            
            if (isSUB) {
                // SUB通道逻辑
                if (hasNonSUBSolo && !hasSUBSolo) {
                    // 场景1：只有非SUB Solo时，SUB通道被强制静音
                    targetState->channels[physicalChannel].shouldMute = true;
                }
                else if (isSolo) {
                    // SUB通道被Solo但可能被手动Mute
                    auto muteIt = state.muteStates.find(channelInfo.name);
                    if (muteIt != state.muteStates.end() && muteIt->second) {
                        targetState->channels[physicalChannel].shouldMute = true;
                    }
                }
                else if (hasSUBSolo) {
                    // 有其他SUB被Solo，这个SUB没有被Solo
                    targetState->channels[physicalChannel].shouldMute = true;
                }
                else {
                    // 混合Solo场景，检查用户Mute
                    auto muteIt = state.muteStates.find(channelInfo.name);
                    if (muteIt != state.muteStates.end() && muteIt->second) {
                        targetState->channels[physicalChannel].shouldMute = true;
                    }
                }
            }
            else {
                // 非SUB通道逻辑
                if (hasSUBSolo && !hasNonSUBSolo) {
                    // 场景2：只有SUB Solo时，非SUB通道强制通过（不静音）
                    targetState->channels[physicalChannel].shouldMute = false;
                }
                else if (isSolo) {
                    // 通道被Solo但可能被手动Mute
                    auto muteIt = state.muteStates.find(channelInfo.name);
                    if (muteIt != state.muteStates.end() && muteIt->second) {
                        targetState->channels[physicalChannel].shouldMute = true;
                    }
                }
                else {
                    // 通道没有被Solo，在Solo模式下应该被静音
                    targetState->channels[physicalChannel].shouldMute = true;
                }
            }
        }
    }
}

//==============================================================================
void StateManager::notifyStateChange(const juce::String& channelName, const juce::String& action, bool state)
{
    // 通知处理器状态变化
    processor.onSemanticStateChanged(channelName, action, state);
}

//==============================================================================
void StateManager::updateLayoutMapping(const Layout& newLayout)
{
    // 更新物理映射器
    processor.getPhysicalMapper().updateMapping(newLayout);
    
    // 清理不存在的通道状态
    std::set<juce::String> validChannels;
    for (const auto& ch : newLayout.channels) {
        validChannels.insert(ch.name);
    }
    
    // 清理Solo状态
    for (auto it = state.soloStates.begin(); it != state.soloStates.end(); ) {
        if (validChannels.count(it->first) == 0) {
            it = state.soloStates.erase(it);
        } else {
            ++it;
        }
    }
    
    // 清理Mute状态
    for (auto it = state.muteStates.begin(); it != state.muteStates.end(); ) {
        if (validChannels.count(it->first) == 0) {
            it = state.muteStates.erase(it);
        } else {
            ++it;
        }
    }
}

//==============================================================================
int StateManager::getPhysicalChannelForSemantic(const juce::String& channelName) const
{
    for (const auto& ch : state.currentLayout.channels) {
        if (ch.name == channelName) {
            return ch.channelIndex;
        }
    }
    return -1;
}

//==============================================================================
void StateManager::syncToValueTreeState()
{
    // 同步状态到AudioProcessorValueTreeState
    // 这里暂时保留接口，后续可能需要实现
}

void StateManager::syncFromValueTreeState()
{
    // 从AudioProcessorValueTreeState同步状态
    // 读取当前的参数值并更新内部状态
    if (auto* param = processor.apvts.getParameter("masterGain")) {
        state.masterGainPercent = param->getValue() * 100.0f;
    }
}