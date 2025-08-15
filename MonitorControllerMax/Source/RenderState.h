#pragma once

#include <JuceHeader.h>
#include <atomic>
#include <array>

//==============================================================================
/**
 * 音频渲染状态快照 - 音频线程专用的预计算数据
 * 特点：POD结构、无锁访问、所有数据预计算完成
 * 
 * 设计原则：
 * - 严格遵循JUCE音频线程规范：零锁、零分配、零复杂计算
 * - 所有数据在消息线程预计算，音频线程只做简单应用
 * - 直接来自现有组件的最终结果，不重新实现任何业务逻辑
 * 
 * 🚀 性能优化：内存对齐设计
 * - 结构体级别缓存行对齐，减少false sharing
 * - 数组级别SIMD对齐，利用向量化指令
 * - 热点数据聚合，提升缓存局部性
 */
struct alignas(64) RenderState  // 🚀 64字节缓存行对齐
{
    static constexpr int MAX_CHANNELS = 26;
    
    //=== 🚀 热点数据区域1：通道状态（SIMD优化，16字节对齐）===
    alignas(16) bool channelShouldMute[MAX_CHANNELS];     // 最终静音状态（包含所有SUB逻辑）
    alignas(16) bool channelIsActive[MAX_CHANNELS];       // 通道是否在当前布局中激活  
    alignas(16) bool channelIsSUB[MAX_CHANNELS];          // SUB通道标识（用于LowBoost处理）
    
    //=== 🚀 热点数据区域2：增益数据（浮点SIMD优化，16字节对齐）===
    alignas(16) float channelFinalGain[MAX_CHANNELS];     // 最终增益（个人增益 + 角色处理）
    
    //=== 🚀 控制数据区域：Master总线状态（缓存行开始）===
    alignas(64) bool monoActive;                          // Mono效果（用于预计算参与通道）
    
    //=== Mono效果预计算数据（紧凑布局）===
    uint8_t monoChannelCount;                             // 参与Mono的通道数量
    uint8_t monoChannelIndices[MAX_CHANNELS];             // 参与Mono的通道索引表
    
    //=== 🚀 版本控制区域（独立缓存行，避免写竞争）===
    alignas(64) mutable std::atomic<uint64_t> version{0}; // ABA问题防护
    
    //=== 构造函数：初始化为安全默认值 ===
    RenderState() noexcept
    {
        // 初始化所有通道为非激活、不静音、单位增益
        for (int i = 0; i < MAX_CHANNELS; ++i) {
            channelShouldMute[i] = false;
            channelFinalGain[i] = 1.0f;
            channelIsActive[i] = false;
            channelIsSUB[i] = false;
            monoChannelIndices[i] = 0;
        }
        
        // 初始化Master总线为默认状态
        monoActive = false;
        monoChannelCount = 0;
    }
    
    //=== 音频处理方法（高度优化，内联，符合JUCE规范）===
    void applyToBuffer(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept
    {
        const int numChannels = juce::jmin(buffer.getNumChannels(), MAX_CHANNELS);
        
        // 通道处理（编译器自动向量化友好的循环）- 只处理Solo/Mute/个人增益
        for (int ch = 0; ch < numChannels; ++ch) {
            // 跳过非激活通道
            if (!channelIsActive[ch]) continue;
            
            if (channelShouldMute[ch]) {
                // 静音通道：直接清零
                buffer.clear(ch, 0, numSamples);
            } else {
                // 仅应用个人通道增益（Master总线效果由MasterBusProcessor处理）
                const float channelGain = channelFinalGain[ch];
                
                // 只有在增益不为1.0时才应用（避免不必要的计算）
                if (std::abs(channelGain - 1.0f) > 0.001f) {
                    buffer.applyGain(ch, 0, numSamples, channelGain);
                }
            }
        }
    }
    
private:
    //=== Mono效果处理（栈分配，块处理，高性能）===
    void applyMonoEffect(juce::AudioBuffer<float>& buffer, int numSamples) const noexcept
    {
        if (monoChannelCount < 2) return;
        
        // 使用栈分配的临时缓冲区进行块处理，避免大数组分配
        constexpr int BLOCK_SIZE = 64;
        
        for (int offset = 0; offset < numSamples; offset += BLOCK_SIZE) {
            const int samplesToProcess = juce::jmin(BLOCK_SIZE, numSamples - offset);
            
            // 栈分配混合缓冲区（对齐以利用SIMD）
            alignas(16) float monoSum[BLOCK_SIZE] = {};
            
            // 第一步：累加所有Mono通道的信号
            for (int i = 0; i < monoChannelCount; ++i) {
                const int chIndex = monoChannelIndices[i];
                if (chIndex < buffer.getNumChannels() && channelIsActive[chIndex]) {
                    const float* src = buffer.getReadPointer(chIndex) + offset;
                    for (int s = 0; s < samplesToProcess; ++s) {
                        monoSum[s] += src[s];
                    }
                }
            }
            
            // 第二步：计算平均值并写回所有Mono通道
            const float scale = 1.0f / static_cast<float>(monoChannelCount);
            for (int i = 0; i < monoChannelCount; ++i) {
                const int chIndex = monoChannelIndices[i];
                if (chIndex < buffer.getNumChannels() && channelIsActive[chIndex]) {
                    float* dst = buffer.getWritePointer(chIndex) + offset;
                    for (int s = 0; s < samplesToProcess; ++s) {
                        dst[s] = monoSum[s] * scale;
                    }
                }
            }
        }
    }
    
    // 禁用拷贝构造和赋值（确保POD特性）
    RenderState(const RenderState&) = delete;
    RenderState& operator=(const RenderState&) = delete;
};