#include "StateManager.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

//==============================================================================
StateManager::StateManager(MonitorControllerMaxAudioProcessor& proc)
    : processor(proc)
{
    // 创建双缓冲的RenderState实例
    renderStateA = std::make_unique<RenderState>();
    renderStateB = std::make_unique<RenderState>();
    
    // 初始设置A为活跃状态，B为写入状态
    activeRenderState.store(renderStateA.get());
    inactiveRenderState = renderStateB.get();
    
    VST3_DBG("StateManager: Created with double-buffered render states");
}

StateManager::~StateManager()
{
    shutdown();
    VST3_DBG("StateManager: Destroyed");
}

//==============================================================================
void StateManager::initialize()
{
    if (initialized) return;
    
    // 注册为SemanticChannelState监听器
    processor.getSemanticState().addStateChangeListener(this);
    
    // 注册为参数监听器（只监听真正存在的参数）
    processor.apvts.addParameterListener("MASTER_GAIN", this);
    
    // 监听所有通道增益参数（GAIN_1 到 GAIN_26）
    for (int i = 1; i <= 26; ++i) {
        const juce::String paramID = "GAIN_" + juce::String(i);
        processor.apvts.addParameterListener(paramID, this);
    }
    
    // 执行初始状态收集
    updateRenderState();
    
    initialized = true;
    VST3_DBG("StateManager: Initialized with parameter and state listeners");
}

void StateManager::shutdown()
{
    if (!initialized) return;
    
    // 移除所有监听器
    processor.getSemanticState().removeStateChangeListener(this);
    
    processor.apvts.removeParameterListener("MASTER_GAIN", this);
    for (int i = 1; i <= 26; ++i) {
        const juce::String paramID = "GAIN_" + juce::String(i);
        processor.apvts.removeParameterListener(paramID, this);
    }
    
    initialized = false;
    VST3_DBG("StateManager: Shutdown complete");
}

//==============================================================================
const RenderState* StateManager::getCurrentRenderState() const noexcept
{
    // 音频线程安全：单次原子读取
    return activeRenderState.load(std::memory_order_acquire);
}

//==============================================================================
// SemanticChannelState::StateChangeListener 接口实现
void StateManager::onSoloStateChanged(const juce::String& channelName, bool state)
{
    VST3_DBG("StateManager: Solo state changed - " + channelName + " = " + (state ? "ON" : "OFF"));
    updateRenderState();
}

void StateManager::onMuteStateChanged(const juce::String& channelName, bool state)
{
    VST3_DBG("StateManager: Mute state changed - " + channelName + " = " + (state ? "ON" : "OFF"));
    updateRenderState();
}

void StateManager::onGlobalModeChanged()
{
    VST3_DBG("StateManager: Global mode changed");
    updateRenderState();
}

//==============================================================================
// AudioProcessorValueTreeState::Listener 接口实现
void StateManager::parameterChanged(const juce::String& parameterID, float newValue)
{
    VST3_DBG("StateManager: Parameter changed - " + parameterID + " = " + juce::String(newValue));
    updateRenderState();
}

//==============================================================================
void StateManager::onLayoutChanged()
{
    VST3_DBG("StateManager: Layout changed");
    updateRenderState();
}

//==============================================================================
// 核心状态更新方法
void StateManager::updateRenderState()
{
    if (!initialized) return;
    
    // 收集当前状态到非活跃缓冲区
    collectCurrentState(inactiveRenderState);
    
    // 原子切换缓冲区
    commitRenderState();
}

void StateManager::collectCurrentState(RenderState* targetState)
{
    // 清空目标状态 (手动初始化所有字段)
    for (int i = 0; i < RenderState::MAX_CHANNELS; ++i) {
        targetState->channelShouldMute[i] = false;
        targetState->channelFinalGain[i] = 1.0f;
        targetState->channelIsActive[i] = false;
        targetState->channelIsSUB[i] = false;
        targetState->monoChannelIndices[i] = 0;
    }
    targetState->monoActive = false;
    targetState->monoChannelCount = 0;
    
    // 收集各组件状态（直接调用现有逻辑，零计算）
    collectChannelStates(targetState);
    collectMasterBusStates(targetState);
    collectMonoChannelData(targetState);
    
    // 更新版本号
    targetState->version.store(targetState->version.load() + 1, std::memory_order_release);
}

