#pragma once

#include <JuceHeader.h>
#include <memory>
#include <atomic>
#include "RenderState.h"
#include "SemanticChannelState.h"

// 前向声明避免循环引用
class MonitorControllerMaxAudioProcessor;

//==============================================================================
/**
 * 状态管理器 - 统一状态控制和收集系统
 * 
 * 🚀 彻底修复 v2.0：从纯收集器升级为完整控制器
 * 
 * 新职责：
 * 1. **状态控制器**: 统一所有UI控制逻辑（Solo/Mute按钮处理）
 * 2. **状态收集器**: 收集各组件状态，生成音频快照
 * 
 * 设计原则：
 * - 统一所有状态管理到一个类，消除架构不一致
 * - 业务逻辑委托给SemanticChannelState（保持职责分离）
 * - 线程安全的双缓冲系统
 * - 严格遵循JUCE消息线程/音频线程分离原则
 * - 保持向后兼容，不破坏现有音频处理逻辑
 */
class StateManager : public SemanticChannelState::StateChangeListener,
                     public juce::AudioProcessorValueTreeState::Listener
{
public:
    StateManager(MonitorControllerMaxAudioProcessor& processor);
    ~StateManager();
    
    //=== 生命周期管理 ===
    void initialize();
    void shutdown();
    
    //=== 音频线程接口（线程安全，无锁）===
    const RenderState* getCurrentRenderState() const noexcept;
    
    //=== SemanticChannelState::StateChangeListener 接口 ===
    void onSoloStateChanged(const juce::String& channelName, bool state) override;
    void onMuteStateChanged(const juce::String& channelName, bool state) override;
    void onGlobalModeChanged() override;
    
    //=== AudioProcessorValueTreeState::Listener 接口 ===
    void parameterChanged(const juce::String& parameterID, float newValue) override;
    
    //=== 布局变化处理 ===
    void onLayoutChanged();
    
    // 🚀 彻底修复：StateManager统一状态控制接口
    // 遵循原始设计意图：统一所有状态管理到StateManager
    //=== UI控制接口（消息线程）===
    void handleSoloButtonClick();
    void handleMuteButtonClick();
    
    //=== 通道控制接口（消息线程）===
    void handleChannelSoloClick(const juce::String& channelName, bool newState);
    void handleChannelMuteClick(const juce::String& channelName, bool newState);
    
    //=== 状态查询接口（线程安全）===
    bool isInSoloSelectionMode() const noexcept;
    bool isInMuteSelectionMode() const noexcept;
    bool hasAnySoloActive() const noexcept;
    bool hasAnyMuteActive() const noexcept;
    
private:
    MonitorControllerMaxAudioProcessor& processor;
    
    //=== 双缓冲系统（确保音频线程无锁访问）===
    std::unique_ptr<RenderState> renderStateA;
    std::unique_ptr<RenderState> renderStateB;
    std::atomic<RenderState*> activeRenderState{nullptr};    // 音频线程读取
    RenderState* inactiveRenderState{nullptr};               // 消息线程更新
    
    //=== 核心方法 ===
    void updateRenderState();
    void collectCurrentState(RenderState* targetState);
    void commitRenderState();
    
    //=== 状态收集方法（直接调用现有组件，不做计算）===
    void collectChannelStates(RenderState* target);
    void collectMasterBusStates(RenderState* target);
    void collectMonoChannelData(RenderState* target);
    
    //=== 内部状态 ===
    bool initialized = false;
    
    // 🚀 彻底修复：UI状态模式管理（线程安全）
    std::atomic<bool> soloSelectionMode{false};
    std::atomic<bool> muteSelectionMode{false};
    
    //=== 业务逻辑委托方法（保持职责分离）===
    SemanticChannelState& getSemanticState();
    void triggerStateUpdate(); // 触发状态更新到音频线程
    void updateProcessorPendingStates(); // 同步processor的pending状态
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(StateManager)
};