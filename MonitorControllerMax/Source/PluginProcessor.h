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

class InterPluginCommunicator;

//==============================================================================
/**
*/
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
    
    // 根据通道数自动选择最合适的布局配置
    void autoSelectLayoutForChannelCount(int channelCount);
    
    // UI更新回调函数类型
    std::function<void(const juce::String&, const juce::String&)> onLayoutAutoChanged;
    
    // 设置UI更新回调
    void setLayoutChangeCallback(std::function<void(const juce::String&, const juce::String&)> callback);
    
    // Mute状态分类管理 - 区分手动mute和solo联动mute
    void setManualMuteState(const juce::String& paramId, bool isManuallyMuted);
    bool isManuallyMuted(const juce::String& paramId) const;
    void setSoloInducedMuteState(const juce::String& paramId, bool isSoloInduced);
    bool isSoloInducedMute(const juce::String& paramId) const;
    void clearAllSoloInducedMutes();
    
    // Solo状态快照管理 - 记忆进入Solo前的完整状态
    void savePreSoloSnapshot();
    void restorePreSoloSnapshot();
    bool hasPreSoloSnapshot() const;
    

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
    
    // 动态I/O通道名函数 - 根据当前布局提供有意义的通道名称
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
    
    // 记录哪些Mute是真正手动激活的（而不是Solo联动产生的）
    std::set<juce::String> manualMuteStates;
    
    // 记录哪些Mute是Solo联动产生的（用于全局状态管理）
    std::set<juce::String> soloInducedMuteStates;
    
    // Solo进入前的状态快照（完整的Mute状态记忆）
    std::map<juce::String, bool> preSoloSnapshot;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessor)
};
