/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin processor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "InterPluginCommunicator.h"
#include "DebugLogger.h"

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
      apvts (*this, nullptr, "Parameters", createParameterLayout()),
      currentRole(standalone)
#endif
{
    // 初始化VST3调试日志系统
    DebugLogger::getInstance().initialize("MonitorControllerMax");
    VST3_DBG("=== MonitorControllerMax Plugin Constructor ===");
    
    communicator.reset(new InterPluginCommunicator(*this));
    
    // Initialize semantic state system
    semanticState.addStateChangeListener(this);
    
    VST3_DBG("Initialize semantic state system");
    
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
    
    VST3_DBG("Semantic state system initialization complete");
    
    // Initialize OSC communication system
    VST3_DBG("Initialize OSC communication system");
    
    // 设置OSC外部控制回调
    oscCommunicator.onExternalStateChange = [this](const juce::String& action, const juce::String& channelName, bool state) 
    {
        // 处理外部OSC控制，更新语义状态
        handleExternalOSCControl(action, channelName, state);
    };
    
    // 尝试初始化OSC连接
    if (oscCommunicator.initialize())
    {
        VST3_DBG("OSC communication system initialized successfully");
    }
    else
    {
        VST3_DBG("OSC communication system initialization failed - continuing without OSC");
    }
    
    // Initialize legacy StateManager (will be phased out)
}



MonitorControllerMaxAudioProcessor::~MonitorControllerMaxAudioProcessor()
{
    VST3_DBG("PluginProcessor: Destructor - cleaning up resources");
    
    // Shutdown OSC communication
    oscCommunicator.shutdown();
    
    // Only remove Gain parameter listeners - Solo/Mute parameters no longer exist
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto gainId = "GAIN_" + juce::String(i + 1);
        apvts.removeParameterListener(gainId, this);
    }
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
    VST3_DBG("PluginProcessor: prepareToPlay - sampleRate: " << sampleRate << ", samplesPerBlock: " << samplesPerBlock);
    
    // 只处理保留的Gain参数
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto gainId = "GAIN_" + juce::String(i + 1);
        gainParams[i] = apvts.getRawParameterValue(gainId);
        apvts.addParameterListener(gainId, this);
    }
    
    // 根据当前总线布局自动选择合适的配置
    int currentChannelCount = getTotalNumInputChannels();
    if (currentChannelCount > 0)
    {
        autoSelectLayoutForChannelCount(currentChannelCount);
    }
    
    // 插件准备就绪后，广播所有当前状态到OSC
    VST3_DBG("PluginProcessor: Broadcasting initial states to OSC");
    oscCommunicator.broadcastAllStates(semanticState, physicalMapper);
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

    // 清除任何多余的输出通道，以防万一 (例如，从单声道到立体声)
    for (auto i = totalNumInputChannels; i < totalNumOutputChannels; ++i)
        buffer.clear (i, 0, buffer.getNumSamples());

    // =================================================================================
    // NEW: Semantic state system processing (additional layer) - TEMPORARILY DISABLED
    // =================================================================================
    
    // TODO: Apply semantic state processing for mapped channels
    // This will be enabled after basic compilation is working
    /*
    for (int physicalPin = 0; physicalPin < totalNumInputChannels; ++physicalPin)
    {
        // Get semantic channel name for this physical pin (if mapped)
        juce::String semanticName = physicalMapper.getSemanticNameSafe(physicalPin);
        
        if (!semanticName.isEmpty())
        {
            // Apply semantic state to physical audio
            bool semanticFinalMute = semanticState.getFinalMuteState(semanticName);
            
            if (semanticFinalMute)
            {
                // Semantic system overrides: mute this channel
                buffer.clear(physicalPin, 0, buffer.getNumSamples());
                // Skip further processing for this channel
                continue;
            }
        }
    }
    */

    // =================================================================================
    // 1. 确定所有 *在当前布局中激活的* 逻辑通道的最终 Mute/Solo 状态
    // =================================================================================

    bool anySoloEngaged = false;
    const auto role = getRole();

    // 检查是否有任何一个 *语义通道* 被 solo
    anySoloEngaged = (role == Role::slave) ? false : semanticState.hasAnySoloActive();

    // =================================================================================
    // 2. 将处理逻辑应用到物理音频缓冲区 (全新逻辑)
    //    这个循环严格遍历宿主提供的物理通道
    // =================================================================================
    
    for (int physicalChannel = 0; physicalChannel < totalNumInputChannels; ++physicalChannel)
    {
        // 获取对应的语义通道名
        juce::String semanticChannelName = physicalMapper.getSemanticName(physicalChannel);
        
        // 如果这个物理通道没有语义映射，我们就跳过它，实现音频直通
        if (semanticChannelName.isEmpty())
        {
            continue;
        }

        // --- 从这里开始，我们确认了 physicalChannel 对应一个有效的语义通道 ---

        // 获取这个语义通道的最终Mute状态（已包含Solo模式联动逻辑）
        const bool shouldBeSilent = (role == Role::slave) ? false : semanticState.getFinalMuteState(semanticChannelName);

        if (shouldBeSilent)
        {
            buffer.clear(physicalChannel, 0, buffer.getNumSamples());
        }
        else
        {
            // 应用增益 - 从物理通道索引获取对应的Gain参数
            const float gainDb = apvts.getRawParameterValue("GAIN_" + juce::String(physicalChannel + 1))->load();
            if (std::abs(gainDb) > 0.01f)
            {
                buffer.applyGain(physicalChannel, 0, buffer.getNumSamples(), juce::Decibels::decibelsToGain(gainDb));
            }
        }
    }
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
    
    // 保存当前布局配置到状态中
    if (!currentLayout.channels.empty())
    {
        // 从当前布局推断speaker和sub配置
        juce::String currentSpeaker = "7.1.4";  // 默认值
        juce::String currentSub = "None";       // 默认值
        
        // 根据通道数推断配置（简化逻辑）
        int totalChannels = currentLayout.totalChannelCount;
        bool hasSUB = false;
        
        // 检查是否有SUB通道
        for (const auto& channel : currentLayout.channels)
        {
            if (channel.name.contains("SUB"))
            {
                hasSUB = true;
                break;
            }
        }
        
        // 推断speaker配置
        if (totalChannels >= 20 || (totalChannels >= 16 && !hasSUB))
        {
            currentSpeaker = "7.1.4.4";
        }
        else if (totalChannels >= 12 && !hasSUB)
        {
            currentSpeaker = "7.1.4";
        }
        
        // 推断sub配置
        if (hasSUB)
        {
            currentSub = "4";  // 假设是4个SUB
        }
        
        // 添加布局信息到状态
        state.setProperty("currentSpeakerLayout", currentSpeaker, nullptr);
        state.setProperty("currentSubLayout", currentSub, nullptr);
        
        VST3_DBG("PluginProcessor: Saving layout state - Speaker: " + currentSpeaker + ", Sub: " + currentSub);
    }
    
    auto xml = state.createXml();
    copyXmlToBinary(*xml, destData);
}

void MonitorControllerMaxAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    // 使用APVTS内建的状态恢复功能 - 这是JUCE推荐的标准方法
    auto xmlState = getXmlFromBinary(data, sizeInBytes);
    
    bool layoutRestored = false;
    
    if (xmlState.get() != nullptr)
    {
        if (xmlState->hasTagName(apvts.state.getType()))
        {
            auto state = juce::ValueTree::fromXml(*xmlState);
            apvts.replaceState(state);
            
            // 尝试恢复保存的布局配置
            if (state.hasProperty("currentSpeakerLayout") && state.hasProperty("currentSubLayout"))
            {
                juce::String savedSpeaker = state.getProperty("currentSpeakerLayout", "7.1.4").toString();
                juce::String savedSub = state.getProperty("currentSubLayout", "None").toString();
                
                VST3_DBG("PluginProcessor: Restoring saved layout - Speaker: " + savedSpeaker + ", Sub: " + savedSub);
                
                // 恢复保存的布局配置
                setCurrentLayout(savedSpeaker, savedSub);
                layoutRestored = true;
            }
        }
    }
    
    // 重要修复：状态恢复后立即重置到干净状态
    VST3_DBG("Performing post-state-restore clean reset");
    semanticState.clearAllStates();
    
    // 只有在没有恢复到保存布局时，才根据通道数自动选择布局
    if (!layoutRestored)
    {
        VST3_DBG("PluginProcessor: No saved layout found, auto-selecting based on channel count");
        int currentChannelCount = getTotalNumInputChannels();
        if (currentChannelCount > 0)
        {
            autoSelectLayoutForChannelCount(currentChannelCount);
        }
    }
    else
    {
        VST3_DBG("PluginProcessor: Layout successfully restored from saved state");
    }
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
    VST3_DBG("Update config layout - Speaker: " + speaker + ", Sub: " + sub);
    
    // 只更新内部状态，不再尝试改变总线布局
    currentLayout = configManager.getLayoutFor(speaker, sub);

    // Update semantic state system mapping
    VST3_DBG("Update physical channel mapping");
    physicalMapper.updateMapping(currentLayout);
    
    // Clear and reinitialize semantic channels
    semanticState.clearAllStates();
    for (const auto& channelInfo : currentLayout.channels)
    {
        semanticState.initializeChannel(channelInfo.name);
        VST3_DBG("Initialize semantic channel: " + channelInfo.name + " -> physical pin " + juce::String(channelInfo.channelIndex));
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
    juce::String bestSpeakerLayout = "2.0"; // 默认立体声
    juce::String bestSubLayout = "None";     // 默认无低音炮
    
    // 根据通道数自动匹配最合适的布局
    switch (channelCount)
    {
        case 1:
            bestSpeakerLayout = "1.0"; // 单声道
            break;
        case 2:
            bestSpeakerLayout = "2.0"; // 立体声
            break;
        case 3:
            bestSpeakerLayout = "2.0"; // 立体声
            bestSubLayout = "Single Sub"; // 加单个低音炮
            break;
        case 4:
            bestSpeakerLayout = "2.0"; // 立体声  
            bestSubLayout = "Dual Sub"; // 加双低音炮
            break;
        case 5:
            bestSpeakerLayout = "5.0"; // 5.0环绕声
            break;
        case 6:
            bestSpeakerLayout = "5.1"; // 5.1环绕声
            break;
        case 7:
            bestSpeakerLayout = "7.0"; // 7.0环绕声
            break;
        case 8:
            bestSpeakerLayout = "7.1"; // 7.1环绕声
            break;
        case 10:
            bestSpeakerLayout = "7.1.2"; // 7.1.2杜比全景声
            break;
        case 12:
            bestSpeakerLayout = "7.1.4"; // 7.1.4杜比全景声
            break;
        case 16:
            bestSpeakerLayout = "7.1.4.4"; // 7.1.4.4杜比全景声
            break;
        case 20:
            bestSpeakerLayout = "7.1.4.4"; // 7.1.4.4杜比全景声
            bestSubLayout = "4"; // 4个SUB通道
            break;
        default:
            // 对于其他通道数，选择最接近的配置
            if (channelCount > 20)
                bestSpeakerLayout = "7.1.4.4";
            else if (channelCount > 16)
                bestSpeakerLayout = "7.1.4.4";
            else if (channelCount > 12)
                bestSpeakerLayout = "7.1.4";
            else if (channelCount > 8)
                bestSpeakerLayout = "7.1.2";
            else if (channelCount > 6)
                bestSpeakerLayout = "7.1";
            break;
    }
    
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

    // Only create Gain parameters - Solo/Mute are now handled by semantic state system
    for (int i = 0; i < maxParams; ++i)
    {
        juce::String chanNumStr = juce::String(i + 1);
        
        // Only Gain parameters remain for VST3 automation
        params.push_back(std::make_unique<juce::AudioParameterFloat>("GAIN_" + chanNumStr, "Gain " + chanNumStr, 
                                                                    juce::NormalisableRange<float>(-100.0f, 12.0f, 0.1f, 3.0f), 0.0f, "dB"));
    }

    return { params.begin(), params.end() };
}

