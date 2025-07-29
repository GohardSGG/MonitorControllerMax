/*
  ==============================================================================

    EffectsPanel.h
    Created: 2025-07-29
    Author:  GohardSGG & Claude Code

    弹出式总线效果面板 - v4.2 UI集中化重构
    
    功能：
    - 集中管理Low Boost和Mono等总线效果按钮
    - 弹出式覆盖面板，不影响通道网格操作
    - 支持角色权限控制和Master-Slave状态同步
    - 可扩展的网格布局设计

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"

//==============================================================================
/**
 * 总线效果面板类 - 弹出式界面组件
 * 
 * 设计原则：
 * - 模态覆盖显示，不影响后台通道功能
 * - 集中管理所有总线效果控件
 * - 维持与MasterBusProcessor的直接连接
 * - 支持角色化权限控制
 */
class EffectsPanel : public juce::Component
{
public:
    //==============================================================================
    explicit EffectsPanel(MonitorControllerMaxAudioProcessor& processor);
    ~EffectsPanel() override;

    //==============================================================================
    // 面板显示控制
    void showPanel();
    void hidePanel(); 
    bool isPanelVisible() const;
    
    // 角色权限更新
    void updateButtonStatesForRole();
    
    // 从外部触发的按钮状态更新 (用于OSC控制同步)
    void updateButtonStatesFromProcessor();

    //==============================================================================
    // Component overrides
    void paint(juce::Graphics& g) override;
    void resized() override;
    void mouseDown(const juce::MouseEvent& event) override;
    
    //==============================================================================
    // 面板样式常量
    static constexpr int PANEL_WIDTH = 250;
    static constexpr int PANEL_HEIGHT = 120;
    static constexpr int PANEL_MARGIN = 20;
    static constexpr float PANEL_CORNER_RADIUS = 8.0f;
    
    // 面板颜色常量 (与现有深色主题一致)
    static const juce::Colour PANEL_BACKGROUND;
    static const juce::Colour PANEL_BORDER;
    static const juce::Colour PANEL_SHADOW;

private:
    //==============================================================================
    // 核心引用
    MonitorControllerMaxAudioProcessor& audioProcessor;
    
    // 面板状态
    bool panelVisible = false;
    
    // 迁移的总线效果按钮
    juce::TextButton lowBoostButton{ "LOW BOOST" };
    juce::TextButton monoButton{ "MONO" };
    
    //==============================================================================
    // 私有方法
    void setupButtons();
    void setupLowBoostButton();
    void setupMonoButton();
    
    // 按钮回调处理
    void handleLowBoostClick();
    void handleMonoClick();
    
    // 布局和绘制辅助
    void layoutButtons(juce::Rectangle<int> area);
    void drawPanelBackground(juce::Graphics& g, juce::Rectangle<int> area);
    void drawPanelBorder(juce::Graphics& g, juce::Rectangle<int> area);
    
    //==============================================================================
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(EffectsPanel)
};