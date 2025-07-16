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
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Mute button click ignored - Slave mode");
            return;
        }
        // 新的强大状态机逻辑 - 基于6大观点设计
        audioProcessor.handleMuteButtonClick();
    };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setButtonText("SOLO");
    globalSoloButton.setClickingTogglesState(false);  // 手动管理状态，避免自动切换冲突
    globalSoloButton.onClick = [this]
    {
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Solo button click ignored - Slave mode");
            return;
        }
        // 新的强大状态机逻辑 - 基于6大观点设计
        audioProcessor.handleSoloButtonClick();
    };
    
    addAndMakeVisible(dimButton);
    dimButton.setButtonText("DIM");
    dimButton.setClickingTogglesState(true);
    dimButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::yellow);
    
    // v4.1: 连接Dim按钮到总线处理器
    dimButton.onClick = [this]
    {
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Dim button click ignored - Slave mode");
            return;
        }
        
        // 切换Dim状态
        audioProcessor.masterBusProcessor.toggleDim();
        
        // 更新按钮状态
        dimButton.setToggleState(audioProcessor.masterBusProcessor.isDimActive(), juce::dontSendNotification);
    };
    
    // v4.1: 设置Dim状态变化回调 - 用于OSC控制时更新UI
    audioProcessor.masterBusProcessor.onDimStateChanged = [this]()
    {
        // 在主线程中更新UI
        juce::MessageManager::callAsync([this]()
        {
            dimButton.setToggleState(audioProcessor.masterBusProcessor.isDimActive(), juce::dontSendNotification);
        });
    };
    
    // v4.1: 设置Low Boost按钮
    addAndMakeVisible(lowBoostButton);
    lowBoostButton.setButtonText("LOW\nBOOST");
    lowBoostButton.setClickingTogglesState(true);
    lowBoostButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // v4.1: 连接Low Boost按钮到总线处理器
    lowBoostButton.onClick = [this]()
    {
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Low Boost button click ignored - Slave mode");
            return;
        }
        
        // 切换Low Boost状态
        audioProcessor.masterBusProcessor.toggleLowBoost();
        
        // 更新按钮状态
        lowBoostButton.setToggleState(audioProcessor.masterBusProcessor.isLowBoostActive(), juce::dontSendNotification);
    };
    
    // v4.1: 设置Low Boost状态变化回调 - 用于OSC控制时更新UI
    audioProcessor.masterBusProcessor.onLowBoostStateChanged = [this]()
    {
        // 在主线程中更新UI
        juce::MessageManager::callAsync([this]()
        {
            lowBoostButton.setToggleState(audioProcessor.masterBusProcessor.isLowBoostActive(), juce::dontSendNotification);
        });
    };
    
    // v4.1: 设置Master Mute按钮
    addAndMakeVisible(masterMuteButton);
    masterMuteButton.setButtonText("MASTER\nMUTE");
    masterMuteButton.setClickingTogglesState(true);
    masterMuteButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::red);
    
    // v4.1: 连接Master Mute按钮到总线处理器
    masterMuteButton.onClick = [this]()
    {
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Master Mute button click ignored - Slave mode");
            return;
        }
        
        // 切换Master Mute状态
        audioProcessor.masterBusProcessor.toggleMasterMute();
        
        // 更新按钮状态
        masterMuteButton.setToggleState(audioProcessor.masterBusProcessor.isMasterMuteActive(), juce::dontSendNotification);
    };
    
    // v4.1: 设置Master Mute状态变化回调 - 用于OSC控制时更新UI
    audioProcessor.masterBusProcessor.onMasterMuteStateChanged = [this]()
    {
        // 在主线程中更新UI
        juce::MessageManager::callAsync([this]()
        {
            masterMuteButton.setToggleState(audioProcessor.masterBusProcessor.isMasterMuteActive(), juce::dontSendNotification);
        });
    };
    
    // v4.1: 设置Master Gain旋钮
    addAndMakeVisible(masterGainSlider);
    masterGainSlider.setSliderStyle(juce::Slider::RotaryVerticalDrag);
    masterGainSlider.setRange(0.0, 100.0, 0.1);
    masterGainSlider.setValue(100.0);
    masterGainSlider.setTextValueSuffix("%");
    masterGainSlider.setTextBoxStyle(juce::Slider::TextBoxBelow, false, 60, 20);
    masterGainSlider.setColour(juce::Slider::rotarySliderFillColourId, juce::Colours::orange);
    masterGainSlider.setColour(juce::Slider::rotarySliderOutlineColourId, juce::Colours::grey);
    masterGainSlider.setColour(juce::Slider::textBoxTextColourId, juce::Colours::white);
    masterGainSlider.setColour(juce::Slider::textBoxBackgroundColourId, juce::Colours::transparentBlack);
    
    // v4.1: Master Gain标签 (移除丑陋的文字说明，保持简洁)
    // masterGainLabel 不再显示
    
    // v4.1: 连接Master Gain旋钮到VST3参数
    masterGainSliderAttachment = std::make_unique<SliderAttachment>(audioProcessor.apvts, "MASTER_GAIN", masterGainSlider);

    addAndMakeVisible(speakerLayoutSelector);
    speakerLayoutSelector.addItemList(configManager.getSpeakerLayoutNames(), 1);
    speakerLayoutSelector.setSelectedId(1);
    speakerLayoutSelector.onChange = [this] 
    { 
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Speaker layout change ignored - Slave mode");
            return;
        }
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
        // 检查角色权限 - Slave模式禁止操作
        if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
            VST3_DBG_ROLE(&audioProcessor, "Sub layout change ignored - Slave mode");
            return;
        }
        // 用户手动选择时，直接更新配置，不强制验证选择
        updatePluginConfiguration();
        
        // 重新布局UI，但跳过下拉框的强制选择逻辑
        updateLayoutWithoutSelectorOverride();
    };
    
    // 设置角色选择器
    setupRoleSelector();
    
    // 设置debug日志窗口
    addAndMakeVisible(debugLogLabel);
    debugLogLabel.setText("Connection Debug:", juce::dontSendNotification);
    debugLogLabel.setFont(juce::Font(12.0f));
    
    addAndMakeVisible(debugLogDisplay);
    debugLogDisplay.setMultiLine(true);
    debugLogDisplay.setReadOnly(true);
    debugLogDisplay.setScrollbarsShown(true);
    debugLogDisplay.setCaretVisible(false);
    debugLogDisplay.setPopupMenuEnabled(false);
    debugLogDisplay.setFont(juce::Font(10.0f));
    debugLogDisplay.setText("Debug logs will appear here...");
    
    addAndMakeVisible(clearLogButton);
    clearLogButton.onClick = [this] { clearDebugLog(); };
    
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
        // 重要修复：从用户选择的配置同步UI，而不是当前布局
        // 这确保UI反映用户的实际选择，而不是自动推断的配置
        syncUIFromUserSelection();
        updateChannelButtonStates(); // 同步按钮状态
        
        // 🔧 关键修复：同步角色的UI状态，解决重新打开编辑器时Slave锁定状态丢失的问题
        updateUIBasedOnRole();
        
        VST3_DBG_ROLE(&audioProcessor, "PluginEditor: UI initialization complete with role-based state");
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
    
    // v4.1: 添加Master Gain旋钮 (移除文字标签，保持简洁)
    sidebarFlex.items.add(juce::FlexItem(masterGainSlider).withHeight(80).withMargin(5));
    
    // v4.1: 添加Low Boost按钮 (与Dim按钮同样大小)
    sidebarFlex.items.add(juce::FlexItem(lowBoostButton).withHeight(50).withMargin(5));
    
    // v4.1: 添加Master Mute按钮 (与Low Boost按钮同样大小)
    sidebarFlex.items.add(juce::FlexItem(masterMuteButton).withHeight(50).withMargin(5));
    
    sidebarFlex.performLayout(sidebarBounds);

    // 3. 在主区域内进一步划分布局
    auto selectorBounds = mainAreaBounds.removeFromTop(40);
    auto debugLogBounds = mainAreaBounds.removeFromBottom(120); // Debug日志区域
    mainAreaBounds.removeFromBottom(5); // 间隙
    auto gridContainerBounds = mainAreaBounds; // 剩下的就是网格容器的区域

    // 3a. 布局顶部的下拉选择器 - 增加角色选择器
    juce::FlexBox selectorFlex;
    selectorFlex.flexDirection = juce::FlexBox::Direction::row;
    selectorFlex.justifyContent = juce::FlexBox::JustifyContent::flexEnd; // 靠右对齐
    selectorFlex.items.add(juce::FlexItem(roleLabel).withWidth(40).withHeight(30).withMargin(5));
    selectorFlex.items.add(juce::FlexItem(roleSelector).withWidth(100).withHeight(30).withMargin(5));
    selectorFlex.items.add(juce::FlexItem(speakerLayoutSelector).withWidth(150).withHeight(30).withMargin(5));
    selectorFlex.items.add(juce::FlexItem(subLayoutSelector).withWidth(100).withHeight(30).withMargin(5));
    selectorFlex.performLayout(selectorBounds);
    
    // 3b. 为网格容器设置正确的边界
    channelGridContainer.setBounds(gridContainerBounds);
    
    // 3c. 布局底部的Debug日志区域
    auto labelBounds = debugLogBounds.removeFromTop(20);
    auto buttonBounds = debugLogBounds.removeFromBottom(25);
    auto logDisplayBounds = debugLogBounds;
    
    debugLogLabel.setBounds(labelBounds.removeFromLeft(120));
    clearLogButton.setBounds(buttonBounds.removeFromRight(60));
    debugLogDisplay.setBounds(logDisplayBounds);

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
    
    // 动态最佳匹配算法 - 自动找到最充分利用通道数的配置组合
    // 使用已定义的变量避免重定义错误
    
    int bestChannelUsage = 0;
    for (const auto& speaker : speakerLayoutNames)
    {
        int speakerChannels = configManager.getChannelCountForLayout("Speaker", speaker);
        
        for (const auto& sub : subLayoutNames)
        {
            int subChannels = configManager.getChannelCountForLayout("SUB", sub);
            int totalChannels = speakerChannels + subChannels;
            
            // 找到在可用通道内的最大使用量
            if (totalChannels <= currentChannelCount && totalChannels > bestChannelUsage)
            {
                bestChannelUsage = totalChannels;
                expectedSpeaker = speaker;
                expectedSub = sub;
            }
        }
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
    
    // NEW: Create semantic channel buttons based on current mapping - TEMPORARILY DISABLED
    // TODO: Re-enable after basic compilation works
    // createSemanticChannelButtons();
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
        
        // UI检测到通道数变化时，只更新显示，不改变布局配置
        // 布局配置的自动选择应该由processor在适当时机处理
        VST3_DBG_ROLE(&audioProcessor, "Channel count changed to " + juce::String(currentChannelCount) + ", updating UI display only");
        
        // 总线布局发生变化，重新更新整个UI布局显示
        updateLayout();
    }
    
    // Update button states to reflect current parameter values
    // This is essential since parameter listener mechanism isn't working properly
    updateChannelButtonStates();
    
    // 减少Debug日志更新频率 - 仅在角色变化或连接状态变化时更新
    static int debugUpdateCounter = 0;
    if (++debugUpdateCounter >= 30) { // 每秒更新一次而不是30次
        debugUpdateCounter = 0;
        updateDebugLogDisplay();
    }
    
    // NEW: Update semantic buttons from semantic state - TEMPORARILY DISABLED
    // TODO: Re-enable after basic compilation works  
    // updateAllSemanticButtonsFromState();
}

