/*
  ==============================================================================

    ConfigManager.h
    Created: 5 Aug 2024 2:05:00pm
    Author:  Your Name

  ==============================================================================
*/

#pragma once
#include "ConfigModels.h"
#include <JuceHeader.h>

class ConfigManager
{
public:
    ConfigManager();

    juce::StringArray getSpeakerLayoutNames() const;
    juce::StringArray getSubLayoutNames() const;
    Layout getLayoutFor(const juce::String& speakerLayoutName, const juce::String& subLayoutName) const;
    int getMaxChannelIndex() const;
    int getChannelCountForLayout(const juce::String& layoutType, const juce::String& layoutName) const;
    
    // 🚀 第八项优化：配置状态查询和错误报告
    bool isConfigValid() const { return configValid; }
    bool isUsingFallbackConfig() const { return usingFallbackConfig; }
    juce::String getConfigErrorMessage() const { return configErrorMessage; }
    juce::String getConfigSourceInfo() const { return configSourceInfo; }

private:
    void loadConfig();
    
    // 🚀 第八项优化：优雅降级机制 - 默认配置生成
    void generateDefaultConfig();
    juce::var createDefaultSpeakerLayouts() const;
    juce::var createDefaultSubLayouts() const;
    
    // 🚀 第八项优化：配置验证机制
    bool validateConfigStructure(const juce::var& config) const;
    bool validateSpeakerSection(const juce::var& speakerSection) const;
    bool validateSubSection(const juce::var& subSection) const;
    bool validateLayoutChannels(const juce::var& layoutObj, const juce::String& layoutName) const;
    
    juce::var configData;
    int maxChannelIndex = 0;
    
    // 🚀 第八项优化：配置状态管理
    bool configValid = false;
    bool usingFallbackConfig = false;
    juce::String configErrorMessage;
    juce::String configSourceInfo;
}; 