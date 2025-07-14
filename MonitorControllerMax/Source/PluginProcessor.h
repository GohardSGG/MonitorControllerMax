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
#include "SemanticChannelState.h"
#include "PhysicalChannelMapper.h"
#include "OSCCommunicator.h"
#include "GlobalPluginState.h"

//==============================================================================

class MonitorControllerMaxAudioProcessor  : public juce::AudioProcessor,
                                          public juce::AudioProcessorValueTreeState::Listener,
                                          public SemanticChannelState::StateChangeListener
{
public:
    //==============================================================================
    // A constant for the number of channels we'll manage.
    static constexpr int numManagedChannels = 26;
    
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
    // Master-Slave角色管理
    void switchToStandalone();
    void switchToMaster();
    void switchToSlave();
    PluginRole getCurrentRole() const { return currentRole; }
    
    // 状态同步接口（供GlobalPluginState调用）
    void receiveMasterState(const juce::String& channelName, const juce::String& action, bool state);
    void onMasterDisconnected();
    void onMasterConnected();
    
    // 连接状态查询
    bool isMasterWithSlaves() const;
    bool isSlaveConnected() const;
    int getConnectedSlaveCount() const;
    juce::String getConnectionStatusText() const;
    
    // 用于UI刷新时维持状态的接口
    void saveCurrentUIState();
    void restoreUIState();
    void restoreSemanticStates(); // 恢复语义状态

    void setCurrentLayout(const juce::String& speaker, const juce::String& sub);
    const Layout& getCurrentLayout() const;
    int getAvailableChannels() const;
    
    // Automatically select the most suitable layout configuration based on channel count
    void autoSelectLayoutForChannelCount(int channelCount);
    
    // UI update callback function type
    std::function<void(const juce::String&, const juce::String&)> onLayoutAutoChanged;
    
    // Set UI update callback
    void setLayoutChangeCallback(std::function<void(const juce::String&, const juce::String&)> callback);
    
    
    // Pure Logic Interface - No State Machine
    void handleSoloButtonClick();
    void handleMuteButtonClick();
    void handleChannelClick(int channelIndex);
    bool hasAnySoloActive() const;
    bool hasAnyMuteActive() const;
    
    // Pure Logic UI Control
    bool isMuteButtonEnabled() const;
    
    // Dual state button activation functions
    bool isSoloButtonActive() const;
    bool isMuteButtonActive() const;

    // Semantic state system access (new interface)
    SemanticChannelState& getSemanticState() { return semanticState; }
    PhysicalChannelMapper& getPhysicalMapper() { return physicalMapper; }
    OSCCommunicator& getOSCCommunicator() { return oscCommunicator; }
    const SemanticChannelState& getSemanticState() const { return semanticState; }
    const PhysicalChannelMapper& getPhysicalMapper() const { return physicalMapper; }
    const OSCCommunicator& getOSCCommunicator() const { return oscCommunicator; }

    // SemanticChannelState::StateChangeListener interface
    void onSoloStateChanged(const juce::String& channelName, bool state) override;
    void onMuteStateChanged(const juce::String& channelName, bool state) override;
    void onGlobalModeChanged() override;
    
    // OSC external control handler
    void handleExternalOSCControl(const juce::String& action, const juce::String& channelName, bool state);
    
    // 状态同步时的回调处理（整合到现有回调中）
    void onSemanticStateChanged(const juce::String& channelName, const juce::String& action, bool state);

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
    
    // 用户实际选择的布局配置 - 用于状态持久化
    juce::String userSelectedSpeakerLayout = "2.0";
    juce::String userSelectedSubLayout = "None";
    
    // 角色和UI状态持久化
    PluginRole savedRole = PluginRole::Standalone;
    juce::String savedSelectedChannels;
    juce::ValueTree savedSemanticStateData; // DEPRECATED: Solo/Mute状态不再持久化

    // New semantic state system (gradually replacing VST3 parameter system)
    SemanticChannelState semanticState;
    PhysicalChannelMapper physicalMapper;
    OSCCommunicator oscCommunicator;

private:
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout();
    //==============================================================================
    // Master-Slave角色管理
    PluginRole currentRole = PluginRole::Standalone;
    bool isRegisteredToGlobalState = false;
    bool suppressStateChange = false;  // 防止循环回调
    
    // 角色管理方法
    void registerToGlobalState();
    void unregisterFromGlobalState();
    void handleRoleTransition(PluginRole newRole);
    void updateUIFromRole();
    
    // OSC系统角色管理
    void initializeOSCForRole();
    void shutdownOSC();
    juce::String getRoleString(PluginRole role) const;

    // We'll need atomic pointers to our parameters for thread-safe access in the audio callback.
    std::array<std::atomic<float>*, 26> muteParams{};
    std::array<std::atomic<float>*, 26> soloParams{};
    std::array<std::atomic<float>*, 26> gainParams{};
    
    
    // Selection mode state functions
    bool isInSoloSelectionMode() const;
    bool isInMuteSelectionMode() const;
    
    // Selection mode state tracking
    std::atomic<bool> pendingSoloSelection{false};
    std::atomic<bool> pendingMuteSelection{false};
    
    // Protection state management (new)
    bool soloModeProtectionActive = false;
    
    // State synchronization and validation
    void updateAllStates();
    void validateStateConsistency();
    
    // Flag to prevent parameter update loops
    std::atomic<bool> isUpdatingFromParameter{false};

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessor)
};

// Role-aware debug macros with role prefix - defined here to avoid circular includes
#define VST3_DBG_ROLE(processorPtr, message) \
    do { \
        juce::String rolePrefix; \
        if (processorPtr) { \
            switch ((processorPtr)->getCurrentRole()) { \
                case PluginRole::Standalone: rolePrefix = "[Standalone]"; break; \
                case PluginRole::Master: rolePrefix = "[Master]"; break; \
                case PluginRole::Slave: rolePrefix = "[Slave]"; break; \
                default: rolePrefix = "[Unknown]"; break; \
            } \
        } else { \
            rolePrefix = "[Unknown]"; \
        } \
        std::ostringstream oss; \
        oss << rolePrefix << " " << message; \
        DBG(oss.str()); \
        DebugLogger::getInstance().log(oss.str()); \
    } while(0)

#define VST3_DBG_ROLE_IMPORTANT(processorPtr, message) \
    do { \
        juce::String rolePrefix; \
        if (processorPtr) { \
            switch ((processorPtr)->getCurrentRole()) { \
                case PluginRole::Standalone: rolePrefix = "[Standalone]"; break; \
                case PluginRole::Master: rolePrefix = "[Master]"; break; \
                case PluginRole::Slave: rolePrefix = "[Slave]"; break; \
                default: rolePrefix = "[Unknown]"; break; \
            } \
        } else { \
            rolePrefix = "[Unknown]"; \
        } \
        std::ostringstream oss; \
        oss << rolePrefix << " " << message; \
        DBG(oss.str()); \
        DebugLogger::getInstance().logImportant(oss.str()); \
    } while(0)
