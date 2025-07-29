/*
  ==============================================================================

    EffectsPanel.cpp
    Created: 2025-07-29
    Author:  GohardSGG & Claude Code

    弹出式总线效果面板实现 - v4.2 UI集中化重构

  ==============================================================================
*/

#include "EffectsPanel.h"
#include "PluginEditor.h"
#include "DebugLogger.h"

//==============================================================================
// 面板颜色常量定义 (与现有深色主题一致)
const juce::Colour EffectsPanel::PANEL_BACKGROUND = juce::Colour(0xff2d2d2d);
const juce::Colour EffectsPanel::PANEL_BORDER = juce::Colour(0xff5d5d5d);
const juce::Colour EffectsPanel::PANEL_SHADOW = juce::Colour(0x80000000);

//==============================================================================
EffectsPanel::EffectsPanel(MonitorControllerMaxAudioProcessor& processor)
    : audioProcessor(processor)
{
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Initialize effects panel");
    
    // 初始状态为隐藏
    panelVisible = false;
    setVisible(false);
    
    // 设置面板尺寸
    setSize(PANEL_WIDTH, PANEL_HEIGHT);
    
    // 初始化所有按钮
    setupButtons();
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Initialization complete");
}

EffectsPanel::~EffectsPanel()
{
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Destructor called");
}

//==============================================================================
// 面板显示控制
void EffectsPanel::showPanel()
{
    if (!panelVisible)
    {
        panelVisible = true;
        setVisible(true);
        toFront(true);  // 确保面板在最前面
        
        // 更新按钮状态以反映当前处理器状态
        updateButtonStatesFromProcessor();
        updateButtonStatesForRole();
        
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Panel shown");
    }
}

void EffectsPanel::hidePanel()
{
    if (panelVisible)
    {
        panelVisible = false;
        setVisible(false);
        
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Panel hidden");
    }
}

bool EffectsPanel::isPanelVisible() const
{
    return panelVisible;
}

//==============================================================================
// 按钮设置和初始化
void EffectsPanel::setupButtons()
{
    setupLowBoostButton();
    setupMonoButton();
}

void EffectsPanel::setupLowBoostButton()
{
    // 基本属性设置
    addAndMakeVisible(lowBoostButton);
    lowBoostButton.setButtonText("LOW BOOST");
    lowBoostButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    lowBoostButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // 点击回调 - 直接调用MasterBusProcessor
    lowBoostButton.onClick = [this]()
    {
        handleLowBoostClick();
    };
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Low Boost button initialized");
}

void EffectsPanel::setupMonoButton()
{
    // 基本属性设置
    addAndMakeVisible(monoButton);
    monoButton.setButtonText("MONO");
    monoButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    monoButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::yellow);
    
    // 点击回调 - 直接调用MasterBusProcessor
    monoButton.onClick = [this]()
    {
        handleMonoClick();
    };
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Mono button initialized");
}

//==============================================================================
// 按钮点击处理
void EffectsPanel::handleLowBoostClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave)
    {
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Low Boost click ignored - Slave mode");
        return;  
    }
    
    // 切换Low Boost状态
    audioProcessor.masterBusProcessor.toggleLowBoost();
    
    // 更新按钮状态 (避免状态不同步)
    lowBoostButton.setToggleState(audioProcessor.masterBusProcessor.isLowBoostActive(), 
                                  juce::dontSendNotification);
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Low Boost toggled to " +
                 juce::String(audioProcessor.masterBusProcessor.isLowBoostActive() ? "ON" : "OFF"));
}

void EffectsPanel::handleMonoClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave)
    {
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Mono click ignored - Slave mode");
        return;
    }
    
    // 切换Mono状态
    audioProcessor.masterBusProcessor.toggleMono();
    
    // 更新按钮状态 (避免状态不同步)
    monoButton.setToggleState(audioProcessor.masterBusProcessor.isMonoActive(),
                              juce::dontSendNotification);
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Mono toggled to " +
                 juce::String(audioProcessor.masterBusProcessor.isMonoActive() ? "ON" : "OFF"));
}

//==============================================================================
// 角色权限和状态更新
void EffectsPanel::updateButtonStatesForRole()
{
    bool isSlaveMode = (audioProcessor.getCurrentRole() == PluginRole::Slave);
    
    // 角色化启用/禁用
    lowBoostButton.setEnabled(!isSlaveMode);
    monoButton.setEnabled(!isSlaveMode);
    
    // 视觉反馈 - Slave模式时降低透明度
    if (isSlaveMode)
    {
        lowBoostButton.setAlpha(0.6f);
        monoButton.setAlpha(0.6f);
    }
    else
    {
        lowBoostButton.setAlpha(1.0f);
        monoButton.setAlpha(1.0f);
    }
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Button states updated for role - " +
                 juce::String(isSlaveMode ? "LOCKED" : "INTERACTIVE"));
}

void EffectsPanel::updateButtonStatesFromProcessor()
{
    // 同步按钮状态到当前处理器状态 (用于OSC控制等外部更新)
    lowBoostButton.setToggleState(audioProcessor.masterBusProcessor.isLowBoostActive(),
                                  juce::dontSendNotification);
    monoButton.setToggleState(audioProcessor.masterBusProcessor.isMonoActive(),
                              juce::dontSendNotification);
}

//==============================================================================
// Component overrides
void EffectsPanel::paint(juce::Graphics& g)
{
    auto area = getLocalBounds();
    
    // 绘制面板背景和边框
    drawPanelBackground(g, area);
    drawPanelBorder(g, area);
}

void EffectsPanel::resized()
{
    auto area = getLocalBounds().reduced(10);  // 内边距
    layoutButtons(area);
}

void EffectsPanel::mouseDown(const juce::MouseEvent& event)
{
    // 点击面板外部关闭面板的逻辑由父组件处理
    // 这里只处理面板内部的点击
    juce::Component::mouseDown(event);
}

//==============================================================================
// 私有绘制和布局方法
void EffectsPanel::layoutButtons(juce::Rectangle<int> area)
{
    // 使用FlexBox进行2x1网格布局
    juce::FlexBox buttonLayout;
    buttonLayout.flexDirection = juce::FlexBox::Direction::row;
    buttonLayout.justifyContent = juce::FlexBox::JustifyContent::spaceBetween;
    buttonLayout.alignItems = juce::FlexBox::AlignItems::center;
    
    // 添加按钮到布局 (每个按钮占用相等空间，带边距)
    buttonLayout.items.add(juce::FlexItem(lowBoostButton).withFlex(1).withMargin(5));
    buttonLayout.items.add(juce::FlexItem(monoButton).withFlex(1).withMargin(5));
    
    // 应用布局
    buttonLayout.performLayout(area);
}

void EffectsPanel::drawPanelBackground(juce::Graphics& g, juce::Rectangle<int> area)
{
    // 绘制阴影效果
    auto shadowArea = area.expanded(2);
    g.setColour(PANEL_SHADOW);
    g.fillRoundedRectangle(shadowArea.toFloat(), PANEL_CORNER_RADIUS + 1.0f);
    
    // 绘制主背景
    g.setColour(PANEL_BACKGROUND);
    g.fillRoundedRectangle(area.toFloat(), PANEL_CORNER_RADIUS);
}

void EffectsPanel::drawPanelBorder(juce::Graphics& g, juce::Rectangle<int> area)
{
    // 绘制边框
    g.setColour(PANEL_BORDER);
    g.drawRoundedRectangle(area.toFloat(), PANEL_CORNER_RADIUS, 1.0f);
}