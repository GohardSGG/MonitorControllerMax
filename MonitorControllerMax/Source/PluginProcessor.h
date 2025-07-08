/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin processor.

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <atomic>
#include <array>
#include <functional>
#include <map>
#include <set>
#include "ConfigManager.h"
#include "StateManager.h"

class InterPluginCommunicator;

//==============================================================================

class MonitorControllerMaxAudioProcessor  : public juce::AudioProcessor,
                                          public juce::AudioProcessorValueTreeState::Listener
{
public:
    //==============================================================================
    // A constant for the number of channels we'll manage.
    static constexpr int numManagedChannels = 16;
    
    // This struct will be used for state synchronization between instances.
    struct MuteSoloState
    {
        std::array<bool, 26> mutes;
        std::array<bool, 26> solos;
    };

    //==============================================================================
    MonitorControllerMaxAudioProcessor();
    ~MonitorControllerMaxAudioProcessor() override;

    //==============================================================================
    enum Role
    {
        standalone,
        master,
        slave
    };

    void setRole(Role newRole);
    Role getRole() const;

    void setRemoteMuteSoloState(const MuteSoloState& state);
    bool getRemoteMuteState(int channel) const;
    bool getRemoteSoloState(int channel) const;

    void setCurrentLayout(const juce::String& speaker, const juce::String& sub);
    const Layout& getCurrentLayout() const;
    int getAvailableChannels() const;
    
    // Automatically select the most suitable layout configuration based on channel count
    void autoSelectLayoutForChannelCount(int channelCount);
    
    // UI update callback function type
    std::function<void(const juce::String&, const juce::String&)> onLayoutAutoChanged;
    
    // Set UI update callback
    void setLayoutChangeCallback(std::function<void(const juce::String&, const juce::String&)> callback);
    
    // 新的强大状态机接口
    StateManager& getStateManager();
    void initializeStateManager();
    
    // 状态机回调接口
    void onParameterUpdate(int channelIndex, float value);
    void onUIUpdate();

    //==============================================================================
    void prepareToPlay (double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;

   #ifndef JucePlugin_PreferredChannelConfigurations
    bool isBusesLayoutSupported (const BusesLayout& layouts) const override;
   #endif

    void processBlock (juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    //==============================================================================
    void parameterChanged (const juce::String& parameterID, float newValue) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override;

    //==============================================================================
    const juce::String getName() const override;
    
    // Dynamic I/O channel name functions - provide meaningful channel names based on current layout
    const juce::String getInputChannelName(int channelIndex) const override;
    const juce::String getOutputChannelName(int channelIndex) const override;

    bool acceptsMidi() const override;
    bool producesMidi() const override;
    bool isMidiEffect() const override;
    double getTailLengthSeconds() const override;

    //==============================================================================
    int getNumPrograms() override;
    int getCurrentProgram() override;
    void setCurrentProgram (int index) override;
    const juce::String getProgramName (int index) override;
    void changeProgramName (int index, const juce::String& newName) override;

    //==============================================================================
    void getStateInformation (juce::MemoryBlock& destData) override;
    void setStateInformation (const void* data, int sizeInBytes) override;
    
    juce::AudioProcessorValueTreeState apvts;
    ConfigManager configManager;
    Layout currentLayout;

private:
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout();
    //==============================================================================
    std::atomic<Role> currentRole;
    std::unique_ptr<InterPluginCommunicator> communicator;

    // Atomics to hold the state received from a master instance when in slave mode.
    std::array<std::atomic<bool>, 26> remoteMutes{};
    std::array<std::atomic<bool>, 26> remoteSolos{};

    // We'll need atomic pointers to our parameters for thread-safe access in the audio callback.
    std::array<std::atomic<float>*, 26> muteParams{};
    std::array<std::atomic<float>*, 26> soloParams{};
    std::array<std::atomic<float>*, 26> gainParams{};
    
    // 新的强大状态机 - 替代所有旧的弱小逻辑
    std::unique_ptr<StateManager> stateManager;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessor)
};
