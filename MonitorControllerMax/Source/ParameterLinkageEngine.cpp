/*
  ==============================================================================
    ParameterLinkageEngine.cpp
    Parameter-driven linkage system implementation
  ==============================================================================
*/

#include "ParameterLinkageEngine.h"
#include "DebugLogger.h"

ParameterLinkageEngine::ParameterLinkageEngine(juce::AudioProcessorValueTreeState& apvts) 
    : parameters(apvts) {
    VST3_DBG("ParameterLinkageEngine initialized");
    
    // 移除构造函数中的激进重置
    // 这会在setStateInformation中正确处理，避免与REAPER的状态恢复冲突
    VST3_DBG("Parameter linkage engine ready - clean state will be set after state restoration");
}

void ParameterLinkageEngine::handleParameterChange(const juce::String& paramID, float value) {
    if (isApplyingLinkage.load()) {
        return;  // Prevent recursion during linkage application
    }
    
    VST3_DBG("ParameterLinkageEngine handling: " << paramID << " = " << value);
    
    // Check if this is a Solo or Mute parameter
    if (!paramID.startsWith("SOLO_") && !paramID.startsWith("MUTE_")) {
        return;
    }
    
    // Check for Solo state change (mimics JSFX: Current_Solo_Active != Pre_Solo_Active)
    bool currentSoloActive = hasAnySoloActive();
    
    if (currentSoloActive != previousSoloActive) {
        VST3_DBG("Solo state changed: " << (currentSoloActive ? "Active" : "Inactive"));
        
        ScopedLinkageGuard guard(isApplyingLinkage);
        
        if (currentSoloActive) {
            // Entering Solo mode (mimics JSFX entry logic)
            VST3_DBG("Entering Solo mode - saving Mute memory and applying auto-mute");
            saveCurrentMuteMemory();    // user_mute_L = slider11
            applyAutoMuteForSolo();     // slider11 = slider31 ? 0 : 1
        } else {
            // Exiting Solo mode (mimics JSFX exit logic)
            VST3_DBG("Exiting Solo mode - restoring Mute memory");
            restoreMuteMemory();        // slider11 = user_mute_L
        }
        
        previousSoloActive = currentSoloActive;
    }
    
    // CRITICAL FIX: Handle immediate Solo parameter changes
    // If user sets any Solo parameter to 1 via parameter window, immediately apply auto-mute
    if (paramID.startsWith("SOLO_") && value > 0.5f && currentSoloActive) {
        VST3_DBG("Solo parameter activated directly - applying immediate auto-mute");
        
        ScopedLinkageGuard guard(isApplyingLinkage);
        
        // If this is the first Solo being activated, save current Mute memory
        if (!previousSoloActive) {
            saveCurrentMuteMemory();
        }
        
        // Apply auto-mute immediately
        applyAutoMuteForSolo();
        
        // Update the previous state
        previousSoloActive = currentSoloActive;
    }
    
    // PARAMETER PROTECTION: Prevent illegal Mute parameter changes in Solo mode
    if (paramID.startsWith("MUTE_") && currentSoloActive) {
        VST3_DBG("Parameter protection: Attempting to change " << paramID << " in Solo mode");
        
        // Extract channel index from parameter ID (MUTE_1 -> channel 0)
        int channelIndex = paramID.getTrailingIntValue() - 1;
        
        // Determine the correct Auto-Mute value for this channel
        bool isChannelSolo = false;
        if (channelIndex >= 0 && channelIndex < 26) {
            juce::String soloParamID = getSoloParameterID(channelIndex);
            float soloValue = getParameterValue(soloParamID);
            isChannelSolo = (soloValue > 0.5f);
        }
        
        // Calculate correct Mute value: Solo channels = not muted, non-Solo channels = muted
        float correctMuteValue = isChannelSolo ? 0.0f : 1.0f;
        
        // Force restore to correct value if user tried to change it
        if (std::abs(value - correctMuteValue) > 0.1f) {
            VST3_DBG("Parameter protection: Forcing " << paramID << " back to " << correctMuteValue);
            
            // Use a delayed call to avoid recursion
            juce::MessageManager::callAsync([this, paramID, correctMuteValue]() {
                setParameterValue(paramID, correctMuteValue);
            });
        }
        
        // Always return early to prevent any further processing of Mute changes in Solo mode
        return;
    }
}

bool ParameterLinkageEngine::hasAnySoloActive() const {
    // Mimics JSFX: Current_Solo_Active = slider31||slider32||...||slider46
    for (int i = 0; i < 26; ++i) {
        if (getParameterValue(getSoloParameterID(i)) > 0.5f) {
            return true;
        }
    }
    return false;
}