void StateManager::collectChannelStates(RenderState* target)
{
    const auto& currentLayout = processor.getCurrentLayout();
    
    // 遍历当前布局中的所有通道
    for (const auto& channelInfo : currentLayout.channels) {
        const int physicalIndex = channelInfo.channelIndex;
        if (physicalIndex < 0 || physicalIndex >= RenderState::MAX_CHANNELS) continue;
        
        const juce::String& channelName = channelInfo.name;
        
        // 直接调用SemanticChannelState的最终结果（保持所有SUB逻辑）
        target->channelShouldMute[physicalIndex] = 
            processor.getSemanticState().getFinalMuteState(channelName);
        
        // 获取通道个人增益（来自VST3参数）
        const juce::String gainParamID = "GAIN_" + juce::String(physicalIndex + 1);
        const float gainDb = processor.apvts.getRawParameterValue(gainParamID)->load();
        target->channelFinalGain[physicalIndex] = juce::Decibels::decibelsToGain(gainDb);
        
        // 标记通道激活
        target->channelIsActive[physicalIndex] = true;
        
        // 标记SUB通道（用于LowBoost处理）
        target->channelIsSUB[physicalIndex] = processor.getSemanticState().isSUBChannel(channelName);
    }
}

void StateManager::collectMasterBusStates(RenderState* target)
{
    const auto& masterBus = processor.masterBusProcessor;
    
    // 只收集Mono状态用于预计算参与通道（其他Master效果由MasterBusProcessor直接处理）
    target->monoActive = masterBus.isMonoActive();
}

void StateManager::collectMonoChannelData(RenderState* target)
{
    if (!target->monoActive) {
        target->monoChannelCount = 0;
        return;
    }
    
    const auto& currentLayout = processor.getCurrentLayout();
    uint8_t monoCount = 0;
    
    // 收集参与Mono效果的通道（通常是所有主声道，不包括SUB）
    for (const auto& channelInfo : currentLayout.channels) {
        const int physicalIndex = channelInfo.channelIndex;
        if (physicalIndex < 0 || physicalIndex >= RenderState::MAX_CHANNELS) continue;
        
        const juce::String& channelName = channelInfo.name;
        
        // 排除SUB通道参与Mono混合
        if (!processor.getSemanticState().isSUBChannel(channelName)) {
            if (monoCount < RenderState::MAX_CHANNELS) {
                target->monoChannelIndices[monoCount] = static_cast<uint8_t>(physicalIndex);
                monoCount++;
            }
        }
    }
    
    target->monoChannelCount = monoCount;
}

void StateManager::commitRenderState()
{
    // 原子切换活跃和非活跃缓冲区
    RenderState* oldActive = activeRenderState.exchange(inactiveRenderState, std::memory_order_acq_rel);
    inactiveRenderState = oldActive;
    
    VST3_DBG("StateManager: Render state committed - version " + 
             juce::String(activeRenderState.load()->version.load()));
}

//==============================================================================
// 🚀 彻底修复：StateManager统一UI控制实现
// 遵循原始设计意图和JUCE规范
//==============================================================================

