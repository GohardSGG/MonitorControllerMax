#include "PhysicalChannelMapper.h"
#include "DebugLogger.h"

PhysicalChannelMapper::PhysicalChannelMapper()
{
    VST3_DBG("PhysicalChannelMapper: Initializephysical channel mapping system");
}

PhysicalChannelMapper::~PhysicalChannelMapper()
{
    VST3_DBG("PhysicalChannelMapper: Destroyphysical channel mapping system");
}

void PhysicalChannelMapper::updateMapping(const Layout& layout)
{
    VST3_DBG("PhysicalChannelMapper: Update mapping configuration");
    
    // Clear existing mappings
    clearMapping();
    
    // Parse mapping relationships from configuration - fully compatible with existing config format
    for (const auto& channelInfo : layout.channels)
    {
        juce::String semanticName = channelInfo.name;         // "L", "R", "C"
        int physicalPin = channelInfo.channelIndex;           // 1, 5, 3 (from config file)
        
        // Convert 1-based gridPosition to 0-based x,y coordinates (5x5 grid)
        int gridPos = channelInfo.gridPosition - 1;  // Convert to 0-based
        int gridX = gridPos % 5;                      // Column (0-4)
        int gridY = gridPos / 5;                      // Row (0-4)
        
        VST3_DBG("PhysicalChannelMapper: Map channel - " + semanticName + " -> physical pin " + juce::String(physicalPin) + 
                 " (grid position: " + juce::String(gridX) + "," + juce::String(gridY) + ")");
        
        addMapping(semanticName, physicalPin, gridX, gridY);
        
        // Store channel info
        channelInfoMap[semanticName] = channelInfo;
    }
    
    VST3_DBG("PhysicalChannelMapper: Mapping update complete - total " + juce::String(semanticToPhysical.size()) + " channels");
    logCurrentMapping();
}

void PhysicalChannelMapper::updateFromConfig(const juce::String& speakerLayout, const juce::String& subLayout)
{
    VST3_DBG("PhysicalChannelMapper: Update mapping from config - Speaker: " + speakerLayout + ", Sub: " + subLayout);
    
    // This method would be called by the main processor when configuration changes
    // For now, we'll log the intent - the actual implementation will be integrated later
    
    // TODO: Integrate with ConfigManager to get Layout from speaker/sub names
    // Layout layout = configManager.getLayout(speakerLayout, subLayout);
    // updateMapping(layout);
}

int PhysicalChannelMapper::getPhysicalPin(const juce::String& semanticName) const
{
    auto it = semanticToPhysical.find(semanticName);
    if (it != semanticToPhysical.end())
    {
        return it->second;
    }
    
    VST3_DBG("PhysicalChannelMapper: Warning - Semantic channel mapping not found: " + semanticName);
    return -1;  // Invalid pin
}

juce::String PhysicalChannelMapper::getSemanticName(int physicalPin) const
{
    auto it = physicalToSemantic.find(physicalPin);
    if (it != physicalToSemantic.end())
    {
        return it->second;
    }
    
    return juce::String();  // Empty string for unmapped pins
}

bool PhysicalChannelMapper::hasSemanticChannel(const juce::String& semanticName) const
{
    return semanticToPhysical.find(semanticName) != semanticToPhysical.end();
}

juce::String PhysicalChannelMapper::getSemanticNameSafe(int physicalPin) const
{
    auto it = physicalToSemantic.find(physicalPin);
    if (it != physicalToSemantic.end())
    {
        return it->second;
    }
    
    // Return empty string for unmapped channels - allows safe processing
    return juce::String();
}

int PhysicalChannelMapper::getPhysicalPinSafe(const juce::String& semanticName) const
{
    auto it = semanticToPhysical.find(semanticName);
    if (it != semanticToPhysical.end())
    {
        return it->second;
    }
    
    // Return -1 for unmapped semantic names
    return -1;
}

std::vector<juce::String> PhysicalChannelMapper::getActiveSemanticChannels() const
{
    std::vector<juce::String> channels;
    
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        channels.push_back(semanticName);
    }
    
    VST3_DBG("PhysicalChannelMapper: Active semantic channels count: " + juce::String(channels.size()));
    
    return channels;
}

std::vector<std::pair<juce::String, int>> PhysicalChannelMapper::getAllMappings() const
{
    std::vector<std::pair<juce::String, int>> mappings;
    
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        mappings.push_back({semanticName, physicalPin});
    }
    
    return mappings;
}

int PhysicalChannelMapper::getChannelCount() const
{
    return static_cast<int>(semanticToPhysical.size());
}

std::pair<int, int> PhysicalChannelMapper::getGridPosition(const juce::String& semanticName) const
{
    auto it = gridPositions.find(semanticName);
    if (it != gridPositions.end())
    {
        return it->second;
    }
    
    VST3_DBG("PhysicalChannelMapper: Warning - Grid position not found: " + semanticName);
    return {-1, -1};  // Invalid position
}

bool PhysicalChannelMapper::hasGridPosition(const juce::String& semanticName) const
{
    auto it = gridPositions.find(semanticName);
    return it != gridPositions.end() && it->second.first >= 0 && it->second.second >= 0;
}

void PhysicalChannelMapper::logCurrentMapping() const
{
    VST3_DBG("PhysicalChannelMapper: === Current mapping overview ===");
    VST3_DBG("  Total channels: " + juce::String(semanticToPhysical.size()));
    
    VST3_DBG("  Semantic -> physical mapping:");
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        auto gridPos = getGridPosition(semanticName);
        VST3_DBG("    " + semanticName + " -> Pin " + juce::String(physicalPin) + 
                 " (grid: " + juce::String(gridPos.first) + "," + juce::String(gridPos.second) + ")");
    }
    
    VST3_DBG("================================");
}

juce::String PhysicalChannelMapper::getMappingDescription() const
{
    juce::String desc = "Physical mapping: ";
    desc += "channels=" + juce::String(semanticToPhysical.size());
    desc += ", grid positions=" + juce::String(gridPositions.size());
    
    return desc;
}

void PhysicalChannelMapper::clearMapping()
{
    VST3_DBG("PhysicalChannelMapper: Clear all mappings");
    
    semanticToPhysical.clear();
    physicalToSemantic.clear();
    gridPositions.clear();
    channelInfoMap.clear();
}

void PhysicalChannelMapper::addMapping(const juce::String& semanticName, int physicalPin, int gridX, int gridY)
{
    semanticToPhysical[semanticName] = physicalPin;
    physicalToSemantic[physicalPin] = semanticName;
    
    if (gridX >= 0 && gridY >= 0)
    {
        gridPositions[semanticName] = {gridX, gridY};
    }
}

void PhysicalChannelMapper::removeMapping(const juce::String& semanticName)
{
    auto it = semanticToPhysical.find(semanticName);
    if (it != semanticToPhysical.end())
    {
        int physicalPin = it->second;
        semanticToPhysical.erase(it);
        physicalToSemantic.erase(physicalPin);
        gridPositions.erase(semanticName);
        channelInfoMap.erase(semanticName);
        
        VST3_DBG("PhysicalChannelMapper: Remove mapping - " + semanticName);
    }
}