//==============================================================================
// This creates new instances of the plugin..
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new MonitorControllerMaxAudioProcessor();
}

void MonitorControllerMaxAudioProcessor::setRole(Role newRole)
{
    currentRole = newRole;
}

MonitorControllerMaxAudioProcessor::Role MonitorControllerMaxAudioProcessor::getRole() const
{
    return currentRole;
}

void MonitorControllerMaxAudioProcessor::setRemoteMuteSoloState(const MuteSoloState& state)
{
    for (int i = 0; i < currentLayout.totalChannelCount; ++i)
    {
        remoteMutes[i] = state.mutes[i];
        remoteSolos[i] = state.solos[i];
    }
}

bool MonitorControllerMaxAudioProcessor::getRemoteMuteState(int channel) const
{
    if (juce::isPositiveAndBelow(channel, configManager.getMaxChannelIndex()))
        return remoteMutes[channel].load();
    return false;
}

bool MonitorControllerMaxAudioProcessor::getRemoteSoloState(int channel) const
{
    if (juce::isPositiveAndBelow(channel, configManager.getMaxChannelIndex()))
        return remoteSolos[channel].load();
    return false;
}

void MonitorControllerMaxAudioProcessor::parameterChanged(const juce::String& parameterID, float newValue)
{
    VST3_DBG("Parameter changed: " << parameterID << " = " << newValue);
    
    // Only handle Gain parameters now - Solo/Mute are managed by semantic state system
    if (parameterID.startsWith("GAIN_"))
    {
        // Gain parameter changes can be logged or processed if needed
        VST3_DBG("Gain parameter updated: " << parameterID << " = " << newValue << " dB");
    }
    
    // Note: Solo/Mute parameters no longer exist in VST3 parameter system
    // They are now handled by the semantic state system directly
}

