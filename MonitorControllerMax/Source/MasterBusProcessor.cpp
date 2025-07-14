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
    
    // 应用到所有通道 (JSFX: spl0 *= ... * Level_Master)
    if (std::abs(masterLevel - 1.0f) > 0.001f)  // 只有非1.0时才应用增益
    {
        int totalChannels = buffer.getNumChannels();
        int numSamples = buffer.getNumSamples();
        
        for (int channel = 0; channel < totalChannels; ++channel)
        {
            buffer.applyGain(channel, 0, numSamples, masterLevel);
        }
        
        // 删除垃圾日志 - 高频音频处理调用
        // VST3_DBG_ROLE(processorPtr, "Applied master level: " << masterLevel << " to " << totalChannels << " channels");
    }
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