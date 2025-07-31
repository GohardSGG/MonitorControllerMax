/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin processor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DebugLogger.h"
#include "MasterBusProcessor.h"

//==============================================================================
MonitorControllerMaxAudioProcessor::MonitorControllerMaxAudioProcessor()
#ifndef JucePlugin_PreferredChannelConfigurations
     : AudioProcessor (BusesProperties()
                     #if ! JucePlugin_IsMidiEffect
                      #if ! JucePlugin_IsSynth
                       .withInput  ("Input",  juce::AudioChannelSet::discreteChannels(26), true)
                      #endif
                       .withOutput ("Output", juce::AudioChannelSet::discreteChannels(26), true)
                     #endif
                       ),
      apvts (*this, nullptr, "Parameters", createParameterLayout())
#endif
{
    // 初始化智能VST3调试日志系统 - INFO级别，过滤重复内容
    DebugLogger::getInstance().initialize("MonitorControllerMax", LogLevel::INFO);
    VST3_DBG_CRITICAL("=== MonitorControllerMax Plugin Constructor ===");
    
    // 注册到GlobalPluginState
    registerToGlobalState();
    
    // Initialize semantic state system
    semanticState.setProcessor(this);  // 设置角色日志支持
    semanticState.addStateChangeListener(this);
    
    // Initialize physical channel mapper
    physicalMapper.setProcessor(this);  // 设置角色日志支持
    
    VST3_DBG_ROLE(this, "Initialize semantic state system");
    
    // Initialize with default layout if available
    if (!currentLayout.channels.empty())
    {
        physicalMapper.updateMapping(currentLayout);
        
        // Initialize semantic channels
        for (const auto& channelInfo : currentLayout.channels)
        {
            semanticState.initializeChannel(channelInfo.name);
        }
    }
    
    VST3_DBG_ROLE(this, "Semantic state system initialization complete");
    
    // 设置OSC的processor指针用于角色日志
    oscCommunicator.setProcessor(this);
    
    // v4.1: 初始化总线处理器
    masterBusProcessor.setProcessor(this);
    VST3_DBG_ROLE(this, "MasterBusProcessor initialized");
    
    // JUCE架构重构：初始化状态管理器
    stateManager = std::make_unique<StateManager>(*this);
    VST3_DBG_ROLE(this, "StateManager initialized - JUCE-compliant architecture active");
    
    // 设置OSC外部控制回调（所有角色都设置，但只有Master/Standalone处理）
    oscCommunicator.onExternalStateChange = [this](const juce::String& action, const juce::String& channelName, bool state) 
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            handleExternalOSCControl(action, channelName, state);
        } else {
            VST3_DBG_ROLE(this, "OSC control ignored - Slave mode does not process OSC");
        }
    };
    
    // v4.1: 设置Master总线OSC控制回调
    oscCommunicator.onMasterVolumeOSC = [this](float volumePercent)
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            masterBusProcessor.handleOSCMasterVolume(volumePercent);
            
            // 同步到VST3参数
            // JUCE的AudioParameterFloat::setValueNotifyingHost()总是需要归一化值(0.0-1.0)
            // 无论参数定义的范围是什么，都需要归一化
            auto* masterGainParam = apvts.getParameter("MASTER_GAIN");
            if (masterGainParam != nullptr) {
                // OSC: 0-100% -> 归一化值: volumePercent/100.0f
                float normalizedValue = volumePercent / 100.0f;
                masterGainParam->setValueNotifyingHost(normalizedValue);
                
                VST3_DBG_ROLE(this, "OSC Master Volume: " << volumePercent << "% -> normalized: " << normalizedValue);
            }
        } else {
            VST3_DBG_ROLE(this, "Master Volume OSC ignored - Slave mode");
        }
    };
    
    oscCommunicator.onMasterDimOSC = [this](bool dimState)
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            masterBusProcessor.handleOSCDim(dimState);
        } else {
            VST3_DBG_ROLE(this, "Master Dim OSC ignored - Slave mode");
        }
    };
    
    oscCommunicator.onMasterLowBoostOSC = [this](bool lowBoostState)
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            masterBusProcessor.handleOSCLowBoost(lowBoostState);
        } else {
            VST3_DBG_ROLE(this, "Master Low Boost OSC ignored - Slave mode");
        }
    };
    
    oscCommunicator.onMasterMonoOSC = [this](bool monoState)
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            masterBusProcessor.handleOSCMono(monoState);
        } else {
            VST3_DBG_ROLE(this, "Master Mono OSC ignored - Slave mode");
        }
    };
    
    oscCommunicator.onMasterMuteOSC = [this](bool masterMuteState)
    {
        // 只有Master和Standalone处理外部OSC控制
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            masterBusProcessor.handleOSCMasterMute(masterMuteState);
        } else {
            VST3_DBG_ROLE(this, "Master Mute OSC ignored - Slave mode");
        }
    };
    
    // 重要：OSC系统将在角色确定后初始化（在setStateInformation或UI初始化完成后）
    VST3_DBG_ROLE(this, "OSC initialization deferred until role is determined");
    
    // Initialize legacy StateManager (will be phased out)
}



MonitorControllerMaxAudioProcessor::~MonitorControllerMaxAudioProcessor()
{
    VST3_DBG_ROLE(this, "Destructor - cleaning up resources");
    
    // Shutdown OSC communication
    oscCommunicator.shutdown();
    
    // Remove parameter listeners - Solo/Mute parameters no longer exist
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto gainId = "GAIN_" + juce::String(i + 1);
        apvts.removeParameterListener(gainId, this);
    }
    
    // v4.1: 移除Master Gain参数监听器
    apvts.removeParameterListener("MASTER_GAIN", this);
    
    // 注销GlobalPluginState
    unregisterFromGlobalState();
    
    // 手动关闭日志系统并清理日志文件
    DebugLogger::getInstance().shutdown();
}

//==============================================================================
const juce::String MonitorControllerMaxAudioProcessor::getName() const
{
    return JucePlugin_Name;
}

bool MonitorControllerMaxAudioProcessor::acceptsMidi() const
{
   #if JucePlugin_WantsMidiInput
    return true;
   #else
    return false;
   #endif
}

bool MonitorControllerMaxAudioProcessor::producesMidi() const
{
   #if JucePlugin_ProducesMidiOutput
    return true;
   #else
    return false;
   #endif
}

bool MonitorControllerMaxAudioProcessor::isMidiEffect() const
{
   #if JucePlugin_IsMidiEffect
    return true;
   #else
    return false;
   #endif
}

double MonitorControllerMaxAudioProcessor::getTailLengthSeconds() const
{
    return 0.0;
}

int MonitorControllerMaxAudioProcessor::getNumPrograms()
{
    return 1;   // NB: some hosts don't cope very well if you tell them there are 0 programs,
                // so this should be at least 1, even if you're not really implementing programs.
}