// =============================================================================
// New unified parameter linkage interface
// =============================================================================

void MonitorControllerMaxAudioProcessor::handleSoloButtonClick()
{
    VST3_DBG("Solo button clicked - using semantic state system");
    
    if (semanticState.hasAnySoloActive()) {
        // 状态1：有Solo状态激活 - 清除所有Solo状态并恢复Mute记忆
        VST3_DBG("Clearing all Solo states and restoring Mute memory");
        
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
        VST3_DBG("Exiting Solo selection mode and restoring Mute memory");
        
        // 恢复之前保存的Mute记忆状态
        semanticState.restoreMuteMemory();
        
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
        
    } else {
        // 状态3：初始状态 - 进入Solo选择模式
        // → 保存当前Mute记忆 + 清空所有当前Mute状态 + 进入Solo选择模式
        VST3_DBG("Entering Solo selection mode - saving Mute memory and clearing current Mute states");
        
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
    VST3_DBG("Mute button clicked - using semantic state system");
    
    // Solo Priority Rule: If any Solo state is active, Mute button is disabled
    if (semanticState.hasAnySoloActive()) {
        VST3_DBG("Mute button ignored - Solo priority rule active");
        return;
    }
    
    if (semanticState.hasAnyMuteActive()) {
        // 状态1：有Mute状态激活 - 清除所有Mute状态
        VST3_DBG("Clearing all Mute states");
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
        
        // 清除所有Mute状态
        semanticState.clearAllMuteStates();
        
    } else if (pendingMuteSelection.load()) {
        // 状态2：无Mute状态，但在Mute选择模式 - 退出选择模式
        VST3_DBG("Exiting Mute selection mode - returning to initial state");
        pendingSoloSelection.store(false);
        pendingMuteSelection.store(false);
    } else {
        // 状态3：初始状态 - 进入Mute选择模式
        VST3_DBG("Entering Mute selection mode - waiting for channel clicks");
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
    VST3_DBG("isInSoloSelectionMode: pending=" << (pendingSoloSelection.load() ? "true" : "false") << " active=" << (semanticState.hasAnySoloActive() ? "true" : "false") << " result=" << (result ? "true" : "false"));
    return result;
}

bool MonitorControllerMaxAudioProcessor::isInMuteSelectionMode() const
{
    // Mute选择模式：待定Mute选择或已有Mute参数激活时（且没有Solo优先级干扰）
    bool result = (pendingMuteSelection.load() || semanticState.hasAnyMuteActive()) && !semanticState.hasAnySoloActive();
    VST3_DBG("isInMuteSelectionMode: pending=" << (pendingMuteSelection.load() ? "true" : "false") << " active=" << (semanticState.hasAnyMuteActive() ? "true" : "false") << " soloActive=" << (semanticState.hasAnySoloActive() ? "true" : "false") << " result=" << (result ? "true" : "false"));
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
        VST3_DBG("Invalid channel index: " << channelIndex);
        return;
    }
    
    VST3_DBG("Channel click: " << channelIndex);
    
    // Get semantic channel name from physical channel index
    juce::String semanticChannelName = physicalMapper.getSemanticName(channelIndex);
    
    // Skip unmapped channels
    if (semanticChannelName.isEmpty()) {
        VST3_DBG("Channel " << channelIndex << " has no semantic mapping - no effect");
        return;
    }
    
    // 检查当前的选择模式状态
    bool inSoloSelection = isInSoloSelectionMode();
    bool inMuteSelection = isInMuteSelectionMode();
    
    VST3_DBG("Channel click state - SoloSel:" << (inSoloSelection ? "true" : "false") << " MuteSel:" << (inMuteSelection ? "true" : "false"));
    
    if (inSoloSelection) {
        // Solo选择模式 -> 切换该语义通道的Solo状态
        bool currentSolo = semanticState.getSoloState(semanticChannelName);
        bool newSolo = !currentSolo;
        semanticState.setSoloState(semanticChannelName, newSolo);
        VST3_DBG("Channel " << channelIndex << " (" << semanticChannelName << ") Solo toggled: " << (newSolo ? "ON" : "OFF"));
        
        // CRITICAL FIX: 不再自动清除待定选择状态，保持在选择模式中
        // pendingSoloSelection.store(false);  // 移除这行
    } else if (inMuteSelection) {
        // Mute选择模式 -> 切换该语义通道的Mute状态
        bool currentMute = semanticState.getMuteState(semanticChannelName);
        bool newMute = !currentMute;
        semanticState.setMuteState(semanticChannelName, newMute);
        VST3_DBG("Channel " << channelIndex << " (" << semanticChannelName << ") Mute toggled: " << (newMute ? "ON" : "OFF"));
        
        // CRITICAL FIX: 不再自动清除待定选择状态，保持在选择模式中
        // pendingMuteSelection.store(false);  // 移除这行
    } else {
        // 初始状态: 通道点击无效果
        VST3_DBG("Channel clicked in Initial state - no effect");
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
    VST3_DBG("State check - Solo:" << (soloActive ? "true" : "false") << " Mute:" << (muteActive ? "true" : "false") 
             << " SoloSel:" << (soloSelection ? "true" : "false") << " MuteSel:" << (muteSelection ? "true" : "false"));
    
    // CRITICAL FIX: 只修复真正不合理的状态组合
    if (soloActive && muteSelection) {
        VST3_DBG("WARNING: Inconsistent state - Solo active but Mute selection pending - auto-fixing");
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
    VST3_DBG("PluginProcessor: Solo state change callback - channel: " + channelName + ", new state: " + (state ? "ON" : "OFF"));
    
    // Log the change for debugging
    semanticState.logCurrentState();
    
    // Send OSC state change notification
    oscCommunicator.sendSoloState(channelName, state);
    
    // Trigger UI updates if needed
    // (UI should use timer-based updates to poll semantic state)
}

void MonitorControllerMaxAudioProcessor::onMuteStateChanged(const juce::String& channelName, bool state)
{
    VST3_DBG("PluginProcessor: Mute state change callback - channel: " + channelName + ", new state: " + (state ? "ON" : "OFF"));
    
    // Log the change for debugging
    semanticState.logCurrentState();
    
    // Send OSC state change notification
    oscCommunicator.sendMuteState(channelName, state);
    
    // Trigger UI updates if needed
    // (UI should use timer-based updates to poll semantic state)
}

void MonitorControllerMaxAudioProcessor::onGlobalModeChanged()
{
    bool isGlobalSoloModeActive = semanticState.isGlobalSoloModeActive();
    VST3_DBG("PluginProcessor: Global mode change callback - Solo mode: " + juce::String(isGlobalSoloModeActive ? "ACTIVE" : "OFF"));
    
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
    VST3_DBG("PluginProcessor: Handle external OSC control - action: " + action + 
             ", channel: " + channelName + ", state: " + (state ? "ON" : "OFF"));
    
    // 验证通道名称是否在当前映射中存在
    if (!physicalMapper.hasSemanticChannel(channelName))
    {
        VST3_DBG("PluginProcessor: OSC control for unmapped channel ignored - " + channelName);
        return;
    }
    
    // 根据action类型和state值更新对应的语义状态
    if (action == "Solo")
    {
        semanticState.setSoloState(channelName, state);
        VST3_DBG("PluginProcessor: External OSC " + juce::String(state ? "activated" : "deactivated") + " Solo for channel " + channelName);
    }
    else if (action == "Mute")
    {
        semanticState.setMuteState(channelName, state);
        VST3_DBG("PluginProcessor: External OSC " + juce::String(state ? "activated" : "deactivated") + " Mute for channel " + channelName);
    }
    else
    {
        VST3_DBG("PluginProcessor: Unknown OSC action - " + action);
    }
}

