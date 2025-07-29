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
    // 面板样式常量 (5×5网格布局，与主界面完全一致)
    // 注意：面板尺寸现在动态匹配通道网格区域，不再使用固定尺寸常量
    static constexpr int PANEL_MARGIN = 20;
    static constexpr float PANEL_CORNER_RADIUS = 6.0f;
    
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
    
    // 总线效果按钮集合 (模块化扩展)
    juce::TextButton lowBoostButton{ "LOW BOOST" };
    juce::TextButton highBoostButton{ "HIGH BOOST" };  // 新增
    juce::TextButton monoButton{ "MONO" };
    juce::TextButton dolbyCurveButton{ "DOLBY CURVE" };  // 新增
    juce::TextButton phoneCurveButton{ "PHONE CURVE" };  // 新增
    
    // 5×5网格布局系统 (与主界面一致)
    juce::Grid effectsGrid;
    
    //==============================================================================
    // 私有方法
    void setupButtons();
    void setupLowBoostButton();
    void setupHighBoostButton();    // 新增
    void setupMonoButton();
    void setupDolbyCurveButton();   // 新增
    void setupPhoneCurveButton();   // 新增
    
    // 按钮回调处理
    void handleLowBoostClick();
    void handleHighBoostClick();    // 新增
    void handleMonoClick();
    void handleDolbyCurveClick();   // 新增
    void handlePhoneCurveClick();   // 新增
    
    // 布局和绘制辅助
    void layoutButtons(juce::Rectangle<int> area);
    void setupEffectsGrid();  // 设置5×5网格布局
    void drawPanelBackground(juce::Graphics& g, juce::Rectangle<int> area);
    void drawPanelBorder(juce::Graphics& g, juce::Rectangle<int> area);
    
    //==============================================================================
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(EffectsPanel)
};