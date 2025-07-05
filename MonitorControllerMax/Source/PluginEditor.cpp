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
    globalMuteButton.onClick = [this] { setUIMode(globalMuteButton.getToggleState() ? UIMode::AssignMute : UIMode::Normal); };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setClickingTogglesState(true);
    globalSoloButton.onClick = [this] { setUIMode(globalSoloButton.getToggleState() ? UIMode::AssignSolo : UIMode::Normal); };
    
    addAndMakeVisible(dimButton);

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
            button->onClick = [this, index = chanInfo.channelIndex] 
            {
                if (currentUIMode == UIMode::AssignMute)
                {
                    if (auto* param = audioProcessor.apvts.getParameter("MUTE_" + juce::String(index + 1)))
                        param->setValueNotifyingHost(param->getValue() < 0.5f ? 1.0f : 0.0f);
                }
                else if (currentUIMode == UIMode::AssignSolo)
                {
                    if (auto* param = audioProcessor.apvts.getParameter("SOLO_" + juce::String(index + 1)))
                        param->setValueNotifyingHost(param->getValue() < 0.5f ? 1.0f : 0.0f);
                }
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
    const auto role = audioProcessor.getRole();
    const bool isSlave = (role == MonitorControllerMaxAudioProcessor::Role::slave);

    globalMuteButton.setEnabled(!isSlave);
    globalSoloButton.setEnabled(!isSlave);
    dimButton.setEnabled(!isSlave);
    speakerLayoutSelector.setEnabled(!isSlave);
    subLayoutSelector.setEnabled(!isSlave);

    bool anySoloEngaged = false;
    std::array<bool, 26> muteStates{};
    std::array<bool, 26> soloStates{};

    for (int i = 0; i < 26; ++i)
    {
        if (isSlave)
        {
            muteStates[i] = audioProcessor.getRemoteMuteState(i);
            soloStates[i] = audioProcessor.getRemoteSoloState(i);
        }
        else
        {
            if (auto* param = audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(i + 1)))
                muteStates[i] = param->load() > 0.5f;
            if (auto* param = audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(i + 1)))
                soloStates[i] = param->load() > 0.5f;
        }

        if (soloStates[i])
            anySoloEngaged = true;
    }

    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible()) continue;
        
        if (index < 0) 
        {
            continue;
        }

        bool isMuted = muteStates[index];
        bool isSoloed = soloStates[index];
        bool shouldBeSilent = isMuted || (anySoloEngaged && !isSoloed);

        juce::Colour colour = juce::Colours::grey;

        if (isSoloed)
            colour = juce::Colours::yellow;
        else if (shouldBeSilent)
            colour = juce::Colours::red;
        
        button->setColour(juce::TextButton::buttonColourId, colour);
        if (currentUIMode == UIMode::AssignMute)
            button->setToggleState(isMuted, juce::dontSendNotification);
        else if (currentUIMode == UIMode::AssignSolo)
            button->setToggleState(isSoloed, juce::dontSendNotification);
        else
            button->setToggleState(false, juce::dontSendNotification);

        button->setEnabled(!isSlave);
    }
}