void StateManager::handleSoloButtonClick()
{
    VST3_DBG("StateManager: Solo button clicked - unified control");
    
    // 确保在消息线程中执行（JUCE规范）
    jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
    
    if (!initialized) {
        VST3_DBG("StateManager: Not initialized, ignoring Solo button click");
        return;
    }
    
    try {
        auto& semanticState = getSemanticState();
        
        if (semanticState.hasAnySoloActive()) {
            // 状态1：有Solo状态激活 - 清除所有Solo状态并恢复Mute记忆
            VST3_DBG("StateManager: Clearing all Solo states and restoring Mute memory");
            
            // 清除选择模式
            soloSelectionMode.store(false);
            muteSelectionMode.store(false);
            
            // 清除所有Solo状态
            semanticState.clearAllSoloStates();
            
            // 恢复之前保存的Mute记忆状态
            semanticState.restoreMuteMemory();
            
            // 同步processor状态
            updateProcessorPendingStates();
            
        } else if (soloSelectionMode.load()) {
            // 状态2：无Solo状态，但在Solo选择模式 - 退出选择模式并恢复记忆
            VST3_DBG("StateManager: Exiting Solo selection mode and restoring Mute memory");
            
            // 恢复之前保存的Mute记忆状态
            semanticState.restoreMuteMemory();
            
            soloSelectionMode.store(false);
            muteSelectionMode.store(false);
            
            // 同步processor状态
            updateProcessorPendingStates();
            
        } else {
            // 状态3：初始状态 - 进入Solo选择模式
            VST3_DBG("StateManager: Entering Solo selection mode - saving Mute memory and clearing current Mute states");
            
            // 保存当前Mute记忆并清空现场，让UI显示干净状态
            semanticState.saveCurrentMuteMemory();
            semanticState.clearAllMuteStates();
            
            soloSelectionMode.store(true);
            muteSelectionMode.store(false);  // 切换到Solo选择模式会取消Mute选择模式
            
            // 同步processor状态
            updateProcessorPendingStates();
        }
        
        // 触发状态更新到音频线程
        triggerStateUpdate();
        
    } catch (const std::exception& e) {
        VST3_DBG("StateManager: Exception in handleSoloButtonClick: " + juce::String(e.what()));
    } catch (...) {
        VST3_DBG("StateManager: Unknown exception in handleSoloButtonClick");
    }
}

void StateManager::handleMuteButtonClick()
{
    VST3_DBG("StateManager: Mute button clicked - unified control");
    
    // 确保在消息线程中执行（JUCE规范）
    jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
    
    if (!initialized) {
        VST3_DBG("StateManager: Not initialized, ignoring Mute button click");
        return;
    }
    
    try {
        auto& semanticState = getSemanticState();
        
        // Solo Priority Rule: If any Solo state is active, Mute button is disabled
        if (semanticState.hasAnySoloActive()) {
            VST3_DBG("StateManager: Mute button ignored - Solo priority rule active");
            return;
        }
        
        if (semanticState.hasAnyMuteActive()) {
            // 状态1：有Mute状态激活 - 清除所有Mute状态
            VST3_DBG("StateManager: Clearing all Mute states");
            soloSelectionMode.store(false);
            muteSelectionMode.store(false);
            
            semanticState.clearAllMuteStates();
            
            // 同步processor状态
            updateProcessorPendingStates();
            
        } else if (muteSelectionMode.load()) {
            // 状态2：无Mute状态，但在Mute选择模式 - 退出选择模式
            VST3_DBG("StateManager: Exiting Mute selection mode");
            muteSelectionMode.store(false);
            soloSelectionMode.store(false);
            
            // 同步processor状态
            updateProcessorPendingStates();
            
        } else {
            // 状态3：初始状态 - 进入Mute选择模式
            VST3_DBG("StateManager: Entering Mute selection mode");
            
            muteSelectionMode.store(true);
            soloSelectionMode.store(false);  // 切换到Mute选择模式会取消Solo选择模式
            
            // 同步processor状态
            updateProcessorPendingStates();
        }
        
        // 触发状态更新到音频线程
        triggerStateUpdate();
        
    } catch (const std::exception& e) {
        VST3_DBG("StateManager: Exception in handleMuteButtonClick: " + juce::String(e.what()));
    } catch (...) {
        VST3_DBG("StateManager: Unknown exception in handleMuteButtonClick");
    }
}

void StateManager::handleChannelSoloClick(const juce::String& channelName, bool newState)
{
    VST3_DBG("StateManager: Channel Solo click - " + channelName + ", state: " + (newState ? "ON" : "OFF"));
    
    // 确保在消息线程中执行（JUCE规范）
    jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
    
    if (!initialized || !soloSelectionMode.load()) {
        VST3_DBG("StateManager: Not in Solo selection mode, ignoring channel Solo click");
        return;
    }
    
    try {
        auto& semanticState = getSemanticState();
        
        // 委托给SemanticChannelState处理业务逻辑
        semanticState.setSoloState(channelName, newState);
        
        // 触发状态更新到音频线程
        triggerStateUpdate();
        
    } catch (const std::exception& e) {
        VST3_DBG("StateManager: Exception in handleChannelSoloClick: " + juce::String(e.what()));
    } catch (...) {
        VST3_DBG("StateManager: Unknown exception in handleChannelSoloClick");
    }
}

