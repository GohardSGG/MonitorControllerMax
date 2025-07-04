/*
  ==============================================================================

    ConfigManager.h
    Created: 5 Aug 2024 2:05:00pm
    Author:  Your Name

  ==============================================================================
*/

#pragma once
#include "ConfigModels.h"
#include <JuceHeader.h>

class ConfigManager
{
public:
    ConfigManager();

    juce::StringArray getSpeakerLayoutNames() const;
    juce::StringArray getSubLayoutNames() const;
    Layout getLayoutFor(const juce::String& speakerLayoutName, const juce::String& subLayoutName) const;
    int getMaxChannelIndex() const;

private:
    void loadConfig();
    
    juce::var configData;
    int maxChannelIndex = 0;
}; 