/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"

//==============================================================================
MonitorControllerMaxAudioProcessorEditor::MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor& p)
    : AudioProcessorEditor (&p), audioProcessor (p), configManager(p.configManager)
{
    addAndMakeVisible(globalMuteButton);
    globalMuteButton.setClickingTogglesState(true);
    globalMuteButton.onClick = [this]
    {
        bool anyMuteActive = false;
        for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
        {
            if (audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
            {
                anyMuteActive = true;
                break;
            }
        }

        if (anyMuteActive)
        {
            for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
            {
                audioProcessor.apvts.getParameter("MUTE_" + juce::String(chanInfo.channelIndex + 1))->setValueNotifyingHost(0.0f);
            }
            globalMuteButton.setToggleState(false, juce::sendNotification);
            currentUIMode = UIMode::Normal;
        }
        else
        {
            currentUIMode = globalMuteButton.getToggleState() ? UIMode::AssignMute : UIMode::Normal;
            if (currentUIMode == UIMode::AssignMute)
                globalSoloButton.setToggleState(false, juce::sendNotification);
        }
        updateChannelButtonStates();
    };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setClickingTogglesState(true);
    globalSoloButton.onClick = [this]
    {
        // 检查当前是否有任何Solo通道激活
        bool anySoloActive = false;
        for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
        {
            if (audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
            {
                anySoloActive = true;
                break;
            }
        }

        if (anySoloActive)
        {
            // 如果有Solo激活，执行"一键取消"操作
            // 需要先恢复preSoloMuteStates中缓存的Mute状态
            for (auto const& [paramId, wasMuted] : preSoloMuteStates)
            {
                audioProcessor.apvts.getParameter(paramId)->setValueNotifyingHost(wasMuted ? 1.0f : 0.0f);
            }
            preSoloMuteStates.clear();
            
            // 然后清除所有Solo状态
            for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
            {
                audioProcessor.apvts.getParameter("SOLO_" + juce::String(chanInfo.channelIndex + 1))->setValueNotifyingHost(0.0f);
            }
            
            currentUIMode = UIMode::Normal;
        }
        else
        {
            // 如果没有Solo激活，进入Solo分配模式
            currentUIMode = globalSoloButton.getToggleState() ? UIMode::AssignSolo : UIMode::Normal;
            if (currentUIMode == UIMode::AssignSolo)
                globalMuteButton.setToggleState(false, juce::sendNotification);
        }
        
        // 立即更新UI状态显示
        updateChannelButtonStates();
    };
    
    addAndMakeVisible(dimButton);
    dimButton.setClickingTogglesState(true);
    dimButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::yellow);

    addAndMakeVisible(speakerLayoutSelector);
    speakerLayoutSelector.addItemList(configManager.getSpeakerLayoutNames(), 1);
    speakerLayoutSelector.setSelectedId(1);
    speakerLayoutSelector.onChange = [this] { resized(); };

    addAndMakeVisible(subLayoutSelector);
    subLayoutSelector.addItemList(configManager.getSubLayoutNames(), 1);
    subLayoutSelector.setSelectedId(1);
    subLayoutSelector.onChange = [this] { resized(); };
    
    addAndMakeVisible(channelGridContainer);

    // Make sure the look and feel is applied to all children
    setLookAndFeel(&customLookAndFeel);
    setSize (800, 600);
    startTimerHz(30);
}

MonitorControllerMaxAudioProcessorEditor::~MonitorControllerMaxAudioProcessorEditor()
{
    setLookAndFeel(nullptr);
    stopTimer();
}

//==============================================================================
void MonitorControllerMaxAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (getLookAndFeel().findColour (juce::ResizableWindow::backgroundColourId));
}