void StateManager::handleChannelMuteClick(const juce::String& channelName, bool newState)
{
    VST3_DBG("StateManager: Channel Mute click - " + channelName + ", state: " + (newState ? "ON" : "OFF"));
    
    // 确保在消息线程中执行（JUCE规范）
    jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
    
    if (!initialized || !muteSelectionMode.load()) {
        VST3_DBG("StateManager: Not in Mute selection mode, ignoring channel Mute click");
        return;
    }
    
    try {
        auto& semanticState = getSemanticState();
        
        // Solo Priority Rule检查
        if (semanticState.hasAnySoloActive()) {
            VST3_DBG("StateManager: Channel Mute ignored - Solo priority rule active");
            return;
        }
        
        // 委托给SemanticChannelState处理业务逻辑
        semanticState.setMuteState(channelName, newState);
        
        // 触发状态更新到音频线程
        triggerStateUpdate();
        
    } catch (const std::exception& e) {
        VST3_DBG("StateManager: Exception in handleChannelMuteClick: " + juce::String(e.what()));
    } catch (...) {
        VST3_DBG("StateManager: Unknown exception in handleChannelMuteClick");
    }
}

//==============================================================================
// 状态查询接口（线程安全）
//==============================================================================

bool StateManager::isInSoloSelectionMode() const noexcept
{
    return soloSelectionMode.load();
}

bool StateManager::isInMuteSelectionMode() const noexcept
{
    return muteSelectionMode.load();
}

bool StateManager::hasAnySoloActive() const noexcept
{
    if (!initialized) return false;
    
    try {
        // 🛡️ const安全修复：使用const版本的访问器
        return processor.getSemanticState().hasAnySoloActive();
    } catch (...) {
        return false;
    }
}

bool StateManager::hasAnyMuteActive() const noexcept
{
    if (!initialized) return false;
    
    try {
        // 🛡️ const安全修复：使用const版本的访问器
        return processor.getSemanticState().hasAnyMuteActive();
    } catch (...) {
        return false;
    }
}

//==============================================================================
// 业务逻辑委托方法（保持职责分离）
//==============================================================================

SemanticChannelState& StateManager::getSemanticState()
{
    return processor.getSemanticState();
}

const SemanticChannelState& StateManager::getSemanticState() const
{
    return processor.getSemanticState();
}

void StateManager::triggerStateUpdate()
{
    // 🚀 使用正确的内存顺序：增加状态版本号，使UI缓存失效
    currentStateVersion.fetch_add(1, std::memory_order_acq_rel);
    
    // 触发render state更新，将最新状态传递到音频线程
    updateRenderState();
    
    // 通知processor更新所有状态
    processor.updateAllStates();
}

void StateManager::updateProcessorPendingStates()
{
    // REMOVED: PluginProcessor中的pending状态变量已删除
    // StateManager现在是选择模式状态的唯一权威，不再需要同步到processor
    // processor通过StateManager的查询接口访问状态
}

void StateManager::onExternalStateChange(const juce::String& channelName, const juce::String& action, bool state)
{
    // 🚀 简化修复：处理OSC等外部控制对StateManager选择模式的影响
    jassert(juce::MessageManager::getInstance()->isThisTheMessageThread());
    
    VST3_DBG("StateManager handling external state change: " + action + " " + channelName + " = " + (state ? "ON" : "OFF"));
    
    // 简化逻辑：根据当前语义状态更新选择模式
    auto& semanticState = getSemanticState();
    
    if (action == "solo") {
        // Solo状态变化：如果有任何Solo激活，进入Solo选择模式；否则退出
        if (semanticState.hasAnySoloActive()) {
            if (!soloSelectionMode.load()) {
                VST3_DBG("External Solo change - entering Solo selection mode");
                soloSelectionMode.store(true);
                muteSelectionMode.store(false);
            }
        } else {
            if (soloSelectionMode.load()) {
                VST3_DBG("No Solo states remaining - exiting Solo selection mode");
                soloSelectionMode.store(false);
                // 恢复Mute记忆
                semanticState.restoreMuteMemory();
            }
        }
    }
    else if (action == "mute") {
        // Mute状态变化：Solo优先级规则
        if (!semanticState.hasAnySoloActive()) {
            if (semanticState.hasAnyMuteActive()) {
                if (!muteSelectionMode.load()) {
                    VST3_DBG("External Mute change - entering Mute selection mode");
                    muteSelectionMode.store(true);
                    soloSelectionMode.store(false);
                }
            } else {
                if (muteSelectionMode.load()) {
                    VST3_DBG("No Mute states remaining - exiting Mute selection mode");
                    muteSelectionMode.store(false);
                }
            }
        }
    }
    
    // 通知processor更新所有状态，确保UI能获取到最新状态
    triggerStateUpdate();
}

