/*
  ==============================================================================

    ConfigModels.h
    Created: 5 Aug 2024 2:00:00pm
    Author:  Your Name

  ==============================================================================
*/

#pragma once
#include <JuceHeader.h>
#include <vector>

struct ChannelInfo
{
    juce::String name; // e.g., "L", "C", "LFE"
    int gridPosition;  // 1-based index in the 5x5 grid
    int channelIndex;  // 0-based audio channel index
};

struct Layout
{
    std::vector<ChannelInfo> channels;
    int totalChannelCount;
}; 