int MonitorControllerMaxAudioProcessor::getCurrentProgram()
{
    return 0;
}

void MonitorControllerMaxAudioProcessor::setCurrentProgram (int index)
{
}

const juce::String MonitorControllerMaxAudioProcessor::getProgramName (int index)
{
    return {};
}

void MonitorControllerMaxAudioProcessor::changeProgramName (int index, const juce::String& newName)
{
}

//==============================================================================
void MonitorControllerMaxAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    VST3_DBG_ROLE(this, "prepareToPlay - sampleRate: " << sampleRate << ", samplesPerBlock: " << samplesPerBlock);
    
    // 处理个人通道Gain参数
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto gainId = "GAIN_" + juce::String(i + 1);
        gainParams[i] = apvts.getRawParameterValue(gainId);
        apvts.addParameterListener(gainId, this);
    }
    
    // v4.1: 处理Master Gain参数
    apvts.addParameterListener("MASTER_GAIN", this);
    
    // 🚀 稳定性优化第3步：初始化预分配音频缓冲区，消除音频线程中的内存分配
    masterBusProcessor.prepare(sampleRate, samplesPerBlock);
    VST3_DBG_ROLE(this, "MasterBusProcessor prepared with preallocated buffers - sampleRate: " << sampleRate << ", maxBlockSize: " << samplesPerBlock);
    
    // 根据当前总线布局自动选择合适的配置
    int currentChannelCount = getTotalNumInputChannels();
    if (currentChannelCount > 0)
    {
        autoSelectLayoutForChannelCount(currentChannelCount);
    }
    
    // OSC广播现在在角色确定后由initializeOSCForRole()处理
    // 如果是首次加载且没有保存的状态，初始化默认角色的OSC
    if (currentRole == PluginRole::Standalone) {
        // 第一次加载时的默认角色初始化
        initializeOSCForRole();
    }
}

void MonitorControllerMaxAudioProcessor::releaseResources()
{
    // When playback stops, you can use this as an opportunity to free up any
    // spare memory, etc.
}

#ifndef JucePlugin_PreferredChannelConfigurations
bool MonitorControllerMaxAudioProcessor::isBusesLayoutSupported (const BusesLayout& layouts) const
{
  #if JucePlugin_IsMidiEffect
    juce::ignoreUnused (layouts);
    return true;
  #else
    // 检查输入输出总线是否匹配
    if (layouts.getMainInputChannelSet() == layouts.getMainOutputChannelSet()
        && !layouts.getMainInputChannelSet().isDisabled())
    {
        // 获取请求的通道数
        int requestedChannelCount = layouts.getMainInputChannelSet().size();
        
        // 支持1到26个通道的任意配置
        if (requestedChannelCount >= 1 && requestedChannelCount <= 26)
        {
            return true;
        }
    }

    return false;
  #endif
}
#endif

void MonitorControllerMaxAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;
    auto totalNumInputChannels  = getTotalNumInputChannels();
    auto totalNumOutputChannels = getTotalNumOutputChannels();
    
    // JUCE架构重构：使用新的无锁处理模式
    if (stateManager != nullptr) {
        // 获取当前渲染状态（单个原子操作）
        const RenderState* renderState = stateManager->getCurrentRenderState();
        if (renderState != nullptr) {
            // 应用预计算的渲染状态（高度优化的内联函数）
            renderState->applyToBuffer(buffer, buffer.getNumSamples());
            
            // 调试：每秒输出一次状态
            static int debugCounter = 0;
            if (++debugCounter > getSampleRate()) {
                debugCounter = 0;
                VST3_DBG_ROLE(this, "New architecture active - RenderState version: " << renderState->version.load());
            }
            
            return;  // 新架构处理完成，直接返回
        }
    }
    
    // 旧架构已被新的StateManager完全替代
    // 如果StateManager未初始化，清除所有音频输出作为安全措施
    VST3_DBG_ROLE(this, "WARNING: StateManager not initialized, clearing audio buffer");
    buffer.clear();

    // =================================================================================
    // 旧架构已完全移除 - 所有处理由StateManager负责
    // =================================================================================
    
    /* 以下是旧架构的代码，已被新的StateManager完全替代：
     * - 语义状态处理
     * - Master-Slave音频处理逻辑
     * - Solo/Mute状态应用
     * - 通道增益应用
     * - 总线效果处理
     * 
     * 新架构优势：
     * - 音频线程零锁设计
     * - 预计算所有状态
     * - 单个原子操作读取
     * - 符合JUCE实时音频规范
     */
    
    // 旧代码开始（已禁用）
    #if 0
    
    // =================================================================================
    // Master-Slave Audio Processing Logic (v4.0 Architecture)
    // =================================================================================
    
    // 确定处理策略
    bool hasNonSUBSolo = semanticState.hasAnyNonSUBSoloActive();
    bool hasSUBSolo = semanticState.hasAnySUBSoloActive();
    bool processingEnabled = false;
    
    // 场景判断和处理策略
    if (!hasNonSUBSolo && !hasSUBSolo) {
        // 无Solo模式：所有角色正常处理所有通道
        processingEnabled = true;
        // VST3_DBG_ROLE(this, "Audio Processing: No Solo mode - all channels processed");
    }
    else if (hasNonSUBSolo && !hasSUBSolo) {
        // 场景1：只有非SUB通道Solo
        processingEnabled = (currentRole == PluginRole::Slave);  // 从插件处理，主插件直通
        // VST3_DBG_ROLE(this, "Audio Processing: Non-SUB Solo only - " + 
        //              juce::String(processingEnabled ? "Slave processes" : "Master passthrough"));
    }
    else if (!hasNonSUBSolo && hasSUBSolo) {
        // 场景2：只有SUB通道Solo
        processingEnabled = (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone);  // 主插件处理，从插件直通
        // VST3_DBG_ROLE(this, "Audio Processing: SUB Solo only - " + 
        //              juce::String(processingEnabled ? "Master processes" : "Slave passthrough"));
    }
    else {
        // 场景3：混合Solo（非SUB + SUB）
        processingEnabled = (currentRole == PluginRole::Slave);  // 从插件处理，主插件直通
        // VST3_DBG_ROLE(this, "Audio Processing: Mixed Solo - " + 
        //              juce::String(processingEnabled ? "Slave processes" : "Master passthrough"));
    }
    
    // 应用处理逻辑到所有物理通道
    for (int physicalChannel = 0; physicalChannel < totalNumInputChannels; ++physicalChannel)
    {
        // 获取对应的语义通道名
        juce::String semanticChannelName = physicalMapper.getSemanticName(physicalChannel);
        
        // 如果这个物理通道没有语义映射，直通
        if (semanticChannelName.isEmpty())
        {
            continue;
        }

        // 判断是否为SUB通道
        bool isSUBChannel = semanticState.isSUBChannel(semanticChannelName);
        bool shouldProcess = false;
        
        // 根据场景和通道类型决定是否处理
        if (!hasNonSUBSolo && !hasSUBSolo) {
            // 无Solo：所有通道都处理
            shouldProcess = processingEnabled;
        }
        else if (hasNonSUBSolo && !hasSUBSolo) {
            // 场景1：从插件处理非SUB，主插件处理SUB
            if (currentRole == PluginRole::Slave) {
                shouldProcess = !isSUBChannel;  // 从插件只处理非SUB通道
            } else {
                shouldProcess = isSUBChannel;   // 主插件只处理SUB通道
            }
        }
        else if (!hasNonSUBSolo && hasSUBSolo) {
            // 场景2：主插件处理所有，从插件直通
            if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
                if (isSUBChannel) {
                    shouldProcess = true;  // SUB通道正常处理
                } else {
                    // 非SUB通道强制静音（SUB Solo逻辑）
                    buffer.clear(physicalChannel, 0, buffer.getNumSamples());
                    continue;
                }
            } else {
                shouldProcess = false;  // 从插件直通
            }
        }
        else {
            // 场景3：从插件处理非SUB，主插件处理SUB
            if (currentRole == PluginRole::Slave) {
                shouldProcess = !isSUBChannel;  // 从插件只处理非SUB通道
            } else {
                shouldProcess = isSUBChannel;   // 主插件只处理SUB通道
            }
        }
        
        if (shouldProcess) {
            // 获取最终Mute状态并应用
            bool finalMuteState = semanticState.getFinalMuteState(semanticChannelName);
            
            if (finalMuteState) {
                // 静音此通道
                buffer.clear(physicalChannel, 0, buffer.getNumSamples());
            } else {
                // v4.1: 根据角色决定是否应用个人通道Gain
                // Slave插件：只处理Solo/Mute状态，不处理Gain
                // Master/Standalone插件：处理Solo/Mute状态和Gain
                if (currentRole != PluginRole::Slave) {
                    // 应用个人通道增益 (Master/Standalone)
                    const float gainDb = apvts.getRawParameterValue("GAIN_" + juce::String(physicalChannel + 1))->load();
                    if (std::abs(gainDb) > 0.01f) {
                        buffer.applyGain(physicalChannel, 0, buffer.getNumSamples(), juce::Decibels::decibelsToGain(gainDb));
                    }
                }
                // 如果是Slave插件：音频直通，不应用Gain
            }
        }
        // 如果不应该处理，音频直通（不做任何修改）
    }
    
    // v4.1: 最后应用总线效果 (Master Gain + Dim) - 所有角色都应用
    // Slave: 只Solo/Mute状态处理 + 总线效果
    // Master/Standalone: Solo/Mute状态 + 个人通道Gain + 总线效果
    masterBusProcessor.process(buffer, currentRole);
    
    #endif // 旧代码结束
}