void MonitorControllerMaxAudioProcessorEditor::resized()
{
    // 恢复到正确的、基于区域划分的布局逻辑
    juce::Rectangle<int> bounds = getLocalBounds().reduced(10);

    // 1. 将界面明确划分为左侧的侧边栏和右侧的主区域
    auto sidebarBounds = bounds.removeFromLeft(120);
    bounds.removeFromLeft(10); // 侧边栏和主区域之间的间隙
    auto mainAreaBounds = bounds;

    // 2. 在侧边栏区域内使用FlexBox进行布局
    juce::FlexBox sidebarFlex;
    sidebarFlex.flexDirection = juce::FlexBox::Direction::column;
    sidebarFlex.justifyContent = juce::FlexBox::JustifyContent::flexStart;
    
    sidebarFlex.items.add(juce::FlexItem(globalSoloButton).withHeight(50).withMargin(5));
    sidebarFlex.items.add(juce::FlexItem(dimButton).withHeight(50).withMargin(5));
    sidebarFlex.items.add(juce::FlexItem(globalMuteButton).withHeight(50).withMargin(5));
    
    sidebarFlex.performLayout(sidebarBounds);

    // 3. 在主区域内进一步划分布局
    auto selectorBounds = mainAreaBounds.removeFromTop(40);
    auto gridContainerBounds = mainAreaBounds; // 剩下的就是网格容器的区域

    // 3a. 布局顶部的下拉选择器
    juce::FlexBox selectorFlex;
    selectorFlex.flexDirection = juce::FlexBox::Direction::row;
    selectorFlex.justifyContent = juce::FlexBox::JustifyContent::flexEnd; // 靠右对齐
    selectorFlex.items.add(juce::FlexItem(speakerLayoutSelector).withWidth(150).withHeight(30).withMargin(5));
    selectorFlex.items.add(juce::FlexItem(subLayoutSelector).withWidth(100).withHeight(30).withMargin(5));
    selectorFlex.performLayout(selectorBounds);
    
    // 3b. 为网格容器设置正确的边界
    channelGridContainer.setBounds(gridContainerBounds);

    // 4. 在所有容器的边界都确定后，再调用updateLayout来填充网格内容
    updateLayout();
}

