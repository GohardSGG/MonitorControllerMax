/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"
#include "ConfigManager.h"
#include "SemanticChannelButton.h"
#include "EffectsPanel.h"
#include <map>

//==============================================================================
/** A simple custom LookAndFeel class for implementing dark UI style. */
class CustomLookAndFeel : public juce::LookAndFeel_V4
{
public:
    CustomLookAndFeel()
    {
        // Set dark theme
        setColour(juce::ResizableWindow::backgroundColourId, juce::Colour(0xff323e44));
        setColour(juce::TextButton::buttonColourId, juce::Colour(0xff4a5860));
        setColour(juce::TextButton::buttonOnColourId, juce::Colour(0xffd13a3a)); // Red for Mute active state
        setColour(juce::TextButton::textColourOffId, juce::Colours::lightgrey);
        setColour(juce::TextButton::textColourOnId, juce::Colours::white);
        setColour(juce::ComboBox::backgroundColourId, juce::Colour(0xff4a5860));
        setColour(juce::ComboBox::outlineColourId, juce::Colours::transparentBlack);
        setColour(juce::ComboBox::arrowColourId, juce::Colours::lightgrey);
        setColour(juce::PopupMenu::backgroundColourId, juce::Colour(0xff4a5860));
        setColour(juce::PopupMenu::highlightedBackgroundColourId, juce::Colour(0xfff07800)); // Orange for highlight

        // Define unique color for Solo state
        soloColour = juce::Colour(0xff2a8c4a);
        // Define unique color for Mute state
        muteColour = juce::Colour(0xffd13a3a); // Red, same as buttonOnColourId but explicit
    }

    void drawButtonBackground(juce::Graphics& g, juce::Button& button, const juce::Colour& backgroundColour,
                              bool shouldDrawButtonAsHighlighted, bool shouldDrawButtonAsDown) override
    {
        auto cornerSize = 6.0f;
        auto originalBounds = button.getLocalBounds();

        // 🚀 关键修复：检查按钮的toggle状态，使用正确的颜色ID
        auto* textButton = dynamic_cast<const juce::TextButton*>(&button);
        bool isToggleOn = textButton ? textButton->getToggleState() : false;
        
        // 根据toggle状态选择正确的颜色ID
        auto colourId = isToggleOn ? juce::TextButton::buttonOnColourId : juce::TextButton::buttonColourId;
        auto actualColour = button.findColour(colourId);
        
        // 如果都是默认色，使用我们设置的buttonColourId
        if (actualColour == juce::LookAndFeel::getDefaultLookAndFeel().findColour(colourId)) {
            actualColour = button.findColour(juce::TextButton::buttonColourId);
        }
        
        auto baseColour = actualColour.withMultipliedAlpha(button.isEnabled() ? 1.0f : 0.5f);

        // Remove mouse hover effects - only respond to button press
        if (shouldDrawButtonAsDown)
            baseColour = baseColour.contrasting(0.2f);
        // Remove: shouldDrawButtonAsHighlighted handling

        g.setColour(baseColour);

        if (button.getButtonText().startsWith("SUB "))
        {
            // Draw circle for SUB channel buttons
            auto diameter = (float)juce::jmin(originalBounds.getWidth(), originalBounds.getHeight());
            g.fillEllipse(originalBounds.toFloat().withSizeKeepingCentre(diameter, diameter));
        }
        else
        {
            // 
            auto side = juce::jmin(originalBounds.getWidth(), originalBounds.getHeight());
            auto squareBounds = originalBounds.toFloat().withSizeKeepingCentre(side, side);
            g.fillRoundedRectangle(squareBounds, cornerSize);
        }
    }
    
    juce::Font getTextButtonFont(juce::TextButton& button, int buttonHeight) override
    {
        // v4.2: 按字符长度判断字体大小 - 6个字符及以上使用小字体
        if (button.getButtonText().length() >= 6)
        {
            // 获取默认字体的样式，但使用较小的字号
            auto defaultFont = juce::LookAndFeel_V4::getTextButtonFont(button, buttonHeight);
            return defaultFont.withHeight(12.0f);  // 保持原有字体粗细，只改变大小
        }
        return juce::LookAndFeel_V4::getTextButtonFont(button, buttonHeight);
    }

