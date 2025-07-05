/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin editor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"

//==============================================================================
MonitorControllerMaxAudioProcessorEditor::MonitorControllerMaxAudioProcessorEditor (MonitorControllerMaxAudioProcessor& p)
    : AudioProcessorEditor (&p), audioProcessor (p), configManager(p.configManager)
{
    addAndMakeVisible(globalMuteButton);
    globalMuteButton.setClickingTogglesState(true);
    globalMuteButton.onClick = [this] { setUIMode(globalMuteButton.getToggleState() ? UIMode::AssignMute : UIMode::Normal); };

    addAndMakeVisible(globalSoloButton);
    globalSoloButton.setClickingTogglesState(true);
    globalSoloButton.onClick = [this] { setUIMode(globalSoloButton.getToggleState() ? UIMode::AssignSolo : UIMode::Normal); };
    
    addAndMakeVisible(dimButton); // Dim button logic to be added later

    addAndMakeVisible(speakerLayoutSelector);
    speakerLayoutSelector.addItemList(configManager.getSpeakerLayoutNames(), 1);
    speakerLayoutSelector.setSelectedId(1); // Default to first layout
    speakerLayoutSelector.onChange = [this] { updateLayout(); };

    addAndMakeVisible(subLayoutSelector);
    subLayoutSelector.addItemList(configManager.getSubLayoutNames(), 1);
    subLayoutSelector.setSelectedId(1); // Default to "None"
    subLayoutSelector.onChange = [this] { updateLayout(); };
    
    updateLayout(); // Initial layout draw

    setSize (800, 600);
    startTimerHz(10);
}

MonitorControllerMaxAudioProcessorEditor::~MonitorControllerMaxAudioProcessorEditor()
{
    stopTimer();
}

//==============================================================================
void MonitorControllerMaxAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (getLookAndFeel().findColour (juce::ResizableWindow::backgroundColourId));
}

void MonitorControllerMaxAudioProcessorEditor::resized()
{
    auto bounds = getLocalBounds();

    // Sidebar
    sidebar.flexDirection = juce::FlexBox::Direction::column;
    sidebar.justifyContent = juce::FlexBox::JustifyContent::flexStart;
    sidebar.alignItems = juce::FlexBox::AlignItems::center;
    sidebar.items.clear();
    sidebar.items.add(juce::FlexItem(globalSoloButton).withWidth(100).withHeight(40).withMargin(10));
    sidebar.items.add(juce::FlexItem(dimButton).withWidth(100).withHeight(40).withMargin(10));
    sidebar.items.add(juce::FlexItem(globalMuteButton).withWidth(100).withHeight(40).withMargin(10));

    auto sidebarBounds = bounds.removeFromLeft(120);
    sidebar.performLayout(sidebarBounds);

    // Main Content Area
    auto contentBounds = bounds;

    // Selector Box at the top-right
    selectorBox.justifyContent = juce::FlexBox::JustifyContent::flexEnd;
    selectorBox.items.clear();
    selectorBox.items.add(juce::FlexItem(speakerLayoutSelector).withWidth(150).withHeight(24));
    selectorBox.items.add(juce::FlexItem(subLayoutSelector).withWidth(150).withHeight(24).withMargin({0, 0, 0, 10}));
    
    auto selectorBounds = contentBounds.removeFromTop(40);
    selectorBox.performLayout(selectorBounds.reduced(5));

    // Channel Grid takes the rest of the space
    channelGrid.performLayout(contentBounds);
}

