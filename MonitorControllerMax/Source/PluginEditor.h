/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"
#include "ConfigManager.h"
#include <map>

//==============================================================================
/** 一个简单的自定义 LookAndFeel 类，用于实现深色UI风格。 */
class CustomLookAndFeel : public juce::LookAndFeel_V4
{
public:
    CustomLookAndFeel()
    {
        // 设置一个深色主题
        setColour(juce::ResizableWindow::backgroundColourId, juce::Colour(0xff323e44));
        setColour(juce::TextButton::buttonColourId, juce::Colour(0xff4a5860));
        setColour(juce::TextButton::buttonOnColourId, juce::Colour(0xffd13a3a)); // 用于Mute激活状态的红色
        setColour(juce::TextButton::textColourOffId, juce::Colours::lightgrey);
        setColour(juce::TextButton::textColourOnId, juce::Colours::white);
        setColour(juce::ComboBox::backgroundColourId, juce::Colour(0xff4a5860));
        setColour(juce::ComboBox::outlineColourId, juce::Colours::transparentBlack);
        setColour(juce::ComboBox::arrowColourId, juce::Colours::lightgrey);
        setColour(juce::PopupMenu::backgroundColourId, juce::Colour(0xff4a5860));
        setColour(juce::PopupMenu::highlightedBackgroundColourId, juce::Colour(0xfff07800)); // 用于高亮的橙色

        // 为Solo状态定义一个独特的颜色
        soloColour = juce::Colour(0xff2a8c4a);
    }

    void drawButtonBackground(juce::Graphics& g, juce::Button& button, const juce::Colour& backgroundColour,
                              bool shouldDrawButtonAsHighlighted, bool shouldDrawButtonAsDown) override
    {
        auto cornerSize = 6.0f;
        auto originalBounds = button.getLocalBounds();

        auto baseColour = backgroundColour.withMultipliedAlpha(button.isEnabled() ? 1.0f : 0.5f);

        if (shouldDrawButtonAsDown || shouldDrawButtonAsHighlighted)
            baseColour = baseColour.contrasting(shouldDrawButtonAsDown ? 0.2f : 0.1f);

        g.setColour(baseColour);

        if (button.getButtonText().startsWith("SUB "))
        {
            // 为SUB通道按钮绘制圆形
            auto diameter = (float)juce::jmin(originalBounds.getWidth(), originalBounds.getHeight());
            g.fillEllipse(originalBounds.toFloat().withSizeKeepingCentre(diameter, diameter));
        }
        else
        {
            // 为所有其他按钮绘制强制的正方形
            auto side = juce::jmin(originalBounds.getWidth(), originalBounds.getHeight());
            auto squareBounds = originalBounds.toFloat().withSizeKeepingCentre(side, side);
            g.fillRoundedRectangle(squareBounds, cornerSize);
        }
    }

    juce::Colour getSoloColour() const { return soloColour; }

private:
    juce::Colour soloColour;
};

//==============================================================================
/**
*/
class MonitorControllerMaxAudioProcessorEditor  : public juce::AudioProcessorEditor,
                                                  public juce::Timer
{
public:
    MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor&);
    ~MonitorControllerMaxAudioProcessorEditor() override;

    //==============================================================================
    void paint (juce::Graphics&) override;
    void resized() override;
    void timerCallback() override;

private:
    using ButtonAttachment = juce::AudioProcessorValueTreeState::ButtonAttachment;
    
    enum class UIMode
    {
        Normal,
        AssignSolo,
        AssignMute
    };

    void updateChannelButtonStates();
    void updateLayout();
    void setUIMode(UIMode newMode);

    // This reference is provided as a quick way for your editor to
    // access the processor object that created it.
    MonitorControllerMaxAudioProcessor& audioProcessor;
    ConfigManager& configManager;

    UIMode currentUIMode { UIMode::Normal };

    juce::TextButton globalMuteButton{ "Mute" };
    juce::TextButton globalSoloButton{ "Solo" };
    juce::TextButton dimButton{ "Dim" };
    
    juce::ComboBox speakerLayoutSelector;
    juce::ComboBox subLayoutSelector;

    juce::FlexBox sidebar;
    juce::FlexBox selectorBox;
    juce::Grid channelGrid; // Grid for the channel buttons
    juce::Component channelGridContainer; // A component to host the grid

    std::map<int, std::unique_ptr<juce::TextButton>> channelButtons;
    std::map<int, std::unique_ptr<ButtonAttachment>> channelButtonAttachments;

    CustomLookAndFeel customLookAndFeel;

    // 用于在Solo操作前缓存Mute状态 - 简化版本不持久化
    std::map<juce::String, bool> preSoloMuteStates;
    
    // 用于检测总线布局变化
    int lastKnownChannelCount = 0;

    // 添加私有函数声明
    void handleSoloButtonClick(int channelIndex, const juce::String& channelName);
    void updatePluginConfiguration(); // 立即更新插件配置并通知宿主

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessorEditor)
};
