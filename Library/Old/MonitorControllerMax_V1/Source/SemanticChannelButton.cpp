#include "SemanticChannelButton.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"
#include "SafeUICallback.h"

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
    // 🚀 稳定性优化：全面异常保护，防止按钮点击崩溃插件
    try {
        bool newState = getToggleState();
        
        VST3_DBG("SemanticSoloButton: Click Solo button - " + semanticChannelName + 
                 ", new state: " + (newState ? "ON" : "OFF"));
        
        // 🚀 彻底修复：统一使用StateManager处理通道Solo点击
        // 优先使用StateManager，降级到SemanticChannelState
        if (processor.stateManager && processor.stateManager->isInSoloSelectionMode()) {
            processor.stateManager->handleChannelSoloClick(semanticChannelName, newState);
        } else {
            // 降级处理：直接调用SemanticChannelState
            processor.getSemanticState().setSoloState(semanticChannelName, newState);
        }
        
        // Update appearance immediately
        updateButtonAppearance(newState);
        
        // Future: OSC communication will be added here
        // processor.getOSCCommunicator().sendSoloState(semanticChannelName, newState);
    }
    catch (const std::exception& e) {
        VST3_DBG("SemanticSoloButton: Exception in clicked(): " + juce::String(e.what()));
        // 安全降级：重置按钮状态
        setToggleState(false, juce::dontSendNotification);
        updateButtonAppearance(false);
    }
    catch (...) {
        VST3_DBG("SemanticSoloButton: Unknown exception in clicked()");
        // 安全降级：重置按钮状态
        setToggleState(false, juce::dontSendNotification);
        updateButtonAppearance(false);
    }
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
    // 🚀 稳定性优化：全面异常保护，防止按钮点击崩溃插件
    try {
        bool newState = getToggleState();
        
        VST3_DBG("SemanticMuteButton: Click Mute button - " + semanticChannelName + 
                 ", new state: " + (newState ? "ON" : "OFF"));
        
        // 🚀 彻底修复：统一使用StateManager处理通道Mute点击
        // 优先使用StateManager，降级到SemanticChannelState
        if (processor.stateManager && processor.stateManager->isInMuteSelectionMode()) {
            processor.stateManager->handleChannelMuteClick(semanticChannelName, newState);
        } else {
            // 降级处理：直接调用SemanticChannelState
            processor.getSemanticState().setMuteState(semanticChannelName, newState);
        }
        
        // Update appearance immediately
        updateButtonAppearance(newState);
    }
    catch (const std::exception& e) {
        VST3_DBG("SemanticMuteButton: Exception in clicked(): " + juce::String(e.what()));
        // 安全降级：重置按钮状态
        setToggleState(false, juce::dontSendNotification);
        updateButtonAppearance(false);
    }
    catch (...) {
        VST3_DBG("SemanticMuteButton: Unknown exception in clicked()");
        // 安全降级：重置按钮状态
        setToggleState(false, juce::dontSendNotification);
        updateButtonAppearance(false);
    }
    
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