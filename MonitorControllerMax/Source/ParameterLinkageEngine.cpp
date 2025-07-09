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
    
    // 检查保护绕过标志
    if (protectionBypass) {
        // 主按钮操作时绕过保护
        VST3_DBG("Parameter protection bypassed for system operation");
        setParameterValue(paramID, value);
        return;
    }
    
    // PARAMETER PROTECTION: Prevent illegal Mute parameter changes in Solo mode
    if (paramID.startsWith("MUTE_") && soloModeProtectionActive) {
        VST3_DBG("Parameter protection: Blocking " << paramID << " change in Solo mode");
        
        // 计算正确的Auto-Mute值并强制恢复
        int channelIndex = paramID.getTrailingIntValue() - 1;
        if (channelIndex >= 0 && channelIndex < 26) {
            juce::String soloParamID = getSoloParameterID(channelIndex);
            float soloValue = getParameterValue(soloParamID);
            float correctMuteValue = (soloValue > 0.5f) ? 0.0f : 1.0f;
            
            if (std::abs(value - correctMuteValue) > 0.1f) {
                VST3_DBG("Parameter protection: Forcing " << paramID << " back to " << correctMuteValue);
                juce::MessageManager::callAsync([this, paramID, correctMuteValue]() {
                    setParameterValue(paramID, correctMuteValue);
                });
            }
        }
        return; // 阻止进一步处理
    }
    
    // CRITICAL FIX: Handle Solo parameter activation at the very beginning
    if (paramID.startsWith("SOLO_") && value > 0.5f && !previousSoloActive) {
        VST3_DBG("First Solo parameter activated - saving memory and clearing scene");
        
        ScopedLinkageGuard guard(isApplyingLinkage);
        
        // 1. 立即保存当前用户的Mute状态（在任何修改前）
        saveCurrentMuteMemory();
        
        // 2. 立即清空所有现有Mute状态，创建干净环境
        clearAllCurrentMuteStates();
        
        // 3. 现在设置Solo参数（因为我们在guard中，不会递归）
        setParameterValue(paramID, value);
        
        // 4. 在干净环境中计算自动Mute
        applyAutoMuteForSolo();
        
        // 5. 更新状态
        previousSoloActive = true;
        
        return; // 早期返回，避免下面的逻辑处理
    }
    
    // Handle Solo parameter deactivation
    if (paramID.startsWith("SOLO_") && value <= 0.5f) {
        // 检查这是否会导致Solo状态变为false
        bool willBeSoloActive = false;
        for (int i = 0; i < 26; ++i) {
            juce::String soloParamID = getSoloParameterID(i);
            if (soloParamID == paramID) {
                continue; // 跳过当前要设置为0的参数
            }
            if (getParameterValue(soloParamID) > 0.5f) {
                willBeSoloActive = true;
                break;
            }
        }
        
        // CRITICAL FIX: 只有在真正的最后一个Solo参数关闭时才恢复记忆
        // 如果仍然有其他Solo参数激活，则只是重新计算Auto-Mute
        if (previousSoloActive && !willBeSoloActive) {
            VST3_DBG("Last Solo parameter deactivated - restoring memory");
            
            ScopedLinkageGuard guard(isApplyingLinkage);
            
            // 1. 先设置Solo参数
            setParameterValue(paramID, value);
            
            // 2. 完整清空所有当前Mute状态
            clearAllCurrentMuteStates();
            
            // 3. 恢复用户原始记忆
            restoreMuteMemory();
            
            // 4. 更新状态
            previousSoloActive = false;
            
            return; // 早期返回
        } else if (previousSoloActive && willBeSoloActive) {
            VST3_DBG("Solo parameter changed but Solo mode continues - recalculating auto-mute");
            
            ScopedLinkageGuard guard(isApplyingLinkage);
            
            // 只是重新计算Auto-Mute，不恢复记忆
            setParameterValue(paramID, value);
            applyAutoMuteForSolo();
            
            return; // 早期返回
        }
    }
    
    // Handle other Solo parameter changes (在Solo模式中的调整)
    if (paramID.startsWith("SOLO_") && previousSoloActive) {
        VST3_DBG("Solo parameter changed in Solo mode - recalculating auto-mute");
        
        ScopedLinkageGuard guard(isApplyingLinkage);
        
        // 1. 设置Solo参数
        setParameterValue(paramID, value);
        
        // 2. 重新计算自动Mute
        applyAutoMuteForSolo();
        
        return; // 早期返回
    }
    
    // 这里的旧保护逻辑已经被前面的新保护逻辑替代
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
    
    // CRITICAL FIX: 禁用保护以允许系统自动计算
    bool wasProtectionBypass = protectionBypass;
    protectionBypass = true;
    
    for (int i = 0; i < 26; ++i) {
        juce::String soloParamID = getSoloParameterID(i);
        juce::String muteParamID = getMuteParameterID(i);
        
        float soloValue = getParameterValue(soloParamID);
        
        // Solo channel = not muted, non-Solo channel = muted
        float newMuteValue = (soloValue > 0.5f) ? 0.0f : 1.0f;
        
        VST3_DBG("Auto-mute channel " << i << ": Solo=" << soloValue << " -> Mute=" << newMuteValue);
        setParameterValue(muteParamID, newMuteValue);
    }
    
    // 恢复原始保护状态
    protectionBypass = wasProtectionBypass;
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
    
    // CRITICAL FIX: 确保在保护禁用情况下恢复记忆
    bool wasProtectionBypass = protectionBypass;
    if (!wasProtectionBypass) {
        protectionBypass = true;  // 临时禁用保护以允许记忆恢复
    }
    
    for (int i = 0; i < 26; ++i) {
        if (muteMemory.find(i) != muteMemory.end()) {
            float restoredValue = muteMemory[i];
            VST3_DBG("Restoring Mute[" << i << "] = " << restoredValue);
            setParameterValue(getMuteParameterID(i), restoredValue);
        }
    }
    
    // 恢复原始保护状态
    if (!wasProtectionBypass) {
        protectionBypass = false;
    }
}