//==============================================================================
bool MonitorControllerMaxAudioProcessor::hasEditor() const
{
    return true; // (change this to false if you choose to not supply an editor)
}

juce::AudioProcessorEditor* MonitorControllerMaxAudioProcessor::createEditor()
{
    return new MonitorControllerMaxAudioProcessorEditor (*this);
}

//==============================================================================
void MonitorControllerMaxAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    // 使用APVTS内建的状态保存功能
    auto state = apvts.copyState();
    
    // 重要修复：保存用户实际选择的布局配置，而不是推断的配置
    // 这样可以保持用户的手动选择，防止界面切换时配置被重置
    state.setProperty("currentSpeakerLayout", userSelectedSpeakerLayout, nullptr);
    state.setProperty("currentSubLayout", userSelectedSubLayout, nullptr);
    
    // 保存角色信息
    state.setProperty("pluginRole", static_cast<int>(currentRole), nullptr);
    
    // 🎯 用户需求：完全移除Solo/Mute状态的持久化保存
    // 只保留Gain参数、角色、布局配置的持久化，确保插件重新加载时Solo/Mute状态为干净初始状态
    // Note: Solo/Mute状态在DAW会话期间（窗口关闭/重开）仍然通过内存对象维持
    
    VST3_DBG_DETAIL("PluginProcessor: Saving complete state - Layout: " + userSelectedSpeakerLayout + " + " + userSelectedSubLayout + 
             ", Role: " + juce::String(static_cast<int>(currentRole)) + 
             " (Solo/Mute states NOT saved - clean startup policy)");
    
    auto xml = state.createXml();
    copyXmlToBinary(*xml, destData);
}

void MonitorControllerMaxAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    // 使用APVTS内建的状态恢复功能 - 这是JUCE推荐的标准方法
    auto xmlState = getXmlFromBinary(data, sizeInBytes);
    
    if (xmlState.get() != nullptr)
    {
        if (xmlState->hasTagName(apvts.state.getType()))
        {
            auto state = juce::ValueTree::fromXml(*xmlState);
            apvts.replaceState(state);
            
            // 恢复角色信息
            if (state.hasProperty("pluginRole")) {
                int savedRoleInt = state.getProperty("pluginRole", 0);
                PluginRole savedRole = static_cast<PluginRole>(savedRoleInt);
                VST3_DBG_ROLE(this, "PluginProcessor: Restoring plugin role - " + juce::String(savedRoleInt));
                
                // 重要修复：不能只设置currentRole，必须调用正确的切换方法来触发连接逻辑
                switch (savedRole) {
                    case PluginRole::Standalone:
                        switchToStandalone();
                        break;
                    case PluginRole::Master:
                        switchToMaster();
                        break;
                    case PluginRole::Slave:
                        switchToSlave();  // 这会触发等待队列逻辑
                        break;
                    default:
                        switchToStandalone();
                        break;
                }
                VST3_DBG_ROLE(this, "Plugin role restoration complete - connection logic triggered");
            }
            
            // 恢复用户选择的布局配置（如果存在）
            if (state.hasProperty("currentSpeakerLayout") && state.hasProperty("currentSubLayout"))
            {
                juce::String savedSpeaker = state.getProperty("currentSpeakerLayout", "2.0").toString();
                juce::String savedSub = state.getProperty("currentSubLayout", "None").toString();
                
                VST3_DBG_ROLE(this, "Restoring user-selected layout - Speaker: " + savedSpeaker + ", Sub: " + savedSub);
                
                // 重要修复：只恢复用户选择变量，不立即应用布局
                // 这样可以避免与UI初始化的冲突
                userSelectedSpeakerLayout = savedSpeaker;
                userSelectedSubLayout = savedSub;
                
                // 延迟应用布局，让UI有时间初始化
                juce::MessageManager::callAsync([this, savedSpeaker, savedSub]()
                {
                    VST3_DBG_ROLE(this, "Applying restored layout after UI initialization");
                    setCurrentLayout(savedSpeaker, savedSub);
                    
                    // 🎯 用户需求：不再恢复Solo/Mute状态，确保干净启动
                    VST3_DBG_ROLE(this, "Layout restored - Solo/Mute states remain clean for fresh start");
                });
            }
        }
    }
    
    // 🎯 状态恢复策略更新：保留Gain、角色、布局配置，Solo/Mute状态始终干净启动
    VST3_DBG_ROLE(this, "State restoration complete - Gain/Role/Layout restored, Solo/Mute clean");
    
    // 角色确定后初始化OSC系统
    initializeOSCForRole();
}

