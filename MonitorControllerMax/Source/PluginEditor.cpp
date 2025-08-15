/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DebugLogger.h"
#include "SafeUICallback.h"

//==============================================================================
MonitorControllerMaxAudioProcessorEditor::MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor& p)
    : AudioProcessorEditor (&p), audioProcessor (p), configManager(p.configManager), effectsPanel(p)
{
    addAndMakeVisible(globalMuteButton);
    globalMuteButton.setButtonText("MUTE");
    globalMuteButton.setClickingTogglesState(false);  // 手动管理状态，避免自动切换冲突
    globalMuteButton.setLookAndFeel(&customLookAndFeel);  // 🚀 关键修复：设置自定义LookAndFeel
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
    globalSoloButton.setLookAndFeel(&customLookAndFeel);  // 🚀 关键修复：设置自定义LookAndFeel
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
    dimButton.setLookAndFeel(&customLookAndFeel);  // 🚀 关键修复：设置自定义LookAndFeel
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
    
    // v4.1: 设置Dim状态变化回调 - 用于OSC控制时更新UI (SafeUICallback重构)
    audioProcessor.masterBusProcessor.onDimStateChanged = SAFE_UI_CALLBACK_SIMPLE(this, [this]()
    {
        // 🚀 稳定性优化：安全的异步UI更新，自动处理组件生命周期
        SAFE_UI_ASYNC_SIMPLE(this, [this]()
        {
            dimButton.setToggleState(audioProcessor.masterBusProcessor.isDimActive(), juce::dontSendNotification);
        });
    });
    
    // v4.2: 设置Effects面板按钮 (替代原Low Boost和Mono按钮)
    setupEffectsPanel();
    
    // v4.2: 设置Effects面板按钮状态同步回调 (用于OSC控制时更新) (SafeUICallback重构)
    audioProcessor.masterBusProcessor.onLowBoostStateChanged = SAFE_UI_CALLBACK_SIMPLE(this, [this]()
    {
        // 🚀 稳定性优化：安全的异步UI更新，防止循环引用
        SAFE_UI_ASYNC_SIMPLE(this, [this]()
        {
            effectsPanel.updateButtonStatesFromProcessor();
        });
    });
    
    audioProcessor.masterBusProcessor.onMonoStateChanged = SAFE_UI_CALLBACK_SIMPLE(this, [this]()
    {
        // 🚀 稳定性优化：安全的异步UI更新，防止循环引用
        SAFE_UI_ASYNC_SIMPLE(this, [this]()
        {
            effectsPanel.updateButtonStatesFromProcessor();
        });
    });
    
    // v4.1: 设置Master Mute按钮
    addAndMakeVisible(masterMuteButton);
    masterMuteButton.setButtonText("MASTER\nMUTE");
    masterMuteButton.setClickingTogglesState(true);
    masterMuteButton.setLookAndFeel(&customLookAndFeel);  // 🚀 关键修复：设置自定义LookAndFeel
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
    
    // v4.1: 设置Master Mute状态变化回调 - 用于OSC控制时更新UI (SafeUICallback重构)
    audioProcessor.masterBusProcessor.onMasterMuteStateChanged = SAFE_UI_CALLBACK_SIMPLE(this, [this]()
    {
        // 🚀 稳定性优化：安全的异步UI更新，自动检测组件有效性
        SAFE_UI_ASYNC_SIMPLE(this, [this]()
        {
            masterMuteButton.setToggleState(audioProcessor.masterBusProcessor.isMasterMuteActive(), juce::dontSendNotification);
        });
    });
    
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
    
    // 设置处理器的布局自动切换回调 (SafeUICallback完整应用)
    // 🚀 稳定性优化：使用SafeUICallback保护，防止循环引用和悬空指针
    auto layoutChangeCallback = [this](const juce::String& speaker, const juce::String& sub)
    {
        // 使用传统的MessageManager但带有SafePointer保护
        auto safePtr = juce::Component::SafePointer<MonitorControllerMaxAudioProcessorEditor>(this);
        juce::MessageManager::callAsync([safePtr, speaker, sub]()
        {
            // 检查组件是否仍然有效
            if (auto* self = safePtr.getComponent())
            {
                // 更新下拉框选择而不触发onChange事件
                auto speakerLayoutNames = self->configManager.getSpeakerLayoutNames();
                auto subLayoutNames = self->configManager.getSubLayoutNames();
                
                for (int i = 0; i < speakerLayoutNames.size(); ++i)
                {
                    if (speakerLayoutNames[i] == speaker)
                    {
                        self->speakerLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
                        break;
                    }
                }
                
                for (int i = 0; i < subLayoutNames.size(); ++i)
                {
                    if (subLayoutNames[i] == sub)
                    {
                        self->subLayoutSelector.setSelectedId(i + 1, juce::dontSendNotification);
                        break;
                    }
                }
                
                // 强制重新布局以显示新的通道配置
                self->resized();
            }
        });
    };
    audioProcessor.setLayoutChangeCallback(layoutChangeCallback);
    
    // 🚀 稳定性优化：降低Timer频率从30Hz到10Hz，遵循JUCE最佳实践
    // 10Hz已足够处理UI状态更新，同时减少CPU占用和竞态条件
    startTimerHz(10);
    
    // 🚀 关键修复：直接在构造函数中完成UI初始化，避免异步回调的死锁风险
    try {
        // 重要修复：从用户选择的配置同步UI，而不是当前布局
        // 这确保UI反映用户的实际选择，而不是自动推断的配置
        syncUIFromUserSelection();
        updateChannelButtonStates(); // 同步按钮状态
        
        // 🔧 关键修复：同步角色的UI状态，解决重新打开编辑器时Slave锁定状态丢失的问题
        updateUIBasedOnRole();
        
        VST3_DBG_ROLE(&audioProcessor, "PluginEditor: UI initialization complete with role-based state (direct initialization)");
        
        // 🚀 稳定性优化：标记UI初始化完成，允许Timer开始更新
        uiInitializationComplete.store(true);
        
    } catch (const std::exception& e) {
        VST3_DBG("UI initialization failed: " + juce::String(e.what()));
        // 即使初始化失败，也允许Timer运行以便后续恢复
        uiInitializationComplete.store(true);
    } catch (...) {
        VST3_DBG("UI initialization failed: unknown exception");
        // 即使初始化失败，也允许Timer运行以便后续恢复
        uiInitializationComplete.store(true);
    }
}

