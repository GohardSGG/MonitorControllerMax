/*
  ==============================================================================

    MasterBusProcessor.h
    Created: 2025-07-14
    Author:  GohardSGG & Claude Code

    总线效果处理器 - 处理Master Gain和Dim效果
    基于JSFX Monitor Controllor 7.1.4的数学运算

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "DebugLogger.h"

// 前向声明避免循环引用
class MonitorControllerMaxAudioProcessor;
enum class PluginRole;

//==============================================================================
/**
    总线效果处理器
    
    负责处理所有总线级别的音频效果：
    - Master Gain: 0-100% 线性衰减器，VST3参数，持久化保存
    - Dim: 内部状态，衰减到16%，不持久化，仅维持窗口会话
    
    OSC控制地址：
    - /Monitor/Master/Volume (Master Gain)
    - /Monitor/Master/Dim (Dim on/off)
*/
class MasterBusProcessor
{
public:
    //==============================================================================
    MasterBusProcessor();
    ~MasterBusProcessor();
    
    //==============================================================================
    // 设置processor指针用于角色日志
    void setProcessor(MonitorControllerMaxAudioProcessor* processor);
    
    //==============================================================================
    // 音频处理接口
    void process(juce::AudioBuffer<float>& buffer, PluginRole currentRole);
    
    //==============================================================================
    // Master Gain控制 (VST3参数，0-100%)
    void setMasterGainPercent(float gainPercent);
    float getMasterGainPercent() const { return masterGainPercent; }
    
    //==============================================================================
    // Dim控制 (内部状态，不持久化)
    void setDimActive(bool active);
    bool isDimActive() const { return dimActive; }
    void toggleDim() { setDimActive(!dimActive); }
    
    // Low Boost控制 (内部状态，不持久化)
    void setLowBoostActive(bool active);
    bool isLowBoostActive() const { return lowBoostActive; }
    void toggleLowBoost() { setLowBoostActive(!lowBoostActive); }
    
    // Master Mute控制 (内部状态，不持久化)
    void setMasterMuteActive(bool active);
    bool isMasterMuteActive() const { return masterMuteActive; }
    void toggleMasterMute() { setMasterMuteActive(!masterMuteActive); }
    
    // Mono控制 (内部状态，不持久化)
    void setMonoActive(bool active);
    bool isMonoActive() const { return monoActive; }
    void toggleMono() { setMonoActive(!monoActive); }
    
    //==============================================================================
    // OSC控制接口
    void handleOSCMasterVolume(float volumePercent);
    void handleOSCDim(bool dimState);
    void handleOSCLowBoost(bool lowBoostState);
    void handleOSCMasterMute(bool masterMuteState);
    void handleOSCMono(bool monoState);
    
    //==============================================================================
    // 状态查询
    float getCurrentMasterLevel() const;
    juce::String getStatusDescription() const;
    
    // v4.1: UI更新回调
    std::function<void()> onDimStateChanged;
    std::function<void()> onLowBoostStateChanged;
    std::function<void()> onMasterMuteStateChanged;
    std::function<void()> onMonoStateChanged;
    
private:
    //==============================================================================
    // 处理器指针（用于角色日志）
    MonitorControllerMaxAudioProcessor* processorPtr = nullptr;
    
    //==============================================================================
    // 内部状态
    float masterGainPercent = 100.0f;  // Master Gain百分比 (0-100%)
    bool dimActive = false;             // Dim状态 (内部状态，不持久化)
    bool lowBoostActive = false;        // Low Boost状态 (内部状态，不持久化)
    bool masterMuteActive = false;      // Master Mute状态 (内部状态，不持久化)
    bool monoActive = false;            // Mono状态 (内部状态，不持久化)
    
    //==============================================================================
    // 音频处理常量 (基于JSFX实现)
    static constexpr float DIM_FACTOR = 0.16f;  // Dim时的衰减因子 (16%)
    static constexpr float SCALE_FACTOR = 0.01f; // 百分比转换因子
    static constexpr float LOW_BOOST_FACTOR = 1.5f; // Low Boost增益因子 (1.5x, 约+3.5dB)
    
    //==============================================================================
    // 内部计算方法
    float calculateMasterLevel() const;
    
    //==============================================================================
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MasterBusProcessor)
};