    juce::Colour getSoloColour() const { return soloColour; }
    juce::Colour getMuteColour() const { return muteColour; }

private:
    juce::Colour soloColour;
    juce::Colour muteColour;
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
    void mouseDown(const juce::MouseEvent& event) override;
    void timerCallback() override;
    
    // Public UI update methods
    void updateChannelButtonStates();
    void updateUIBasedOnRole(); // Master-Slave UI状态更新

private:
    using ButtonAttachment = juce::AudioProcessorValueTreeState::ButtonAttachment;
    
    enum class UIMode
    {
        Normal,
        AssignSolo,
        AssignMute
    };

    void updateLayout();
    void updateLayoutWithoutSelectorOverride(); // Layout update without forcing selector choice
    void setUIMode(UIMode newMode);
    
    // New semantic UI methods
    void updateLayoutFromSemanticMapping();
    void createSemanticChannelButtons();
    void clearSemanticChannelButtons();
    void updateAllSemanticButtonsFromState();

    // This reference is provided as a quick way for your editor to
    // access the processor object that created it.
    MonitorControllerMaxAudioProcessor& audioProcessor;
    ConfigManager& configManager;

    UIMode currentUIMode { UIMode::Normal };

    juce::TextButton globalMuteButton{ "Mute" };
    juce::TextButton globalSoloButton{ "Solo" };
    juce::TextButton dimButton{ "Dim" };
    juce::TextButton masterMuteButton{ "Master\nMute" };
    
    // v4.2: Effects面板按钮 (替代原来的lowBoostButton和monoButton)
    juce::TextButton effectsPanelButton{ "EFFECTS" };
    
    // v4.2: 弹出式总线效果面板
    EffectsPanel effectsPanel;
    
    // v4.1: Master Gain旋钮控件
    juce::Slider masterGainSlider;
    juce::Label masterGainLabel;
    
    juce::ComboBox speakerLayoutSelector;
    juce::ComboBox subLayoutSelector;
    
    // Master-Slave角色选择器
    juce::ComboBox roleSelector;
    juce::Label roleLabel;
    
    // Debug连接日志窗口
    juce::TextEditor debugLogDisplay;
    juce::Label debugLogLabel;
    juce::TextButton clearLogButton{ "Clear" };

    juce::FlexBox sidebar;
    juce::FlexBox selectorBox;
    juce::Grid channelGrid; // Grid for the channel buttons
    juce::Component channelGridContainer; // A component to host the grid

    std::map<int, std::unique_ptr<juce::TextButton>> channelButtons;
    std::map<int, std::unique_ptr<ButtonAttachment>> channelButtonAttachments;
    
    // v4.1: Master Gain参数连接
    using SliderAttachment = juce::AudioProcessorValueTreeState::SliderAttachment;
    std::unique_ptr<SliderAttachment> masterGainSliderAttachment;
    
    // New semantic channel button system (gradually replacing legacy system)
    std::map<juce::String, std::unique_ptr<SemanticChannelButtonPair>> semanticChannelButtons;

    CustomLookAndFeel customLookAndFeel;

    
    // For detecting bus layout changes
    int lastKnownChannelCount = 0;
    
    // 🚀 稳定性优化：UI初始化状态管理，防止Timer竞态条件
    std::atomic<bool> uiInitializationComplete{false};
    std::atomic<bool> safeToUpdateUI{true};  // UI更新安全标志
    std::atomic<uint32_t> timerCallCount{0};  // 🚀 实例级Timer计数器，避免static竞争
    std::atomic<uint32_t> updateButtonStatesCount{0};  // 🚀 实例级按钮更新计数器

    // Private function declarations
    void updatePluginConfiguration(); // Update plugin configuration and notify host immediately
    void syncUIFromUserSelection(); // 从用户选择同步UI状态
    
    // Master-Slave UI管理  
    void setupRoleSelector();
    void handleRoleChange();
    void updateDebugLogDisplay();
    void clearDebugLog();
    
    // v4.2: Effects面板管理
    void setupEffectsPanel();
    void handleEffectsPanelButtonClick();
    void handleEffectsPanelOutsideClick(const juce::MouseEvent& event);

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessorEditor)
};
