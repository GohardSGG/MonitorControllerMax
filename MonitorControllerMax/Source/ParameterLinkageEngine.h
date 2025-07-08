/*
  ==============================================================================
    ParameterLinkageEngine.h
    Parameter-driven linkage system for Solo/Mute interactions
    
    This engine implements the core linkage logic mimicking the JSFX version,
    ensuring UI and parameter window stay perfectly synchronized.
  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <map>
#include <atomic>

class ParameterLinkageEngine {
public:
    explicit ParameterLinkageEngine(juce::AudioProcessorValueTreeState& apvts);
    ~ParameterLinkageEngine() = default;
    
    // Core linkage logic - mimics JSFX behavior
    void handleParameterChange(const juce::String& paramID, float value);
    
    // State detection methods
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    
    // Batch parameter operations for main buttons
    void clearAllSoloParameters();
    void clearAllMuteParameters();
    void resetToCleanState();
    
    // Memory management
    void saveCurrentMuteMemory();
    void restoreMuteMemory();
    void clearMuteMemory();
    
private:
    juce::AudioProcessorValueTreeState& parameters;
    
    // Linkage computation methods
    void applyAutoMuteForSolo();    // Mimics: slider11 = slider31 ? 0 : 1
    
    // Memory management (mimics JSFX user_mute_xxx variables)
    std::map<int, float> muteMemory;
    
    // State tracking (mimics JSFX Pre_Solo_Active)
    bool previousSoloActive = false;
    
    // Recursion prevention
    std::atomic<bool> isApplyingLinkage{false};
    
    // Helper methods
    juce::String getSoloParameterID(int channelIndex) const;
    juce::String getMuteParameterID(int channelIndex) const;
    float getParameterValue(const juce::String& paramID) const;
    void setParameterValue(const juce::String& paramID, float value);
    
    // RAII helper for recursion prevention
    class ScopedLinkageGuard {
    public:
        explicit ScopedLinkageGuard(std::atomic<bool>& flag);
        ~ScopedLinkageGuard();
    private:
        std::atomic<bool>& guardFlag;
    };
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ParameterLinkageEngine)
};