// 动态获取输入通道名称，根据当前音箱布局映射物理通道到逻辑声道名
// channelIndex: 物理通道索引（从0开始）
// 返回: 对应的声道名称（如"LFE"）或默认名称
const juce::String MonitorControllerMaxAudioProcessor::getInputChannelName(int channelIndex) const
{
    // 获取当前输入总线的通道数
    int totalChannels = getTotalNumInputChannels();
    
    // 检查通道索引是否有效
    if (channelIndex >= 0 && channelIndex < totalChannels)
    {
        // 根据总通道数和通道索引返回标准通道名称
        if (totalChannels == 2)
        {
            // 立体声
            if (channelIndex == 0) return "Left";
            if (channelIndex == 1) return "Right";
        }
        else if (totalChannels == 6)
        {
            // 5.1环绕声
            if (channelIndex == 0) return "Left";
            if (channelIndex == 1) return "Right";
            if (channelIndex == 2) return "Centre";
            if (channelIndex == 3) return "LFE";
            if (channelIndex == 4) return "Left Surround";
            if (channelIndex == 5) return "Right Surround";
        }
        else if (totalChannels == 8)
        {
            // 7.1环绕声
            if (channelIndex == 0) return "Left";
            if (channelIndex == 1) return "Right";
            if (channelIndex == 2) return "Centre";
            if (channelIndex == 3) return "LFE";
            if (channelIndex == 4) return "Left Surround";
            if (channelIndex == 5) return "Right Surround";
            if (channelIndex == 6) return "Left Side";
            if (channelIndex == 7) return "Right Side";
        }
        else if (totalChannels == 12)
        {
            // 7.1.4杜比全景声
            if (channelIndex == 0) return "Left";
            if (channelIndex == 1) return "Right";
            if (channelIndex == 2) return "Centre";
            if (channelIndex == 3) return "LFE";
            if (channelIndex == 4) return "Left Surround";
            if (channelIndex == 5) return "Right Surround";
            if (channelIndex == 6) return "Left Side";
            if (channelIndex == 7) return "Right Side";
            if (channelIndex == 8) return "Top Front Left";
            if (channelIndex == 9) return "Top Front Right";
            if (channelIndex == 10) return "Top Rear Left";
            if (channelIndex == 11) return "Top Rear Right";
        }
    }
    
    // 回退到默认名称
    return "Input " + juce::String(channelIndex + 1);
}

// 动态获取输出通道名称，与输入通道使用相同的映射逻辑
// channelIndex: 物理通道索引（从0开始）
// 返回: 对应的声道名称（如"LFE"）或默认名称
const juce::String MonitorControllerMaxAudioProcessor::getOutputChannelName(int channelIndex) const
{
    // 复用输入通道名称逻辑，但替换前缀
    auto inputName = getInputChannelName(channelIndex);
    
    // 如果是默认格式，替换为Output前缀
    if (inputName.startsWith("Input "))
    {
        return "Output " + inputName.substring(6);
    }
    
    // 否则直接返回通道名称（如"Left", "Right", "Centre"等）
    return inputName;
}

void MonitorControllerMaxAudioProcessor::setCurrentLayout(const juce::String& speaker, const juce::String& sub)
{
    // 删除重复的配置更新日志 - 会被多次调用产生垃圾信息
    
    // 跟踪用户实际选择的布局配置，用于状态持久化
    userSelectedSpeakerLayout = speaker;
    userSelectedSubLayout = sub;
    
    // 只更新内部状态，不再尝试改变总线布局
    currentLayout = configManager.getLayoutFor(speaker, sub);

    // Update semantic state system mapping
    VST3_DBG_ROLE(this, "Update physical channel mapping");
    physicalMapper.updateMapping(currentLayout);
    
    // 智能更新语义通道：只初始化新通道，保持现有状态
    VST3_DBG_ROLE(this, "Smart channel update - preserving existing states");
    for (const auto& channelInfo : currentLayout.channels)
    {
        // 只初始化不存在的通道，保持已有状态
        if (!semanticState.hasChannel(channelInfo.name)) {
            semanticState.initializeChannel(channelInfo.name);
            VST3_DBG_ROLE(this, "Initialize new semantic channel: " + channelInfo.name + " -> physical pin " + juce::String(channelInfo.channelIndex));
        }
    }
    
    // Log current mapping
    physicalMapper.logCurrentMapping();
    semanticState.logCurrentState();

    // 立即请求宿主更新显示信息
    updateHostDisplay();
    
    // 为REAPER等DAW添加延迟的额外刷新 - 某些DAW需要多次通知
    juce::Timer::callAfterDelay(50, [this]()
    {
        updateHostDisplay();
    });
    
    juce::Timer::callAfterDelay(200, [this]()
    {
        updateHostDisplay();
    });
}

const Layout& MonitorControllerMaxAudioProcessor::getCurrentLayout() const
{
    return currentLayout;
}

int MonitorControllerMaxAudioProcessor::getAvailableChannels() const
{
    return getTotalNumInputChannels();
}

// 根据通道数自动选择最合适的布局配置
void MonitorControllerMaxAudioProcessor::autoSelectLayoutForChannelCount(int channelCount)
{
    juce::String bestSpeakerLayout = "2.0"; // 保守的默认值
    juce::String bestSubLayout = "None";     // 默认无低音炮
    
    // 动态最佳匹配算法 - 自动找到最充分利用通道数的配置组合
    auto speakerLayoutNames = configManager.getSpeakerLayoutNames();
    auto subLayoutNames = configManager.getSubLayoutNames();
    
    int bestChannelUsage = 0;
    for (const auto& speaker : speakerLayoutNames)
    {
        int speakerChannels = configManager.getChannelCountForLayout("Speaker", speaker);
        
        for (const auto& sub : subLayoutNames)
        {
            int subChannels = configManager.getChannelCountForLayout("SUB", sub);
            int totalChannels = speakerChannels + subChannels;
            
            // 找到在可用通道内的最大使用量
            if (totalChannels <= channelCount && totalChannels > bestChannelUsage)
            {
                bestChannelUsage = totalChannels;
                bestSpeakerLayout = speaker;
                bestSubLayout = sub;
            }
        }
    }
    
    VST3_DBG_ROLE(this, "AutoSelect: " + juce::String(channelCount) + " channels -> " + bestSpeakerLayout + " + " + bestSubLayout + " (" + juce::String(bestChannelUsage) + " used)");
    
    // 应用新的布局配置
    setCurrentLayout(bestSpeakerLayout, bestSubLayout);
    
    // 通知UI更新下拉框选择
    if (onLayoutAutoChanged)
    {
        onLayoutAutoChanged(bestSpeakerLayout, bestSubLayout);
    }
}

