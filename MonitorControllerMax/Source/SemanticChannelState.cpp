#include "SemanticChannelState.h"
#include "DebugLogger.h"

SemanticChannelState::SemanticChannelState()
{
    VST3_DBG("SemanticChannelState: Initialize semantic state management system");
    globalSoloModeActive = false;
    previousGlobalSoloMode = false;
}

SemanticChannelState::~SemanticChannelState()
{
    VST3_DBG("SemanticChannelState: Destroy semantic state management system");
}

void SemanticChannelState::setSoloState(const juce::String& channelName, bool state)
{
    VST3_DBG("SemanticChannelState: Set Solo state - channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
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
        VST3_DBG("SemanticChannelState: Global Solo mode changed - " + juce::String(globalSoloModeActive ? "ACTIVE" : "OFF"));
        notifyGlobalModeChange();
    }
}

void SemanticChannelState::setMuteState(const juce::String& channelName, bool state)
{
    VST3_DBG("SemanticChannelState: Set Mute state - channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
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
    // Core logic: Solo mode auto-mute linkage
    if (globalSoloModeActive)
    {
        // In solo mode, channels are muted unless they are solo
        bool isSolo = getSoloState(channelName);
        bool finalMute = !isSolo;
        
        if (finalMute != getMuteState(channelName))
        {
            VST3_DBG("SemanticChannelState: Final Mute state calculation - channel: " + channelName + 
                     ", Solo mode: " + (isSolo ? "ON" : "OFF") + 
                     ", Final Mute: " + (finalMute ? "ON" : "OFF"));
        }
        
        return finalMute;
    }
    else
    {
        // In normal mode, use direct mute state
        return getMuteState(channelName);
    }
}

void SemanticChannelState::calculateSoloModeLinkage()
{
    // Preserve existing complex solo logic
    // When solo mode is active, non-solo channels should be auto-muted
    
    // 这个函数现在只做逻辑计算，不发送回调
    // 实际的状态通知由全局模式变化时统一处理
    
    if (globalSoloModeActive)
    {
        VST3_DBG("SemanticChannelState: Calculate Solo mode linkage logic - mode active");
        
        // Solo模式激活时的逻辑计算
        // 实际状态同步由notifyGlobalModeChange()处理
    }
    else
    {
        VST3_DBG("SemanticChannelState: Calculate Solo mode linkage logic - mode inactive");
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
    VST3_DBG("SemanticChannelState: Initialize channel - " + channelName);
    
    // Initialize with default states
    soloStates[channelName] = false;
    muteStates[channelName] = false;
    muteMemory[channelName] = false;
}

void SemanticChannelState::clearAllStates()
{
    VST3_DBG("SemanticChannelState: Clear all states");
    
    soloStates.clear();
    muteStates.clear();
    muteMemory.clear();
    globalSoloModeActive = false;
    previousGlobalSoloMode = false;
}

void SemanticChannelState::clearAllSoloStates()
{
    VST3_DBG("SemanticChannelState: Clear all Solo states");
    
    for (auto& [channelName, soloState] : soloStates)
    {
        soloState = false;
    }
    
    updateGlobalSoloMode();
    calculateSoloModeLinkage();
}

void SemanticChannelState::clearAllMuteStates()
{
    VST3_DBG("SemanticChannelState: Clear all Mute states");
    
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
    VST3_DBG("SemanticChannelState: Save current Mute memory");
    
    // Save current mute states for complex logic
    for (const auto& [channelName, muteState] : muteStates)
    {
        muteMemory[channelName] = muteState;
    }
}

void SemanticChannelState::restoreMuteMemory()
{
    VST3_DBG("SemanticChannelState: Restore Mute memory");
    
    // Restore mute states from memory
    for (const auto& [channelName, memorizedMute] : muteMemory)
    {
        muteStates[channelName] = memorizedMute;
        notifyStateChange(channelName, "mute", memorizedMute);
    }
}

void SemanticChannelState::clearMuteMemory()
{
    VST3_DBG("SemanticChannelState: Clear Mute memory");
    
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
    VST3_DBG("SemanticChannelState: === Current state overview ===");
    VST3_DBG("  Global Solo mode: " + juce::String(globalSoloModeActive ? "ACTIVE" : "OFF"));
    
    VST3_DBG("  Solo states:");
    for (const auto& [channelName, soloState] : soloStates)
    {
        VST3_DBG("    " + channelName + ": " + (soloState ? "ON" : "OFF"));
    }
    
    VST3_DBG("  Mute states:");
    for (const auto& [channelName, muteState] : muteStates)
    {
        VST3_DBG("    " + channelName + ": " + (muteState ? "ON" : "OFF"));
    }
    
    VST3_DBG("  Final Mute states:");
    for (const auto& [channelName, _] : soloStates)
    {
        bool finalMute = getFinalMuteState(channelName);
        VST3_DBG("    " + channelName + ": " + (finalMute ? "MUTED" : "ACTIVE"));
    }
    
    VST3_DBG("=========================");
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
    
    // 当全局Solo模式变化时，广播所有通道的最终Mute状态
    // 这确保外部控制器获得正确的状态同步
    if (globalSoloModeActive)
    {
        VST3_DBG("SemanticChannelState: Global Solo mode activated - broadcasting final mute states");
        
        // 为所有通道发送最终的Mute状态
        for (const auto& [channelName, _] : soloStates)
        {
            bool finalMuteState = getFinalMuteState(channelName);
            VST3_DBG("SemanticChannelState: Broadcasting final mute state - channel: " + channelName + 
                     ", Final Mute: " + (finalMuteState ? "ON" : "OFF"));
            
            // 发送最终的Mute状态
            notifyStateChange(channelName, "mute", finalMuteState);
        }
    }
}

void SemanticChannelState::updateGlobalSoloMode()
{
    previousGlobalSoloMode = globalSoloModeActive;
    globalSoloModeActive = hasAnySoloActive();
}