void MonitorControllerMaxAudioProcessorEditor::updateLayout()
{
    const int availableChannels = audioProcessor.getAvailableChannels();
    auto speakerLayoutNames = configManager.getSpeakerLayoutNames();
    auto subLayoutNames = configManager.getSubLayoutNames();

    // 1. 根据可用通道数，动态启用/禁用下拉菜单项
    int firstValidSpeakerId = 0;
    for (int i = 0; i < speakerLayoutNames.size(); ++i)
    {
        const auto& name = speakerLayoutNames[i];
        const int requiredChannels = configManager.getChannelCountForLayout("Speaker", name);
        bool isEnabled = (requiredChannels <= availableChannels);
        speakerLayoutSelector.setItemEnabled(i + 1, isEnabled);
        if (isEnabled && firstValidSpeakerId == 0)
        {
            firstValidSpeakerId = i + 1;
        }
    }

    // 2. 确保当前选择的 Speaker 布局是有效的
    if (!speakerLayoutSelector.isItemEnabled(speakerLayoutSelector.getSelectedId()))
    {
        speakerLayoutSelector.setSelectedId(firstValidSpeakerId, juce::dontSendNotification);
    }
    
    auto selectedSpeakerName = speakerLayoutSelector.getText();
    const int speakerChannelsUsed = configManager.getChannelCountForLayout("Speaker", selectedSpeakerName);
    
    int firstValidSubId = 1; // "None" is always valid
    for (int i = 1; i < subLayoutNames.size(); ++i) // Start from 1 to skip "None"
    {
        const auto& name = subLayoutNames[i];
        const int requiredChannels = configManager.getChannelCountForLayout("SUB", name);
        bool isEnabled = (speakerChannelsUsed + requiredChannels <= availableChannels);
        subLayoutSelector.setItemEnabled(i + 1, isEnabled);
    }

    if (!subLayoutSelector.isItemEnabled(subLayoutSelector.getSelectedId()))
    {
        subLayoutSelector.setSelectedId(firstValidSubId, juce::dontSendNotification);
    }
    
    // 3. 获取最终有效的布局名称并更新处理器
    auto speakerLayoutName = speakerLayoutSelector.getText();
    auto subLayoutName = subLayoutSelector.getText();

    if (speakerLayoutName.isEmpty()) return;

    audioProcessor.setCurrentLayout(speakerLayoutName, subLayoutName);
    const auto& layout = audioProcessor.getCurrentLayout();
    
    // 4. 根据新布局重绘UI网格
    for(auto& pair : channelButtons)
        pair.second->setVisible(false);

    channelGrid.items.clear();
    channelGrid.setGap(juce::Grid::Px(5));
    channelGrid.templateRows.clear();
    channelGrid.templateColumns.clear();

    for (int i = 0; i < 5; ++i)
    {
        channelGrid.templateRows.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
        channelGrid.templateColumns.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
    }
    
    // 创建一个包含25个空GridItem的向量，代表5x5网格
    std::vector<juce::GridItem> gridItems(25);

    // 将实际的按钮放置到网格的正确位置
    for (const auto& chanInfo : layout.channels)
    {
        if (channelButtons.find(chanInfo.channelIndex) == channelButtons.end())
        {
            channelButtons[chanInfo.channelIndex] = std::make_unique<juce::TextButton>(chanInfo.name);
            channelGridContainer.addAndMakeVisible(*channelButtons[chanInfo.channelIndex]);
            
            auto* button = channelButtons[chanInfo.channelIndex].get();
            button->setClickingTogglesState(true);

            // ================== 新的事务性 onClick 逻辑 ==================
            button->onClick = [this, channelIndex = chanInfo.channelIndex, channelName = chanInfo.name]
            {
                if (currentUIMode == UIMode::AssignSolo)
                {
                    handleSoloButtonClick(channelIndex, channelName);
                }
                else if (currentUIMode == UIMode::AssignMute)
                {
                    // Mute逻辑很简单，直接切换状态
                    auto* muteParam = audioProcessor.apvts.getParameter("MUTE_" + juce::String(channelIndex + 1));
                    muteParam->setValueNotifyingHost(muteParam->getValue() < 0.5f ? 1.0f : 0.0f);
                }
                // 非分配模式下点击无效
            };
        }
        
        auto* button = channelButtons[chanInfo.channelIndex].get();
        button->setButtonText(chanInfo.name);
        button->setVisible(true);

        int gridPosIndex = chanInfo.gridPosition - 1; // 转换为0-based索引
        if (gridPosIndex >= 0 && gridPosIndex < 25)
        {
            gridItems[gridPosIndex] = juce::GridItem(*button);
        }
    }

    // 处理特殊的SUB按钮
    if (subLayoutName != "None")
    {
        const int subChannelIndex = -1; 
        if (channelButtons.find(subChannelIndex) == channelButtons.end())
        {
             channelButtons[subChannelIndex] = std::make_unique<juce::TextButton>("SUB");
             channelGridContainer.addAndMakeVisible(*channelButtons[subChannelIndex]);
        }
        auto* button = channelButtons[subChannelIndex].get();
        button->setVisible(true);
        int gridPosIndex = 23 - 1; // 23号位置，0-based索引为22
        if (gridPosIndex >= 0 && gridPosIndex < 25)
        {
            gridItems[gridPosIndex] = juce::GridItem(*button);
        }
    }
    
    // 将包含按钮和占位符的完整项列表赋给Grid
    for (const auto& item : gridItems)
        channelGrid.items.add(item);

    channelGrid.performLayout(channelGridContainer.getLocalBounds());
    updateChannelButtonStates(); // Ensure button states are updated immediately
}