// 设置UI更新回调
void MonitorControllerMaxAudioProcessor::setLayoutChangeCallback(std::function<void(const juce::String&, const juce::String&)> callback)
{
    onLayoutAutoChanged = callback;
}


juce::AudioProcessorValueTreeState::ParameterLayout MonitorControllerMaxAudioProcessor::createParameterLayout()
{
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;
    
    const int maxParams = 26; 

    // Create individual channel Gain parameters - Solo/Mute are handled by semantic state system
    for (int i = 0; i < maxParams; ++i)
    {
        juce::String chanNumStr = juce::String(i + 1);
        
        // Individual channel Gain parameters for VST3 automation
        params.push_back(std::make_unique<juce::AudioParameterFloat>("GAIN_" + chanNumStr, "Gain " + chanNumStr, 
                                                                    juce::NormalisableRange<float>(-100.0f, 12.0f, 0.1f, 3.0f), 0.0f, "dB"));
    }
    
    // v4.1: 添加Master Gain总线参数 (基于JSFX实现: 0-100%)
    params.push_back(std::make_unique<juce::AudioParameterFloat>("MASTER_GAIN", "Master Gain", 
                                                                juce::NormalisableRange<float>(0.0f, 100.0f, 0.1f), 100.0f, "%"));

    return { params.begin(), params.end() };
}

//==============================================================================
// This creates new instances of the plugin..
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new MonitorControllerMaxAudioProcessor();
}


void MonitorControllerMaxAudioProcessor::parameterChanged(const juce::String& parameterID, float newValue)
{
    // 删除垃圾日志 - 参数变化高频调用
    
    // Handle individual channel Gain parameters - Solo/Mute are managed by semantic state system
    if (parameterID.startsWith("GAIN_"))
    {
        // Individual channel gain parameter changes can be logged or processed if needed
        // 删除垃圾日志 - 增益参数更新高频调用
    }
    // v4.1: Handle Master Gain parameter
    else if (parameterID == "MASTER_GAIN")
    {
        // 同步Master Gain到总线处理器
        masterBusProcessor.setMasterGainPercent(newValue);
        
        // 发送OSC消息 (只有Master/Standalone发送)
        if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
            oscCommunicator.sendMasterVolume(newValue);
        }
        
        // 删除垃圾日志 - Master Gain参数更新调用
        // VST3_DBG_ROLE(this, "Master Gain parameter changed: " << newValue << "%");
    }
    
    // Note: Solo/Mute parameters no longer exist in VST3 parameter system
    // They are now handled by the semantic state system directly
}

// =============================================================================
// New unified parameter linkage interface
// =============================================================================

void MonitorControllerMaxAudioProcessor::handleSoloButtonClick()
{
    VST3_DBG_ROLE(this, "Solo button clicked - using StateManager");
    
    // 使用新的StateManager架构
    if (stateManager) {
        // StateManager会处理所有逻辑
        // TODO: 实现Solo按钮的复杂逻辑
        return;
    }
    
    // 以下是旧架构的备用处理
    if (semanticState.hasAnySoloActive()) {
        // 状态1：有Solo状态激活 - 清除所有Solo状态并恢复Mute记忆
        VST3_DBG_ROLE(this, "Clearing all Solo states and restoring Mute memory");
        
        // 清除选择模式
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
        
        // 清除所有Solo状态
        semanticState.clearAllSoloStates();
        
        // 恢复之前保存的Mute记忆状态
        semanticState.restoreMuteMemory();
        
        // 关闭保护状态
        soloModeProtectionActive = false;
        
    } else if (pendingSoloSelection.load()) {
        // 状态2：无Solo状态，但在Solo选择模式 - 退出选择模式并恢复记忆
        VST3_DBG_ROLE(this, "Exiting Solo selection mode and restoring Mute memory");
        
        // 恢复之前保存的Mute记忆状态
        semanticState.restoreMuteMemory();
        
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
        
    } else {
        // 状态3：初始状态 - 进入Solo选择模式
        // → 保存当前Mute记忆 + 清空所有当前Mute状态 + 进入Solo选择模式
        VST3_DBG_ROLE(this, "Entering Solo selection mode - saving Mute memory and clearing current Mute states");
        
        // 保存当前Mute记忆并清空现场，让UI显示干净状态
        semanticState.saveCurrentMuteMemory();
        semanticState.clearAllMuteStates();
        
        pendingSoloSelection.store(true);
        pendingMuteSelection.store(false);  // 切换到Solo选择模式会取消Mute选择模式
    }
    
    // 更新所有状态
    updateAllStates();
}

void MonitorControllerMaxAudioProcessor::handleMuteButtonClick()
{
    VST3_DBG_ROLE(this, "Mute button clicked - using semantic state system");
    
    // Solo Priority Rule: If any Solo state is active, Mute button is disabled
    if (semanticState.hasAnySoloActive()) {
        VST3_DBG_ROLE(this, "Mute button ignored - Solo priority rule active");
        return;
    }
    
    if (semanticState.hasAnyMuteActive()) {
        // 状态1：有Mute状态激活 - 清除所有Mute状态
        VST3_DBG_ROLE(this, "Clearing all Mute states");
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
        
        // 清除所有Mute状态
        semanticState.clearAllMuteStates();
        
    } else if (pendingMuteSelection.load()) {
        // 状态2：无Mute状态，但在Mute选择模式 - 退出选择模式
        VST3_DBG_ROLE(this, "Exiting Mute selection mode - returning to initial state");
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
    } else {
        // 状态3：初始状态 - 进入Mute选择模式
        VST3_DBG_ROLE(this, "Entering Mute selection mode - waiting for channel clicks");
        pendingMuteSelection.store(true);
        pendingSoloSelection.store(false);  // 切换到Mute选择模式会取消Solo选择模式
    }
    
    // 更新所有状态
    updateAllStates();
}

bool MonitorControllerMaxAudioProcessor::hasAnySoloActive() const
{
    return semanticState.hasAnySoloActive();
}

bool MonitorControllerMaxAudioProcessor::hasAnyMuteActive() const
{
    return semanticState.hasAnyMuteActive();
}

// Selection mode state functions based on button activation
bool MonitorControllerMaxAudioProcessor::isInSoloSelectionMode() const
{
    // Solo选择模式：待定Solo选择或已有Solo参数激活时
    bool result = pendingSoloSelection.load() || semanticState.hasAnySoloActive();
    // 删除垃圾日志 - 选择模式状态检查高频调用
    return result;
}

