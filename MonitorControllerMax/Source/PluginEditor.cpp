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
    
    addAndMakeVisible(dimButton);

    addAndMakeVisible(speakerLayoutSelector);
    speakerLayoutSelector.addItemList(configManager.getSpeakerLayoutNames(), 1);
    speakerLayoutSelector.setSelectedId(1);
    speakerLayoutSelector.onChange = [this] { resized(); };

    addAndMakeVisible(subLayoutSelector);
    subLayoutSelector.addItemList(configManager.getSubLayoutNames(), 1);
    subLayoutSelector.setSelectedId(1);
    subLayoutSelector.onChange = [this] { resized(); };
    
    addAndMakeVisible(channelGridContainer);

    juce::MessageManager::callAsync([this] { resized(); });

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

    sidebar.flexDirection = juce::FlexBox::Direction::column;
    sidebar.justifyContent = juce::FlexBox::JustifyContent::flexStart;
    sidebar.alignItems = juce::FlexBox::AlignItems::center;
    sidebar.items.clear();
    sidebar.items.add(juce::FlexItem(globalSoloButton).withWidth(100).withHeight(40).withMargin(10));
    sidebar.items.add(juce::FlexItem(dimButton).withWidth(100).withHeight(40).withMargin(10));
    sidebar.items.add(juce::FlexItem(globalMuteButton).withWidth(100).withHeight(40).withMargin(10));

    auto sidebarBounds = bounds.removeFromLeft(120);
    sidebar.performLayout(sidebarBounds.reduced(5));

    auto contentBounds = bounds;

    selectorBox.justifyContent = juce::FlexBox::JustifyContent::flexEnd;
    selectorBox.alignItems = juce::FlexBox::AlignItems::center;
    selectorBox.items.clear();
    selectorBox.items.add(juce::FlexItem(speakerLayoutSelector).withWidth(150).withHeight(24));
    selectorBox.items.add(juce::FlexItem(subLayoutSelector).withWidth(150).withHeight(24).withMargin({0, 0, 0, 10}));
    
    auto selectorBounds = contentBounds.removeFromTop(40);
    selectorBox.performLayout(selectorBounds.reduced(5));

    channelGridContainer.setBounds(contentBounds);
    updateLayout();
}

void MonitorControllerMaxAudioProcessorEditor::updateLayout()
{
    auto speakerLayoutName = speakerLayoutSelector.getText();
    auto subLayoutName = subLayoutSelector.getText();

    if (speakerLayoutName.isEmpty()) return;

    audioProcessor.setCurrentLayout(speakerLayoutName, subLayoutName);
    const auto& layout = audioProcessor.getCurrentLayout();
    
    for(auto& pair : channelButtons)
        pair.second->setVisible(false);

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
            channelButtons[chanInfo.channelIndex] = std::make_unique<juce::TextButton>(chanInfo.name);
            channelGridContainer.addAndMakeVisible(*channelButtons[chanInfo.channelIndex]);
            
            auto* button = channelButtons[chanInfo.channelIndex].get();
            button->setClickingTogglesState(true);
            button->onClick = [this, index = chanInfo.channelIndex] 
            {
                if (currentUIMode == UIMode::AssignMute)
                {
                    if (auto* param = audioProcessor.apvts.getParameter("MUTE_" + juce::String(index + 1)))
                        param->setValueNotifyingHost(param->getValue() < 0.5f ? 1.0f : 0.0f);
                }
                else if (currentUIMode == UIMode::AssignSolo)
                {
                    if (auto* param = audioProcessor.apvts.getParameter("SOLO_" + juce::String(index + 1)))
                        param->setValueNotifyingHost(param->getValue() < 0.5f ? 1.0f : 0.0f);
                }
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
        const int subChannelIndex = -1; 
        if (channelButtons.find(subChannelIndex) == channelButtons.end())
        {
             channelButtons[subChannelIndex] = std::make_unique<juce::TextButton>("SUB");
             channelGridContainer.addAndMakeVisible(*channelButtons[subChannelIndex]);
        }
        auto* button = channelButtons[subChannelIndex].get();
        button->setVisible(true);
        int row = (23 - 1) / 5;
        int col = (23 - 1) % 5;
        channelGrid.items.add(juce::GridItem(*button).withArea(row + 1, col + 1));
    }
    
    channelGrid.performLayout(channelGridContainer.getLocalBounds());
}

void MonitorControllerMaxAudioProcessorEditor::timerCallback()
{
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::setUIMode(UIMode newMode)
{
    currentUIMode = newMode;
    updateChannelButtonStates();
}

void MonitorControllerMaxAudioProcessorEditor::updateChannelButtonStates()
{
    const auto role = audioProcessor.getRole();
    const bool isSlave = (role == MonitorControllerMaxAudioProcessor::Role::slave);

    globalMuteButton.setEnabled(!isSlave);
    globalSoloButton.setEnabled(!isSlave);
    dimButton.setEnabled(!isSlave);
    speakerLayoutSelector.setEnabled(!isSlave);
    subLayoutSelector.setEnabled(!isSlave);

    bool anySoloEngaged = false;
    std::array<bool, 26> muteStates{};
    std::array<bool, 26> soloStates{};

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

    for (auto const& [index, button] : channelButtons)
    {
        if (!button->isVisible()) continue;
        
        if (index < 0) 
        {
            continue;
        }

        bool isMuted = muteStates[index];
        bool isSoloed = soloStates[index];
        bool shouldBeSilent = isMuted || (anySoloEngaged && !isSoloed);

        juce::Colour colour = juce::Colours::grey;

        if (isSoloed)
            colour = juce::Colours::yellow;
        else if (shouldBeSilent)
            colour = juce::Colours::red;
        
        button->setColour(juce::TextButton::buttonColourId, colour);
        if (currentUIMode == UIMode::AssignMute)
            button->setToggleState(isMuted, juce::dontSendNotification);
        else if (currentUIMode == UIMode::AssignSolo)
            button->setToggleState(isSoloed, juce::dontSendNotification);
        else
            button->setToggleState(false, juce::dontSendNotification);

        button->setEnabled(!isSlave);
    }
}
