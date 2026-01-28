#include "SemanticChannelState.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

// SemanticChannelState类专用角色日志宏
#define SEMANTIC_DBG_ROLE(message) \
    do { \
        if (processorPtr) { \
            VST3_DBG_ROLE(processorPtr, message); \
        } else { \
            VST3_DBG("[Semantic] " + juce::String(message)); \
        } \
    } while(0)

SemanticChannelState::SemanticChannelState()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Initialize semantic state management system");
    globalSoloModeActive = false;
    previousGlobalSoloMode = false;
}

void SemanticChannelState::setProcessor(MonitorControllerMaxAudioProcessor* processor)
{
    processorPtr = processor;
}

SemanticChannelState::~SemanticChannelState()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Destroy semantic state management system");
}

void SemanticChannelState::setSoloState(const juce::String& channelName, bool state)
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Set Solo state - channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
    // 写锁保护：UI线程修改状态时防止音频线程读取时崩溃
    juce::ScopedWriteLock lock(stateLock);
    
    soloStates[channelName] = state;
    
    // Update global solo mode
    bool previousGlobalMode = globalSoloModeActive;
    updateGlobalSoloMode();
    
    // Apply solo mode linkage logic (preserve existing complex logic)
    calculateSoloModeLinkage();
    
    // Notify state change
    notifyStateChange(channelName, "solo", state);
    
    // If global mode changed, notify global mode change
    if (previousGlobalMode != globalSoloModeActive)
    {
        SEMANTIC_DBG_ROLE("SemanticChannelState: Global Solo mode changed - " + juce::String(globalSoloModeActive ? "ACTIVE" : "OFF"));
        notifyGlobalModeChange();
    }
}

void SemanticChannelState::setMuteState(const juce::String& channelName, bool state)
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Set Mute state - channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
    // 写锁保护：UI线程修改状态时防止音频线程读取时崩溃
    juce::ScopedWriteLock lock(stateLock);
    
    muteStates[channelName] = state;
    
    // Notify state change
    notifyStateChange(channelName, "mute", state);
}

bool SemanticChannelState::getSoloState(const juce::String& channelName) const
{
    // 读锁保护：音频线程安全读取，避免UI线程修改时的迭代器失效
    juce::ScopedReadLock lock(stateLock);
    
    auto it = soloStates.find(channelName);
    return it != soloStates.end() ? it->second : false;
}

bool SemanticChannelState::getMuteState(const juce::String& channelName) const
{
    // 读锁保护：音频线程安全读取，避免UI线程修改时的迭代器失效
    juce::ScopedReadLock lock(stateLock);
    
    auto it = muteStates.find(channelName);
    return it != muteStates.end() ? it->second : false;
}

bool SemanticChannelState::getFinalMuteState(const juce::String& channelName) const
{
    // 🚀 优化：读锁保护，但避免嵌套锁调用，提升音频线程性能
    juce::ScopedReadLock lock(stateLock);
    
    // SUB channel logic based on original JSFX script
    if (globalSoloModeActive)
    {
        bool isChannelSUB = isSUBChannel(channelName);
        
        // 🚀 性能优化：在已持有锁的情况下直接计算Solo状态，避免嵌套锁
        bool nonSUBSoloActive = false;
        bool subSoloActive = false;
        
        // 单次遍历计算所有Solo状态 - 避免多次调用和嵌套锁
        for (const auto& [chName, soloState] : soloStates)
        {
            if (soloState)
            {
                if (isSUBChannel(chName)) {
                    subSoloActive = true;
                } else {
                    nonSUBSoloActive = true;
                }
                
                // 🚀 早期退出优化：如果两种类型的Solo都找到了，无需继续遍历
                if (nonSUBSoloActive && subSoloActive) break;
            }
        }
        
        // 直接查找当前通道状态 - 已在锁保护下
        auto soloIt = soloStates.find(channelName);
        bool isSolo = (soloIt != soloStates.end()) ? soloIt->second : false;
        
        if (isChannelSUB)
        {
            // SUB channel logic
            if (subSoloActive)
            {
                // When SUB Solo is active, SUB channels follow Solo logic
                bool finalMute = !isSolo;
                // 删除垃圾日志 - 音频处理高频调用
                return finalMute;
            }
            else
            {
                // When only non-SUB Solo is active, SUB channels keep user Mute setting
                // 直接查找 - 已在锁保护下
                auto muteIt = muteStates.find(channelName);
                bool userMute = (muteIt != muteStates.end()) ? muteIt->second : false;
                // 删除垃圾日志 - 音频处理高频调用
                return userMute;
            }
        }
        else
        {
            // Non-SUB channel logic
            if (subSoloActive && !nonSUBSoloActive)
            {
                // When only SUB Solo is active, non-SUB channels are forced through (not muted)
                // 删除垃圾日志 - 音频处理高频调用
                return false;
            }
            else if (nonSUBSoloActive)
            {
                // When non-SUB Solo is active, follow normal Solo logic
                bool finalMute = !isSolo;
                // 删除垃圾日志 - 这会在每次音频处理时调用
                return finalMute;
            }
            else
            {
                // Mixed Solo case, follow normal Solo logic
                bool finalMute = !isSolo;
                return finalMute;
            }
        }
    }
    else
    {
        // Non-Solo mode, use direct Mute state
        // 直接查找 - 已在锁保护下
        auto muteIt = muteStates.find(channelName);
        return (muteIt != muteStates.end()) ? muteIt->second : false;
    }
}