void MonitorControllerMaxAudioProcessorEditor::setUIMode(UIMode newMode)
{
    currentUIMode = newMode;
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateChannelButtonStates()
{
    // Semantic state-driven UI update logic
    // UI state is calculated directly from semantic channel states
    
    // 1. Update each channel button based on semantic state
    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible() || index < 0) continue;
        
        // Get semantic channel name from physical channel index
        juce::String semanticChannelName = audioProcessor.getPhysicalMapper().getSemanticName(index);
        
        // Skip unmapped channels
        if (semanticChannelName.isEmpty()) continue;
        
        // Get current semantic states
        bool soloState = audioProcessor.getSemanticState().getSoloState(semanticChannelName);
        bool finalMuteState = audioProcessor.getSemanticState().getFinalMuteState(semanticChannelName);
        
        // Determine button state and color based on semantic states
        bool shouldBeActive = false;
        juce::Colour buttonColor = getLookAndFeel().findColour(juce::TextButton::buttonColourId);
        
        if (soloState) {
            // Solo active - use proper Solo color and active state
            shouldBeActive = true;
            buttonColor = customLookAndFeel.getSoloColour();
        } else if (finalMuteState) {
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
    
    // 2. Update main control buttons using semantic state system
    bool soloButtonActive = audioProcessor.isSoloButtonActive();
    bool muteButtonActive = audioProcessor.isMuteButtonActive();
    
    // Update main Solo button state and color
    if (globalSoloButton.getToggleState() != soloButtonActive) {
        globalSoloButton.setToggleState(soloButtonActive, juce::dontSendNotification);
    }
    
    // Set correct Solo button color based on state
    if (soloButtonActive) {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getSoloColour());
        globalSoloButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getSoloColour());
    } else {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalSoloButton.setColour(juce::TextButton::buttonColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    
    // Update main Mute button state and color
    if (globalMuteButton.getToggleState() != muteButtonActive) {
        globalMuteButton.setToggleState(muteButtonActive, juce::dontSendNotification);
    }
    
    // Set correct Mute button color based on state
    if (muteButtonActive) {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getMuteColour());
        globalMuteButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getMuteColour());
    } else {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalMuteButton.setColour(juce::TextButton::buttonColourId, getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    
    // Apply Solo Priority Rule: Disable Mute button when Solo is active
    // 重要修复：Slave模式时，按钮必须保持禁用状态，不受Solo Priority规则影响
    bool muteButtonEnabled = audioProcessor.isMuteButtonEnabled();
    PluginRole currentRole = audioProcessor.getCurrentRole();
    bool isSlaveMode = (currentRole == PluginRole::Slave);
    
    // Slave模式下强制禁用，否则按照Solo Priority规则
    globalMuteButton.setEnabled(!isSlaveMode && muteButtonEnabled);

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

//==============================================================================
// New semantic UI methods
//==============================================================================

void MonitorControllerMaxAudioProcessorEditor::createSemanticChannelButtons()
{
    VST3_DBG("PluginEditor: Create semantic channel buttons");
    
    // Clear existing semantic buttons
    clearSemanticChannelButtons();
    
    // Get active semantic channels from processor's mapping
    auto activeChannels = audioProcessor.getPhysicalMapper().getActiveSemanticChannels();
    
    VST3_DBG("PluginEditor: Detected " + juce::String(activeChannels.size()) + " active semantic channels");
    
    // Create button pairs for each semantic channel
    for (const auto& channelName : activeChannels)
    {
        // Get grid position for this semantic channel
        auto gridPos = audioProcessor.getPhysicalMapper().getGridPosition(channelName);
        
        VST3_DBG("PluginEditor: Create semantic button pair - " + channelName + 
                 " (grid position: " + juce::String(gridPos.first) + "," + juce::String(gridPos.second) + ")");
        
        // Create button pair
        auto buttonPair = std::make_unique<SemanticChannelButtonPair>(audioProcessor, channelName, gridPos);
        
        // Set up button appearance to match existing system
        buttonPair->soloButton->setLookAndFeel(&customLookAndFeel);
        buttonPair->muteButton->setLookAndFeel(&customLookAndFeel);
        
        // Add to component hierarchy (initially hidden - will be shown when legacy system is phased out)
        addChildComponent(buttonPair->soloButton.get());
        addChildComponent(buttonPair->muteButton.get());
        
        // Store the button pair
        semanticChannelButtons[channelName] = std::move(buttonPair);
    }
    
    VST3_DBG("PluginEditor: Semantic button creation complete - total " + juce::String(semanticChannelButtons.size()) + " button pairs");
    
    // Update button states from semantic state
    updateAllSemanticButtonsFromState();
}

void MonitorControllerMaxAudioProcessorEditor::clearSemanticChannelButtons()
{
    VST3_DBG("PluginEditor: Clear semantic channel buttons");
    
    // Remove from component hierarchy and clear
    for (auto& [channelName, buttonPair] : semanticChannelButtons)
    {
        if (buttonPair)
        {
            removeChildComponent(buttonPair->soloButton.get());
            removeChildComponent(buttonPair->muteButton.get());
        }
    }
    
    semanticChannelButtons.clear();
}

void MonitorControllerMaxAudioProcessorEditor::updateAllSemanticButtonsFromState()
{
    // Update all semantic buttons from processor's semantic state
    for (auto& [channelName, buttonPair] : semanticChannelButtons)
    {
        if (buttonPair)
        {
            buttonPair->updateFromSemanticState();
        }
    }
}

void MonitorControllerMaxAudioProcessorEditor::updateLayoutFromSemanticMapping()
{
    VST3_DBG("PluginEditor: Update UI layout from semantic mapping");
    
    // This method will be used to transition from legacy layout to semantic layout
    // For now, it creates semantic buttons in parallel with legacy system
    createSemanticChannelButtons();
    
    VST3_DBG("PluginEditor: Semantic UI layout update complete");
}

void MonitorControllerMaxAudioProcessorEditor::syncUIFromUserSelection()
{
    VST3_DBG_ROLE(&audioProcessor, "Syncing UI from user selection");
    
    // 获取用户实际选择的配置
    juce::String userSpeaker = audioProcessor.userSelectedSpeakerLayout;
    juce::String userSub = audioProcessor.userSelectedSubLayout;
    
    VST3_DBG_ROLE(&audioProcessor, "User selected - Speaker: " + userSpeaker + ", Sub: " + userSub);
    
    // 更新下拉框选择到用户选择的配置（不触发onChange事件）
    auto speakerLayoutNames = configManager.getSpeakerLayoutNames();
    auto subLayoutNames = configManager.getSubLayoutNames();
    
    for (int i = 0; i < speakerLayoutNames.size(); ++i)
    {
        if (speakerLayoutNames[i] == userSpeaker)
        {
            speakerLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
            break;
        }
    }
    
    for (int i = 0; i < subLayoutNames.size(); ++i)
    {
        if (subLayoutNames[i] == userSub)
        {
            subLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
            break;
        }
    }
    
    // 应用用户选择的配置到处理器
    audioProcessor.setCurrentLayout(userSpeaker, userSub);
    
    // 更新UI布局
    updateLayout();
    
    VST3_DBG_ROLE(&audioProcessor, "UI sync complete");
}

//==============================================================================
// Master-Slave UI管理方法实现

void MonitorControllerMaxAudioProcessorEditor::setupRoleSelector()
{
    addAndMakeVisible(roleLabel);
    roleLabel.setText("Role:", juce::dontSendNotification);
    roleLabel.setFont(juce::Font(12.0f));
    
    addAndMakeVisible(roleSelector);
    roleSelector.addItem("Standalone", 1);
    roleSelector.addItem("Master", 2);
    roleSelector.addItem("Slave", 3);
    
    // 设置当前角色
    PluginRole currentRole = audioProcessor.getCurrentRole();
    roleSelector.setSelectedId(static_cast<int>(currentRole) + 1, juce::dontSendNotification);
    
    roleSelector.onChange = [this] { handleRoleChange(); };
    
    VST3_DBG_ROLE(&audioProcessor, "Role selector setup complete");
}

void MonitorControllerMaxAudioProcessorEditor::handleRoleChange()
{
    int selectedIndex = roleSelector.getSelectedId() - 1;
    PluginRole newRole = static_cast<PluginRole>(selectedIndex);
    
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: Role change requested - " + juce::String(selectedIndex));
    
    // 调用处理器的角色切换方法
    switch (newRole)
    {
        case PluginRole::Standalone:
            audioProcessor.switchToStandalone();
            break;
        case PluginRole::Master:
            audioProcessor.switchToMaster();
            break;
        case PluginRole::Slave:
            audioProcessor.switchToSlave();
            break;
    }
    
    // 更新UI状态
    updateUIBasedOnRole();
    
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: Role change completed");
}

void MonitorControllerMaxAudioProcessorEditor::updateUIBasedOnRole()
{
    PluginRole currentRole = audioProcessor.getCurrentRole();
    
    // 根据角色调整UI可用性
    bool isSlaveMode = (currentRole == PluginRole::Slave);
    
    // Slave模式时，完全禁用所有交互控件
    globalSoloButton.setEnabled(!isSlaveMode);
    globalMuteButton.setEnabled(!isSlaveMode);
    dimButton.setEnabled(!isSlaveMode);
    
    // v4.1: Slave模式禁用Master总线控件
    masterGainSlider.setEnabled(!isSlaveMode);
    lowBoostButton.setEnabled(!isSlaveMode);
    masterMuteButton.setEnabled(!isSlaveMode);
    
    // 禁用布局选择器（Slave不能更改布局）
    speakerLayoutSelector.setEnabled(!isSlaveMode);
    subLayoutSelector.setEnabled(!isSlaveMode);
    
    // 禁用所有通道按钮
    for (auto& [index, button] : channelButtons) {
        if (button) {
            button->setEnabled(!isSlaveMode);
        }
    }
    
    // 禁用语义通道按钮
    for (auto& [name, buttonPair] : semanticChannelButtons) {
        if (buttonPair) {
            buttonPair->setButtonsEnabled(!isSlaveMode);
        }
    }
    
    // Slave模式时添加视觉指示
    if (isSlaveMode) {
        // 设置半透明效果表示只读状态
        globalSoloButton.setAlpha(0.6f);
        globalMuteButton.setAlpha(0.6f);
        dimButton.setAlpha(0.6f);
        masterGainSlider.setAlpha(0.6f);
        lowBoostButton.setAlpha(0.6f);
        masterMuteButton.setAlpha(0.6f);
        speakerLayoutSelector.setAlpha(0.6f);
        subLayoutSelector.setAlpha(0.6f);
    } else {
        // 恢复正常透明度
        globalSoloButton.setAlpha(1.0f);
        globalMuteButton.setAlpha(1.0f);
        dimButton.setAlpha(1.0f);
        masterGainSlider.setAlpha(1.0f);
        lowBoostButton.setAlpha(1.0f);
        masterMuteButton.setAlpha(1.0f);
        speakerLayoutSelector.setAlpha(1.0f);
        subLayoutSelector.setAlpha(1.0f);
    }
    
    // 通道按钮的启用状态会在updateChannelButtonStates中处理
    updateChannelButtonStates();
    
    // 更新调试日志显示角色状态
    updateDebugLogDisplay();
    
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: UI updated for role - " + juce::String(static_cast<int>(currentRole)) + 
             (isSlaveMode ? " (LOCKED)" : " (INTERACTIVE)"));
}

void MonitorControllerMaxAudioProcessorEditor::updateDebugLogDisplay()
{
    // 获取连接日志
    auto& globalState = GlobalPluginState::getInstance();
    auto logs = globalState.getConnectionLogs();
    
    juce::String logText;
    
    // 添加当前连接状态摘要
    logText += "=== Connection Status ===\n";
    logText += globalState.getConnectionInfo() + "\n";
    logText += "Current Role: ";
    
    switch (audioProcessor.getCurrentRole())
    {
        case PluginRole::Standalone: logText += "Standalone"; break;
        case PluginRole::Master: logText += "Master"; break;
        case PluginRole::Slave: logText += "Slave"; break;
    }
    
    logText += "\n\n=== Connection Logs ===\n";
    
    // 显示最新的日志条目
    for (const auto& log : logs)
    {
        logText += log + "\n";
    }
    
    debugLogDisplay.setText(logText);
    debugLogDisplay.moveCaretToEnd();
    
    // 移除无意义的日志输出 - 避免垃圾日志
}

void MonitorControllerMaxAudioProcessorEditor::clearDebugLog()
{
    auto& globalState = GlobalPluginState::getInstance();
    globalState.clearConnectionLogs();
    updateDebugLogDisplay();
    
    // 移除无意义的日志输出
}
