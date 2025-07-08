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
                       .withInput  ("Input",  juce::AudioChannelSet::stereo(), true)
                      #endif
                       .withOutput ("Output", juce::AudioChannelSet::stereo(), true)
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
    
    // 初始化强大的新状态机
    initializeStateManager();
}

void MonitorControllerMaxAudioProcessor::initializeStateManager()
{
    stateManager = std::make_unique<StateManager>();
    
    // 首先设置状态机回调
    stateManager->setParameterUpdateCallback([this](int channelIndex, float value) {
        onParameterUpdate(channelIndex, value);
    });
    
    stateManager->setUIUpdateCallback([this]() {
        onUIUpdate();
    });
    
    // 然后完成初始化（包括内存恢复）
    stateManager->completeInitialization();
    
    VST3_DBG("StateManager initialization completed");
}

StateManager& MonitorControllerMaxAudioProcessor::getStateManager()
{
    return *stateManager;
}

void MonitorControllerMaxAudioProcessor::onParameterUpdate(int channelIndex, float value)
{
    // StateManager回调：更新JUCE参数以反映状态机的变化
    // channelIndex是0-based的通道索引，需要转换为1-based的参数ID
    
    // 安全检查：确保StateManager有效
    if (!stateManager) {
        VST3_DBG("Warning: onParameterUpdate called but StateManager is null");
        return;
    }
    
    // 验证通道索引范围
    if (channelIndex < 0 || channelIndex >= 26) {
        VST3_DBG("Warning: Invalid channel index: " << channelIndex);
        return;
    }
    
    // 获取当前通道的Solo和Mute参数ID
    auto muteParamId = "MUTE_" + juce::String(channelIndex + 1);
    auto soloParamId = "SOLO_" + juce::String(channelIndex + 1);
    
    // 获取状态机中的通道状态
    auto channelState = stateManager->getChannelState(channelIndex);
    
    // 更新Solo参数 - 基于当前通道状态
    auto* soloParam = apvts.getParameter(soloParamId);
    if (soloParam) {
        float soloValue = (channelState == ChannelState::Solo) ? 1.0f : 0.0f;
        soloParam->setValueNotifyingHost(soloValue);
    } else {
        VST3_DBG("Warning: Solo parameter not found: " << soloParamId);
    }
    
    // 更新Mute参数 - 基于当前通道状态
    auto* muteParam = apvts.getParameter(muteParamId);
    if (muteParam) {
        float muteValue = (channelState == ChannelState::ManualMute || 
                          channelState == ChannelState::AutoMute) ? 1.0f : 0.0f;
        muteParam->setValueNotifyingHost(muteValue);
    } else {
        VST3_DBG("Warning: Mute parameter not found: " << muteParamId);
    }
    
    VST3_DBG("Parameter sync update: Channel " << channelIndex << " | Solo=" << 
        (channelState == ChannelState::Solo ? "Active" : "Inactive") << 
        " | Mute=" << ((channelState == ChannelState::ManualMute || 
                      channelState == ChannelState::AutoMute) ? "Active" : "Inactive"));
}

