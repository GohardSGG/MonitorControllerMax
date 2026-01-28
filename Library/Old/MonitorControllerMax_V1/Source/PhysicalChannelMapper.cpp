#include "PhysicalChannelMapper.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

// PhysicalChannelMapper类专用角色日志宏
#define MAPPER_DBG_ROLE(message) \
    do { \
        if (processorPtr) { \
            VST3_DBG_ROLE(processorPtr, message); \
        } else { \
            VST3_DBG("[Mapper] " + juce::String(message)); \
        } \
    } while(0)

PhysicalChannelMapper::PhysicalChannelMapper()
{
    MAPPER_DBG_ROLE("PhysicalChannelMapper: Initialize physical channel mapping system");
}

PhysicalChannelMapper::~PhysicalChannelMapper()
{
    MAPPER_DBG_ROLE("PhysicalChannelMapper: Destroy physical channel mapping system");
}

void PhysicalChannelMapper::setProcessor(MonitorControllerMaxAudioProcessor* processor)
{
    processorPtr = processor;
}

void PhysicalChannelMapper::updateMapping(const Layout& layout)
{
    // 删除垃圾日志 - 高频映射配置更新
    
    // 🚀 第九项优化：布局边界检查
    if (!isChannelCountWithinLimits(layout.channels.size()))
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: ERROR - Layout channel count " + juce::String(layout.channels.size()) + " exceeds maximum " + juce::String(MAX_CHANNEL_COUNT));
        return; // 拒绝处理超出限制的布局
    }
    
    // Clear existing mappings
    clearMapping();
    
    // Parse mapping relationships from configuration - fully compatible with existing config format
    for (const auto& channelInfo : layout.channels)
    {
        juce::String semanticName = channelInfo.name;         // "L", "R", "C"
        int physicalPin = channelInfo.channelIndex;           // 1, 5, 3 (from config file)
        
        // 🚀 第九项优化：输入验证
        if (!isSemanticNameValid(semanticName))
        {
            MAPPER_DBG_ROLE("PhysicalChannelMapper: WARNING - Invalid semantic name '" + semanticName + "', skipping");
            continue;
        }
        
        if (!isPhysicalPinValid(physicalPin))
        {
            MAPPER_DBG_ROLE("PhysicalChannelMapper: WARNING - Invalid physical pin " + juce::String(physicalPin) + " for channel '" + semanticName + "', skipping");
            continue;
        }
        
        if (!isGridPositionValid(channelInfo.gridPosition))
        {
            MAPPER_DBG_ROLE("PhysicalChannelMapper: WARNING - Invalid grid position " + juce::String(channelInfo.gridPosition) + " for channel '" + semanticName + "', skipping");
            continue;
        }
        
        // Convert 1-based gridPosition to 0-based x,y coordinates (5x5 grid)
        int gridPos = channelInfo.gridPosition - 1;  // Convert to 0-based
        int gridX = gridPos % MAX_GRID_SIZE;          // Column (0-4)
        int gridY = gridPos / MAX_GRID_SIZE;          // Row (0-4)
        
        VST3_DBG_DETAIL("PhysicalChannelMapper: Map channel - " + semanticName + " -> physical pin " + juce::String(physicalPin) + 
                 " (grid position: " + juce::String(gridX) + "," + juce::String(gridY) + ")");
        
        addMapping(semanticName, physicalPin, gridX, gridY);
        
        // Store channel info
        channelInfoMap[semanticName] = channelInfo;
    }
    
    // 🚀 第九项优化：映射完整性验证
    if (!validateMappingIntegrity())
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: WARNING - Mapping integrity validation failed: " + getValidationErrorReport());
    }
    
    // 删除垃圾日志 - 重复的映射统计
    logCurrentMapping();
}