bool MonitorControllerMaxAudioProcessor::isInMuteSelectionMode() const
{
    // Mute选择模式：待定Mute选择或已有Mute参数激活时（且没有Solo优先级干扰）
    bool result = (pendingMuteSelection.load() || semanticState.hasAnyMuteActive()) && !semanticState.hasAnySoloActive();
    // 删除垃圾日志 - 选择模式状态检查高频调用
    return result;
}

// Dual state button activation functions
bool MonitorControllerMaxAudioProcessor::isSoloButtonActive() const
{
    // Solo按钮激活状态 = 有通道被Solo OR 处于Solo选择模式
    return semanticState.hasAnySoloActive() || pendingSoloSelection.load();
}

bool MonitorControllerMaxAudioProcessor::isMuteButtonActive() const
{
    // Mute按钮激活状态 = 有通道被Mute OR 处于Mute选择模式（且没有Solo优先级干扰）
    return (semanticState.hasAnyMuteActive() || pendingMuteSelection.load()) && !semanticState.hasAnySoloActive();
}

void MonitorControllerMaxAudioProcessor::handleChannelClick(int channelIndex)
{
    // Validate channel index
    if (channelIndex < 0 || channelIndex >= 26) {
        VST3_DBG_ROLE(this, "Invalid channel index: " + juce::String(channelIndex));
        return;
    }
    
    VST3_DBG_ROLE(this, "Channel click: " << channelIndex);
    
    // Get semantic channel name from physical channel index
    juce::String semanticChannelName = physicalMapper.getSemanticName(channelIndex);
    
    // Skip unmapped channels
    if (semanticChannelName.isEmpty()) {
        VST3_DBG_ROLE(this, "Channel " + juce::String(channelIndex) + " has no semantic mapping - no effect");
        return;
    }
    
    // 检查当前的选择模式状态
    bool inSoloSelection = isInSoloSelectionMode();
    bool inMuteSelection = isInMuteSelectionMode();
    
    // 删除垃圾日志 - 内部状态检查
    
    if (inSoloSelection) {
        // Solo选择模式 -> 切换该语义通道的Solo状态
        bool currentSolo = semanticState.getSoloState(semanticChannelName);
        bool newSolo = !currentSolo;
        semanticState.setSoloState(semanticChannelName, newSolo);
        // 删除垃圾日志 - 重复的状态变更信息
        
        // CRITICAL FIX: 不再自动清除待定选择状态，保持在选择模式中
        // pendingSoloSelection.store(false);  // 移除这行
    } else if (inMuteSelection) {
        // Mute选择模式 -> 切换该语义通道的Mute状态
        bool currentMute = semanticState.getMuteState(semanticChannelName);
        bool newMute = !currentMute;
        semanticState.setMuteState(semanticChannelName, newMute);
        // 删除垃圾日志 - 重复的状态变更信息
        
        // CRITICAL FIX: 不再自动清除待定选择状态，保持在选择模式中
        // pendingMuteSelection.store(false);  // 移除这行
    } else {
        // 初始状态: 通道点击无效果
        // 删除垃圾日志 - 无意义的状态提示
    }
}


bool MonitorControllerMaxAudioProcessor::isMuteButtonEnabled() const
{
    // Mute button is disabled when any Solo parameter is active (Solo Priority Rule)
    return !semanticState.hasAnySoloActive();
}


// State synchronization and validation functions

void MonitorControllerMaxAudioProcessor::updateAllStates()
{
    // 1. 更新语义状态 (这里不需要更新，语义状态会自动管理)
    bool currentSoloActive = semanticState.hasAnySoloActive();
    bool currentMuteActive = semanticState.hasAnyMuteActive();
    
    // 2. 更新保护状态 (语义状态系统内部管理，无需外部保护)
    
    // 3. 通知UI更新
    // UI会在定时器中自动查询最新状态
    
    // 4. 验证状态一致性
    validateStateConsistency();
}

void MonitorControllerMaxAudioProcessor::validateStateConsistency()
{
    // 验证状态标志的一致性
    bool soloActive = hasAnySoloActive();
    bool muteActive = hasAnyMuteActive();
    bool soloSelection = pendingSoloSelection.load();
    bool muteSelection = pendingMuteSelection.load();
    
    // 记录状态用于调试
    // 删除垃圾日志 - 状态检查高频调用
    
    // CRITICAL FIX: 只修复真正不合理的状态组合
    if (soloActive && muteSelection) {
        VST3_DBG_ROLE(this, "WARNING: Inconsistent state - Solo active but Mute selection pending - auto-fixing");
        pendingMuteSelection.store(false);
    }
    
    // REMOVED: 删除错误的孤立状态检查
    // 正常情况：!soloActive && !muteActive && !soloSelection && muteSelection
    // 这是用户点击Mute按钮进入选择模式的正常状态，不应该被清除
}

//==============================================================================
// Semantic state change callbacks
//==============================================================================

void MonitorControllerMaxAudioProcessor::onSoloStateChanged(const juce::String& channelName, bool state)
{
    VST3_DBG_ROLE(this, "Solo state change callback - channel: " + channelName + ", new state: " + (state ? "ON" : "OFF"));
    
    // 使用新的统一状态处理方法
    onSemanticStateChanged(channelName, "solo", state);
}

void MonitorControllerMaxAudioProcessor::onMuteStateChanged(const juce::String& channelName, bool state)
{
    VST3_DBG_ROLE(this, "Mute state change callback - channel: " + channelName + ", new state: " + (state ? "ON" : "OFF"));
    
    // 使用新的统一状态处理方法
    onSemanticStateChanged(channelName, "mute", state);
}

void MonitorControllerMaxAudioProcessor::onGlobalModeChanged()
{
    bool isGlobalSoloModeActive = semanticState.isGlobalSoloModeActive();
    VST3_DBG_ROLE(this, "Global mode change callback - Solo mode: " + juce::String(isGlobalSoloModeActive ? "ACTIVE" : "OFF"));
    
    // Log complete state for debugging
    semanticState.logCurrentState();
    
    // Broadcast all states when global mode changes
    oscCommunicator.broadcastAllStates(semanticState, physicalMapper);
    
    // Trigger UI updates if needed
    // (UI should use timer-based updates to poll semantic state)
}

//==============================================================================
// OSC external control handler
//==============================================================================