void MonitorControllerMaxAudioProcessor::onUIUpdate()
{
    // 通知UI更新按钮状态
    // 使用MessageManager确保在主线程中执行UI更新
    juce::MessageManager::callAsync([this]() {
        // 安全检查：确保编辑器存在且有效
        if (auto* editor = dynamic_cast<MonitorControllerMaxAudioProcessorEditor*>(getActiveEditor())) {
            editor->updateChannelButtonStates();
        } else {
            VST3_DBG("UI update skipped: No active editor found");
        }
    });
    
    VST3_DBG("UI update request sent");
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
    
    // 根据当前总线布局自动选择合适的配置
    int currentChannelCount = getTotalNumInputChannels();
    if (currentChannelCount > 0)
    {
        autoSelectLayoutForChannelCount(currentChannelCount);
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
    // 保存插件状态时，同时保存状态机的记忆状态
    if (stateManager) {
        stateManager->saveMuteMemoryNow();
    }
    // 使用APVTS内建的状态保存功能
    auto state = apvts.copyState();
    
    // 新的状态机已经自动处理Mute记忆的保存
    // 无需手动保存状态标记
    
    // 状态机已处理所有必要的状态保存
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
            
            // 新的强大状态机会自动从文件系统恢复Mute记忆
            // 状态机的持久化记忆系统完全独立于APVTS状态
            if (stateManager) {
                stateManager->restoreMuteMemoryNow();
                DBG("State restoration completed - StateManager memory restored");
            }
        }
    }
    
    // 状态恢复后立即根据当前通道数选择布局
    int currentChannelCount = getTotalNumInputChannels();
    if (currentChannelCount > 0)
    {
        autoSelectLayoutForChannelCount(currentChannelCount);
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
    // 只更新内部状态，不再尝试改变总线布局
    currentLayout = configManager.getLayoutFor(speaker, sub);

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
        default:
            // 对于其他通道数，选择最接近的配置
            if (channelCount > 12)
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

// OLD WEAK LOGIC - COMMENTED OUT - NOW HANDLED BY STATEMANAGER
/*
void MonitorControllerMaxAudioProcessor::setManualMuteState(const juce::String& paramId, bool isManuallyMuted)
{
    if (isManuallyMuted)
    {
        manualMuteStates.insert(paramId);
        DBG("Manual Mute flag: " << paramId << " -> Activated");
    }
    else
    {
        manualMuteStates.erase(paramId);
        DBG("Manual Mute flag: " << paramId << " -> Cleared");
    }
}

bool MonitorControllerMaxAudioProcessor::isManuallyMuted(const juce::String& paramId) const
{
    return manualMuteStates.find(paramId) != manualMuteStates.end();
}
*/

// OLD WEAK LOGIC - COMMENTED OUT - NOW HANDLED BY STATEMANAGER
/*
void MonitorControllerMaxAudioProcessor::setSoloInducedMuteState(const juce::String& paramId, bool isSoloInduced)
{
    if (isSoloInduced)
    {
        soloInducedMuteStates.insert(paramId);
        DBG("Solo-induced Mute flag: " << paramId << " -> Activated");
    }
    else
    {
        soloInducedMuteStates.erase(paramId);
        DBG("Solo-induced Mute flag: " << paramId << " -> Cleared");
    }
}

bool MonitorControllerMaxAudioProcessor::isSoloInducedMute(const juce::String& paramId) const
{
    return soloInducedMuteStates.find(paramId) != soloInducedMuteStates.end();
}
*/

// OLD WEAK LOGIC - COMMENTED OUT - NOW HANDLED BY STATEMANAGER
/*
void MonitorControllerMaxAudioProcessor::clearAllSoloInducedMutes()
{
    // 清除所有Solo联动的Mute状态
    for (const auto& paramId : soloInducedMuteStates)
    {
        auto* muteParam = apvts.getParameter(paramId);
        if (muteParam && muteParam->getValue() > 0.5f)
        {
            muteParam->setValueNotifyingHost(0.0f);
        }
    }
    soloInducedMuteStates.clear();
}
*/

// OLD WEAK LOGIC - LARGE BLOCK COMMENTED OUT - NOW HANDLED BY STATEMANAGER
/*
void MonitorControllerMaxAudioProcessor::savePreSoloSnapshot()
{
    preSoloSnapshot.clear();
    
    // 保存当前所有通道的Mute状态
    for (int i = 0; i < 26; ++i)
    {
        auto muteParamId = "MUTE_" + juce::String(i + 1);
        auto* muteParam = apvts.getRawParameterValue(muteParamId);
        if (muteParam)
        {
            preSoloSnapshot[muteParamId] = muteParam->load() > 0.5f;
        }
    }
}

void MonitorControllerMaxAudioProcessor::restorePreSoloSnapshot()
{
    // 注意：此函数保留用于向后兼容，但新的checkSoloStateChange()中已包含智能恢复逻辑
    // 恢复到Solo前的完整状态
    for (auto it = preSoloSnapshot.begin(); it != preSoloSnapshot.end(); ++it)
    {
        auto* muteParam = apvts.getParameter(it->first);
        if (muteParam)
        {
            muteParam->setValueNotifyingHost(it->second ? 1.0f : 0.0f);
            
            // 同时更新手动Mute标记：快照中为true的就是真正的手动Mute
            setManualMuteState(it->first, it->second);
        }
    }
    
    // 清除快照和Solo联动状态
    preSoloSnapshot.clear();
    soloInducedMuteStates.clear();
}

bool MonitorControllerMaxAudioProcessor::hasPreSoloSnapshot() const
{
    return !preSoloSnapshot.empty();
}

// JS-style Solo state management - 修复版本，正确追踪状态分类
void MonitorControllerMaxAudioProcessor::checkSoloStateChange()
{
    // 检查是否有任何Solo当前激活
    bool currentSoloActive = false;
    for (int i = 0; i < 26; ++i)
    {
        auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
        if (soloParam && soloParam->load() > 0.5f)
        {
            currentSoloActive = true;
            break;
        }
    }
    
    // 只在Solo状态变化时采取行动（如JSFX代码）
    if (currentSoloActive != previousSoloActive)
    {
        if (currentSoloActive)
        {
            // 进入Solo模式：保存当前用户手动Mute状态到快照
            preSoloSnapshot.clear();
            for (int i = 0; i < 26; ++i)
            {
                auto muteParamId = "MUTE_" + juce::String(i + 1);
                auto* muteParam = apvts.getRawParameterValue(muteParamId);
                if (muteParam)
                {
                    preSoloSnapshot[muteParamId] = muteParam->load() > 0.5f;
                }
            }
            // 进入Solo时清除之前的Solo联动状态
            soloInducedMuteStates.clear();
        }
        else
        {
            // 退出Solo模式：智能恢复状态 - 只清除Solo联动的Mute，保留手动Mute
            for (const auto& paramId : soloInducedMuteStates)
            {
                // 检查这个通道是否在进入Solo前就是手动Mute的
                bool wasManuallyMuted = preSoloSnapshot.find(paramId) != preSoloSnapshot.end() && 
                                       preSoloSnapshot[paramId];
                
                if (!wasManuallyMuted)
                {
                    // 只有不是手动Mute的才清除
                    auto* muteParam = apvts.getParameter(paramId);
                    if (muteParam)
                    {
                        muteParam->setValueNotifyingHost(0.0f);
                    }
                }
            }
            
            // 恢复进入Solo前的手动Mute状态
            for (auto it = preSoloSnapshot.begin(); it != preSoloSnapshot.end(); ++it)
            {
                if (it->second) // 如果在快照中是Mute的，恢复它
                {
                    auto* muteParam = apvts.getParameter(it->first);
                    if (muteParam)
                    {
                        muteParam->setValueNotifyingHost(1.0f);
                        // 标记为手动Mute
                        setManualMuteState(it->first, true);
                    }
                }
            }
            
            preSoloSnapshot.clear();
            soloInducedMuteStates.clear();
        }
        
        previousSoloActive = currentSoloActive;
    }
    
    // 当Solo激活时应用Solo逻辑（如JSFX第204-238行）
    if (currentSoloActive)
    {
        for (int i = 0; i < 26; ++i)
        {
            auto muteParamId = "MUTE_" + juce::String(i + 1);
            auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
            auto* muteParam = apvts.getParameter(muteParamId);
            
            if (soloParam && muteParam)
            {
                // 如果这个通道是Solo的：取消静音。如果不是：静音它
                bool shouldBeMuted = soloParam->load() <= 0.5f;
                bool currentlyMuted = muteParam->getValue() > 0.5f;
                
                if (shouldBeMuted && !currentlyMuted)
                {
                    // 需要静音且当前未静音 - 这是Solo联动的Mute
                    muteParam->setValueNotifyingHost(1.0f);
                    setSoloInducedMuteState(muteParamId, true);
                }
                else if (!shouldBeMuted && currentlyMuted)
                {
                    // 需要取消静音 - 检查是否是Solo联动的Mute
                    if (isSoloInducedMute(muteParamId))
                    {
                        muteParam->setValueNotifyingHost(0.0f);
                        setSoloInducedMuteState(muteParamId, false);
                    }
                    // 如果是手动Mute，保持不变
                }
            }
        }
    }
}

// 六大原则支持函数实现
bool MonitorControllerMaxAudioProcessor::hasAnySoloActive() const
{
    for (int i = 0; i < 26; ++i)
    {
        auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
        if (soloParam && soloParam->load() > 0.5f)
        {
            return true;
        }
    }
    return false;
}

bool MonitorControllerMaxAudioProcessor::hasAnyMuteActive() const
{
    for (int i = 0; i < 26; ++i)
    {
        auto* muteParam = apvts.getRawParameterValue("MUTE_" + juce::String(i + 1));
        if (muteParam && muteParam->load() > 0.5f)
        {
            return true;
        }
    }
    return false;
}

void MonitorControllerMaxAudioProcessor::clearAllSolos()
{
    for (int i = 0; i < 26; ++i)
    {
        auto soloParamId = "SOLO_" + juce::String(i + 1);
        auto* soloParam = apvts.getParameter(soloParamId);
        if (soloParam && soloParam->getValue() > 0.5f)
        {
            soloParam->setValueNotifyingHost(0.0f);
        }
    }
}

void MonitorControllerMaxAudioProcessor::clearAllMutes()
{
    for (int i = 0; i < 26; ++i)
    {
        auto muteParamId = "MUTE_" + juce::String(i + 1);
        auto* muteParam = apvts.getParameter(muteParamId);
        if (muteParam && muteParam->getValue() > 0.5f)
        {
            muteParam->setValueNotifyingHost(0.0f);
        }
    }
    // 清除所有状态标记
    manualMuteStates.clear();
    soloInducedMuteStates.clear();
}

void MonitorControllerMaxAudioProcessor::clearAllAutoMutes()
{
    // 只清除Solo联动的Mute状态，保留手动Mute
    for (const auto& paramId : soloInducedMuteStates)
    {
        auto* muteParam = apvts.getParameter(paramId);
        if (muteParam && muteParam->getValue() > 0.5f)
        {
            muteParam->setValueNotifyingHost(0.0f);
        }
    }
    soloInducedMuteStates.clear();
}
*/

// ADDITIONAL OLD WEAK LOGIC - COMMENTED OUT - NOW HANDLED BY STATEMANAGER
/*
bool MonitorControllerMaxAudioProcessor::hasPreSoloSnapshot() const
{
    return !preSoloSnapshot.empty();
}

// JS-style Solo state management - 修复版本，正确追踪状态分类
void MonitorControllerMaxAudioProcessor::checkSoloStateChange()
{
    // 检查是否有任何Solo当前激活
    bool currentSoloActive = false;
    for (int i = 0; i < 26; ++i)
    {
        auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
        if (soloParam && soloParam->load() > 0.5f)
        {
            currentSoloActive = true;
            break;
        }
    }
    
    // 只在Solo状态变化时采取行动（如JSFX代码）
    if (currentSoloActive != previousSoloActive)
    {
        if (currentSoloActive)
        {
            // 进入Solo模式：保存当前用户手动Mute状态到快照
            preSoloSnapshot.clear();
            for (int i = 0; i < 26; ++i)
            {
                auto muteParamId = "MUTE_" + juce::String(i + 1);
                auto* muteParam = apvts.getRawParameterValue(muteParamId);
                if (muteParam)
                {
                    preSoloSnapshot[muteParamId] = muteParam->load() > 0.5f;
                }
            }
            // 进入Solo时清除之前的Solo联动状态
            soloInducedMuteStates.clear();
        }
        else
        {
            // 退出Solo模式：智能恢复状态 - 只清除Solo联动的Mute，保留手动Mute
            for (const auto& paramId : soloInducedMuteStates)
            {
                // 检查这个通道是否在进入Solo前就是手动Mute的
                bool wasManuallyMuted = preSoloSnapshot.find(paramId) != preSoloSnapshot.end() && 
                                       preSoloSnapshot[paramId];
                
                if (!wasManuallyMuted)
                {
                    // 只有不是手动Mute的才清除
                    auto* muteParam = apvts.getParameter(paramId);
                    if (muteParam)
                    {
                        muteParam->setValueNotifyingHost(0.0f);
                    }
                }
            }
            
            // 恢复进入Solo前的手动Mute状态
            for (auto it = preSoloSnapshot.begin(); it != preSoloSnapshot.end(); ++it)
            {
                if (it->second) // 如果在快照中是Mute的，恢复它
                {
                    auto* muteParam = apvts.getParameter(it->first);
                    if (muteParam)
                    {
                        muteParam->setValueNotifyingHost(1.0f);
                        // 标记为手动Mute
                        setManualMuteState(it->first, true);
                    }
                }
            }
            
            preSoloSnapshot.clear();
            soloInducedMuteStates.clear();
        }
        
        previousSoloActive = currentSoloActive;
    }
    
    // 当Solo激活时应用Solo逻辑（如JSFX第204-238行）
    if (currentSoloActive)
    {
        for (int i = 0; i < 26; ++i)
        {
            auto muteParamId = "MUTE_" + juce::String(i + 1);
            auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
            auto* muteParam = apvts.getParameter(muteParamId);
            
            if (soloParam && muteParam)
            {
                // 如果这个通道是Solo的：取消静音。如果不是：静音它
                bool shouldBeMuted = soloParam->load() <= 0.5f;
                bool currentlyMuted = muteParam->getValue() > 0.5f;
                
                if (shouldBeMuted && !currentlyMuted)
                {
                    // 需要静音且当前未静音 - 这是Solo联动的Mute
                    muteParam->setValueNotifyingHost(1.0f);
                    setSoloInducedMuteState(muteParamId, true);
                }
                else if (!shouldBeMuted && currentlyMuted)
                {
                    // 需要取消静音 - 检查是否是Solo联动的Mute
                    if (isSoloInducedMute(muteParamId))
                    {
                        muteParam->setValueNotifyingHost(0.0f);
                        setSoloInducedMuteState(muteParamId, false);
                    }
                    // 如果是手动Mute，保持不变
                }
            }
        }
    }
}

// 六大原则支持函数实现
bool MonitorControllerMaxAudioProcessor::hasAnySoloActive() const
{
    for (int i = 0; i < 26; ++i)
    {
        auto* soloParam = apvts.getRawParameterValue("SOLO_" + juce::String(i + 1));
        if (soloParam && soloParam->load() > 0.5f)
        {
            return true;
        }
    }
    return false;
}

bool MonitorControllerMaxAudioProcessor::hasAnyMuteActive() const
{
    for (int i = 0; i < 26; ++i)
    {
        auto* muteParam = apvts.getRawParameterValue("MUTE_" + juce::String(i + 1));
        if (muteParam && muteParam->load() > 0.5f)
        {
            return true;
        }
    }
    return false;
}

void MonitorControllerMaxAudioProcessor::clearAllSolos()
{
    for (int i = 0; i < 26; ++i)
    {
        auto soloParamId = "SOLO_" + juce::String(i + 1);
        auto* soloParam = apvts.getParameter(soloParamId);
        if (soloParam && soloParam->getValue() > 0.5f)
        {
            soloParam->setValueNotifyingHost(0.0f);
        }
    }
}

void MonitorControllerMaxAudioProcessor::clearAllMutes()
{
    for (int i = 0; i < 26; ++i)
    {
        auto muteParamId = "MUTE_" + juce::String(i + 1);
        auto* muteParam = apvts.getParameter(muteParamId);
        if (muteParam && muteParam->getValue() > 0.5f)
        {
            muteParam->setValueNotifyingHost(0.0f);
        }
    }
    // 清除所有状态标记
    manualMuteStates.clear();
    soloInducedMuteStates.clear();
}

void MonitorControllerMaxAudioProcessor::clearAllAutoMutes()
{
    // 只清除Solo联动的Mute状态，保留手动Mute
    for (const auto& paramId : soloInducedMuteStates)
    {
        auto* muteParam = apvts.getParameter(paramId);
        if (muteParam && muteParam->getValue() > 0.5f)
        {
            muteParam->setValueNotifyingHost(0.0f);
        }
    }
    soloInducedMuteStates.clear();
}
*/

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
