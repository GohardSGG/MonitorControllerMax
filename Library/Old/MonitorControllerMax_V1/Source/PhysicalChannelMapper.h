#pragma once

#include <JuceHeader.h>
#include "ConfigModels.h"
#include <map>
#include <vector>
#include <utility>

// Forward declaration
class MonitorControllerMaxAudioProcessor;

class PhysicalChannelMapper
{
public:
    PhysicalChannelMapper();
    ~PhysicalChannelMapper();
    
    // 设置processor指针用于角色日志
    void setProcessor(MonitorControllerMaxAudioProcessor* processor);

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
    
    // 🚀 第九项优化：全面边界检查和安全保护
    bool isPhysicalPinValid(int physicalPin) const;
    bool isSemanticNameValid(const juce::String& semanticName) const;
    bool isGridPositionValid(int gridX, int gridY) const;
    bool isGridPositionValid(int gridPosition) const;
    bool isChannelCountWithinLimits(int channelCount) const;
    
    // 安全的范围检查版本
    int getPhysicalPinWithBounds(const juce::String& semanticName, int minPin = 1, int maxPin = 64) const;
    std::pair<int, int> getGridPositionWithBounds(const juce::String& semanticName, int maxGridSize = 5) const;
    
    // 映射完整性验证
    bool validateMappingIntegrity() const;
    juce::String getValidationErrorReport() const;

private:
    // 🚀 第九项优化：安全边界常量定义
    static constexpr int MIN_PHYSICAL_PIN = 1;
    static constexpr int MAX_PHYSICAL_PIN = 64;
    static constexpr int MAX_GRID_SIZE = 5;
    static constexpr int MAX_CHANNEL_COUNT = 32;
    static constexpr int MIN_SEMANTIC_NAME_LENGTH = 1;
    static constexpr int MAX_SEMANTIC_NAME_LENGTH = 16;
    
    // Semantic name <-> Physical Pin mapping (inherits from existing config system)
    std::map<juce::String, int> semanticToPhysical;        // "L" -> 1, "R" -> 5, etc.
    std::map<int, juce::String> physicalToSemantic;        // 1 -> "L", 5 -> "R", etc.
    std::map<juce::String, std::pair<int, int>> gridPositions;  // "L" -> {gridX, gridY}
    
    // Channel information storage
    std::map<juce::String, ChannelInfo> channelInfoMap;
    
    // Processor指针用于角色日志
    MonitorControllerMaxAudioProcessor* processorPtr = nullptr;
    
    // 🚀 第九项优化：验证错误存储
    mutable juce::String lastValidationError;
    
    // Helper methods
    void addMapping(const juce::String& semanticName, int physicalPin, int gridX = -1, int gridY = -1);
    void removeMapping(const juce::String& semanticName);
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(PhysicalChannelMapper)
};