MonitorControllerMaxAudioProcessorEditor::~MonitorControllerMaxAudioProcessorEditor()
{
    // 🚀 稳定性优化：安全的组件清理，防止悬空指针访问
    
    // 停止Timer并标记UI不安全更新
    stopTimer();
    safeToUpdateUI.store(false);
    uiInitializationComplete.store(false);
    
    // 🛡️ JUCE LookAndFeel生命周期修复：清理所有组件的LookAndFeel引用
    // 必须在customLookAndFeel对象销毁前清理所有WeakReference
    globalMuteButton.setLookAndFeel(nullptr);
    globalSoloButton.setLookAndFeel(nullptr);
    dimButton.setLookAndFeel(nullptr);
    masterMuteButton.setLookAndFeel(nullptr);
    effectsPanelButton.setLookAndFeel(nullptr);
    
    // 清理所有语义通道按钮的LookAndFeel引用
    for (auto& [channelName, buttonPair] : semanticChannelButtons)
    {
        if (buttonPair && buttonPair->soloButton)
            buttonPair->soloButton->setLookAndFeel(nullptr);
        if (buttonPair && buttonPair->muteButton)
            buttonPair->muteButton->setLookAndFeel(nullptr);
    }
    
    // 清理传统通道按钮的LookAndFeel引用
    for (auto& [channelIndex, button] : channelButtons)
    {
        if (button)
            button->setLookAndFeel(nullptr);
    }
    
    // 清理主编辑器的LookAndFeel引用
    setLookAndFeel(nullptr);
    
    // 清理所有不安全的回调引用（SafeUICallback将自动处理）
    // 但为了明确性，手动清理主要回调
    audioProcessor.masterBusProcessor.onDimStateChanged = nullptr;
    audioProcessor.masterBusProcessor.onLowBoostStateChanged = nullptr;
    audioProcessor.masterBusProcessor.onMonoStateChanged = nullptr;
    audioProcessor.masterBusProcessor.onMasterMuteStateChanged = nullptr;
    
    // 清理布局变化回调
    audioProcessor.setLayoutChangeCallback(nullptr);
    
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: Safe destruction complete - all callbacks and LookAndFeel cleared");
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
    
    // v4.1: 添加Master Mute按钮 (与Dim按钮同样大小)
    sidebarFlex.items.add(juce::FlexItem(masterMuteButton).withHeight(50).withMargin(5));
    
    // v4.2: 添加空隙分隔
    sidebarFlex.items.add(juce::FlexItem().withHeight(10));
    
    // v4.2: 添加Effects面板按钮
    sidebarFlex.items.add(juce::FlexItem(effectsPanelButton).withHeight(50).withMargin(5));
    
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

void MonitorControllerMaxAudioProcessorEditor::mouseDown(const juce::MouseEvent& event)
{
    // v4.2: 处理Effects面板外部点击关闭
    handleEffectsPanelOutsideClick(event);
    
    // 调用基类处理
    juce::Component::mouseDown(event);
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
            
            // 🚀 关键修复：设置自定义LookAndFeel，确保颜色能正确显示
            button->setLookAndFeel(&customLookAndFeel);

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
             
             // 🚀 关键修复：SUB按钮也需要设置自定义LookAndFeel
             auto* subButton = channelButtons[subChannelIndex].get();
             subButton->setLookAndFeel(&customLookAndFeel);
             subButton->setClickingTogglesState(false); // 手动管理状态
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
            
            // 🚀 关键修复：设置自定义LookAndFeel，确保颜色能正确显示
            button->setLookAndFeel(&customLookAndFeel);

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
             
             // 🚀 关键修复：SUB按钮也需要设置自定义LookAndFeel
             auto* subButton = channelButtons[subChannelIndex].get();
             subButton->setLookAndFeel(&customLookAndFeel);
             subButton->setClickingTogglesState(false); // 手动管理状态
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
    // 🚀 修复static变量竞争：使用实例级原子计数器
    const uint32_t currentCall = timerCallCount.fetch_add(1, std::memory_order_acq_rel);
    if (currentCall % 100 == 1) {  // 每10秒记录一次（10Hz * 100）
        VST3_DBG("Timer callback running - count: " + juce::String(currentCall) + 
                ", uiInitComplete: " + (uiInitializationComplete.load(std::memory_order_acquire) ? "true" : "false") + 
                ", safeToUpdate: " + (safeToUpdateUI.load(std::memory_order_acquire) ? "true" : "false"));
    }
    
    // 🚀 稳定性优化：检查UI初始化状态，防止与初始化的竞态条件
    if (!uiInitializationComplete.load() || !safeToUpdateUI.load()) {
        return; // 初始化未完成或不安全时，跳过Timer更新
    }
    
    try {
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
        
        // 🚀 稳定性优化：降低Debug日志更新频率 - 10Hz Timer下每秒更新一次
        if (currentCall % 10 == 0) { // 10Hz Timer下每秒更新1次
            updateDebugLogDisplay();
        }
    }
    catch (const std::exception& e) {
        VST3_DBG("SafeUICallback: Exception in timerCallback: " + juce::String(e.what()));
    }
    catch (...) {
        VST3_DBG("SafeUICallback: Unknown exception in timerCallback");
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
    // 🚀 修复：使用StateManager作为UI状态的唯一数据源，符合稳定性架构
    auto* stateManager = audioProcessor.stateManager.get();
    if (!stateManager) {
        VST3_DBG("PluginEditor: StateManager is null in updateChannelButtonStates");
        return;
    }
    
    // 🚀 修复static变量竞争：使用实例级计数器
    const uint32_t updateCount = updateButtonStatesCount.fetch_add(1, std::memory_order_acq_rel);
    VST3_DBG("PluginEditor: updateChannelButtonStates called - count: " + juce::String(updateCount));
    
    // 1. 更新每个通道按钮（基于StateManager的线程安全查询）
    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible() || index < 0) continue;
        
        // 获取语义通道名
        juce::String semanticChannelName = audioProcessor.getPhysicalMapper().getSemanticName(index);
        if (semanticChannelName.isEmpty()) continue;
        
        // 从StateManager获取按钮颜色（线程安全，带缓存）
        juce::Colour buttonColor = stateManager->getChannelButtonColor(semanticChannelName, getLookAndFeel());
        
        // 获取Solo状态以决定按钮的toggle状态
        bool soloState = stateManager->getChannelSoloStateForUI(semanticChannelName);
        bool muteState = stateManager->getChannelMuteStateForUI(semanticChannelName);
        bool shouldBeActive = soloState;  // 只有Solo时按钮才显示为激活状态
        
        // 调试输出
        VST3_DBG("UI Update - Channel: " + semanticChannelName + 
                 ", Solo: " + (soloState ? "ON" : "OFF") + 
                 ", Mute: " + (muteState ? "ON" : "OFF") +
                 ", Color: 0x" + buttonColor.toDisplayString(false));
        
        // 更新按钮toggle状态
        if (button->getToggleState() != shouldBeActive) {
            button->setToggleState(shouldBeActive, juce::dontSendNotification);
        }
        
        // 更新按钮颜色
        button->setColour(juce::TextButton::buttonColourId, buttonColor);
        button->setColour(juce::TextButton::buttonOnColourId, buttonColor);
        
        // 强制重绘确保颜色更新
        button->repaint();
    }
    
    // 2. 更新主控按钮（使用现有的processor查询方法）
    bool soloButtonActive = audioProcessor.isSoloButtonActive();
    bool muteButtonActive = audioProcessor.isMuteButtonActive();
    
    // 更新Solo按钮
    if (globalSoloButton.getToggleState() != soloButtonActive) {
        globalSoloButton.setToggleState(soloButtonActive, juce::dontSendNotification);
    }
    
    if (soloButtonActive) {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getSoloColour());
        globalSoloButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getSoloColour());
    } else {
        globalSoloButton.setColour(juce::TextButton::buttonOnColourId, 
                                  getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalSoloButton.setColour(juce::TextButton::buttonColourId, 
                                  getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    globalSoloButton.repaint();
    
    // 更新Mute按钮
    if (globalMuteButton.getToggleState() != muteButtonActive) {
        globalMuteButton.setToggleState(muteButtonActive, juce::dontSendNotification);
    }
    
    if (muteButtonActive) {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, customLookAndFeel.getMuteColour());
        globalMuteButton.setColour(juce::TextButton::buttonColourId, customLookAndFeel.getMuteColour());
    } else {
        globalMuteButton.setColour(juce::TextButton::buttonOnColourId, 
                                  getLookAndFeel().findColour(juce::TextButton::buttonColourId));
        globalMuteButton.setColour(juce::TextButton::buttonColourId, 
                                  getLookAndFeel().findColour(juce::TextButton::buttonColourId));
    }
    globalMuteButton.repaint();
    
    // 3. Solo优先规则处理
    bool muteButtonEnabled = audioProcessor.isMuteButtonEnabled();
    PluginRole currentRole = audioProcessor.getCurrentRole();
    bool isSlaveMode = (currentRole == PluginRole::Slave);
    
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
    // 🚀 稳定性优化：使用SafeUICallback保护所有异步调用
    SAFE_UI_ASYNC_SIMPLE(this, [this]()
    {
        audioProcessor.updateHostDisplay();
        
        // 延迟额外刷新，确保REAPER能获取到最新的通道名称
        juce::Timer::callAfterDelay(100, SAFE_UI_CALLBACK_SIMPLE(this, [this]()
        {
            audioProcessor.updateHostDisplay();
        }));
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
    masterMuteButton.setEnabled(!isSlaveMode);
    
    // v4.2: Slave模式禁用Effects面板按钮
    effectsPanelButton.setEnabled(!isSlaveMode);
    
    // v4.2: 更新Effects面板内部按钮的角色权限
    effectsPanel.updateButtonStatesForRole();
    
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
        masterMuteButton.setAlpha(0.6f);
        effectsPanelButton.setAlpha(0.6f);
        speakerLayoutSelector.setAlpha(0.6f);
        subLayoutSelector.setAlpha(0.6f);
    } else {
        // 恢复正常透明度
        globalSoloButton.setAlpha(1.0f);
        globalMuteButton.setAlpha(1.0f);
        dimButton.setAlpha(1.0f);
        masterGainSlider.setAlpha(1.0f);
        masterMuteButton.setAlpha(1.0f);
        effectsPanelButton.setAlpha(1.0f);
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
    auto& globalState = GlobalPluginState::getRef();
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
    auto& globalState = GlobalPluginState::getRef();
    globalState.clearConnectionLogs();
    updateDebugLogDisplay();
    
    // 移除无意义的日志输出
}

//==============================================================================
// v4.2: Effects面板管理方法

void MonitorControllerMaxAudioProcessorEditor::setupEffectsPanel()
{
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: Setting up Effects panel");
    
    // 设置Effects面板按钮 (完全参照MASTER MUTE按钮的实现)
    addAndMakeVisible(effectsPanelButton);
    effectsPanelButton.setButtonText("EFFECT");  // 简化为单行文字
    effectsPanelButton.setClickingTogglesState(true);
    effectsPanelButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::green);
    
    // 连接按钮点击事件
    effectsPanelButton.onClick = [this]()
    {
        handleEffectsPanelButtonClick();
    };
    
    // 添加Effects面板为子组件但初始隐藏
    addAndMakeVisible(effectsPanel);
    effectsPanel.setVisible(false);
    
    VST3_DBG_ROLE(&audioProcessor, "PluginEditor: Effects panel setup complete");
}

void MonitorControllerMaxAudioProcessorEditor::handleEffectsPanelButtonClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave) {
        VST3_DBG_ROLE(&audioProcessor, "Effects panel button click ignored - Slave mode");
        return;
    }
    
    // 切换面板显示状态
    if (effectsPanel.isPanelVisible()) {
        effectsPanel.hidePanel();
        effectsPanelButton.setToggleState(false, juce::dontSendNotification);
        VST3_DBG_ROLE(&audioProcessor, "Effects panel hidden via button");
    } else {
        // 设置面板位置 (完全覆盖通道网格区域)
        auto channelGridBounds = channelGridContainer.getBounds();
        int panelX = channelGridBounds.getX();
        int panelY = channelGridBounds.getY();
        
        effectsPanel.setBounds(panelX, panelY, 
                              channelGridBounds.getWidth(), 
                              channelGridBounds.getHeight());
        
        effectsPanel.showPanel();
        effectsPanelButton.setToggleState(true, juce::dontSendNotification);
        VST3_DBG_ROLE(&audioProcessor, "Effects panel shown via button");
    }
}

void MonitorControllerMaxAudioProcessorEditor::handleEffectsPanelOutsideClick(const juce::MouseEvent& event)
{
    // 检查点击是否在面板外部
    if (effectsPanel.isPanelVisible() && !effectsPanel.getBounds().contains(event.getPosition()))
    {
        // 点击面板外部，隐藏面板
        effectsPanel.hidePanel();
        effectsPanelButton.setToggleState(false, juce::dontSendNotification);
        VST3_DBG_ROLE(&audioProcessor, "Effects panel hidden via outside click");
    }
}