bool ParameterLinkageEngine::hasAnyMuteActive() const {
    for (int i = 0; i < 26; ++i) {
        if (getParameterValue(getMuteParameterID(i)) > 0.5f) {
            return true;
        }
    }
    return false;
}

void ParameterLinkageEngine::applyAutoMuteForSolo() {
    // Mimics JSFX: slider11 = slider31 ? 0 : 1
    VST3_DBG("Applying auto-mute for Solo mode");
    
    for (int i = 0; i < 26; ++i) {
        juce::String soloParamID = getSoloParameterID(i);
        juce::String muteParamID = getMuteParameterID(i);
        
        float soloValue = getParameterValue(soloParamID);
        
        // Solo channel = not muted, non-Solo channel = muted
        float newMuteValue = (soloValue > 0.5f) ? 0.0f : 1.0f;
        
        VST3_DBG("Auto-mute channel " << i << ": Solo=" << soloValue << " -> Mute=" << newMuteValue);
        setParameterValue(muteParamID, newMuteValue);
    }
}

void ParameterLinkageEngine::saveCurrentMuteMemory() {
    // Mimics JSFX: user_mute_L = slider11
    VST3_DBG("Saving current Mute memory");
    
    muteMemory.clear();
    for (int i = 0; i < 26; ++i) {
        float currentMuteValue = getParameterValue(getMuteParameterID(i));
        muteMemory[i] = currentMuteValue;
        VST3_DBG("Saved Mute memory[" << i << "] = " << currentMuteValue);
    }
}

void ParameterLinkageEngine::restoreMuteMemory() {
    // Mimics JSFX: slider11 = user_mute_L
    VST3_DBG("Restoring Mute memory");
    
    for (int i = 0; i < 26; ++i) {
        if (muteMemory.find(i) != muteMemory.end()) {
            float restoredValue = muteMemory[i];
            VST3_DBG("Restoring Mute[" << i << "] = " << restoredValue);
            setParameterValue(getMuteParameterID(i), restoredValue);
        }
    }
}

void ParameterLinkageEngine::clearAllSoloParameters() {
    VST3_DBG("Clearing all Solo parameters");
    
    ScopedLinkageGuard guard(isApplyingLinkage);
    
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getSoloParameterID(i), 0.0f);
    }
    
    // This will trigger Solo state change and automatic Mute memory restoration
}

void ParameterLinkageEngine::clearAllMuteParameters() {
    VST3_DBG("Clearing all Mute parameters");
    
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getMuteParameterID(i), 0.0f);
    }
}

void ParameterLinkageEngine::resetToCleanState() {
    VST3_DBG("Resetting to clean state - clearing all Solo and Mute parameters");
    
    // DO NOT use ScopedLinkageGuard here - we want parameter changes to trigger UI updates
    // Clear all Solo parameters first
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getSoloParameterID(i), 0.0f);
    }
    
    // Clear all Mute parameters
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getMuteParameterID(i), 0.0f);
    }
    
    // Clear memory
    muteMemory.clear();
    previousSoloActive = false;
    
    VST3_DBG("Clean state reset completed - UI should update automatically");
}

void ParameterLinkageEngine::clearMuteMemory() {
    VST3_DBG("Clearing Mute memory");
    muteMemory.clear();
}

// Helper methods

juce::String ParameterLinkageEngine::getSoloParameterID(int channelIndex) const {
    return "SOLO_" + juce::String(channelIndex + 1);
}

juce::String ParameterLinkageEngine::getMuteParameterID(int channelIndex) const {
    return "MUTE_" + juce::String(channelIndex + 1);
}

float ParameterLinkageEngine::getParameterValue(const juce::String& paramID) const {
    if (auto* param = parameters.getParameter(paramID)) {
        return param->getValue();
    }
    return 0.0f;
}

void ParameterLinkageEngine::setParameterValue(const juce::String& paramID, float value) {
    if (auto* param = parameters.getParameter(paramID)) {
        param->setValueNotifyingHost(value);
    }
}

// RAII Guard implementation

ParameterLinkageEngine::ScopedLinkageGuard::ScopedLinkageGuard(std::atomic<bool>& flag) 
    : guardFlag(flag) {
    guardFlag.store(true);
}

ParameterLinkageEngine::ScopedLinkageGuard::~ScopedLinkageGuard() {
    guardFlag.store(false);
}