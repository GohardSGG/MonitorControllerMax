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
        // This should not happen if the binary resource was added correctly.
        DBG("ERROR: Speaker_Config.json binary data not found!");
        return;
    }

    juce::var parsedJson;
    auto result = juce::JSON::parse(juce::String::fromUTF8(configJsonData, configJsonDataSize), parsedJson);

    if (result.wasOk())
    {
        configData = parsedJson;
        
        // Calculate max channel index
        if (auto* speakerLayouts = configData["Speaker"].getDynamicObject())
        {
            for (auto& layout : speakerLayouts->getProperties())
            {
                if(auto* channels = layout.value.getDynamicObject())
                {
                    for (auto& channel : channels->getProperties())
                    {
                        int channelIndex = channel.value;
                        if (channelIndex > maxChannelIndex)
                            maxChannelIndex = channelIndex;
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
                        int channelIndex = channel.value;
                        if (channelIndex > maxChannelIndex)
                            maxChannelIndex = channelIndex;
                    }
                }
            }
        }
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

    auto speakerConfig = configData.getProperty("Speaker", juce::var());
    auto speakerLayout = speakerConfig.getProperty(speakerLayoutName, juce::var());
    if (speakerLayout.isObject())
    {
        if (auto* dynObj = speakerLayout.getDynamicObject())
        {
            for (const auto& prop : dynObj->getProperties())
            {
                int channelIndex = prop.value;
                layout.channels.push_back({ prop.name.toString(), channelIndex, channelIndex - 1 });
                if (channelIndex > layout.totalChannelCount)
                    layout.totalChannelCount = channelIndex;
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
                    int channelIndex = prop.value;
                    layout.channels.push_back({ prop.name.toString(), channelIndex, channelIndex - 1 });
                    if (channelIndex > layout.totalChannelCount)
                        layout.totalChannelCount = channelIndex;
                }
            }
        }
    }

    return layout;
}

int ConfigManager::getMaxChannelIndex() const
{
    return maxChannelIndex;
} 