void PhysicalChannelMapper::updateFromConfig(const juce::String& speakerLayout, const juce::String& subLayout)
{
    // 删除垃圾日志 - 配置更新高频调用
    
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
    
    MAPPER_DBG_ROLE("PhysicalChannelMapper: Warning - Semantic channel mapping not found: " + semanticName);
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
    
    // 删除垃圾日志 - 高频统计调用
    
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
    
    MAPPER_DBG_ROLE("PhysicalChannelMapper: Warning - Grid position not found: " + semanticName);
    return {-1, -1};  // Invalid position
}

bool PhysicalChannelMapper::hasGridPosition(const juce::String& semanticName) const
{
    auto it = gridPositions.find(semanticName);
    return it != gridPositions.end() && it->second.first >= 0 && it->second.second >= 0;
}

void PhysicalChannelMapper::logCurrentMapping() const
{
    // 使用DETAIL级别 - 重复内容会被智能过滤
    VST3_DBG_DETAIL("PhysicalChannelMapper: === Current mapping overview ===");
    VST3_DBG_DETAIL("  Total channels: " + juce::String(semanticToPhysical.size()));
    
    VST3_DBG_DETAIL("  Semantic -> physical mapping:");
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        auto gridPos = getGridPosition(semanticName);
        VST3_DBG_DETAIL("    " + semanticName + " -> Pin " + juce::String(physicalPin) + 
                 " (grid: " + juce::String(gridPos.first) + "," + juce::String(gridPos.second) + ")");
    }
    
    VST3_DBG_DETAIL("================================");
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
    // 删除垃圾日志 - 映射清理高频调用
    
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
        
        MAPPER_DBG_ROLE("PhysicalChannelMapper: Remove mapping - " + semanticName);
    }
}

// 🚀 第九项优化：全面边界检查和安全保护实现
bool PhysicalChannelMapper::isPhysicalPinValid(int physicalPin) const
{
    return physicalPin >= MIN_PHYSICAL_PIN && physicalPin <= MAX_PHYSICAL_PIN;
}

bool PhysicalChannelMapper::isSemanticNameValid(const juce::String& semanticName) const
{
    if (semanticName.isEmpty())
        return false;
        
    int length = semanticName.length();
    if (length < MIN_SEMANTIC_NAME_LENGTH || length > MAX_SEMANTIC_NAME_LENGTH)
        return false;
        
    // 检查是否包含非法字符 (只允许字母、数字、下划线、空格)
    for (int i = 0; i < length; ++i)
    {
        juce::juce_wchar c = semanticName[i];
        if (!juce::CharacterFunctions::isLetterOrDigit(c) && c != '_' && c != ' ')
        {
            return false;
        }
    }
    
    return true;
}

bool PhysicalChannelMapper::isGridPositionValid(int gridX, int gridY) const
{
    return gridX >= 0 && gridX < MAX_GRID_SIZE && gridY >= 0 && gridY < MAX_GRID_SIZE;
}

bool PhysicalChannelMapper::isGridPositionValid(int gridPosition) const
{
    // 1-based grid position validation for 5x5 grid (1-25)
    return gridPosition >= 1 && gridPosition <= (MAX_GRID_SIZE * MAX_GRID_SIZE);
}

bool PhysicalChannelMapper::isChannelCountWithinLimits(int channelCount) const
{
    return channelCount >= 0 && channelCount <= MAX_CHANNEL_COUNT;
}

int PhysicalChannelMapper::getPhysicalPinWithBounds(const juce::String& semanticName, int minPin, int maxPin) const
{
    if (!isSemanticNameValid(semanticName))
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getPhysicalPinWithBounds - Invalid semantic name: " + semanticName);
        return -1;
    }
    
    auto it = semanticToPhysical.find(semanticName);
    if (it == semanticToPhysical.end())
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getPhysicalPinWithBounds - Semantic channel not found: " + semanticName);
        return -1;
    }
    
    int physicalPin = it->second;
    
    // 应用边界限制
    if (physicalPin < minPin || physicalPin > maxPin)
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getPhysicalPinWithBounds - Physical pin " + juce::String(physicalPin) + " out of bounds [" + juce::String(minPin) + "," + juce::String(maxPin) + "] for channel: " + semanticName);
        return juce::jlimit(minPin, maxPin, physicalPin); // 钳制到有效范围
    }
    
    return physicalPin;
}

