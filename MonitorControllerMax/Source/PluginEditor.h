/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"

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
    using SliderAttachment = juce::AudioProcessorValueTreeState::SliderAttachment;

    enum class UIMode
    {
        Normal,
        AssignSolo,
        AssignMute
    };

    void updateChannelButtonStates();
    void setUIMode(UIMode newMode);

    // This reference is provided as a quick way for your editor to
    // access the processor object that created it.
    MonitorControllerMaxAudioProcessor& audioProcessor;

    UIMode currentUIMode { UIMode::Normal };

    juce::TextButton globalMuteButton{ "Mute" };
    juce::TextButton globalSoloButton{ "Solo" };

    std::array<std::unique_ptr<juce::TextButton>, MonitorControllerMaxAudioProcessor::numManagedChannels> channelButtons;
    std::array<std::unique_ptr<juce::Slider>, MonitorControllerMaxAudioProcessor::numManagedChannels> gainSliders;
    
    std::array<std::unique_ptr<ButtonAttachment>, MonitorControllerMaxAudioProcessor::numManagedChannels> channelButtonAttachments;
    std::array<std::unique_ptr<SliderAttachment>, MonitorControllerMaxAudioProcessor::numManagedChannels> gainSliderAttachments;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (MonitorControllerMaxAudioProcessorEditor)
};
