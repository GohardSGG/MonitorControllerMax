/*
  ==============================================================================

    MasterBusProcessor.cpp
    Created: 2025-07-14
    Author:  GohardSGG & Claude Code

    总线效果处理器实现 - 基于JSFX Monitor Controllor 7.1.4

  ==============================================================================
*/

#include "MasterBusProcessor.h"
#include "PluginProcessor.h"

//==============================================================================
MasterBusProcessor::MasterBusProcessor()
{
    VST3_DBG("MasterBusProcessor: Initialize master bus processor");
}

MasterBusProcessor::~MasterBusProcessor()
{
    VST3_DBG("MasterBusProcessor: Destroy master bus processor");
}

//==============================================================================
void MasterBusProcessor::setProcessor(MonitorControllerMaxAudioProcessor* processor)
{
    processorPtr = processor;
}

//==============================================================================
void MasterBusProcessor::process(juce::AudioBuffer<float>& buffer, PluginRole currentRole)
{
    // 计算当前Master Level (基于JSFX算法)
    float masterLevel = calculateMasterLevel();
    
    int totalChannels = buffer.getNumChannels();
    int numSamples = buffer.getNumSamples();
    
    // v4.1: 检查Master Mute状态 - 如果激活则静音所有通道
    if (masterMuteActive)
    {
        // Master Mute激活时，静音所有通道
        for (int channel = 0; channel < totalChannels; ++channel)
        {
            buffer.clear(channel, 0, numSamples);
        }
        return;  // 直接返回，不应用其他效果
    }
    
    // 应用Master Level和Low Boost到各通道
    for (int channel = 0; channel < totalChannels; ++channel)
    {
        float channelGain = masterLevel;
        
        // v4.1: 检查是否是SUB通道并应用Low Boost
        if (lowBoostActive && processorPtr)
        {
            // 获取通道名称来判断是否为SUB通道
            auto channelName = processorPtr->getPhysicalMapper().getSemanticNameSafe(channel);
            if (channelName.startsWith("SUB"))
            {
                channelGain *= LOW_BOOST_FACTOR;  // 1.5x boost for SUB channels
            }
        }
        
        // 只有非1.0时才应用增益
        if (std::abs(channelGain - 1.0f) > 0.001f)
        {
            buffer.applyGain(channel, 0, numSamples, channelGain);
        }
    }
    
    // 删除垃圾日志 - 高频音频处理调用
    // VST3_DBG_ROLE(processorPtr, "Applied master level: " << masterLevel << " to " << totalChannels << " channels");
}

//==============================================================================
void MasterBusProcessor::setMasterGainPercent(float gainPercent)
{
    // 限制范围到0-100%
    masterGainPercent = juce::jlimit(0.0f, 100.0f, gainPercent);
    
    if (processorPtr)
    {
        VST3_DBG_ROLE(processorPtr, "Master Gain set to: " << masterGainPercent << "%");
    }
}

//==============================================================================
void MasterBusProcessor::setDimActive(bool active)
{
    if (dimActive != active)
    {
        dimActive = active;
        
        if (processorPtr)
        {
            VST3_DBG_ROLE(processorPtr, "Dim " << (dimActive ? "ACTIVATED" : "DEACTIVATED") 
                         << " (Level: " << (dimActive ? "16%" : "100%") << ")");
            
            // v4.1: 发送OSC Dim状态 (通过PluginProcessor发送，确保角色检查)
            processorPtr->sendDimOSCState(dimActive);
        }
        
        // v4.1: 通知UI更新
        if (onDimStateChanged)
        {
            onDimStateChanged();
        }
    }
}

//==============================================================================
void MasterBusProcessor::setLowBoostActive(bool active)
{
    if (lowBoostActive != active)
    {
        lowBoostActive = active;
        
        if (processorPtr)
        {
            VST3_DBG_ROLE(processorPtr, "Low Boost " << (lowBoostActive ? "ACTIVATED" : "DEACTIVATED") 
                         << " (SUB channels: " << (lowBoostActive ? "1.5x" : "1.0x") << ")");
            
            // v4.1: 发送OSC Low Boost状态 (通过PluginProcessor发送，确保角色检查)
            processorPtr->sendLowBoostOSCState(lowBoostActive);
        }
        
        // v4.1: 通知UI更新
        if (onLowBoostStateChanged)
        {
            onLowBoostStateChanged();
        }
    }
}

//==============================================================================
void MasterBusProcessor::setMasterMuteActive(bool active)
{
    if (masterMuteActive != active)
    {
        masterMuteActive = active;
        
        if (processorPtr)
        {
            VST3_DBG_ROLE(processorPtr, "Master Mute " << (masterMuteActive ? "ACTIVATED" : "DEACTIVATED") 
                         << " (All channels: " << (masterMuteActive ? "MUTED" : "ACTIVE") << ")");
            
            // v4.1: 发送OSC Master Mute状态 (通过PluginProcessor发送，确保角色检查)
            processorPtr->sendMasterMuteOSCState(masterMuteActive);
        }
        
        // v4.1: 通知UI更新
        if (onMasterMuteStateChanged)
        {
            onMasterMuteStateChanged();
        }
    }
}

//==============================================================================
void MasterBusProcessor::handleOSCMasterVolume(float volumePercent)
{
    setMasterGainPercent(volumePercent);
    
    if (processorPtr)
    {
        VST3_DBG_ROLE(processorPtr, "OSC Master Volume received: " << volumePercent << "%");
    }
}

void MasterBusProcessor::handleOSCDim(bool dimState)
{
    setDimActive(dimState);
    
    if (processorPtr)
    {
        VST3_DBG_ROLE(processorPtr, "OSC Dim received: " << (dimState ? "ON" : "OFF"));
    }
}

void MasterBusProcessor::handleOSCLowBoost(bool lowBoostState)
{
    setLowBoostActive(lowBoostState);
    
    if (processorPtr)
    {
        VST3_DBG_ROLE(processorPtr, "OSC Low Boost received: " << (lowBoostState ? "ON" : "OFF"));
    }
}

void MasterBusProcessor::handleOSCMasterMute(bool masterMuteState)
{
    setMasterMuteActive(masterMuteState);
    
    if (processorPtr)
    {
        VST3_DBG_ROLE(processorPtr, "OSC Master Mute received: " << (masterMuteState ? "ON" : "OFF"));
    }
}

//==============================================================================
float MasterBusProcessor::getCurrentMasterLevel() const
{
    return calculateMasterLevel();
}

juce::String MasterBusProcessor::getStatusDescription() const
{
    float currentLevel = calculateMasterLevel();
    
    juce::String desc = "Master Bus: ";
    desc += juce::String(masterGainPercent, 1) + "%";
    
    if (dimActive)
    {
        desc += " + DIM(16%)";
    }
    
    if (lowBoostActive)
    {
        desc += " + LOW_BOOST(1.5x)";
    }
    
    if (masterMuteActive)
    {
        desc += " + MASTER_MUTE(ALL_MUTED)";
    }
    
    desc += " = " + juce::String(currentLevel * 100.0f, 1) + "%";
    
    return desc;
}

//==============================================================================
float MasterBusProcessor::calculateMasterLevel() const
{
    // 基于JSFX算法: Level_Master = (slider99 * scale) * (Dim_Master ? 0.16 : 1)
    // 其中 scale = 0.01, slider99 = 0-100
    
    float baseLevel = masterGainPercent * SCALE_FACTOR;  // 0-100% -> 0.0-1.0
    float dimFactor = dimActive ? DIM_FACTOR : 1.0f;     // Dim时衰减到16%
    
    return baseLevel * dimFactor;
}