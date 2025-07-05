/*
  ==============================================================================

    ConfigManager.cpp
    Created: 5 Aug 2024 2:05:00pm
    Author:  Your Name

  ==============================================================================
*/

#include "ConfigManager.h"

// This includes the auto-generated header with our binary data.
#include "BinaryData.h"

ConfigManager::ConfigManager()
{
    loadConfig();
}

void ConfigManager::loadConfig()
{
    const char* configJsonData = BinaryData::Speaker_Config_json;
    int configJsonDataSize = BinaryData::Speaker_Config_jsonSize;

    if (configJsonData == nullptr)
    {
        DBG("ERROR: Speaker_Config.json binary data not found!");
        return;
    }

    juce::var parsedJson;
    auto result = juce::JSON::parse(juce::String::fromUTF8(configJsonData, configJsonDataSize), parsedJson);

    if (result.wasOk())
    {
        configData = parsedJson;
        
        // The max channel index is now dynamic based on layout, so we don't calculate it here.
        // Instead, the processor will query the default layout's channel count.
        maxChannelIndex = 0; // Reset this, it's no longer used globally.
    }
    else
    {
        DBG("ERROR: Failed to parse Speaker_Config.json: " + result.getErrorMessage());
    }
}


juce::StringArray ConfigManager::getSpeakerLayoutNames() const
{
    juce::StringArray names;
    if (auto* speakerConfig = configData.getProperty("Speaker", {}).getDynamicObject())
    {
        for (const auto& prop : speakerConfig->getProperties())
        {
            names.add(prop.name.toString());
        }
    }
    return names;
}

juce::StringArray ConfigManager::getSubLayoutNames() const
{
    juce::StringArray names { "None" };
    if (auto* subConfig = configData.getProperty("SUB", {}).getDynamicObject())
    {
        for (const auto& prop : subConfig->getProperties())
        {
            names.add(prop.name.toString());
        }
    }
    return names;
}

Layout ConfigManager::getLayoutFor(const juce::String& speakerLayoutName, const juce::String& subLayoutName) const
{
    Layout layout;
    int currentChannelIndex = 0;

    // Helper lambda to process a layout section (Speaker or SUB)
    auto processSection = [&](const juce::var& sectionConfig, const juce::String& layoutName)
    {
        if (layoutName.isEmpty() || layoutName == "None")
            return;

        auto layoutJson = sectionConfig.getProperty(layoutName, juce::var());

        if (layoutJson.isObject())
        {
            if (auto* layoutObj = layoutJson.getDynamicObject())
            {
                for (const auto& prop : layoutObj->getProperties())
                {
                    // 修复：直接将JSON中的整数值解析为网格位置
                    int gridPosition = (int)prop.value;
                    
                    // 恢复之前的逻辑：按顺序递增分配通道索引。
                    // 注意：这个逻辑依赖于JSON中属性的顺序，可能不稳定。
                    layout.channels.push_back({ prop.name.toString(), gridPosition, currentChannelIndex });
                    currentChannelIndex++;
                }
            }
        }
    };

    auto speakerConfig = configData.getProperty("Speaker", juce::var());
    processSection(speakerConfig, speakerLayoutName);

    auto subConfig = configData.getProperty("SUB", juce::var());
    processSection(subConfig, subLayoutName);

    // 总通道数就是我们添加的通道总数
    layout.totalChannelCount = currentChannelIndex;
    
    return layout;
}

int ConfigManager::getMaxChannelIndex() const
{
    // This function will now return the highest grid position, not channel index, for parameter creation.
    // Let's calculate it on the fly.
    int maxGridPos = 0;
    if (auto* speakerLayouts = configData["Speaker"].getDynamicObject())
    {
        for (auto& layout : speakerLayouts->getProperties())
        {
            if(auto* channels = layout.value.getDynamicObject())
            {
                for (auto& channel : channels->getProperties())
                {
                    int gridPos = channel.value;
                    if (gridPos > maxGridPos)
                        maxGridPos = gridPos;
                }
            }
        }
    }
    if (auto* subLayouts = configData["SUB"].getDynamicObject())
    {
         for (auto& layout : subLayouts->getProperties())
        {
            if(auto* channels = layout.value.getDynamicObject())
            {
                for (auto& channel : channels->getProperties())
                {
                    int gridPos = channel.value;
                    if (gridPos > maxGridPos)
                        maxGridPos = gridPos;
                }
            }
        }
    }
    return maxGridPos;
}

int ConfigManager::getChannelCountForLayout(const juce::String& layoutType, const juce::String& layoutName) const
{
    if (layoutName.isEmpty() || layoutName == "None")
        return 0;

    auto configSection = configData.getProperty(layoutType, juce::var());
    if (configSection.isObject())
    {
        auto layout = configSection.getProperty(layoutName, juce::var());
        if (layout.isObject())
        {
            if (auto* dynObj = layout.getDynamicObject())
            {
                return dynObj->getProperties().size();
            }
        }
    }

    return 0;
} 