void MonitorControllerMaxAudioProcessor::handleExternalOSCControl(const juce::String& action, const juce::String& channelName, bool state)
{
    VST3_DBG_ROLE(this, "Handle external OSC control - action: " + action + 
             ", channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
    // v4.1: 处理总线级别的OSC控制
    if (action == "Master")
    {
        if (channelName == "Dim")
        {
            // OSC控制Dim: /Monitor/Master/Dim
            masterBusProcessor.handleOSCDim(state);
            return;
        }
        else if (channelName == "Volume")
        {
            // OSC控制Master Volume: /Monitor/Master/Volume
            // 注意：这里state参数作为float值使用，需要特殊处理
            float volumePercent = state ? 100.0f : 0.0f;  // 简化处理，实际应该传递float值
            masterBusProcessor.handleOSCMasterVolume(volumePercent);
            return;
        }
    }
    
    // 验证通道名称是否在当前映射中存在
    if (!physicalMapper.hasSemanticChannel(channelName))
    {
        VST3_DBG_ROLE(this, "OSC control for unmapped channel ignored - " + channelName);
        return;
    }
    
    // 根据action类型和state值更新对应的语义状态
    if (action == "Solo")
    {
        semanticState.setSoloState(channelName, state);
        VST3_DBG_ROLE(this, "External OSC " + juce::String(state ? "activated" : "deactivated") + " Solo for channel " + channelName);
    }
    else if (action == "Mute")
    {
        semanticState.setMuteState(channelName, state);
        VST3_DBG_ROLE(this, "External OSC " + juce::String(state ? "activated" : "deactivated") + " Mute for channel " + channelName);
    }
    else
    {
        VST3_DBG_ROLE(this, "Unknown OSC action - " + action);
    }
}

void MonitorControllerMaxAudioProcessor::sendDimOSCState(bool dimState)
{
    // v4.1: 发送Dim状态OSC消息 (只有Master/Standalone发送)
    if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
        oscCommunicator.sendMasterDim(dimState);
    }
}

void MonitorControllerMaxAudioProcessor::sendLowBoostOSCState(bool lowBoostState)
{
    // v4.1: 发送Low Boost状态OSC消息 (只有Master/Standalone发送)
    if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
        oscCommunicator.sendMasterLowBoost(lowBoostState);
    }
}

void MonitorControllerMaxAudioProcessor::sendMasterMuteOSCState(bool masterMuteState)
{
    // v4.1: 发送Master Mute状态OSC消息 (只有Master/Standalone发送)
    if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
        oscCommunicator.sendMasterMute(masterMuteState);
    }
}

void MonitorControllerMaxAudioProcessor::sendMonoOSCState(bool monoState)
{
    // v4.1: 发送Mono状态OSC消息 (只有Master/Standalone发送)
    if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
        oscCommunicator.sendMasterMono(monoState);
    }
}

//==============================================================================
// Master-Slave角色管理实现

void MonitorControllerMaxAudioProcessor::registerToGlobalState() {
    if (!isRegisteredToGlobalState) {
        GlobalPluginState::getInstance().registerPlugin(this);
        isRegisteredToGlobalState = true;
        VST3_DBG_ROLE(this, "Plugin registered to GlobalPluginState");
    }
}

void MonitorControllerMaxAudioProcessor::unregisterFromGlobalState() {
    if (isRegisteredToGlobalState) {
        GlobalPluginState::getInstance().unregisterPlugin(this);
        isRegisteredToGlobalState = false;
        VST3_DBG_ROLE(this, "Plugin unregistered from GlobalPluginState");
    }
}

void MonitorControllerMaxAudioProcessor::switchToStandalone() {
    if (currentRole == PluginRole::Standalone) return;
    
    auto& globalState = GlobalPluginState::getInstance();
    
    if (currentRole == PluginRole::Master) {
        globalState.removeMaster(this);
    } else if (currentRole == PluginRole::Slave) {
        globalState.removeSlavePlugin(this);
    }
    
    handleRoleTransition(PluginRole::Standalone);
    VST3_DBG_ROLE(this, "Successfully switched to Standalone mode");
}

void MonitorControllerMaxAudioProcessor::switchToMaster() {
    if (currentRole == PluginRole::Master) return;
    
    auto& globalState = GlobalPluginState::getInstance();
    
    if (globalState.setAsMaster(this)) {
        if (currentRole == PluginRole::Slave) {
            globalState.removeSlavePlugin(this);
        }
        
        handleRoleTransition(PluginRole::Master);
        VST3_DBG_ROLE(this, "Successfully switched to Master mode");
        
        // 同步当前状态到所有Slave
        auto activeChannels = physicalMapper.getActiveSemanticChannels();
        for (const auto& channelName : activeChannels) {
            bool soloState = semanticState.getSoloState(channelName);
            bool muteState = semanticState.getMuteState(channelName);
            
            globalState.setGlobalSoloState(channelName, soloState);
            globalState.setGlobalMuteState(channelName, muteState);
            globalState.broadcastStateToSlaves(channelName, "solo", soloState);
            globalState.broadcastStateToSlaves(channelName, "mute", muteState);
        }
    } else {
        VST3_DBG_ROLE(this, "Failed to switch to Master - another Master exists");
    }
}

void MonitorControllerMaxAudioProcessor::switchToSlave() {
    auto& globalState = GlobalPluginState::getInstance();
    
    if (currentRole == PluginRole::Master) {
        globalState.removeMaster(this);
    }
    
    if (globalState.addSlavePlugin(this)) {
        handleRoleTransition(PluginRole::Slave);
        
        // 同步Master状态到本地
        globalState.syncAllStatesToSlave(this);
        VST3_DBG_ROLE(this, "Successfully switched to Slave mode");
    } else {
        VST3_DBG_ROLE(this, "Failed to switch to Slave - no Master available");
        switchToStandalone();
    }
}

void MonitorControllerMaxAudioProcessor::handleRoleTransition(PluginRole newRole) {
    PluginRole oldRole = currentRole;
    currentRole = newRole;
    savedRole = newRole;  // 保存角色用于状态持久化
    
    VST3_DBG_ROLE(this, "Role transition: " + getRoleString(oldRole) + " -> " + getRoleString(newRole));
    
    // 重要：角色变化时重新初始化OSC系统
    initializeOSCForRole();
    
    // 异步更新UI
    juce::MessageManager::callAsync([this]() {
        updateUIFromRole();
    });
}

void MonitorControllerMaxAudioProcessor::updateUIFromRole() {
    if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
        editor->updateUIBasedOnRole();
    }
}

void MonitorControllerMaxAudioProcessor::receiveMasterState(const juce::String& channelName, const juce::String& action, bool state) {
    if (currentRole != PluginRole::Slave) return;
    
    // 防止循环回调
    suppressStateChange = true;
    
    try {
        // 应用Master状态到本地语义状态
        if (action == "solo") {
            semanticState.setSoloState(channelName, state);
        } else if (action == "mute") {
            semanticState.setMuteState(channelName, state);
        }
        
        VST3_DBG_ROLE(this, "Slave received Master state: " + action + " " + channelName + " = " + (state ? "true" : "false"));
        
        // 异步通知UI更新
        juce::MessageManager::callAsync([this]() {
            if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
                editor->updateChannelButtonStates();
            }
        });
        
    } catch (const std::exception& e) {
        VST3_DBG_ROLE(this, "Error receiving Master state: " + juce::String(e.what()));
    }
    
    // 重新启用回调
    suppressStateChange = false;
}

