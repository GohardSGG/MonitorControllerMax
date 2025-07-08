/*
  ==============================================================================
    DebugLogger.h
    VST3 Debug Logging System - 为VST3插件提供调试输出功能
    
    此系统解决VST3插件无法直接查看DBG()输出的问题，通过文件日志记录
    所有调试信息，使开发者能够在VST3环境中进行有效调试。
  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <fstream>
#include <memory>
#include <chrono>
#include <iomanip>
#include <sstream>

class DebugLogger
{
public:
    static DebugLogger& getInstance()
    {
        static DebugLogger instance;
        return instance;
    }
    
    // 初始化日志系统
    void initialize(const juce::String& pluginName = "MonitorControllerMax")
    {
        if (isInitialized) return;
        
        // 创建日志文件路径：%TEMP%/[PluginName]_Debug.log
        auto tempDir = juce::File::getSpecialLocation(juce::File::tempDirectory);
        logFile = tempDir.getChildFile(pluginName + "_Debug.log");
        
        // 打开日志文件
        logStream = std::make_unique<std::ofstream>(logFile.getFullPathName().toStdString(), 
                                                    std::ios::out | std::ios::app);
        
        if (logStream->is_open())
        {
            isInitialized = true;
            log("=== VST3 Debug Logger Initialized ===");
            log("Log file: " + logFile.getFullPathName());
        }
    }
    
    // 记录调试信息
    void log(const juce::String& message)
    {
        if (!isInitialized || !logStream || !logStream->is_open())
        {
            return;
        }
        
        // 获取当前时间戳
        auto now = std::chrono::system_clock::now();
        auto time_t = std::chrono::system_clock::to_time_t(now);
        auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
            now.time_since_epoch()) % 1000;
        
        std::stringstream timeStream;
        timeStream << std::put_time(std::localtime(&time_t), "%H:%M:%S");
        timeStream << "." << std::setfill('0') << std::setw(3) << ms.count();
        
        // 写入格式化的日志条目
        *logStream << "[" << timeStream.str() << "] " << message << std::endl;
        logStream->flush();
    }
    
    // 获取日志文件路径
    juce::File getLogFile() const
    {
        return logFile;
    }
    
    // 清空日志文件
    void clearLog()
    {
        if (logStream && logStream->is_open())
        {
            logStream->close();
        }
        
        if (logFile.exists())
        {
            logFile.deleteFile();
        }
        
        // 重新打开日志文件
        logStream = std::make_unique<std::ofstream>(logFile.getFullPathName().toStdString(), 
                                                    std::ios::out | std::ios::trunc);
        
        if (logStream->is_open())
        {
            log("=== Debug Log Cleared ===");
        }
    }
    
    // 检查日志系统是否已初始化
    bool isLoggerInitialized() const
    {
        return isInitialized;
    }
    
    ~DebugLogger()
    {
        if (logStream && logStream->is_open())
        {
            log("=== VST3 Debug Logger Shutdown ===");
            logStream->close();
        }
    }

private:
    DebugLogger() : isInitialized(false) {}
    
    bool isInitialized;
    juce::File logFile;
    std::unique_ptr<std::ofstream> logStream;
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(DebugLogger)
};

// 便捷宏定义 - 同时支持标准DBG和文件日志
#define VST3_DBG(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().log(oss.str()); \
    } while(0)