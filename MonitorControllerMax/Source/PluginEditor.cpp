/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DebugLogger.h"

//==============================================================================
MonitorControllerMaxAudioProcessorEditor::MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor& p)
    : AudioProcessorEditor (&p), audioProcessor (p), configManager(p.configManager)
{
    addAndMakeVisible(globalMuteButton);
    globalMuteButton.setButtonText("MUTE");
    globalMuteButton.setClickingTogglesState(false);  // 手动管理状态，避免自动切换冲突
    globalMuteButton.onClick = [this]
    {
        // 新的强大状态机逻辑 - 基于6大观点设计
        audioProcessor.handleMuteButtonClick();
    };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setButtonText("SOLO");
    globalSoloButton.setClickingTogglesState(false);  // 手动管理状态，避免自动切换冲突
    globalSoloButton.onClick = [this]
    {
        // 新的强大状态机逻辑 - 基于6大观点设计
        audioProcessor.handleSoloButtonClick();
    };
    
    addAndMakeVisible(dimButton);
    dimButton.setButtonText("DIM");
    dimButton.setClickingTogglesState(true);
    dimButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::yellow);

    addAndMakeVisible(speakerLayoutSelector);
    speakerLayoutSelector.addItemList(configManager.getSpeakerLayoutNames(), 1);
    speakerLayoutSelector.setSelectedId(1);
    speakerLayoutSelector.onChange = [this] 
    { 
        // 用户手动选择时，直接更新配置，不强制验证选择
        updatePluginConfiguration();
        
        // 重新布局UI，但跳过下拉框的强制选择逻辑
        updateLayoutWithoutSelectorOverride();
    };

    addAndMakeVisible(subLayoutSelector);
    subLayoutSelector.addItemList(configManager.getSubLayoutNames(), 1);
    subLayoutSelector.setSelectedId(1);
    subLayoutSelector.onChange = [this] 
    { 
        // 用户手动选择时，直接更新配置，不强制验证选择
        updatePluginConfiguration();
        
        // 重新布局UI，但跳过下拉框的强制选择逻辑
        updateLayoutWithoutSelectorOverride();
    };
    
    addAndMakeVisible(channelGridContainer);

    // Make sure the look and feel is applied to all children
    setLookAndFeel(&customLookAndFeel);
    setSize (800, 600);
    
    // 初始化已知的通道数
    lastKnownChannelCount = audioProcessor.getTotalNumInputChannels();
    
    // 设置处理器的布局自动切换回调
    audioProcessor.setLayoutChangeCallback([this](const juce::String& speaker, const juce::String& sub)
    {
        // 在主线程中更新UI选择器
        juce::MessageManager::callAsync([this, speaker, sub]()
        {
            // 更新下拉框选择而不触发onChange事件
            auto speakerLayoutNames = configManager.getSpeakerLayoutNames();
            auto subLayoutNames = configManager.getSubLayoutNames();
            
            for (int i = 0; i < speakerLayoutNames.size(); ++i)
            {
                if (speakerLayoutNames[i] == speaker)
                {
                    speakerLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
                    break;
                }
            }
            
            for (int i = 0; i < subLayoutNames.size(); ++i)
            {
                if (subLayoutNames[i] == sub)
                {
                    subLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
                    break;
                }
            }
            
            // 强制重新布局以显示新的通道配置
            resized();
        });
    });
    
    startTimerHz(30);
    
    // 编辑器创建后，同步处理器的当前状态到UI
    // 这解决了关闭/重新打开编辑器时配置重置的问题
    juce::MessageManager::callAsync([this]()
    {
        updateLayout(); // 强制UI与处理器状态同步
        updateChannelButtonStates(); // 同步按钮状态
    });
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
    
    // 0. 首先同步下拉框选择与处理器当前布局状态（解决重新打开编辑器的问题）
    const auto& currentLayout = audioProcessor.getCurrentLayout();
    
    // 获取当前通道数用于下拉框同步
    int currentChannelCount = audioProcessor.getTotalNumInputChannels();
    
    // 根据当前总通道数找到最合适的配置并设置下拉框
    juce::String expectedSpeaker = "2.0";
    juce::String expectedSub = "None";
    
    // 使用相同的自动选择逻辑
    switch (currentChannelCount)
    {
        case 1: expectedSpeaker = "1.0"; break;
        case 2: expectedSpeaker = "2.0"; break;
        case 6: expectedSpeaker = "5.1"; break;
        case 8: expectedSpeaker = "7.1"; break;
        case 12: expectedSpeaker = "7.1.4"; break;
        default:
            if (currentChannelCount > 12) expectedSpeaker = "7.1.4";
            else if (currentChannelCount > 8) expectedSpeaker = "7.1.2";
            else if (currentChannelCount > 6) expectedSpeaker = "7.1";
            break;
    }
    
    // 设置下拉框到期望的值（不触发onChange）
    for (int i = 0; i < speakerLayoutNames.size(); ++i)
    {
        if (speakerLayoutNames[i] == expectedSpeaker)
        {
            speakerLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
            break;
        }
    }
    
    for (int i = 0; i < subLayoutNames.size(); ++i)
    {
        if (subLayoutNames[i] == expectedSub)
        {
            subLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
            break;
        }
    }

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
            button->setClickingTogglesState(false); // 手动管理状态

            // ================== 全新强大状态机逻辑 ==================
            button->onClick = [this, channelIndex = chanInfo.channelIndex]
            {
                // 统一通过状态机处理所有通道点击
                audioProcessor.handleChannelClick(channelIndex);
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

void MonitorControllerMaxAudioProcessorEditor::updateLayoutWithoutSelectorOverride()
{
    // 这个函数和updateLayout()基本相同，但不会强制改变用户的下拉框选择
    const int availableChannels = audioProcessor.getAvailableChannels();
    auto speakerLayoutNames = configManager.getSpeakerLayoutNames();
    auto subLayoutNames = configManager.getSubLayoutNames();
    
    // 1. 根据可用通道数，动态启用/禁用下拉菜单项
    for (int i = 0; i < speakerLayoutNames.size(); ++i)
    {
        const auto& name = speakerLayoutNames[i];
        const int requiredChannels = configManager.getChannelCountForLayout("Speaker", name);
        bool isEnabled = (requiredChannels <= availableChannels);
        speakerLayoutSelector.setItemEnabled(i + 1, isEnabled);
    }

    // 2. 跳过强制选择逻辑，尊重用户的选择
    // (用户手动选择时，即使选择了超出可用通道的配置也允许)
    
    auto selectedSpeakerName = speakerLayoutSelector.getText();
    auto selectedSubName = subLayoutSelector.getText();
    
    if (selectedSpeakerName.isEmpty()) return;

    audioProcessor.setCurrentLayout(selectedSpeakerName, selectedSubName);
    const auto& layout = audioProcessor.getCurrentLayout();
    
    // 3. 根据新布局重绘UI网格 (与updateLayout()相同的网格重绘逻辑)
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
            button->setClickingTogglesState(false); // 手动管理状态

            // ================== 全新强大状态机逻辑 ==================
            button->onClick = [this, channelIndex = chanInfo.channelIndex]
            {
                // 统一通过状态机处理所有通道点击
                audioProcessor.handleChannelClick(channelIndex);
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
    if (selectedSubName != "None")
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
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::timerCallback()
{
    // 检查总线布局是否发生变化
    int currentChannelCount = audioProcessor.getTotalNumInputChannels();
    if (currentChannelCount != lastKnownChannelCount && currentChannelCount > 0)
    {
        lastKnownChannelCount = currentChannelCount;
        
        // 安全地触发布局自动选择（在UI线程中）
        audioProcessor.autoSelectLayoutForChannelCount(currentChannelCount);
        
        // 总线布局发生变化，重新更新整个UI布局
        updateLayout();
    }
    
    // Update button states to reflect current parameter values
    // This is essential since parameter listener mechanism isn't working properly
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::setUIMode(UIMode newMode)
{
    currentUIMode = newMode;
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateChannelButtonStates()
{
    // Parameter-driven UI update logic
    // UI state is calculated directly from parameter values
    
    // 1. Update each channel button based on parameter state
    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible() || index < 0) continue;
        
        // Get parameter IDs for this channel
        auto muteParamId = "MUTE_" + juce::String(index + 1);
        auto soloParamId = "SOLO_" + juce::String(index + 1);
        
        // Get current parameter values
        auto* muteParam = audioProcessor.apvts.getRawParameterValue(muteParamId);
        auto* soloParam = audioProcessor.apvts.getRawParameterValue(soloParamId);
        
        if (!muteParam || !soloParam) continue;
        
        float muteValue = muteParam->load();
        float soloValue = soloParam->load();
        
        // Determine button state and color based on parameter values
        bool shouldBeActive = false;
        juce::Colour buttonColor = getLookAndFeel().findColour(juce::TextButton::buttonColourId);
        
        if (soloValue > 0.5f) {
            // Solo active - use proper Solo color and active state
            shouldBeActive = true;
            buttonColor = customLookAndFeel.getSoloColour();
        } else if (muteValue > 0.5f) {
            // Mute active - use proper Mute color and inactive state (showing muted)
            shouldBeActive = false;
            buttonColor = customLookAndFeel.getMuteColour();
        } else {
            // Normal state - default color and inactive
            shouldBeActive = false;
            buttonColor = getLookAndFeel().findColour(juce::TextButton::buttonColourId);
        }
        
        // Update button state if changed
        if (button->getToggleState() != shouldBeActive) {
            button->setToggleState(shouldBeActive, juce::dontSendNotification);
        }
        
        // Update button color
        button->setColour(juce::TextButton::buttonColourId, buttonColor);
        button->setColour(juce::TextButton::buttonOnColourId, buttonColor);
    }
    
    // 2. Update main control buttons
    bool hasSolo = audioProcessor.hasAnySoloActive();
    bool hasMute = audioProcessor.hasAnyMuteActive();
    
    // Update main Solo button state and color
    if (globalSoloButton.getToggleState() != hasSolo) {
        globalSoloButton.setToggleState(hasSolo, juce::dontSendNotification);
    }
    
    // Set correct Solo button color based on state
    if (hasSolo) {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getSoloColour());
        globalSoloButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getSoloColour());
    } else {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalSoloButton.setColour(juce::TextButton::buttonColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    
    // Update main Mute button state and color
    if (globalMuteButton.getToggleState() != hasMute) {
        globalMuteButton.setToggleState(hasMute, juce::dontSendNotification);
    }
    
    // Set correct Mute button color based on state
    if (hasMute) {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getMuteColour());
        globalMuteButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getMuteColour());
    } else {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalMuteButton.setColour(juce::TextButton::buttonColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    
    // Apply Solo Priority Rule: Disable Mute button when Solo is active
    bool muteButtonEnabled = audioProcessor.isMuteButtonEnabled();
    globalMuteButton.setEnabled(muteButtonEnabled);

}

// 旧的handleSoloButtonClick函数已被新的状态机逻辑替代
// 现在所有逻辑都通过StateManager.handleChannelClick()统一处理

// 立即更新插件配置并通知宿主刷新I/O针脚名
void MonitorControllerMaxAudioProcessorEditor::updatePluginConfiguration()
{
    // 获取当前选择的配置
    auto speakerLayoutName = speakerLayoutSelector.getText();
    auto subLayoutName = subLayoutSelector.getText();

    if (speakerLayoutName.isEmpty()) return;

    // 立即更新插件配置，这会触发updateHostDisplay()
    audioProcessor.setCurrentLayout(speakerLayoutName, subLayoutName);
    
    // 强制通知宿主更新显示信息 - 多次调用确保REAPER响应
    juce::MessageManager::callAsync([this]()
    {
        audioProcessor.updateHostDisplay();
        
        // 延迟额外刷新，确保REAPER能获取到最新的通道名称
        juce::Timer::callAfterDelay(100, [this]()
        {
            audioProcessor.updateHostDisplay();
        });
    });
    
    // 确保UI状态同步更新
    updateChannelButtonStates();
}
