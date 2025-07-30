/*
  ==============================================================================

    RenderState.h
    Created: 2025-07-30
    Author:  GohardSGG & Claude Code

    渲染状态 - 音频线程专用的预计算数据
    特点：POD结构、缓存对齐、无动态内存

    JUCE架构重构核心组件 - 实现无锁音频处理

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <array>
#include <atomic>

//==============================================================================
/**
 * 渲染状态 - 音频线程专用的预计算数据
 * 特点：POD结构、缓存对齐、无动态内存
 */
struct RenderState
{
    static constexpr int MAX_CHANNELS = 26;
    
    //=== 通道渲染数据 ===
    struct ChannelData {
        float targetGain;      // 目标增益（含个人增益、Master增益、Dim）
        mutable float currentGain;     // 当前增益（用于平滑）- mutable允许在const方法中修改
        bool shouldMute;       // 最终静音状态（Solo逻辑结果）
        bool isMonoChannel;    // 是否参与Mono混合
        uint8_t padding[2];    // 对齐到8字节
    };
    
    alignas(64) std::array<ChannelData, MAX_CHANNELS> channels;
    
    //=== Master总线数据 ===
    struct MasterData {
        bool masterMuteActive;
        bool monoEffectActive;
        uint8_t monoChannelCount;
        uint8_t padding[5];
        alignas(8) std::array<uint8_t, MAX_CHANNELS> monoChannelIndices;
    };
    
    alignas(64) MasterData master;
    
    //=== 版本控制 ===
    std::atomic<uint64_t> version{0};
    
    //=== 音频线程方法（内联优化）===
    void applyToBuffer(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept;
    void applyMonoEffect(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept;
    void smoothGainTransition(float smoothingFactor) noexcept;
};

//==============================================================================
// RenderState 的音频处理实现（高性能、无锁）
inline void RenderState::applyToBuffer(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept
{
    const int numChannels = juce::jmin(buffer.getNumChannels(), MAX_CHANNELS);
    
    // Master Mute 快速路径
    if (master.masterMuteActive) {
        buffer.clear();
        return;
    }
    
    // Mono 效果处理（如果激活）
    if (master.monoEffectActive && master.monoChannelCount > 1) {
        applyMonoEffect(buffer, numSamples);
    }
    
    // 并行处理每个通道（编译器可自动向量化）
    for (int ch = 0; ch < numChannels; ++ch) {
        const ChannelData& chData = channels[ch];
        
        if (chData.shouldMute) {
            buffer.clear(ch, 0, numSamples);
        }
        else if (std::abs(chData.currentGain - 1.0f) > 0.001f) {
            buffer.applyGainRamp(ch, 0, numSamples, chData.currentGain, chData.targetGain);
            // 更新当前增益（无锁，因为每个通道独立）
            chData.currentGain = chData.targetGain;
        }
    }
}

//==============================================================================
inline void RenderState::applyMonoEffect(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept
{
    if (master.monoChannelCount < 2) return;
    
    // 计算所有参与Mono的通道的平均值
    alignas(16) float monoSum[16];  // 使用小块处理，利于SIMD
    const int blockSize = 16;
    
    for (int offset = 0; offset < numSamples; offset += blockSize) {
        const int samplesToProcess = juce::jmin(blockSize, numSamples - offset);
        
        // 清零累加器
        for (int i = 0; i < samplesToProcess; ++i) {
            monoSum[i] = 0.0f;
        }
        
        // 累加所有Mono通道
        for (uint8_t i = 0; i < master.monoChannelCount; ++i) {
            const uint8_t chIndex = master.monoChannelIndices[i];
            if (chIndex < buffer.getNumChannels()) {
                const float* channelData = buffer.getReadPointer(chIndex) + offset;
                for (int s = 0; s < samplesToProcess; ++s) {
                    monoSum[s] += channelData[s];
                }
            }
        }
        
        // 计算平均值并写回所有Mono通道
        const float scale = 1.0f / static_cast<float>(master.monoChannelCount);
        for (uint8_t i = 0; i < master.monoChannelCount; ++i) {
            const uint8_t chIndex = master.monoChannelIndices[i];
            if (chIndex < buffer.getNumChannels()) {
                float* channelData = buffer.getWritePointer(chIndex) + offset;
                for (int s = 0; s < samplesToProcess; ++s) {
                    channelData[s] = monoSum[s] * scale;
                }
            }
        }
    }
}

//==============================================================================
inline void RenderState::smoothGainTransition(float smoothingFactor) noexcept
{
    for (auto& chData : channels) {
        if (std::abs(chData.currentGain - chData.targetGain) > 0.001f) {
            chData.currentGain += (chData.targetGain - chData.currentGain) * smoothingFactor;
        }
    }
}