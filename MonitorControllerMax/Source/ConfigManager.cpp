﻿/*
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
    DBG("ConfigManager: Starting constructor");
    loadConfig();
    DBG("ConfigManager: Constructor finished");
}

void ConfigManager::loadConfig()
{
    const char* configJsonData = BinaryData::Speaker_Config_json;
    int configJsonDataSize = BinaryData::Speaker_Config_jsonSize;
    
    juce::var parsedJson;
    bool loadSuccess = false;

    // 首先尝试从BinaryData加载
    DBG("ConfigManager: BinaryData pointer: " + juce::String::toHexString((juce::pointer_sized_int)configJsonData));
    DBG("ConfigManager: BinaryData size: " + juce::String(configJsonDataSize));
    
    if (configJsonData != nullptr)
    {
        DBG("ConfigManager: BinaryData found, parsing JSON...");
        auto result = juce::JSON::parse(juce::String::fromUTF8(configJsonData, configJsonDataSize), parsedJson);
        if (result.wasOk())
        {
            configData = parsedJson;
            loadSuccess = true;
            DBG("ConfigManager: Loaded from BinaryData successfully");
        }
        else
        {
            DBG("ConfigManager: Failed to parse Speaker_Config.json from BinaryData: " + result.getErrorMessage());
        }
    }
    else
    {
        DBG("ConfigManager: BinaryData is nullptr!");
    }
    
    // 如果BinaryData加载失败，尝试从文件系统加载（开发时后备方案）
    if (!loadSuccess)
    {
        DBG("ConfigManager: BinaryData not available, trying filesystem fallback...");
        DBG("ConfigManager: Current working directory: " + juce::File::getCurrentWorkingDirectory().getFullPathName());
        
        // 尝试相对路径和几个可能的位置
        juce::StringArray possiblePaths = {
            "C:\\REAPER\\Effects\\Masking Effects\\MonitorControllerMax\\Source\\Config\\Speaker_Config.json",  // Windows绝对路径
            "Source/Config/Speaker_Config.json",
            "Config/Speaker_Config.json", 
            "../Source/Config/Speaker_Config.json",
            "MonitorControllerMax/Source/Config/Speaker_Config.json",
            "../../Source/Config/Speaker_Config.json",
            "../../../Source/Config/Speaker_Config.json"
        };
        
        for (const auto& path : possiblePaths)
        {
            juce::File configFile;
            
            // 判断是否为绝对路径
            if (path.startsWith("C:\\") || path.startsWith("/"))
            {
                configFile = juce::File(path);
            }
            else
            {
                configFile = juce::File::getCurrentWorkingDirectory().getChildFile(path);
            }
            
            DBG("ConfigManager: Trying path: " + configFile.getFullPathName() + " (exists: " + (configFile.existsAsFile() ? "YES" : "NO") + ")");
            
            if (configFile.existsAsFile())
            {
                juce::String jsonText = configFile.loadFileAsString();
                DBG("ConfigManager: File content length: " + juce::String(jsonText.length()));
                
                auto result = juce::JSON::parse(jsonText, parsedJson);
                if (result.wasOk())
                {
                    configData = parsedJson;
                    loadSuccess = true;
                    DBG("ConfigManager: ✅ Successfully loaded from filesystem: " + configFile.getFullPathName());
                    break;
                }
                else
                {
                    DBG("ConfigManager: JSON parse failed: " + result.getErrorMessage());
                }
            }
        }
    }
    
    if (loadSuccess)
    {
        // The max channel index is now dynamic based on layout, so we don't calculate it here.
        // Instead, the processor will query the default layout's channel count.
        maxChannelIndex = 0; // Reset this, it's no longer used globally.
    }
    else
    {
        DBG("ERROR: Failed to load Speaker_Config.json from both BinaryData and filesystem!");
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