#pragma once

#include <JuceHeader.h>
#include "ConfigModels.h"
#include <map>
#include <vector>
#include <utility>

class PhysicalChannelMapper
{
public:
    PhysicalChannelMapper();
    ~PhysicalChannelMapper();

    // Configuration-driven mapping updates (compatible with existing system)
    void updateMapping(const Layout& layout);
    void updateFromConfig(const juce::String& speakerLayout, const juce::String& subLayout);

    // Mapping conversion interface
    int getPhysicalPin(const juce::String& semanticName) const;
    juce::String getSemanticName(int physicalPin) const;
    bool hasSemanticChannel(const juce::String& semanticName) const;

    // Safe handling: return default for unmapped channels
    juce::String getSemanticNameSafe(int physicalPin) const;
    int getPhysicalPinSafe(const juce::String& semanticName) const;

    // Get mapping information
    std::vector<juce::String> getActiveSemanticChannels() const;
    std::vector<std::pair<juce::String, int>> getAllMappings() const;
    int getChannelCount() const;

    // Preserve existing grid position system
    std::pair<int, int> getGridPosition(const juce::String& semanticName) const;
    bool hasGridPosition(const juce::String& semanticName) const;

    // Debug and logging
    void logCurrentMapping() const;
    juce::String getMappingDescription() const;

    // Clear all mappings
    void clearMapping();

private:
    // Semantic name <-> Physical Pin mapping (inherits from existing config system)
    std::map<juce::String, int> semanticToPhysical;        // "L" -> 1, "R" -> 5, etc.
    std::map<int, juce::String> physicalToSemantic;        // 1 -> "L", 5 -> "R", etc.
    std::map<juce::String, std::pair<int, int>> gridPositions;  // "L" -> {gridX, gridY}
    
    // Channel information storage
    std::map<juce::String, ChannelInfo> channelInfoMap;
    
    // Helper methods
    void addMapping(const juce::String& semanticName, int physicalPin, int gridX = -1, int gridY = -1);
    void removeMapping(const juce::String& semanticName);
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(PhysicalChannelMapper)
};