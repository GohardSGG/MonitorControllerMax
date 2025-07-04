/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"

//==============================================================================
MonitorControllerMaxAudioProcessorEditor::MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor& p)
    : AudioProcessorEditor (&p), audioProcessor (p)
{
    addAndMakeVisible(globalMuteButton);
    globalMuteButton.setClickingTogglesState(true);
    globalMuteButton.onClick = [this] { setUIMode(globalMuteButton.getToggleState() ? UIMode::AssignMute : UIMode::Normal); };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setClickingTogglesState(true);
    globalSoloButton.onClick = [this] { setUIMode(globalSoloButton.getToggleState() ? UIMode::AssignSolo : UIMode::Normal); };

    for (int i = 0; i < MonitorControllerMaxAudioProcessor::numManagedChannels; ++i)
    {
        // Create Buttons
        channelButtons[i] = std::make_unique<juce::TextButton>(juce::String(i + 1));
        addAndMakeVisible(*channelButtons[i]);
        channelButtons[i]->onClick = [this, i] {
            if (currentUIMode == UIMode::AssignMute)
            {
                auto* param = audioProcessor.apvts.getParameter("MUTE_" + juce::String(i + 1));
                param->setValueNotifyingHost(!param->getValue());
            }
            else if (currentUIMode == UIMode::AssignSolo)
            {
                auto* param = audioProcessor.apvts.getParameter("SOLO_" + juce::String(i + 1));
                param->setValueNotifyingHost(!param->getValue());
            }
        };

        // Create Sliders
        gainSliders[i] = std::make_unique<juce::Slider>(juce::Slider::RotaryVerticalDrag, juce::Slider::TextBoxBelow);
        addAndMakeVisible(*gainSliders[i]);

        // Create Attachments
        juce::String muteId = "MUTE_" + juce::String(i + 1);
        juce::String soloId = "SOLO_" + juce::String(i + 1);
        juce::String gainId = "GAIN_" + juce::String(i + 1);

        // Note: For channel buttons, we will manage their clicks manually based on the UIMode, so no attachments are needed for them yet.
        
        gainSliderAttachments[i] = std::make_unique<SliderAttachment>(audioProcessor.apvts, gainId, *gainSliders[i]);
    }

    setSize (800, 600);
    startTimerHz(10);
}

MonitorControllerMaxAudioProcessorEditor::~MonitorControllerMaxAudioProcessorEditor()
{
}

//==============================================================================
void MonitorControllerMaxAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (getLookAndFeel().findColour (juce::ResizableWindow::backgroundColourId));
}

void MonitorControllerMaxAudioProcessorEditor::resized()
{
    juce::FlexBox mainFlexBox;
    mainFlexBox.flexDirection = juce::FlexBox::Direction::column;
    mainFlexBox.justifyContent = juce::FlexBox::JustifyContent::flexStart;

    // Header (global controls)
    juce::FlexBox headerBox;
    headerBox.justifyContent = juce::FlexBox::JustifyContent::center;
    headerBox.alignItems = juce::FlexBox::AlignItems::center;
    headerBox.items.add(juce::FlexItem(globalMuteButton).withWidth(100).withHeight(30).withMargin(5));
    headerBox.items.add(juce::FlexItem(globalSoloButton).withWidth(100).withHeight(30).withMargin(5));
    mainFlexBox.items.add(juce::FlexItem(headerBox).withHeight(50));

    // Channel Strips
    juce::FlexBox channelStripBox;
    channelStripBox.flexWrap = juce::FlexBox::Wrap::wrap;
    channelStripBox.justifyContent = juce::FlexBox::JustifyContent::center;
    channelStripBox.alignContent = juce::FlexBox::AlignContent::flexStart;

    for (int i = 0; i < MonitorControllerMaxAudioProcessor::numManagedChannels; ++i)
    {
        juce::FlexBox strip;
        strip.flexDirection = juce::FlexBox::Direction::column;
        strip.alignItems = juce::FlexBox::AlignItems::center;
        
        strip.items.add(juce::FlexItem(*gainSliders[i]).withFlex(1.0f));
        strip.items.add(juce::FlexItem(*channelButtons[i]).withWidth(60).withHeight(30));

        channelStripBox.items.add(juce::FlexItem(strip).withWidth(100).withHeight(150).withMargin(5));
    }

    mainFlexBox.items.add(juce::FlexItem(channelStripBox).withFlex(1.0f));

    mainFlexBox.performLayout(getLocalBounds().reduced(10));
}

void MonitorControllerMaxAudioProcessorEditor::timerCallback()
{
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::setUIMode(UIMode newMode)
{
    currentUIMode = newMode;
    // When mode changes, we might want to update visuals immediately
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateChannelButtonStates()
{
    const auto role = audioProcessor.getRole();
    const bool isSlave = (role == MonitorControllerMaxAudioProcessor::Role::slave);

    // Disable controls if we are a slave
    globalMuteButton.setEnabled(!isSlave);
    globalSoloButton.setEnabled(!isSlave);

    bool anySoloEngaged = false;
    std::array<bool, MonitorControllerMaxAudioProcessor::numManagedChannels> muteStates;
    std::array<bool, MonitorControllerMaxAudioProcessor::numManagedChannels> soloStates;

    // Get current state from APVTS or remote state for slaves
    for (int i = 0; i < MonitorControllerMaxAudioProcessor::numManagedChannels; ++i)
    {
        if (isSlave)
        {
            muteStates[i] = audioProcessor.getRemoteMuteState(i);
            soloStates[i] = audioProcessor.getRemoteSoloState(i);
        }
        else
        {
            muteStates[i] = audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(i + 1))->load() > 0.5f;
            soloStates[i] = audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(i + 1))->load() > 0.5f;
        }

        if (soloStates[i])
            anySoloEngaged = true;

        // Also disable channel controls if slave
        channelButtons[i]->setEnabled(!isSlave);
        gainSliders[i]->setEnabled(!isSlave);
    }

    // Determine visuals based on state
    for (int i = 0; i < MonitorControllerMaxAudioProcessor::numManagedChannels; ++i)
    {
        bool shouldBeSilent = muteStates[i] || (anySoloEngaged && !soloStates[i]);

        juce::Colour colour = juce::Colours::grey; // Default non-active colour

        if (soloStates[i])
            colour = juce::Colours::yellow;
        else if (shouldBeSilent)
            colour = juce::Colours::red;

        channelButtons[i]->setColour(juce::TextButton::buttonColourId, colour);
    }
}