void SemanticChannelState::calculateSoloModeLinkage()
{
    // Preserve existing complex solo logic
    // When solo mode is active, non-solo channels should be auto-muted
    
    // This function now only does logic calculation, no callbacks sent
    // Actual state notification handled by global mode change
    
    if (globalSoloModeActive)
    {
        // 删除垃圾日志 - Solo模式计算高频调用
        
        // Logic calculation when Solo mode is active
        // Actual state sync handled by notifyGlobalModeChange()
    }
    else
    {
        // 删除垃圾日志 - Solo模式计算高频调用
    }
}

bool SemanticChannelState::hasAnySoloActive() const
{
    // 读锁保护：防止遍历时map被修改
    juce::ScopedReadLock lock(stateLock);
    
    for (const auto& [channelName, soloState] : soloStates)
    {
        if (soloState)
        {
            return true;
        }
    }
    return false;
}

// SUB channel logic implementation (based on original JSFX script)
bool SemanticChannelState::isSUBChannel(const juce::String& channelName) const
{
    // SUB channel identification: channel name contains "SUB"
    return channelName.contains("SUB");
}

bool SemanticChannelState::hasAnyNonSUBSoloActive() const
{
    // 读锁保护：防止遍历时map被修改
    juce::ScopedReadLock lock(stateLock);
    
    for (const auto& [channelName, soloState] : soloStates)
    {
        if (soloState && !isSUBChannel(channelName))
        {
            return true;
        }
    }
    return false;
}

bool SemanticChannelState::hasAnySUBSoloActive() const
{
    // 读锁保护：防止遍历时map被修改
    juce::ScopedReadLock lock(stateLock);
    
    for (const auto& [channelName, soloState] : soloStates)
    {
        if (soloState && isSUBChannel(channelName))
        {
            return true;
        }
    }
    return false;
}

bool SemanticChannelState::hasAnyMuteActive() const
{
    // 读锁保护：防止遍历时map被修改
    juce::ScopedReadLock lock(stateLock);
    
    for (const auto& [channelName, muteState] : muteStates)
    {
        if (muteState)
        {
            return true;
        }
    }
    return false;
}

void SemanticChannelState::initializeChannel(const juce::String& channelName)
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Initialize channel - " + channelName);
    
    // 写锁保护：初始化时修改map
    juce::ScopedWriteLock lock(stateLock);
    
    // Initialize with default states
    soloStates[channelName] = false;
    muteStates[channelName] = false;
    muteMemory[channelName] = false;
}

bool SemanticChannelState::hasChannel(const juce::String& channelName) const
{
    // 读锁保护：查找时防止map被修改
    juce::ScopedReadLock lock(stateLock);
    
    return soloStates.find(channelName) != soloStates.end();
}

void SemanticChannelState::clearAllStates()
{
    // 删除垃圾日志 - 状态清理高频调用
    
    // 写锁保护：清除所有状态时修改map
    juce::ScopedWriteLock lock(stateLock);
    
    soloStates.clear();
    muteStates.clear();
    muteMemory.clear();
    globalSoloModeActive = false;
    previousGlobalSoloMode = false;
}