//==============================================================================
// UI状态查询接口实现（线程安全）
//==============================================================================

bool StateManager::getChannelSoloStateForUI(const juce::String& channelName) const
{
    if (!initialized) return false;
    
    try {
        const uint64_t currentVersion = currentStateVersion.load(std::memory_order_acquire);
        
        // 🚀 修复TOCTOU竞态：使用双重检查锁定
        {
            juce::ScopedReadLock readLock(uiStateCacheLock);
            if (uiStateCache.cacheVersion == currentVersion) {
                // 缓存有效，直接返回
                auto it = uiStateCache.soloStates.find(channelName);
                if (it != uiStateCache.soloStates.end()) {
                    return it->second;
                }
                // 通道不存在于缓存中，返回false
                return false;
            }
        }
        
        // 需要更新缓存：使用写锁确保原子性
        {
            juce::ScopedWriteLock writeLock(uiStateCacheLock);
            // 再次检查版本，防止在获取写锁期间其他线程已更新
            if (uiStateCache.cacheVersion != currentVersion) {
                updateUIStateCacheUnsafe(currentVersion);
            }
            
            auto it = uiStateCache.soloStates.find(channelName);
            return it != uiStateCache.soloStates.end() ? it->second : false;
        }
        
    } catch (...) {
        return false;
    }
}

bool StateManager::getChannelMuteStateForUI(const juce::String& channelName) const
{
    if (!initialized) return false;
    
    try {
        const uint64_t currentVersion = currentStateVersion.load(std::memory_order_acquire);
        
        // 🚀 修复TOCTOU竞态：使用双重检查锁定
        {
            juce::ScopedReadLock readLock(uiStateCacheLock);
            if (uiStateCache.cacheVersion == currentVersion) {
                auto it = uiStateCache.muteStates.find(channelName);
                if (it != uiStateCache.muteStates.end()) {
                    return it->second;
                }
                return false;
            }
        }
        
        // 需要更新缓存：使用写锁确保原子性
        {
            juce::ScopedWriteLock writeLock(uiStateCacheLock);
            if (uiStateCache.cacheVersion != currentVersion) {
                updateUIStateCacheUnsafe(currentVersion);
            }
            
            auto it = uiStateCache.muteStates.find(channelName);
            return it != uiStateCache.muteStates.end() ? it->second : false;
        }
        
    } catch (...) {
        return false;
    }
}

bool StateManager::getChannelFinalMuteStateForUI(const juce::String& channelName) const
{
    if (!initialized) return false;
    
    try {
        const uint64_t currentVersion = currentStateVersion.load();
        
        {
            juce::ScopedReadLock lock(uiStateCacheLock);
            if (uiStateCache.cacheVersion == currentVersion) {
                auto it = uiStateCache.finalMuteStates.find(channelName);
                if (it != uiStateCache.finalMuteStates.end()) {
                    return it->second;
                }
            }
        }
        
        updateUIStateCache();
        
        juce::ScopedReadLock lock(uiStateCacheLock);
        auto it = uiStateCache.finalMuteStates.find(channelName);
        return it != uiStateCache.finalMuteStates.end() ? it->second : false;
        
    } catch (...) {
        return false;
    }
}

