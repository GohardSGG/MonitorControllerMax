/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin processor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "InterPluginCommunicator.h"

//==============================================================================
MonitorControllerMaxAudioProcessor::MonitorControllerMaxAudioProcessor()
#ifndef JucePlugin_PreferredChannelConfigurations
     : AudioProcessor (BusesProperties()
                     #if ! JucePlugin_IsMidiEffect
                      #if ! JucePlugin_IsSynth
                       .withInput  ("Input",  juce::AudioChannelSet::create7point1(), true)
                      #endif
                       .withOutput ("Output", juce::AudioChannelSet::create7point1(), true)
                     #endif
                       ),
      apvts (*this, nullptr, "Parameters", createParameterLayout()),
      currentRole(standalone)
#endif
{
    communicator.reset(new InterPluginCommunicator(*this));
}

MonitorControllerMaxAudioProcessor::~MonitorControllerMaxAudioProcessor()
{
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto muteId = "MUTE_" + juce::String(i + 1);
        auto soloId = "SOLO_" + juce::String(i + 1);
        auto gainId = "GAIN_" + juce::String(i + 1);
        apvts.removeParameterListener(muteId, this);
        apvts.removeParameterListener(soloId, this);
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
    const int maxChannels = 26;
    for (int i = 0; i < maxChannels; ++i)
    {
        auto muteId = "MUTE_" + juce::String(i + 1);
        auto soloId = "SOLO_" + juce::String(i + 1);
        auto gainId = "GAIN_" + juce::String(i + 1);

        muteParams[i] = apvts.getRawParameterValue(muteId);
        soloParams[i] = apvts.getRawParameterValue(soloId);
        gainParams[i] = apvts.getRawParameterValue(gainId);

        // Simply add the listeners. JUCE is robust enough to handle this.
        apvts.addParameterListener(muteId, this);
        apvts.addParameterListener(soloId, this);
        apvts.addParameterListener(gainId, this);
    }
    
    setCurrentLayout("7.1.4", "None");
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
    // We now accept any layout where input and output busses match.
    // The UI will be responsible for disabling layouts that are not supported by the current channel count.
    if (layouts.getMainInputChannelSet() == layouts.getMainOutputChannelSet()
        && !layouts.getMainInputChannelSet().isDisabled())
    {
        return true;
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
    // 1. 确定所有 *在当前布局中激活的* 逻辑通道的最终 Mute/Solo 状态
    // =================================================================================

    bool anySoloEngaged = false;
    const auto role = getRole();

    // 检查是否有任何一个 *逻辑通道* 被 solo
    for (const auto& chanInfo : currentLayout.channels)
    {
        // 参数索引从1开始，所以是 channelIndex + 1
        bool isSoloed = (role == Role::slave) ? remoteSolos[chanInfo.channelIndex].load()
                                              : apvts.getRawParameterValue("SOLO_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f;
        if (isSoloed)
        {
            anySoloEngaged = true;
            break;
        }
    }

    // =================================================================================
    // 2. 将处理逻辑应用到物理音频缓冲区 (全新逻辑)
    //    这个循环严格遍历宿主提供的物理通道
    // =================================================================================
    
    for (int physicalChannel = 0; physicalChannel < totalNumInputChannels; ++physicalChannel)
    {
        // 尝试将当前物理通道映射到我们布局中的一个逻辑通道
        const ChannelInfo* mappedChannelInfo = nullptr;
        for (const auto& chanInfo : currentLayout.channels)
        {
            // 我们的 `channelIndex` 在 `ConfigManager` 中是基于0的，这正好可以和物理通道索引对应
            if (chanInfo.channelIndex == physicalChannel)
            {
                mappedChannelInfo = &chanInfo;
                break;
            }
        }

        // 如果这个物理通道没有在当前布局中定义，我们就跳过它，实现音频直通
        if (mappedChannelInfo == nullptr)
        {
            continue;
        }

        // --- 从这里开始，我们确认了 physicalChannel 对应一个有效的逻辑通道 ---

        // 获取这个逻辑通道的 Mute 和 Solo 状态
        const bool isMuted = (role == Role::slave) ? remoteMutes[mappedChannelInfo->channelIndex].load()
                                                   : apvts.getRawParameterValue("MUTE_" + juce::String(mappedChannelInfo->channelIndex + 1))->load() > 0.5f;

        const bool isSoloed = (role == Role::slave) ? remoteSolos[mappedChannelInfo->channelIndex].load()
                                                    : apvts.getRawParameterValue("SOLO_" + juce::String(mappedChannelInfo->channelIndex + 1))->load() > 0.5f;
        
        // 计算最终是否应该静音
        const bool shouldBeSilent = isMuted || (anySoloEngaged && !isSoloed);

        if (shouldBeSilent)
        {
            buffer.clear(physicalChannel, 0, buffer.getNumSamples());
        }
        else
        {
            // 应用增益
            const float gainDb = apvts.getRawParameterValue("GAIN_" + juce::String(mappedChannelInfo->channelIndex + 1))->load();
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
    // You should use this method to store your parameters in the memory block.
    // You could do that either as raw data, or use the XML or ValueTree classes
    // as intermediaries to make it easy to save and load complex data.
}

void MonitorControllerMaxAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    // You should use this method to restore your parameters from this memory block,
    // whose contents will have been created by the getStateInformation() call.
}

juce::String MonitorControllerMaxAudioProcessor::getParameterName(int parameterIndex, int maximumStringLength)
{
    const int numParamsPerChannel = 3;
    const int channelIndex = parameterIndex / numParamsPerChannel;
    const int paramType = parameterIndex % numParamsPerChannel;

    for (const auto& chanInfo : currentLayout.channels)
    {
        if (chanInfo.channelIndex == channelIndex)
        {
            switch (paramType)
            {
                case 0: return "Mute " + chanInfo.name;
                case 1: return "Solo " + chanInfo.name;
                case 2: return "Gain " + chanInfo.name;
            }
        }
    }
    
    juce::String genericName = "Mute " + juce::String(channelIndex + 1);
    switch (paramType)
    {
        case 1: genericName = "Solo " + juce::String(channelIndex + 1); break;
        case 2: genericName = "Gain " + juce::String(channelIndex + 1); break;
    }
    return genericName.substring(0, maximumStringLength);
}

juce::String MonitorControllerMaxAudioProcessor::getParameterLabel(int parameterIndex) const
{
     // For now, we don't need a special label.
    return {};
}

void MonitorControllerMaxAudioProcessor::setCurrentLayout(const juce::String& speaker, const juce::String& sub)
{
    // 只更新内部状态，不再尝试改变总线布局
    currentLayout = configManager.getLayoutFor(speaker, sub);

    // 请求宿主更新参数名称等显示信息
    updateHostDisplay();
}

const Layout& MonitorControllerMaxAudioProcessor::getCurrentLayout() const
{
    return currentLayout;
}

juce::AudioProcessorValueTreeState::ParameterLayout MonitorControllerMaxAudioProcessor::createParameterLayout()
{
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;
    
    const int maxParams = 26; 

    for (int i = 0; i < maxParams; ++i)
    {
        juce::String chanNumStr = juce::String(i + 1);
        
        // Create with generic names. They will be updated by getParameterName.
        params.push_back(std::make_unique<juce::AudioParameterBool>("MUTE_" + chanNumStr, "Mute " + chanNumStr, false));
        params.push_back(std::make_unique<juce::AudioParameterBool>("SOLO_" + chanNumStr, "Solo " + chanNumStr, false));
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
    if (getRole() == Role::master)
    {
        if (parameterID.startsWith("MUTE_") || parameterID.startsWith("SOLO_"))
        {
            MuteSoloState currentState;
            // We must pack the state for ALL possible parameters, not just the active ones,
            // to ensure slaves receive a complete and consistent state object.
            for (int i = 0; i < 26; ++i)
            {
                // Note: The Parameter ID suffix is (physical channel index + 1)
                if (auto* muteParam = apvts.getRawParameterValue("MUTE_" + juce::String(i + 1)))
                    currentState.mutes[i] = muteParam->load() > 0.5f;
                else
                    currentState.mutes[i] = false;

                if (auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1)))
                    currentState.solos[i] = soloParam->load() > 0.5f;
                else
                    currentState.solos[i] = false;
            }
            communicator->sendMuteSoloState(currentState);
        }
    }
}
