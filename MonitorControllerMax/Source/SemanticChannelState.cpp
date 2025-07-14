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
    
    muteStates[channelName] = state;
    
    // Notify state change
    notifyStateChange(channelName, "mute", state);
}

bool SemanticChannelState::getSoloState(const juce::String& channelName) const
{
    auto it = soloStates.find(channelName);
    return it != soloStates.end() ? it->second : false;
}

bool SemanticChannelState::getMuteState(const juce::String& channelName) const
{
    auto it = muteStates.find(channelName);
    return it != muteStates.end() ? it->second : false;
}

bool SemanticChannelState::getFinalMuteState(const juce::String& channelName) const
{
    // SUB channel logic based on original JSFX script
    if (globalSoloModeActive)
    {
        bool isChannelSUB = isSUBChannel(channelName);
        bool nonSUBSoloActive = hasAnyNonSUBSoloActive();
        bool subSoloActive = hasAnySUBSoloActive();
        bool isSolo = getSoloState(channelName);
        
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
                bool userMute = getMuteState(channelName);
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
        return getMuteState(channelName);
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
    
    // Initialize with default states
    soloStates[channelName] = false;
    muteStates[channelName] = false;
    muteMemory[channelName] = false;
}

void SemanticChannelState::clearAllStates()
{
    // 删除垃圾日志 - 状态清理高频调用
    
    soloStates.clear();
    muteStates.clear();
    muteMemory.clear();
    globalSoloModeActive = false;
    previousGlobalSoloMode = false;
}

void SemanticChannelState::clearAllSoloStates()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Clear all Solo states");
    
    for (auto& [channelName, soloState] : soloStates)
    {
        soloState = false;
    }
    
    updateGlobalSoloMode();
    calculateSoloModeLinkage();
}

void SemanticChannelState::clearAllMuteStates()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Clear all Mute states");
    
    for (auto& [channelName, muteState] : muteStates)
    {
        muteState = false;
    }
}

std::vector<juce::String> SemanticChannelState::getActiveChannels() const
{
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
    
    // Save current mute states for complex logic
    for (const auto& [channelName, muteState] : muteStates)
    {
        muteMemory[channelName] = muteState;
    }
}

void SemanticChannelState::restoreMuteMemory()
{
    SEMANTIC_DBG_ROLE("SemanticChannelState: Restore Mute memory");
    
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
        bool finalMute = getFinalMuteState(channelName);
        VST3_DBG_DETAIL("    " + channelName + ": " + (finalMute ? "MUTED" : "ACTIVE"));
    }
    
    VST3_DBG_DETAIL("=========================");
}

juce::String SemanticChannelState::getStateDescription() const
{
    juce::String desc = "Semantic state: ";
    desc += "Solo mode=" + juce::String(globalSoloModeActive ? "ON" : "OFF");
    desc += ", Active Solo=" + juce::String(hasAnySoloActive() ? "YES" : "NO");
    desc += ", Active Mute=" + juce::String(hasAnyMuteActive() ? "YES" : "NO");
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
    stateChangeListeners.call([](StateChangeListener& l) { l.onGlobalModeChanged(); });
    
    // When global Solo mode changes, broadcast all channels' final Mute states
    // This ensures external controllers get correct state sync
    if (globalSoloModeActive)
    {
        SEMANTIC_DBG_ROLE("SemanticChannelState: Global Solo mode activated - broadcasting final mute states");
        
        // Send final Mute state for all channels
        for (const auto& [channelName, _] : soloStates)
        {
            bool finalMuteState = getFinalMuteState(channelName);
            SEMANTIC_DBG_ROLE("SemanticChannelState: Broadcasting final mute state - channel: " + channelName + 
                     ", Final Mute: " + (finalMuteState ? "ON" : "OFF"));
            
            // Send final Mute state
            notifyStateChange(channelName, "mute", finalMuteState);
        }
    }
}

void SemanticChannelState::updateGlobalSoloMode()
{
    previousGlobalSoloMode = globalSoloModeActive;
    globalSoloModeActive = hasAnySoloActive();
}