std::pair<int, int> PhysicalChannelMapper::getGridPositionWithBounds(const juce::String& semanticName, int maxGridSize) const
{
    if (!isSemanticNameValid(semanticName))
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getGridPositionWithBounds - Invalid semantic name: " + semanticName);
        return {-1, -1};
    }
    
    auto it = gridPositions.find(semanticName);
    if (it == gridPositions.end())
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getGridPositionWithBounds - Grid position not found: " + semanticName);
        return {-1, -1};
    }
    
    int gridX = it->second.first;
    int gridY = it->second.second;
    
    // 应用边界限制
    if (gridX < 0 || gridX >= maxGridSize || gridY < 0 || gridY >= maxGridSize)
    {
        MAPPER_DBG_ROLE("PhysicalChannelMapper: getGridPositionWithBounds - Grid position (" + juce::String(gridX) + "," + juce::String(gridY) + ") out of bounds for channel: " + semanticName);
        return {juce::jlimit(0, maxGridSize-1, gridX), juce::jlimit(0, maxGridSize-1, gridY)};
    }
    
    return {gridX, gridY};
}

bool PhysicalChannelMapper::validateMappingIntegrity() const
{
    lastValidationError.clear();
    
    // 检查双向映射一致性
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        auto reverseIt = physicalToSemantic.find(physicalPin);
        if (reverseIt == physicalToSemantic.end() || reverseIt->second != semanticName)
        {
            lastValidationError = "Bidirectional mapping inconsistency: " + semanticName + " -> " + juce::String(physicalPin);
            return false;
        }
    }
    
    // 检查反向映射一致性
    for (const auto& [physicalPin, semanticName] : physicalToSemantic)
    {
        auto forwardIt = semanticToPhysical.find(semanticName);
        if (forwardIt == semanticToPhysical.end() || forwardIt->second != physicalPin)
        {
            lastValidationError = "Reverse mapping inconsistency: " + juce::String(physicalPin) + " -> " + semanticName;
            return false;
        }
    }
    
    // 检查物理pin的唯一性（一个物理pin不能被映射到多个语义通道）
    std::set<int> usedPhysicalPins;
    for (const auto& [semanticName, physicalPin] : semanticToPhysical)
    {
        if (usedPhysicalPins.find(physicalPin) != usedPhysicalPins.end())
        {
            lastValidationError = "Duplicate physical pin mapping: " + juce::String(physicalPin);
            return false;
        }
        usedPhysicalPins.insert(physicalPin);
    }
    
    // 检查语义名称的唯一性（一个语义名称不能被映射到多个物理pin）
    std::set<juce::String> usedSemanticNames;
    for (const auto& [physicalPin, semanticName] : physicalToSemantic)
    {
        if (usedSemanticNames.find(semanticName) != usedSemanticNames.end())
        {
            lastValidationError = "Duplicate semantic name mapping: " + semanticName;
            return false;
        }
        usedSemanticNames.insert(semanticName);
    }
    
    // 检查网格位置的唯一性（一个网格位置不能被多个通道占用）
    std::set<std::pair<int, int>> usedGridPositions;
    for (const auto& [semanticName, gridPos] : gridPositions)
    {
        if (gridPos.first >= 0 && gridPos.second >= 0) // 只检查有效的网格位置
        {
            if (usedGridPositions.find(gridPos) != usedGridPositions.end())
            {
                lastValidationError = "Duplicate grid position: (" + juce::String(gridPos.first) + "," + juce::String(gridPos.second) + ")";
                return false;
            }
            usedGridPositions.insert(gridPos);
        }
    }
    
    return true;
}

juce::String PhysicalChannelMapper::getValidationErrorReport() const
{
    return lastValidationError.isEmpty() ? "No validation errors" : lastValidationError;
}