void SemanticChannelState::clearAllSoloStates()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Clear all Solo states");
    
    // 写锁保护：批量修改solo状态
    juce::ScopedWriteLock lock(stateLock);
    
    for (auto& [channelName, soloState] : soloStates)
    {
        if (soloState) {  // 只处理实际改变的状态
            soloState = false;
            // 重要修复：广播状态变化到Master-Slave系统
            notifyStateChange(channelName, "solo", false);
        }
    }
    
    updateGlobalSoloMode();
    calculateSoloModeLinkage();
}

void SemanticChannelState::clearAllMuteStates()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Clear all Mute states");
    
    // 写锁保护：批量修改mute状态
    juce::ScopedWriteLock lock(stateLock);
    
    for (auto& [channelName, muteState] : muteStates)
    {
        if (muteState) {  // 只处理实际改变的状态
            muteState = false;
            // 重要修复：广播状态变化到Master-Slave系统
            notifyStateChange(channelName, "mute", false);
        }
    }
}

std::vector<juce::String> SemanticChannelState::getActiveChannels() const
{
    // 读锁保护：遍历map时防止被修改
    juce::ScopedReadLock lock(stateLock);
    
    std::vector<juce::String> channels;
    
    for (const auto& [channelName, _] : soloStates)
    {
        channels.push_back(channelName);
    }
    
    return channels;
}

void SemanticChannelState::saveCurrentMuteMemory()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Save current Mute memory");
    
    // 写锁保护：修改memory map
    juce::ScopedWriteLock lock(stateLock);
    
    // Save current mute states for complex logic
    for (const auto& [channelName, muteState] : muteStates)
    {
        muteMemory[channelName] = muteState;
    }
}

void SemanticChannelState::restoreMuteMemory()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Restore Mute memory");
    
    // 写锁保护：从memory恢复状态时修改map
    juce::ScopedWriteLock lock(stateLock);
    
    // Restore mute states from memory
    for (const auto& [channelName, memorizedMute] : muteMemory)
    {
        muteStates[channelName] = memorizedMute;
        notifyStateChange(channelName, "mute", memorizedMute);
    }
}

void SemanticChannelState::clearMuteMemory()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Clear Mute memory");
    
    // 写锁保护：清除memory map
    juce::ScopedWriteLock lock(stateLock);
    
    muteMemory.clear();
}

void SemanticChannelState::addStateChangeListener(StateChangeListener* listener)
{
    stateChangeListeners.add(listener);
}

void SemanticChannelState::removeStateChangeListener(StateChangeListener* listener)
{
    stateChangeListeners.remove(listener);
}

void SemanticChannelState::logCurrentState() const
{
    // 读锁保护：日志记录时遍历多个map
    juce::ScopedReadLock lock(stateLock);
    
    // 使用DETAIL级别 - 重复内容会被智能过滤
    VST3_DBG_DETAIL("SemanticChannelState: === Current state overview ===");
    VST3_DBG_DETAIL("  Global Solo mode: " + juce::String(globalSoloModeActive ? "ACTIVE" : "OFF"));
    
    VST3_DBG_DETAIL("  Solo states:");
    for (const auto& [channelName, soloState] : soloStates)
    {
        VST3_DBG_DETAIL("    " + channelName + ": " + (soloState ? "ON" : "OFF"));
    }
    
    VST3_DBG_DETAIL("  Mute states:");
    for (const auto& [channelName, muteState] : muteStates)
    {
        VST3_DBG_DETAIL("    " + channelName + ": " + (muteState ? "ON" : "OFF"));
    }
    
    VST3_DBG_DETAIL("  Final Mute states:");
    for (const auto& [channelName, _] : soloStates)
    {
        // 注意：getFinalMuteState会尝试获取锁，但我们已经持有读锁
        // 这里需要避免递归锁调用，直接计算finalMute
        bool finalMute = false;
        if (globalSoloModeActive)
        {
            bool isChannelSUB = isSUBChannel(channelName);
            auto soloIt = soloStates.find(channelName);
            bool isSolo = (soloIt != soloStates.end()) ? soloIt->second : false;
            
            if (!isChannelSUB) {
                finalMute = !isSolo;
            } else {
                auto muteIt = muteStates.find(channelName);
                finalMute = (muteIt != muteStates.end()) ? muteIt->second : false;
            }
        }
        else
        {
            auto muteIt = muteStates.find(channelName);
            finalMute = (muteIt != muteStates.end()) ? muteIt->second : false;
        }
        
        VST3_DBG_DETAIL("    " + channelName + ": " + (finalMute ? "MUTED" : "ACTIVE"));
    }
    
    VST3_DBG_DETAIL("=========================");
}

