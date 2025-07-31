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
 * 状态管理器 - 收集各组件状态，生成音频快照
 * 
 * 职责：作为各组件的状态收集器，不做任何业务逻辑计算
 * 设计原则：
 * - 只收集现有组件的最终计算结果
 * - 不重新实现任何Solo/Mute/总线逻辑
 * - 线程安全的双缓冲系统
 * - 严格遵循JUCE消息线程/音频线程分离原则
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
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(StateManager)
};