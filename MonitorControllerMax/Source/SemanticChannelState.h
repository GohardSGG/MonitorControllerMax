#pragma once

#include <JuceHeader.h>
#include <map>
#include <vector>
#include <memory>

// Forward declaration
class MonitorControllerMaxAudioProcessor;

class SemanticChannelState
{
public:
    class StateChangeListener
    {
    public:
        virtual ~StateChangeListener() = default;
        virtual void onSoloStateChanged(const juce::String& channelName, bool state) = 0;
        virtual void onMuteStateChanged(const juce::String& channelName, bool state) = 0;
        virtual void onGlobalModeChanged() = 0;
    };

    SemanticChannelState();
    ~SemanticChannelState();
    
    // 设置processor指针用于角色日志
    void setProcessor(MonitorControllerMaxAudioProcessor* processor);

    // Core state management
    void setSoloState(const juce::String& channelName, bool state);
    void setMuteState(const juce::String& channelName, bool state);
    bool getSoloState(const juce::String& channelName) const;
    bool getMuteState(const juce::String& channelName) const;
    bool getFinalMuteState(const juce::String& channelName) const;

    // Solo mode linkage logic (preserve existing complex logic)
    void calculateSoloModeLinkage();
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    bool isGlobalSoloModeActive() const { return globalSoloModeActive; }
    
    // SUB channel logic (based on original JSFX script)
    bool isSUBChannel(const juce::String& channelName) const;
    bool hasAnyNonSUBSoloActive() const;
    bool hasAnySUBSoloActive() const;

    // Channel management
    void initializeChannel(const juce::String& channelName);
    bool hasChannel(const juce::String& channelName) const;
    void clearAllStates();
    void clearAllSoloStates();
    void clearAllMuteStates();
    std::vector<juce::String> getActiveChannels() const;

    // Memory management for complex logic (preserve existing functionality)
    void saveCurrentMuteMemory();
    void restoreMuteMemory();
    void clearMuteMemory();

    // State change listeners
    void addStateChangeListener(StateChangeListener* listener);
    void removeStateChangeListener(StateChangeListener* listener);

    // Debug and logging
    void logCurrentState() const;
    juce::String getStateDescription() const;

private:
    // Core state storage - replacement for VST3 parameters
    std::map<juce::String, bool> soloStates;
    std::map<juce::String, bool> muteStates;
    std::map<juce::String, bool> muteMemory;  // For complex solo logic
    
    bool globalSoloModeActive = false;
    bool previousGlobalSoloMode = false;
    
    juce::ListenerList<StateChangeListener> stateChangeListeners;
    
    // Processor指针用于角色日志
    MonitorControllerMaxAudioProcessor* processorPtr = nullptr;
    
    // Internal helper methods
    void notifyStateChange(const juce::String& channelName, const juce::String& action, bool state);
    void notifyGlobalModeChange();
    void updateGlobalSoloMode();
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(SemanticChannelState)
};