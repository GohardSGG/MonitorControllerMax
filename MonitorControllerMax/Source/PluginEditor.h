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

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessorEditor)
};
