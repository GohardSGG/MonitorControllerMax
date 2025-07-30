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
#include "GlobalPluginState.h"

//==============================================================================
MasterBusProcessor::MasterBusProcessor()
{
    VST3_DBG("MasterBusProcessor: Initialize master bus processor");
    
    // 预分配缓冲区初始化
    std::fill(monoMixBuffer.begin(), monoMixBuffer.end(), 0.0f);
    std::fill(nonSubChannelsBuffer.begin(), nonSubChannelsBuffer.end(), 0);
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
void MasterBusProcessor::prepare(double sampleRate, int maximumExpectedSamplesPerBlock)
{
    // 验证缓冲区大小足够
    jassert(maximumExpectedSamplesPerBlock <= static_cast<int>(MAX_BLOCK_SIZE));
    
    if (maximumExpectedSamplesPerBlock > static_cast<int>(MAX_BLOCK_SIZE))
    {
        VST3_DBG("MasterBusProcessor: WARNING - Block size " << maximumExpectedSamplesPerBlock 
                 << " exceeds MAX_BLOCK_SIZE " << MAX_BLOCK_SIZE);
    }
    
    // 预热缓存 - 确保内存页面被分配和初始化
    std::fill(monoMixBuffer.begin(), monoMixBuffer.end(), 0.0f);
    std::fill(nonSubChannelsBuffer.begin(), nonSubChannelsBuffer.end(), 0);
    
    VST3_DBG("MasterBusProcessor: Prepared for sampleRate=" << sampleRate 
             << ", maxBlockSize=" << maximumExpectedSamplesPerBlock);
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
    
    // v4.1: 处理Mono效果 - 将所有非SUB通道混合成单声道并复制到所有非SUB通道
    // 重要：Mono处理只在Slave/Standalone模式下进行 (pre-calibration)
    // 🚀 稳定性优化：使用预分配缓冲区，消除音频线程中的内存分配
    if (monoActive && processorPtr && 
        (currentRole == PluginRole::Slave || currentRole == PluginRole::Standalone))
    {
        // 边界检查：确保不超过预分配缓冲区大小
        if (numSamples > static_cast<int>(MAX_BLOCK_SIZE) || totalChannels > static_cast<int>(MAX_CHANNELS))
        {
            jassertfalse;  // Debug断言
            return;        // Release版本安全返回，跳过Mono处理
        }
        
        // 步骤1: 收集所有非SUB通道的索引（使用预分配缓冲区）
        nonSubChannelsCount = 0;
        for (int channel = 0; channel < totalChannels && nonSubChannelsCount < MAX_CHANNELS; ++channel)
        {
            auto channelName = processorPtr->getPhysicalMapper().getSemanticNameSafe(channel);
            if (!channelName.startsWith("SUB"))
            {
                nonSubChannelsBuffer[nonSubChannelsCount++] = channel;
            }
        }
        
        // 步骤2: 混合所有非SUB通道内容（使用预分配缓冲区）
        if (nonSubChannelsCount > 1)
        {
            // 使用预分配的混音缓冲区
            float* monoMix = monoMixBuffer.data();
            
            // 清零混音缓冲区
            std::fill_n(monoMix, numSamples, 0.0f);
            
            // 混合所有非SUB通道到单声道
            for (size_t i = 0; i < nonSubChannelsCount; ++i)
            {
                int channel = nonSubChannelsBuffer[i];
                const float* channelData = buffer.getReadPointer(channel);
                
                // 手动累加避免AudioBuffer分配
                for (int sample = 0; sample < numSamples; ++sample)
                {
                    monoMix[sample] += channelData[sample];
                }
            }
            
            // 电压补偿混合结果 (每两个通道-6dB的对数衰减)
            float mixGain = 1.0f / static_cast<float>(nonSubChannelsCount);
            for (int sample = 0; sample < numSamples; ++sample)
            {
                monoMix[sample] *= mixGain;
            }
            
            // 步骤3: 将混合结果复制到所有非SUB通道
            for (size_t i = 0; i < nonSubChannelsCount; ++i)
            {
                int channel = nonSubChannelsBuffer[i];
                float* channelData = buffer.getWritePointer(channel);
                
                // 手动复制避免AudioBuffer操作
                for (int sample = 0; sample < numSamples; ++sample)
                {
                    channelData[sample] = monoMix[sample];
                }
            }
        }
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
        // VST3_DBG_ROLE(processorPtr, "Master Gain set to: " << masterGainPercent << "%");  // 注释：音频处理实时性优化
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
            // VST3_DBG_ROLE(processorPtr, "Dim " << (dimActive ? "ACTIVATED" : "DEACTIVATED") 
            //              << " (Level: " << (dimActive ? "16%" : "100%") << ")");  // 注释：音频处理实时性优化
            
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
            // VST3_DBG_ROLE(processorPtr, "Low Boost " << (lowBoostActive ? "ACTIVATED" : "DEACTIVATED") 
            //              << " (SUB channels: " << (lowBoostActive ? "1.5x" : "1.0x") << ")");  // 注释：音频处理实时性优化
            
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
            // VST3_DBG_ROLE(processorPtr, "Master Mute " << (masterMuteActive ? "ACTIVATED" : "DEACTIVATED") 
            //              << " (All channels: " << (masterMuteActive ? "MUTED" : "ACTIVE") << ")");  // 注释：音频处理实时性优化
            
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
void MasterBusProcessor::setMonoActive(bool active)
{
    if (monoActive != active)
    {
        monoActive = active;
        
        if (processorPtr)
        {
            // VST3_DBG_ROLE(processorPtr, "Mono " << (monoActive ? "ACTIVATED" : "DEACTIVATED") 
            //              << " (Non-SUB channels: " << (monoActive ? "mono mix" : "stereo") << ")");  // 注释：音频处理实时性优化
            
            // v4.1: 发送OSC Mono状态 (通过PluginProcessor发送，确保角色检查)
            processorPtr->sendMonoOSCState(monoActive);
            
            // v4.1: Master-Slave状态同步 - 只有Master/Standalone可以控制全局状态
            if (processorPtr->getCurrentRole() == PluginRole::Master || 
                processorPtr->getCurrentRole() == PluginRole::Standalone)
            {
                // 更新全局状态
                auto& globalState = GlobalPluginState::getInstance();
                globalState.setGlobalMonoState(monoActive);
                
                // 如果是Master模式，广播状态到所有Slave
                if (processorPtr->getCurrentRole() == PluginRole::Master)
                {
                    globalState.broadcastMonoStateToSlaves(monoActive);
                }
            }
        }
        
        // v4.1: 通知UI更新
        if (onMonoStateChanged)
        {
            onMonoStateChanged();
        }
    }
}

//==============================================================================
void MasterBusProcessor::handleOSCMasterVolume(float volumePercent)
{
    setMasterGainPercent(volumePercent);
    
    if (processorPtr)
    {
        // VST3_DBG_ROLE(processorPtr, "OSC Master Volume received: " << volumePercent << "%");  // 注释：OSC回调实时性优化
    }
}

void MasterBusProcessor::handleOSCDim(bool dimState)
{
    setDimActive(dimState);
    
    if (processorPtr)
    {
        // VST3_DBG_ROLE(processorPtr, "OSC Dim received: " << (dimState ? "ON" : "OFF"));  // 注释：OSC回调实时性优化
    }
}

void MasterBusProcessor::handleOSCLowBoost(bool lowBoostState)
{
    setLowBoostActive(lowBoostState);
    
    if (processorPtr)
    {
        // VST3_DBG_ROLE(processorPtr, "OSC Low Boost received: " << (lowBoostState ? "ON" : "OFF"));  // 注释：OSC回调实时性优化
    }
}

void MasterBusProcessor::handleOSCMasterMute(bool masterMuteState)
{
    setMasterMuteActive(masterMuteState);
    
    if (processorPtr)
    {
        // VST3_DBG_ROLE(processorPtr, "OSC Master Mute received: " << (masterMuteState ? "ON" : "OFF"));  // 注释：OSC回调实时性优化
    }
}

void MasterBusProcessor::handleOSCMono(bool monoState)
{
    setMonoActive(monoState);
    
    if (processorPtr)
    {
        // VST3_DBG_ROLE(processorPtr, "OSC Mono received: " << (monoState ? "ON" : "OFF"));  // 注释：OSC回调实时性优化
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