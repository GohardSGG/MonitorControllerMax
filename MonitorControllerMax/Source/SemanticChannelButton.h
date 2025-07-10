#pragma once

#include <JuceHeader.h>

// Forward declaration
class MonitorControllerMaxAudioProcessor;

//==============================================================================
// Semantic Solo Button - preserves existing interaction logic
//==============================================================================
class SemanticSoloButton : public juce::TextButton
{
public:
    SemanticSoloButton(MonitorControllerMaxAudioProcessor& processor, 
                       const juce::String& semanticChannelName);
    ~SemanticSoloButton() override;

    void clicked() override;
    void updateFromSemanticState();
    
    // Get the semantic channel name
    const juce::String& getSemanticChannelName() const { return semanticChannelName; }

private:
    MonitorControllerMaxAudioProcessor& processor;
    juce::String semanticChannelName;  // "L", "R", "C", etc.
    
    void updateButtonAppearance(bool state);
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(SemanticSoloButton)
};

//==============================================================================
// Semantic Mute Button - preserves existing interaction logic
//==============================================================================
class SemanticMuteButton : public juce::TextButton
{
public:
    SemanticMuteButton(MonitorControllerMaxAudioProcessor& processor, 
                       const juce::String& semanticChannelName);
    ~SemanticMuteButton() override;

    void clicked() override;
    void updateFromSemanticState();
    
    // Get the semantic channel name
    const juce::String& getSemanticChannelName() const { return semanticChannelName; }

private:
    MonitorControllerMaxAudioProcessor& processor;
    juce::String semanticChannelName;  // "L", "R", "C", etc.
    
    void updateButtonAppearance(bool state);
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(SemanticMuteButton)
};

//==============================================================================
// Semantic Channel Button Pair - combines Solo and Mute for one channel
//==============================================================================
struct SemanticChannelButtonPair
{
    std::unique_ptr<SemanticSoloButton> soloButton;
    std::unique_ptr<SemanticMuteButton> muteButton;
    juce::String semanticChannelName;
    std::pair<int, int> gridPosition;  // {gridX, gridY}
    
    SemanticChannelButtonPair(MonitorControllerMaxAudioProcessor& processor,
                              const juce::String& channelName,
                              std::pair<int, int> gridPos)
        : semanticChannelName(channelName)
        , gridPosition(gridPos)
    {
        soloButton = std::make_unique<SemanticSoloButton>(processor, channelName);
        muteButton = std::make_unique<SemanticMuteButton>(processor, channelName);
    }
    
    void updateFromSemanticState()
    {
        soloButton->updateFromSemanticState();
        muteButton->updateFromSemanticState();
    }
};