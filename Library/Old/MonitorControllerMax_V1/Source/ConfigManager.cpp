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
        // 🚀 第八项优化：配置验证机制
        if (validateConfigStructure(configData))
        {
            configValid = true;
            configErrorMessage.clear();
            configSourceInfo = loadSuccess ? "External Configuration File" : "BinaryData Configuration";
            DBG("ConfigManager: Configuration validation passed");
        }
        else
        {
            DBG("ConfigManager: Configuration validation failed, falling back to default config");
            configErrorMessage = "Loaded configuration failed validation, using default configuration";
            generateDefaultConfig(); // 降级到默认配置
            configValid = true; // 默认配置总是有效的
            usingFallbackConfig = true;
        }
        
        // The max channel index is now dynamic based on layout, so we don't calculate it here.
        maxChannelIndex = 0; // Reset this, it's no longer used globally.
    }
    else
    {
        // 🚀 第八项优化：优雅降级机制 - 生成默认配置而非失败
        DBG("ConfigManager: Failed to load Speaker_Config.json from both BinaryData and filesystem, generating default configuration");
        configErrorMessage = "Failed to load configuration file, using built-in default configuration";
        generateDefaultConfig();
        configValid = true; // 默认配置总是有效的
        usingFallbackConfig = true;
        configSourceInfo = "Built-in Default Configuration (Fallback)";
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

// 🚀 第八项优化：优雅降级机制实现
void ConfigManager::generateDefaultConfig()
{
    DBG("ConfigManager: Generating built-in default configuration");
    
    // 创建默认配置结构
    juce::var defaultConfig;
    auto* configObj = new juce::DynamicObject();
    defaultConfig = configObj;
    
    // 添加默认Speaker布局
    configObj->setProperty("Speaker", createDefaultSpeakerLayouts());
    
    // 添加默认SUB布局
    configObj->setProperty("SUB", createDefaultSubLayouts());
    
    configData = defaultConfig;
    configSourceInfo = "Built-in Default Configuration";
    
    DBG("ConfigManager: Default configuration generated successfully");
}

juce::var ConfigManager::createDefaultSpeakerLayouts() const
{
    auto* speakerLayouts = new juce::DynamicObject();
    
    // 2.0立体声布局 (基础配置)
    auto* stereoLayout = new juce::DynamicObject();
    stereoLayout->setProperty("L", 1);
    stereoLayout->setProperty("R", 5);
    speakerLayouts->setProperty("2.0", stereoLayout);
    
    // 5.1环绕声布局
    auto* surround51Layout = new juce::DynamicObject();
    surround51Layout->setProperty("L", 1);
    surround51Layout->setProperty("R", 5);
    surround51Layout->setProperty("C", 3);
    surround51Layout->setProperty("LFE", 13);
    surround51Layout->setProperty("LR", 21);
    surround51Layout->setProperty("RR", 25);
    speakerLayouts->setProperty("5.1", surround51Layout);
    
    // 7.1.4杜比全景声布局 (专业配置)
    auto* atmos714Layout = new juce::DynamicObject();
    atmos714Layout->setProperty("L", 1);
    atmos714Layout->setProperty("R", 5);
    atmos714Layout->setProperty("C", 3);
    atmos714Layout->setProperty("LFE", 13);
    atmos714Layout->setProperty("LR", 21);
    atmos714Layout->setProperty("RR", 25);
    atmos714Layout->setProperty("LTF", 17);  // Left Top Front
    atmos714Layout->setProperty("RTF", 19);  // Right Top Front
    atmos714Layout->setProperty("LTR", 23);  // Left Top Rear
    atmos714Layout->setProperty("RTR", 27);  // Right Top Rear
    speakerLayouts->setProperty("7.1.4", atmos714Layout);
    
    return juce::var(speakerLayouts);
}

juce::var ConfigManager::createDefaultSubLayouts() const
{
    auto* subLayouts = new juce::DynamicObject();
    
    // 单超低音布局
    auto* singleSubLayout = new juce::DynamicObject();
    singleSubLayout->setProperty("SUB M", 9);
    subLayouts->setProperty("Single Sub", singleSubLayout);
    
    // 双超低音布局
    auto* dualSubLayout = new juce::DynamicObject();
    dualSubLayout->setProperty("SUB L", 9);
    dualSubLayout->setProperty("SUB R", 11);
    subLayouts->setProperty("Dual Sub", dualSubLayout);
    
    return juce::var(subLayouts);
}

// 🚀 第八项优化：配置验证机制实现
bool ConfigManager::validateConfigStructure(const juce::var& config) const
{
    DBG("ConfigManager: Validating configuration structure");
    
    if (!config.isObject())
    {
        DBG("ConfigManager: Config root is not an object");
        return false;
    }
    
    auto* configObj = config.getDynamicObject();
    if (!configObj)
    {
        DBG("ConfigManager: Failed to get config dynamic object");
        return false;
    }
    
    // 验证Speaker部分
    auto speakerSection = config.getProperty("Speaker", juce::var());
    if (!validateSpeakerSection(speakerSection))
    {
        DBG("ConfigManager: Speaker section validation failed");
        return false;
    }
    
    // 验证SUB部分 (可选)
    auto subSection = config.getProperty("SUB", juce::var());
    if (subSection.isObject() && !validateSubSection(subSection))
    {
        DBG("ConfigManager: SUB section validation failed");
        return false;
    }
    
    DBG("ConfigManager: Configuration structure validation passed");
    return true;
}

bool ConfigManager::validateSpeakerSection(const juce::var& speakerSection) const
{
    if (!speakerSection.isObject())
    {
        DBG("ConfigManager: Speaker section is not an object");
        return false;
    }
    
    auto* speakerObj = speakerSection.getDynamicObject();
    if (!speakerObj)
    {
        DBG("ConfigManager: Failed to get Speaker dynamic object");
        return false;
    }
    
    // 确保至少有一种扬声器布局
    if (speakerObj->getProperties().size() == 0)
    {
        DBG("ConfigManager: No speaker layouts found");
        return false;
    }
    
    // 验证每个扬声器布局
    for (const auto& prop : speakerObj->getProperties())
    {
        if (!validateLayoutChannels(prop.value, prop.name.toString()))
        {
            DBG("ConfigManager: Speaker layout '" + prop.name.toString() + "' validation failed");
            return false;
        }
    }
    
    return true;
}

bool ConfigManager::validateSubSection(const juce::var& subSection) const
{
    if (!subSection.isObject())
    {
        DBG("ConfigManager: SUB section is not an object");
        return false;
    }
    
    auto* subObj = subSection.getDynamicObject();
    if (!subObj)
    {
        DBG("ConfigManager: Failed to get SUB dynamic object");
        return false;
    }
    
    // 验证每个SUB布局
    for (const auto& prop : subObj->getProperties())
    {
        if (!validateLayoutChannels(prop.value, prop.name.toString()))
        {
            DBG("ConfigManager: SUB layout '" + prop.name.toString() + "' validation failed");
            return false;
        }
    }
    
    return true;
}

bool ConfigManager::validateLayoutChannels(const juce::var& layoutObj, const juce::String& layoutName) const
{
    if (!layoutObj.isObject())
    {
        DBG("ConfigManager: Layout '" + layoutName + "' is not an object");
        return false;
    }
    
    auto* layoutDynObj = layoutObj.getDynamicObject();
    if (!layoutDynObj)
    {
        DBG("ConfigManager: Failed to get layout '" + layoutName + "' dynamic object");
        return false;
    }
    
    // 确保布局至少包含一个通道
    if (layoutDynObj->getProperties().size() == 0)
    {
        DBG("ConfigManager: Layout '" + layoutName + "' has no channels");
        return false;
    }
    
    // 验证每个通道配置
    for (const auto& channelProp : layoutDynObj->getProperties())
    {
        // 检查通道名称不为空
        juce::String channelName = channelProp.name.toString();
        if (channelName.isEmpty())
        {
            DBG("ConfigManager: Empty channel name found in layout '" + layoutName + "'");
            return false;
        }
        
        // 检查网格位置是有效的数字
        if (!channelProp.value.isInt() && !channelProp.value.isDouble())
        {
            DBG("ConfigManager: Invalid grid position for channel '" + channelName + "' in layout '" + layoutName + "'");
            return false;
        }
        
        int gridPos = (int)channelProp.value;
        if (gridPos < 1 || gridPos > 50) // 合理的网格位置范围
        {
            DBG("ConfigManager: Grid position " + juce::String(gridPos) + " out of range for channel '" + channelName + "'");
            return false;
        }
    }
    
    return true;
} 