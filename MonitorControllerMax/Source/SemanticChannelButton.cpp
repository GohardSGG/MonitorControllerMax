#include "SemanticChannelButton.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

//==============================================================================
// Semantic Solo Button Implementation
//==============================================================================

SemanticSoloButton::SemanticSoloButton(MonitorControllerMaxAudioProcessor& proc, 
                                       const juce::String& channelName)
    : processor(proc), semanticChannelName(channelName)
{
    VST3_DBG("SemanticSoloButton: Create Solo button - " + channelName);
    
    setButtonText("Solo " + channelName);
    setClickingTogglesState(true);
    setToggleState(false, juce::dontSendNotification);
    
    // Set initial appearance
    updateButtonAppearance(false);
}

SemanticSoloButton::~SemanticSoloButton()
{
    VST3_DBG("SemanticSoloButton: Destroy Solo button - " + semanticChannelName);
}

void SemanticSoloButton::clicked()
{
    bool newState = getToggleState();
    
    VST3_DBG("SemanticSoloButton: Click Solo button - " + semanticChannelName + 
             ", new state: " + (newState ? "ON" : "OFF"));
    
    // Preserve existing complex logic by calling processor's semantic state
    // This replaces the VST3 parameter calls with semantic state calls
    processor.getSemanticState().setSoloState(semanticChannelName, newState);
    
    // Update appearance immediately
    updateButtonAppearance(newState);
    
    // Future: OSC communication will be added here
    // processor.getOSCCommunicator().sendSoloState(semanticChannelName, newState);
}

void SemanticSoloButton::updateFromSemanticState()
{
    bool currentState = processor.getSemanticState().getSoloState(semanticChannelName);
    setToggleState(currentState, juce::dontSendNotification);
    
    // Update button appearance
    updateButtonAppearance(currentState);
}

void SemanticSoloButton::updateButtonAppearance(bool state)
{
    // Preserve existing button appearance logic
    if (state)
    {
        setColour(juce::TextButton::buttonOnColourId, juce::Colours::green);
        setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    }
    else
    {
        setColour(juce::TextButton::buttonOnColourId, juce::Colours::grey);
        setColour(juce::TextButton::textColourOnId, juce::Colours::black);
    }
    
    repaint();
}

//==============================================================================
// Semantic Mute Button Implementation
//==============================================================================

SemanticMuteButton::SemanticMuteButton(MonitorControllerMaxAudioProcessor& proc, 
                                       const juce::String& channelName)
    : processor(proc), semanticChannelName(channelName)
{
    VST3_DBG("SemanticMuteButton: Create Mute button - " + channelName);
    
    setButtonText("Mute " + channelName);
    setClickingTogglesState(true);
    setToggleState(false, juce::dontSendNotification);
    
    // Set initial appearance
    updateButtonAppearance(false);
}

SemanticMuteButton::~SemanticMuteButton()
{
    VST3_DBG("SemanticMuteButton: Destroy Mute button - " + semanticChannelName);
}

void SemanticMuteButton::clicked()
{
    bool newState = getToggleState();
    
    VST3_DBG("SemanticMuteButton: Click Mute button - " + semanticChannelName + 
             ", new state: " + (newState ? "ON" : "OFF"));
    
    // Preserve existing complex logic by calling processor's semantic state
    // This replaces the VST3 parameter calls with semantic state calls
    processor.getSemanticState().setMuteState(semanticChannelName, newState);
    
    // Update appearance immediately
    updateButtonAppearance(newState);
    
    // Future: OSC communication will be added here
    // processor.getOSCCommunicator().sendMuteState(semanticChannelName, newState);
}

void SemanticMuteButton::updateFromSemanticState()
{
    // Get the final mute state (including solo mode linkage)
    bool finalMuteState = processor.getSemanticState().getFinalMuteState(semanticChannelName);
    bool directMuteState = processor.getSemanticState().getMuteState(semanticChannelName);
    
    // For UI display, show the direct mute state, not the final computed state
    // This preserves existing UI behavior where users see their direct actions
    setToggleState(directMuteState, juce::dontSendNotification);
    
    // Update button appearance based on final state for visual feedback
    updateButtonAppearance(finalMuteState);
    
    // Debug logging for complex state
    if (finalMuteState != directMuteState)
    {
        VST3_DBG("SemanticMuteButton: Complex state update - " + semanticChannelName + 
                 ", Direct Mute: " + (directMuteState ? "ON" : "OFF") + 
                 ", Final Mute: " + (finalMuteState ? "ON" : "OFF"));
    }
}

void SemanticMuteButton::updateButtonAppearance(bool state)
{
    // Preserve existing button appearance logic
    if (state)
    {
        setColour(juce::TextButton::buttonOnColourId, juce::Colours::red);
        setColour(juce::TextButton::textColourOnId, juce::Colours::white);
    }
    else
    {
        setColour(juce::TextButton::buttonOnColourId, juce::Colours::grey);
        setColour(juce::TextButton::textColourOnId, juce::Colours::black);
    }
    
    repaint();
}