void MonitorControllerMaxAudioProcessorEditor::updateLayout()
{
    auto speakerLayoutName = speakerLayoutSelector.getText();
    auto subLayoutName = subLayoutSelector.getText();

    if (speakerLayoutName.isEmpty()) return;

    auto layout = configManager.getLayoutFor(speakerLayoutName, subLayoutName);
    
    // Hide all current buttons first
    for(auto& pair : channelButtons)
        pair.second->setVisible(false);

    // Use the member 'channelGrid', don't create a temporary one.
    channelGrid.items.clear();
    channelGrid.setGap(juce::Grid::Px(5));
    channelGrid.templateRows.clear();
    channelGrid.templateColumns.clear();

    for (int i = 0; i < 5; ++i)
    {
        channelGrid.templateRows.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
        channelGrid.templateColumns.add(juce::Grid::TrackInfo(juce::Grid::Fr(1)));
    }
    
    for (const auto& chanInfo : layout.channels)
    {
        if (channelButtons.find(chanInfo.channelIndex) == channelButtons.end())
        {
            // Button doesn't exist, create it
            channelButtons[chanInfo.channelIndex] = std::make_unique<juce::TextButton>(chanInfo.name);
            addAndMakeVisible(*channelButtons[chanInfo.channelIndex]);
            channelButtons[chanInfo.channelIndex]->onClick = [this, index = chanInfo.channelIndex] {
                if (currentUIMode == UIMode::AssignMute)
                    audioProcessor.apvts.getParameter("MUTE_" + juce::String(index + 1))->setValueNotifyingHost(0.0f);
                else if (currentUIMode == UIMode::AssignSolo)
                     audioProcessor.apvts.getParameter("SOLO_" + juce::String(index + 1))->setValueNotifyingHost(0.0f);
            };
        }
        
        auto* button = channelButtons[chanInfo.channelIndex].get();
        button->setButtonText(chanInfo.name);
        button->setVisible(true);

        int gridPos = chanInfo.gridPosition;
        int row = (gridPos - 1) / 5;
        int col = (gridPos - 1) % 5;
        channelGrid.items.add(juce::GridItem(*button).withArea(row + 1, col + 1));
    }

    if (subLayoutName != "None")
    {
        const int subChannelIndex = -1; // Special index for the master SUB button
        if (channelButtons.find(subChannelIndex) == channelButtons.end())
        {
             channelButtons[subChannelIndex] = std::make_unique<juce::TextButton>("SUB");
             addAndMakeVisible(*channelButtons[subChannelIndex]);
             // Add logic for this master sub button if needed
        }
        auto* button = channelButtons[subChannelIndex].get();
        button->setVisible(true);
        int row = (23 - 1) / 5;
        int col = (23 - 1) % 5;
        channelGrid.items.add(juce::GridItem(*button).withArea(row + 1, col + 1));
    }
    
    // The grid will now be laid out correctly as part of resized()
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
    dimButton.setEnabled(!isSlave);
    speakerLayoutSelector.setEnabled(!isSlave);
    subLayoutSelector.setEnabled(!isSlave);

    bool anySoloEngaged = false;
    std::array<bool, 26> muteStates;
    std::array<bool, 26> soloStates;

    // Get current state from APVTS or remote state for slaves
    for (int i = 0; i < 26; ++i)
    {
        if (isSlave)
        {
            muteStates[i] = audioProcessor.getRemoteMuteState(i);
            soloStates[i] = audioProcessor.getRemoteSoloState(i);
        }
        else
        {
            if (auto* param = audioProcessor.apvts.getRawParameterValue("MUTE_" + juce::String(i + 1)))
                muteStates[i] = param->load() > 0.5f;
            if (auto* param = audioProcessor.apvts.getRawParameterValue("SOLO_" + juce::String(i + 1)))
                soloStates[i] = param->load() > 0.5f;
        }

        if (soloStates[i])
            anySoloEngaged = true;
    }

    // Determine visuals based on state
    for (auto const& [index, button] : channelButtons)
    {
        if (index < 0) continue; // Skip master SUB button for now

        bool shouldBeSilent = muteStates[index] || (anySoloEngaged && !soloStates[index]);

        juce::Colour colour = juce::Colours::grey; // Default non-active colour

        if (soloStates[index])
            colour = juce::Colours::yellow;
        else if (shouldBeSilent)
            colour = juce::Colours::red;
        
        button->setColour(juce::TextButton::buttonColourId, colour);
        button->setEnabled(!isSlave);
    }
}
