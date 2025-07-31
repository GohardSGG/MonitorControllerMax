/*
  ==============================================================================

    StateManager.h
    Created: 2025-07-30
    Author:  GohardSGG & Claude Code

    状态管理器 - 所有业务逻辑的中心
    负责：状态计算、布局管理、Master-Slave协调、OSC通信
    线程：仅在消息线程中访问

    JUCE架构重构核心组件 - 彻底解决线程安全和实时性能问题

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "ConfigManager.h"
#include "SemanticChannelState.h"
#include "OSCCommunicator.h"
#include "DebugLogger.h"
#include "GlobalPluginState.h"  // 为了PluginRole枚举

// 前向声明
class MonitorControllerMaxAudioProcessor;
struct RenderState;

//==============================================================================
/**
 * 状态管理器 - 所有业务逻辑的中心
 * 负责：状态计算、布局管理、Master-Slave协调、OSC通信
 * 线程：仅在消息线程中访问
 */
class StateManager : public juce::AudioProcessorValueTreeState::Listener,
                     public SemanticChannelState::StateChangeListener
{
public:
    StateManager(MonitorControllerMaxAudioProcessor& processor);
    ~StateManager();
    
    //=== 用户接口（消息线程）===
    void setSoloState(const juce::String& channelName, bool state);
    void setMuteState(const juce::String& channelName, bool state);
    void setChannelGain(const juce::String& channelName, float gainDb);
    void setMasterGain(float gainPercent);
    void setDimActive(bool active);
    void setLowBoostActive(bool active);
    void setMasterMuteActive(bool active);
    void setMonoActive(bool active);
    void setCurrentLayout(const juce::String& speakerLayout, const juce::String& subLayout);
    
    //=== 状态查询（消息线程）===
    bool getSoloState(const juce::String& channelName) const;
    bool getMuteState(const juce::String& channelName) const;
    float getMasterGain() const;
    bool isDimActive() const;
    bool isMonoActive() const;
    
    //=== Master-Slave通信（消息线程）===
    void setPluginRole(PluginRole role);
    void receiveMasterState(const juce::String& channelName, const juce::String& action, bool state);
    void broadcastToSlaves();
    
    //=== OSC控制（消息线程）===
    void handleOSCMessage(const juce::String& address, float value);
    void sendOSCUpdate(const juce::String& channelName, const juce::String& action, bool state);
    
    //=== 实时渲染接口 ===
    RenderState* beginStateUpdate();  // 开始更新，返回非活跃缓冲区
    void commitStateUpdate();          // 提交更新，原子切换缓冲区
    const RenderState* getCurrentRenderState() const; // 音频线程访问
    
    //=== AudioProcessorValueTreeState::Listener 接口 ===
    void parameterChanged(const juce::String& parameterID, float newValue) override;
    
    //=== SemanticChannelState::StateChangeListener 接口 ===
    void onSoloStateChanged(const juce::String& channelName, bool state) override;
    void onMuteStateChanged(const juce::String& channelName, bool state) override;
    void onGlobalModeChanged() override;
    
private:
    //=== 内部状态（仅消息线程访问）===
    struct InternalState {
        std::map<juce::String, bool> soloStates;
        std::map<juce::String, bool> muteStates;
        std::map<juce::String, float> gainStates;
        
        float masterGainPercent = 100.0f;
        bool dimActive = false;
        bool lowBoostActive = false;
        bool masterMuteActive = false;
        bool monoActive = false;
        
        Layout currentLayout;
        PluginRole currentRole = PluginRole::Standalone;
    };
    
    InternalState state;
    mutable std::mutex stateMutex;  // 保护内部状态（仅消息线程使用）
    
    //=== 双缓冲渲染状态 ===
    std::unique_ptr<RenderState> renderStateA;
    std::unique_ptr<RenderState> renderStateB;
    std::atomic<RenderState*> activeRenderState;
    
    //=== 依赖组件 ===
    MonitorControllerMaxAudioProcessor& processor;
    std::unique_ptr<OSCCommunicator> oscComm;
    
    //=== 内部方法 ===
    void recalculateRenderState(RenderState* targetState);
    void applyComplexSoloLogic(RenderState* targetState);
    void notifyStateChange(const juce::String& channelName, const juce::String& action, bool state);
    
    //=== 布局管理 ===
    void updateLayoutMapping(const Layout& newLayout);
    int getPhysicalChannelForSemantic(const juce::String& channelName) const;
    
    //=== 状态同步 ===
    void syncToValueTreeState();
    void syncFromValueTreeState();
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(StateManager)
};