// v4.1: 接收Master总线效果状态 - 用于Master-Slave同步
void MonitorControllerMaxAudioProcessor::receiveMasterBusState(const juce::String& busEffect, bool state) {
    if (currentRole != PluginRole::Slave) return;
    
    // 防止循环回调
    suppressStateChange = true;
    
    try {
        // 应用Master总线效果状态到本地MasterBusProcessor
        if (busEffect == "mono") {
            masterBusProcessor.setMonoActive(state);
        }
        // 未来可以添加其他总线效果，如dim、lowBoost等
        
        VST3_DBG_ROLE(this, "Slave received Master bus state: " + busEffect + " = " + (state ? "ON" : "OFF"));
        
        // 异步通知UI更新
        juce::MessageManager::callAsync([this]() {
            if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
                editor->updateChannelButtonStates();
            }
        });
        
    } catch (const std::exception& e) {
        VST3_DBG_ROLE(this, "Error receiving Master bus state: " + juce::String(e.what()));
    }
    
    // 重新启用回调
    suppressStateChange = false;
}

void MonitorControllerMaxAudioProcessor::onMasterDisconnected() {
    if (currentRole == PluginRole::Slave) {
        VST3_DBG_ROLE(this, "Master disconnected - switching to Standalone");
        switchToStandalone();
    }
}

void MonitorControllerMaxAudioProcessor::onMasterConnected() {
    if (currentRole == PluginRole::Standalone) {
        VST3_DBG_ROLE(this, "Master connected - switching to Slave");
        switchToSlave();
    }
}

bool MonitorControllerMaxAudioProcessor::isMasterWithSlaves() const {
    return currentRole == PluginRole::Master && GlobalPluginState::getInstance().getSlaveCount() > 0;
}

bool MonitorControllerMaxAudioProcessor::isSlaveConnected() const {
    return currentRole == PluginRole::Slave && GlobalPluginState::getInstance().hasMaster();
}

int MonitorControllerMaxAudioProcessor::getConnectedSlaveCount() const {
    if (currentRole == PluginRole::Master) {
        return GlobalPluginState::getInstance().getSlaveCount();
    }
    return 0;
}

juce::String MonitorControllerMaxAudioProcessor::getConnectionStatusText() const {
    auto& globalState = GlobalPluginState::getInstance();
    
    switch (currentRole) {
        case PluginRole::Standalone:
            return "Standalone";
            
        case PluginRole::Master:
            if (globalState.getSlaveCount() > 0) {
                return "Master (" + juce::String(globalState.getSlaveCount()) + " slaves)";
            } else {
                return "Master (no slaves)";
            }
            
        case PluginRole::Slave:
            if (globalState.hasMaster()) {
                return "Slave (connected)";
            } else {
                return "Slave (no master)";
            }
    }
    
    return "Unknown";
}

void MonitorControllerMaxAudioProcessor::saveCurrentUIState() {
    // 保存当前选中的通道信息用于UI刷新时恢复
    // 这里可以添加更多需要保存的UI状态
    savedSelectedChannels = "";  // 实际实现中需要从UI获取选中状态
}

void MonitorControllerMaxAudioProcessor::restoreUIState() {
    // 恢复UI状态
    if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
        editor->updateUIBasedOnRole();
        editor->updateChannelButtonStates();
        // 恢复选中的通道状态等
    }
}

void MonitorControllerMaxAudioProcessor::restoreSemanticStates() {
    // 🎯 方法已弃用：根据用户需求，完全移除Solo/Mute状态的持久化
    // 插件重新加载时，Solo/Mute状态始终保持干净的初始状态
    // 只有Gain参数、角色选择、布局配置会被持久化保存和恢复
    
    VST3_DBG_ROLE(this, "restoreSemanticStates() called but DEPRECATED - clean startup policy active");
    VST3_DBG_ROLE(this, "Solo/Mute states remain in clean initial state for fresh plugin load");
    
    // Note: 如果将来需要恢复部分状态持久化，可以在这里重新实现
}

void MonitorControllerMaxAudioProcessor::onSemanticStateChanged(const juce::String& channelName, const juce::String& action, bool state) {
    // 防止循环回调
    if (suppressStateChange) return;
    
    // 现有OSC通信（保持不变）
    if (currentRole != PluginRole::Slave) {
        // 只有非Slave角色才发送OSC消息
        if (action == "solo") {
            oscCommunicator.sendSoloState(channelName, state);
        } else if (action == "mute") {
            oscCommunicator.sendMuteState(channelName, state);
        }
    }
    
    // 新增主从同步（最小侵入）
    if (currentRole == PluginRole::Master) {
        auto& globalState = GlobalPluginState::getInstance();
        
        // 更新全局状态
        if (action == "solo") {
            globalState.setGlobalSoloState(channelName, state);
        } else if (action == "mute") {
            globalState.setGlobalMuteState(channelName, state);
        }
        
        // 广播给所有Slave
        globalState.broadcastStateToSlaves(channelName, action, state);
    }
}

// OSC系统角色管理实现
void MonitorControllerMaxAudioProcessor::initializeOSCForRole() {
    // 先关闭旧的OSC连接
    shutdownOSC();
    
    // 根据角色决定是否初始化OSC
    bool shouldInitializeOSC = (currentRole == PluginRole::Standalone || currentRole == PluginRole::Master);
    
    if (shouldInitializeOSC) {
        VST3_DBG_ROLE(this, "Initializing OSC for role: " + getRoleString(currentRole));
        
        if (oscCommunicator.initialize()) {
            VST3_DBG_ROLE(this, "OSC communication system initialized successfully for " + getRoleString(currentRole));
            
            // 如果是主控模式，广播初始状态
            if (currentRole == PluginRole::Master || currentRole == PluginRole::Standalone) {
                VST3_DBG_ROLE(this, "Broadcasting initial states to OSC");
                oscCommunicator.broadcastAllStates(semanticState, physicalMapper);
            }
        } else {
            VST3_DBG_ROLE(this, "OSC communication system initialization failed for " + getRoleString(currentRole));
        }
    } else {
        VST3_DBG_ROLE(this, "OSC initialization skipped for Slave role");
    }
}

void MonitorControllerMaxAudioProcessor::shutdownOSC() {
    if (oscCommunicator.isConnected()) {
        VST3_DBG_ROLE(this, "Shutting down OSC communication");
        oscCommunicator.shutdown();
    }
}

juce::String MonitorControllerMaxAudioProcessor::getRoleString(PluginRole role) const {
    switch (role) {
        case PluginRole::Standalone: return "Standalone";
        case PluginRole::Master: return "Master";
        case PluginRole::Slave: return "Slave";
        default: return "Unknown";
    }
}