juce::String SemanticChannelState::getStateDescription() const
{
    // 读锁保护：访问map大小和调用其他方法
    juce::ScopedReadLock lock(stateLock);
    
    juce::String desc = "Semantic state: ";
    desc += "Solo mode=" + juce::String(globalSoloModeActive ? "ON" : "OFF");
    
    // 直接计算避免递归锁调用
    bool anySoloActive = false;
    bool anyMuteActive = false;
    
    for (const auto& [channelName, soloState] : soloStates)
    {
        if (soloState) anySoloActive = true;
    }
    
    for (const auto& [channelName, muteState] : muteStates)
    {
        if (muteState) anyMuteActive = true;
    }
    
    desc += ", Active Solo=" + juce::String(anySoloActive ? "YES" : "NO");
    desc += ", Active Mute=" + juce::String(anyMuteActive ? "YES" : "NO");
    desc += ", Channels=" + juce::String(soloStates.size());
    
    return desc;
}

void SemanticChannelState::notifyStateChange(const juce::String& channelName, const juce::String& action, bool state)
{
    if (action == "solo")
    {
        stateChangeListeners.call([channelName, state](StateChangeListener& l) { l.onSoloStateChanged(channelName, state); });
    }
    else if (action == "mute")
    {
        stateChangeListeners.call([channelName, state](StateChangeListener& l) { l.onMuteStateChanged(channelName, state); });
    }
}

void SemanticChannelState::notifyGlobalModeChange()
{
    // 注意：此方法假设调用者已经持有写锁（通常从setSoloState调用）
    // 因此不能调用会获取锁的方法，需要直接计算状态避免递归锁
    
    stateChangeListeners.call([](StateChangeListener& l) { l.onGlobalModeChanged(); });
    
    // When global Solo mode changes, ALWAYS broadcast all channels' final Mute states
    // This ensures complete state sync for both Solo ON and Solo OFF scenarios
    SEMANTIC_DBG_ROLE("SemanticChannelState: Global Solo mode changed to " + 
                     juce::String(globalSoloModeActive ? "ACTIVE" : "OFF") + 
                     " - broadcasting all final mute states");
    
    // Send final Mute state for all channels - regardless of Solo mode direction
    // 直接计算final state，避免调用getFinalMuteState()导致的递归锁
    for (const auto& [channelName, _] : soloStates)
    {
        bool finalMuteState = false;
        
        // 直接实现final mute逻辑，避免递归锁
        if (globalSoloModeActive)
        {
            bool isChannelSUB = isSUBChannel(channelName);
            auto soloIt = soloStates.find(channelName);
            bool isSolo = (soloIt != soloStates.end()) ? soloIt->second : false;
            
            if (isChannelSUB)
            {
                // SUB channel logic - 简化版本
                auto muteIt = muteStates.find(channelName);
                finalMuteState = (muteIt != muteStates.end()) ? muteIt->second : false;
            }
            else
            {
                // Non-SUB channel logic
                finalMuteState = !isSolo;
            }
        }
        else
        {
            // Non-Solo mode, use direct Mute state
            auto muteIt = muteStates.find(channelName);
            finalMuteState = (muteIt != muteStates.end()) ? muteIt->second : false;
        }
        
        SEMANTIC_DBG_ROLE("SemanticChannelState: Broadcasting final mute state - channel: " + channelName + 
                 ", Final Mute: " + (finalMuteState ? "ON" : "OFF"));
        
        // Send final Mute state - this will reach both OSC and Master-Slave sync
        notifyStateChange(channelName, "mute", finalMuteState);
    }
}

void SemanticChannelState::updateGlobalSoloMode()
{
    previousGlobalSoloMode = globalSoloModeActive;
    globalSoloModeActive = hasAnySoloActive();
}