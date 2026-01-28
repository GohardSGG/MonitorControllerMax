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
// 面板颜色常量定义 (简化设计)
const juce::Colour EffectsPanel::PANEL_BACKGROUND = juce::Colour(0x80404040);  // 半透明灰色
const juce::Colour EffectsPanel::PANEL_BORDER = juce::Colour(0xff606060);     // 简单边框
const juce::Colour EffectsPanel::PANEL_SHADOW = juce::Colour(0x40000000);     // 简单阴影

//==============================================================================
EffectsPanel::EffectsPanel(MonitorControllerMaxAudioProcessor& processor)
    : audioProcessor(processor)
{
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Initialize effects panel");
    
    // 初始状态为隐藏
    panelVisible = false;
    setVisible(false);
    
    // 注意：面板尺寸现在由父组件动态设置，无需在构造函数中设置固定尺寸
    
    // 初始化所有按钮
    setupButtons();
    
    // 设置5×5网格布局系统
    setupEffectsGrid();
    
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
        
        // 强制触发resized()来确保布局正确
        resized();
        
        // 更新按钮状态以反映当前处理器状态
        updateButtonStatesFromProcessor();
        updateButtonStatesForRole();
        
        // 详细调试信息
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Panel shown - Size: " +
                     juce::String(getWidth()) + "x" + juce::String(getHeight()) +
                     ", Position: " + juce::String(getX()) + "," + juce::String(getY()));
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: LowBoost visible: " + juce::String(lowBoostButton.isVisible() ? "YES" : "NO"));
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Mono visible: " + juce::String(monoButton.isVisible() ? "YES" : "NO"));
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
    setupHighBoostButton();
    setupMonoButton(); 
    setupDolbyCurveButton();
    setupPhoneCurveButton();
}

void EffectsPanel::setupLowBoostButton()
{
    // 基本属性设置
    addAndMakeVisible(lowBoostButton);
    lowBoostButton.setButtonText("LOW BOOST");
    lowBoostButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    lowBoostButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // 字体颜色设置 (与其他按钮保持一致)
    lowBoostButton.setColour(juce::TextButton::textColourOffId, juce::Colours::white);
    lowBoostButton.setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    
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

void EffectsPanel::setupHighBoostButton()
{
    // 基本属性设置 (完全参照LOW BOOST按钮)
    addAndMakeVisible(highBoostButton);
    highBoostButton.setButtonText("HIGH BOOST");
    highBoostButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    highBoostButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // 字体大小设置 (使用LookAndFeel设置统一字体大小)
    highBoostButton.setColour(juce::TextButton::textColourOffId, juce::Colours::white);
    highBoostButton.setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    
    // 点击回调 - 直接调用MasterBusProcessor
    highBoostButton.onClick = [this]()
    {
        handleHighBoostClick();
    };
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: High Boost button initialized");
}

void EffectsPanel::setupDolbyCurveButton()
{
    // 基本属性设置 (完全参照LOW BOOST按钮)
    addAndMakeVisible(dolbyCurveButton);
    dolbyCurveButton.setButtonText("DOLBY CURVE");
    dolbyCurveButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    dolbyCurveButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // 字体大小设置 (使用LookAndFeel设置统一字体大小)
    dolbyCurveButton.setColour(juce::TextButton::textColourOffId, juce::Colours::white);
    dolbyCurveButton.setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    
    // 点击回调 - 直接调用MasterBusProcessor
    dolbyCurveButton.onClick = [this]()
    {
        handleDolbyCurveClick();
    };
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Dolby Curve button initialized");
}

void EffectsPanel::setupPhoneCurveButton()
{
    // 基本属性设置 (完全参照LOW BOOST按钮)
    addAndMakeVisible(phoneCurveButton);
    phoneCurveButton.setButtonText("PHONE CURVE");
    phoneCurveButton.setClickingTogglesState(true);
    
    // 颜色设置 (与原实现保持一致)
    phoneCurveButton.setColour(juce::TextButton::buttonOnColourId, juce::Colours::orange);
    
    // 字体大小设置 (使用LookAndFeel设置统一字体大小)
    phoneCurveButton.setColour(juce::TextButton::textColourOffId, juce::Colours::white);
    phoneCurveButton.setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    
    // 点击回调 - 直接调用MasterBusProcessor
    phoneCurveButton.onClick = [this]()
    {
        handlePhoneCurveClick();
    };
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Phone Curve button initialized");
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

void EffectsPanel::handleHighBoostClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave)
    {
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: High Boost click ignored - Slave mode");
        return;
    }
    
    // TODO: 实现High Boost功能 - 模块化设计便于后续扩展
    // audioProcessor.masterBusProcessor.toggleHighBoost();
    
    // 临时设置按钮状态切换 (实际功能待实现)
    bool newState = !highBoostButton.getToggleState();
    highBoostButton.setToggleState(newState, juce::dontSendNotification);
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: High Boost toggled to " +
                 juce::String(newState ? "ON" : "OFF") + " (功能待实现)");
}

void EffectsPanel::handleDolbyCurveClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave)
    {
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Dolby Curve click ignored - Slave mode");
        return;
    }
    
    // TODO: 实现Dolby Curve功能 - 模块化设计便于后续扩展
    // audioProcessor.masterBusProcessor.toggleDolbyCurve();
    
    // 临时设置按钮状态切换 (实际功能待实现)
    bool newState = !dolbyCurveButton.getToggleState();
    dolbyCurveButton.setToggleState(newState, juce::dontSendNotification);
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Dolby Curve toggled to " +
                 juce::String(newState ? "ON" : "OFF") + " (功能待实现)");
}