void MonitorControllerMaxAudioProcessorEditor::timerCallback()
{
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::setUIMode(UIMode newMode)
{
    currentUIMode = newMode;
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateChannelButtonStates()
{
    // ================== 全新的、最终的状态显示逻辑 ==================
    const auto& layout = audioProcessor.getCurrentLayout();
    if (layout.channels.empty() && layout.totalChannelCount == 0) return;

    // 1. 计算真实的聚合状态
    bool anySoloEngaged = false;
    bool anyMuteEngaged = false;
    for (const auto& chanInfo : layout.channels)
    {
        if (audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
            anySoloEngaged = true;
        if (audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
            anyMuteEngaged = true;
    }

    // 2. 更新每个通道按钮的状态
    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible() || index < 0) continue;

        const bool isSoloed = audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(index + 1))->load() > 0.5f;
        const bool isMuted = audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(index + 1))->load() > 0.5f;

        if (isSoloed)
        {
            button->setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getSoloColour());
            button->setToggleState(true, juce::dontSendNotification);
        }
        else if (isMuted)
        {
            button->setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonOnColourId));
            button->setToggleState(true, juce::dontSendNotification);
        }
        else
        {
            button->setToggleState(false, juce::dontSendNotification);
        }
    }
    
    // 3. 更新主控制按钮的外观，使其反映真实的聚合状态
    globalMuteButton.setToggleState(anyMuteEngaged, juce::dontSendNotification);

    if (anySoloEngaged)
    {
        // 当有Solo通道激活时，主按钮显示为激活状态（绿色）
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getSoloColour());
        globalSoloButton.setToggleState(true, juce::dontSendNotification);
    }
    else
    {
        // 如果没有通道被solo，则恢复默认颜色，并根据UI模式决定状态
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalSoloButton.setToggleState(currentUIMode == UIMode::AssignSolo, juce::dontSendNotification);
    }
}

void MonitorControllerMaxAudioProcessorEditor::handleSoloButtonClick(int channelIndex, const juce::String& channelName)
{
    // ================== 全新的、最终的事务逻辑 ==================
    
    // 步骤1: 获取动作前的全局Solo状态
    bool wasAnySoloActive = false;
    for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
    {
        if (audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
        {
            wasAnySoloActive = true;
            break;
        }
    }
    
    // 步骤2: 切换被点击按钮的Solo状态
    auto* targetSoloParam = audioProcessor.apvts.getParameter("SOLO_" + juce::String(channelIndex + 1));
    bool isTargetCurrentlySoloed = targetSoloParam->getValue() > 0.5f;
    targetSoloParam->setValueNotifyingHost(!isTargetCurrentlySoloed);

    // 步骤3: 获取动作后的全局Solo状态
    bool isAnySoloNowActive = false;
    for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
    {
         if (audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(chanInfo.channelIndex + 1))->load() > 0.5f)
        {
            isAnySoloNowActive = true;
            break;
        }
    }

    // --- 事务处理 ---

    // 情况A: 刚刚进入Solo模式 (从无到有)
    if (isAnySoloNowActive && !wasAnySoloActive)
    {
        preSoloMuteStates.clear();
        for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
        {
            if (!chanInfo.name.startsWith("SUB")) // 只缓存主声道
            {
                auto muteParamId = "MUTE_" + juce::String(chanInfo.channelIndex + 1);
                preSoloMuteStates[muteParamId] = audioProcessor.apvts.getRawParameterValue(muteParamId)->load() > 0.5f;
            }
        }
    }

    // 情况B: 刚刚退出Solo模式 (从有到无)
    if (!isAnySoloNowActive && wasAnySoloActive)
    {
        // 恢复所有主声道的Mute状态
        for (auto const& [paramId, wasMuted] : preSoloMuteStates)
        {
            audioProcessor.apvts.getParameter(paramId)->setValueNotifyingHost(wasMuted ? 1.0f : 0.0f);
        }
        preSoloMuteStates.clear();
    }

    // 步骤4: 统一应用Solo联动Mute规则
    if (isAnySoloNowActive)
    {
        for (const auto& chanInfo : audioProcessor.getCurrentLayout().channels)
        {
            if (!chanInfo.name.startsWith("SUB")) // 只影响主声道
            {
                auto* soloP = audioProcessor.apvts.getParameter("SOLO_" + juce::String(chanInfo.channelIndex + 1));
                auto* muteP = audioProcessor.apvts.getParameter("MUTE_" + juce::String(chanInfo.channelIndex + 1));
                if (soloP->getValue() < 0.5f) // 如果没被solo
                    muteP->setValueNotifyingHost(1.0f); // 就强制mute
                else
                    muteP->setValueNotifyingHost(0.0f); // 如果被solo，就解除mute
            }
        }
    }
    
    updateChannelButtonStates();
}