void ParameterLinkageEngine::clearAllCurrentMuteStates() {
    // Clear all current Mute states to create a clean environment
    VST3_DBG("Clearing all current Mute states");
    
    // CRITICAL FIX: 禁用保护以允许系统清理操作
    bool wasProtectionBypass = protectionBypass;
    protectionBypass = true;
    
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getMuteParameterID(i), 0.0f);
        VST3_DBG("Cleared Mute[" << i << "] = 0");
    }
    
    // 恢复原始保护状态
    protectionBypass = wasProtectionBypass;
}

void ParameterLinkageEngine::clearAllSoloParameters() {
    VST3_DBG("Clearing all Solo parameters");
    
    ScopedLinkageGuard guard(isApplyingLinkage);
    
    // CRITICAL FIX: 禁用保护以允许系统清理操作
    bool wasProtectionBypass = protectionBypass;
    protectionBypass = true;
    
    for (int i = 0; i < 26; ++i) {
        setParameterValue(getSoloParameterID(i), 0.0f);
    }
    
    // 恢复原始保护状态
    protectionBypass = wasProtectionBypass;
    
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

void ParameterLinkageEngine::enterSoloSelectionMode() {
    VST3_DBG("enterSoloSelectionMode called - this function is now deprecated");
    VST3_DBG("Memory management is now handled in parameterChanged unified trigger");
    // This function is now deprecated - all memory management is in parameterChanged
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
    // CRITICAL FIX: 使用AudioProcessorValueTreeState的正确方法
    if (auto* param = parameters.getParameter(paramID)) {
        // 方法1：使用APVTS的状态管理方法
        if (auto* rawParam = parameters.getRawParameterValue(paramID)) {
            // 直接设置值并触发监听器
            param->beginChangeGesture();
            param->setValueNotifyingHost(value);
            param->endChangeGesture();
            
            // 强制触发APVTS状态更新
            parameters.state.setProperty(paramID, value, nullptr);
            
            VST3_DBG("Parameter " << paramID << " updated: gesture+APVTS state");
        }
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

// Parameter protection control functions

void ParameterLinkageEngine::setParameterProtectionBypass(bool bypass) {
    protectionBypass = bypass;
    VST3_DBG("Parameter protection bypass: " << (bypass ? "ENABLED" : "DISABLED"));
}

void ParameterLinkageEngine::updateParameterProtection() {
    bool shouldProtect = hasAnySoloActive();
    
    if (shouldProtect && !soloModeProtectionActive) {
        soloModeProtectionActive = true;
        VST3_DBG("Parameter protection ENABLED");
    } else if (!shouldProtect && soloModeProtectionActive) {
        soloModeProtectionActive = false;
        VST3_DBG("Parameter protection DISABLED");
    }
}