juce::Colour StateManager::getChannelButtonColor(const juce::String& channelName, 
                                                 const juce::LookAndFeel& lookAndFeel) const
{
    // 🚀 修复Auto-Mute显示：使用最终状态而非手动状态
    bool soloState = getChannelSoloStateForUI(channelName);
    bool finalMuteState = getChannelFinalMuteStateForUI(channelName);  // 使用最终Mute状态（包含Auto-Mute）
    
    // 根据状态返回对应颜色
    if (soloState) {
        // Solo激活 - 绿色
        return juce::Colour(0xff2a8c4a);  // 与CustomLookAndFeel中的soloColour一致
    } else if (finalMuteState) {
        // 最终Mute激活（包含Auto-Mute）- 红色
        return juce::Colour(0xffd13a3a);  // 与CustomLookAndFeel中的muteColour一致
    } else {
        // 正常状态 - 默认灰色
        return lookAndFeel.findColour(juce::TextButton::buttonColourId);
    }
}

void StateManager::updateUIStateCache() const
{
    juce::ScopedWriteLock lock(uiStateCacheLock);
    const uint64_t currentVersion = currentStateVersion.load(std::memory_order_acquire);
    updateUIStateCacheUnsafe(currentVersion);
}

void StateManager::updateUIStateCacheUnsafe(uint64_t targetVersion) const
{
    // 🚀 线程不安全版本，调用者必须持有写锁
    
    // 检查是否有待更新的通道（增量更新）
    if (!pendingChannelUpdates.empty()) {
        // 🚀 增量更新：只更新变化的通道
        const auto& semanticState = processor.getSemanticState();
        
        for (const auto& channelName : pendingChannelUpdates) {
            uiStateCache.soloStates[channelName] = semanticState.getSoloState(channelName);
            uiStateCache.muteStates[channelName] = semanticState.getMuteState(channelName);
            uiStateCache.finalMuteStates[channelName] = semanticState.getFinalMuteState(channelName);
        }
        
        // 清空待更新列表
        pendingChannelUpdates.clear();
        
        VST3_DBG("StateManager: UI cache incrementally updated " << pendingChannelUpdates.size() << " channels - version: " + juce::String(targetVersion));
    } else {
        // 🚀 全量更新：首次初始化或重大状态变化
        // 清空旧缓存
        uiStateCache.soloStates.clear();
        uiStateCache.muteStates.clear();
        uiStateCache.finalMuteStates.clear();
        
        // 从SemanticChannelState获取最新状态
        const auto& semanticState = processor.getSemanticState();
        const auto& physicalMapper = processor.getPhysicalMapper();
        
        // 获取所有活动的语义通道
        auto activeChannels = physicalMapper.getActiveSemanticChannels();
        
        // 缓存每个通道的状态
        for (const auto& channelName : activeChannels) {
            uiStateCache.soloStates[channelName] = semanticState.getSoloState(channelName);
            uiStateCache.muteStates[channelName] = semanticState.getMuteState(channelName);
            uiStateCache.finalMuteStates[channelName] = semanticState.getFinalMuteState(channelName);
        }
        
        VST3_DBG("StateManager: UI cache fully updated - version: " + juce::String(targetVersion));
    }
    
    // 原子更新缓存版本
    uiStateCache.cacheVersion = targetVersion;
}

// 🚀 新增：单通道增量更新机制
void StateManager::updateSingleChannelCache(const juce::String& channelName) const
{
    juce::ScopedWriteLock lock(uiStateCacheLock);
    
    // 添加到待更新列表
    pendingChannelUpdates.insert(channelName);
    
    // 立即更新该通道的状态
    const auto& semanticState = processor.getSemanticState();
    uiStateCache.soloStates[channelName] = semanticState.getSoloState(channelName);
    uiStateCache.muteStates[channelName] = semanticState.getMuteState(channelName);
    uiStateCache.finalMuteStates[channelName] = semanticState.getFinalMuteState(channelName);
    
    // 更新缓存版本
    const uint64_t currentVersion = currentStateVersion.fetch_add(1, std::memory_order_acq_rel);
    uiStateCache.cacheVersion = currentVersion;
    
    VST3_DBG("StateManager: Single channel '" + channelName + "' cache updated - version: " + juce::String(currentVersion));
}