void EffectsPanel::handlePhoneCurveClick()
{
    // 检查角色权限 - Slave模式禁止操作
    if (audioProcessor.getCurrentRole() == PluginRole::Slave)
    {
        VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Phone Curve click ignored - Slave mode");
        return;
    }
    
    // TODO: 实现Phone Curve功能 - 模块化设计便于后续扩展
    // audioProcessor.masterBusProcessor.togglePhoneCurve();
    
    // 临时设置按钮状态切换 (实际功能待实现)
    bool newState = !phoneCurveButton.getToggleState();
    phoneCurveButton.setToggleState(newState, juce::dontSendNotification);
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Phone Curve toggled to " +
                 juce::String(newState ? "ON" : "OFF") + " (功能待实现)");
}

//==============================================================================
// 角色权限和状态更新
void EffectsPanel::updateButtonStatesForRole()
{
    bool isSlaveMode = (audioProcessor.getCurrentRole() == PluginRole::Slave);
    
    // 角色化启用/禁用 (所有按钮)
    lowBoostButton.setEnabled(!isSlaveMode);
    highBoostButton.setEnabled(!isSlaveMode);
    monoButton.setEnabled(!isSlaveMode);
    dolbyCurveButton.setEnabled(!isSlaveMode);
    phoneCurveButton.setEnabled(!isSlaveMode);
    
    // 视觉反馈 - Slave模式时降低透明度
    float alpha = isSlaveMode ? 0.6f : 1.0f;
    lowBoostButton.setAlpha(alpha);
    highBoostButton.setAlpha(alpha);
    monoButton.setAlpha(alpha);
    dolbyCurveButton.setAlpha(alpha);
    phoneCurveButton.setAlpha(alpha);
    
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
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: paint() called - Area: " +
                 juce::String(area.getWidth()) + "x" + juce::String(area.getHeight()));
    
    // 绘制面板背景和边框
    drawPanelBackground(g, area);
    drawPanelBorder(g, area);
}

void EffectsPanel::resized()
{
    // 使用JUCE Grid布局系统，与主界面完全一致（无内边距以实现完美对齐）
    effectsGrid.performLayout(getLocalBounds());
}

void EffectsPanel::mouseDown(const juce::MouseEvent& event)
{
    // 点击面板外部关闭面板的逻辑由父组件处理
    // 这里只处理面板内部的点击
    juce::Component::mouseDown(event);
}

//==============================================================================
// 私有绘制和布局方法

void EffectsPanel::setupEffectsGrid()
{
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: Setting up 5x5 grid layout");
    
    // 清空现有网格配置
    effectsGrid.items.clear();
    effectsGrid.setGap(juce::Grid::Px(5));  // 与主界面相同的间距
    effectsGrid.templateRows.clear();
    effectsGrid.templateColumns.clear();
    
    // 创建5×5网格布局
    for (int i = 0; i < 5; ++i)
    {
        effectsGrid.templateRows.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
        effectsGrid.templateColumns.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
    }
    
    // 创建25个空的GridItem（代表5×5网格的所有位置）
    std::vector<juce::GridItem> gridItems(25);
    
    // 第1行：LOW BOOST (位置1), HIGH BOOST (位置2)
    gridItems[0] = juce::GridItem(lowBoostButton);   // 位置1 (第1行第1列)
    gridItems[1] = juce::GridItem(highBoostButton);  // 位置2 (第1行第2列) 
    
    // 第2行：DOLBY CURVE (位置6), PHONE CURVE (位置7)  
    gridItems[5] = juce::GridItem(dolbyCurveButton); // 位置6 (第2行第1列)
    gridItems[6] = juce::GridItem(phoneCurveButton); // 位置7 (第2行第2列)
    
    // 第5行：MONO (位置21) - 最后一行第1个位置
    gridItems[20] = juce::GridItem(monoButton);      // 位置21 (第5行第1列)
    
    // 将所有GridItem添加到网格中
    for (auto& item : gridItems)
    {
        effectsGrid.items.add(item);
    }
    
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: 5x5 grid setup complete - " +
                 juce::String("LOW BOOST@1, HIGH BOOST@2, DOLBY CURVE@6, PHONE CURVE@7, MONO@21"));
}

void EffectsPanel::layoutButtons(juce::Rectangle<int> area)
{
    // 这个方法现在由setupEffectsGrid()和resized()中的effectsGrid.performLayout()替代
    // 保留用于调试目的
    VST3_DBG_ROLE(&audioProcessor, "EffectsPanel: layoutButtons called (now using grid layout)");
}

void EffectsPanel::drawPanelBackground(juce::Graphics& g, juce::Rectangle<int> area)
{
    // 简单阴影
    auto shadowArea = area.expanded(2);
    g.setColour(PANEL_SHADOW);
    g.fillRoundedRectangle(shadowArea.toFloat(), PANEL_CORNER_RADIUS);
    
    // 半透明灰色背景 (可透视背后内容)
    g.setColour(PANEL_BACKGROUND);
    g.fillRoundedRectangle(area.toFloat(), PANEL_CORNER_RADIUS);
}

void EffectsPanel::drawPanelBorder(juce::Graphics& g, juce::Rectangle<int> area)
{
    // 简单边框
    g.setColour(PANEL_BORDER);
    g.drawRoundedRectangle(area.toFloat(), PANEL_CORNER_RADIUS, 1.0f);
}