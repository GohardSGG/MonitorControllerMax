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
    if (auto* speakerObj = configData["Speaker"].getDynamicObject())
    {
        for (auto& prop : speakerObj->getProperties())
            names.add(prop.name.toString());
    }
    return names;
}

juce::StringArray ConfigManager::getSubLayoutNames() const
{
    juce::StringArray names;
    names.add("None"); // Always have a "None" option
    if (auto* subObj = configData["SUB"].getDynamicObject())
    {
        for (auto& prop : subObj->getProperties())
            names.add(prop.name.toString());
    }
    return names;
}

Layout ConfigManager::getLayoutFor(const juce::String& speakerLayoutName, const juce::String& subLayoutName) const
{
    Layout layout;
    layout.totalChannelCount = 0;
    int currentChannelIndex = 0;

    auto speakerConfig = configData.getProperty("Speaker", juce::var());
    auto speakerLayout = speakerConfig.getProperty(speakerLayoutName, juce::var());
    if (speakerLayout.isObject())
    {
        if (auto* dynObj = speakerLayout.getDynamicObject())
        {
            for (const auto& prop : dynObj->getProperties())
            {
                layout.channels.push_back({ prop.name.toString(), (int)prop.value, currentChannelIndex });
                currentChannelIndex++;
            }
        }
    }

    if (subLayoutName != "None")
    {
        auto subConfig = configData.getProperty("SUB", juce::var());
        auto subLayout = subConfig.getProperty(subLayoutName, juce::var());
        if (subLayout.isObject())
        {
            if (auto* dynObj = subLayout.getDynamicObject())
            {
                for (const auto& prop : dynObj->getProperties())
                {
                    layout.channels.push_back({ prop.name.toString(), (int)prop.value, currentChannelIndex });
                    currentChannelIndex++;
                }
            }
        }
    }

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