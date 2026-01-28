/*
  ==============================================================================
    DebugLogger.h
    Intelligent VST3 Debug Logger - Solves repetitive logs and excessive output
    
    Features:
    - Hierarchical log level control
    - Smart duplicate content filtering
    - Initialization phase log optimization
    - Important change detection
  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <fstream>
#include <memory>
#include <chrono>
#include <iomanip>
#include <sstream>
#include <unordered_map>
#include <unordered_set>

// Log level definitions
enum class LogLevel {
    CRITICAL = 0,    // Critical errors and important state changes
    IMPORTANT = 1,   // Important operations and state changes
    INFO = 2,        // General information
    DETAIL = 3,      // Detailed information (including repetitive content)
    VERBOSE = 4      // Very detailed information (for debugging)
};

class DebugLogger
{
public:
    static DebugLogger& getInstance()
    {
        static DebugLogger instance;
        return instance;
    }
    
    // Initialize logging system
    void initialize(const juce::String& pluginName = "MonitorControllerMax", LogLevel level = LogLevel::INFO)
    {
        if (isInitialized) return;
        
        currentLogLevel = level;
        
        // Create log file path: %TEMP%/[PluginName]_Debug.log
        auto tempDir = juce::File::getSpecialLocation(juce::File::tempDirectory);
        logFile = tempDir.getChildFile(pluginName + "_Debug.log");
        
        // Open log file
        logStream = std::make_unique<std::ofstream>(logFile.getFullPathName().toStdString(), 
                                                    std::ios::out | std::ios::app);
        
        if (logStream->is_open())
        {
            isInitialized = true;
            logInternal("=== VST3 Debug Logger Initialized ===", LogLevel::CRITICAL);
            logInternal("Log file: " + logFile.getFullPathName(), LogLevel::CRITICAL);
            logInternal("Current log level: " + getLogLevelName(currentLogLevel), LogLevel::CRITICAL);
            logInternal("Duplicate filtering: Enabled", LogLevel::CRITICAL);
        }
    }
    
    // Set log level
    void setLogLevel(LogLevel level)
    {
        currentLogLevel = level;
        logInternal("Log level changed to: " + getLogLevelName(level), LogLevel::CRITICAL);
    }
    
    // Get current log level
    LogLevel getLogLevel() const
    {
        return currentLogLevel;
    }
    
    // Log debug information (backward compatible, default INFO level)
    void log(const juce::String& message)
    {
        logWithLevel(message, LogLevel::INFO);
    }
    
    // Log with specific level
    void logWithLevel(const juce::String& message, LogLevel level)
    {
        logInternal(message, level);
    }
    
    // Convenience methods
    void logCritical(const juce::String& message) { logWithLevel(message, LogLevel::CRITICAL); }
    void logImportant(const juce::String& message) { logWithLevel(message, LogLevel::IMPORTANT); }
    void logInfo(const juce::String& message) { logWithLevel(message, LogLevel::INFO); }
    void logDetail(const juce::String& message) { logWithLevel(message, LogLevel::DETAIL); }
    void logVerbose(const juce::String& message) { logWithLevel(message, LogLevel::VERBOSE); }
    
    // Get log file path
    juce::File getLogFile() const
    {
        return logFile;
    }
    
    // Clear log file
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
        
        // Reopen log file
        logStream = std::make_unique<std::ofstream>(logFile.getFullPathName().toStdString(), 
                                                    std::ios::out | std::ios::trunc);
        
        if (logStream->is_open())
        {
            log("=== Debug Log Cleared ===");
        }
    }
    
    // Check if logger is initialized
    bool isLoggerInitialized() const
    {
        return isInitialized;
    }
    
    // 手动关闭并清理日志系统
    void shutdown()
    {
        if (logStream && logStream->is_open())
        {
            log("=== VST3 Debug Logger Shutdown ===");
            logStream->close();
        }
        
        // 插件关闭时自动删除日志文件，保持系统整洁
        if (logFile.exists())
        {
            bool deleted = logFile.deleteFile();
            // 如果删除失败，等待一小段时间后重试
            if (!deleted)
            {
                juce::Thread::sleep(100);
                logFile.deleteFile();
            }
        }
        
        isInitialized = false;
    }
    
    ~DebugLogger()
    {
        shutdown();
    }

private:
    DebugLogger() : isInitialized(false), currentLogLevel(LogLevel::INFO) {}
    
    // Internal log implementation - supports duplicate detection and level filtering
    void logInternal(const juce::String& message, LogLevel level)
    {
        if (!isInitialized || !logStream || !logStream->is_open())
        {
            return;
        }
        
        // Level filtering: only log current level and more important logs
        if (level > currentLogLevel)
        {
            return;
        }
        
        // Duplicate content detection
        if (isDuplicateMessage(message, level))
        {
            return;
        }
        
        // Get current timestamp
        auto now = std::chrono::system_clock::now();
        auto time_t = std::chrono::system_clock::to_time_t(now);
        auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
            now.time_since_epoch()) % 1000;
        
        std::stringstream timeStream;
        timeStream << std::put_time(std::localtime(&time_t), "%H:%M:%S");
        timeStream << "." << std::setfill('0') << std::setw(3) << ms.count();
        
        // Format log entry
        juce::String levelPrefix = getLogLevelPrefix(level);
        *logStream << "[" << timeStream.str() << "]" << levelPrefix << " " << message << std::endl;
        logStream->flush();
        
        // Record message for duplicate detection
        recordMessageForDuplicateDetection(message, level);
    }
    
    // Duplicate message detection
    bool isDuplicateMessage(const juce::String& message, LogLevel level)
    {
        // CRITICAL level messages are always logged
        if (level == LogLevel::CRITICAL)
        {
            return false;
        }
        
        // Check for duplicate overview information
        if (message.contains("=== Current mapping overview ===") || 
            message.contains("=== Current state overview ==="))
        {
            auto hash = generateMessageHash(message);
            if (recentOverviewHashes.find(hash) != recentOverviewHashes.end())
            {
                duplicateOverviewCount++;
                if (duplicateOverviewCount % 10 == 0)  // Every 10 duplicates, output a reminder
                {
                    logInternal("... Overview info repeated " + juce::String(duplicateOverviewCount) + " times, omitted", LogLevel::INFO);
                }
                return true;
            }
            recentOverviewHashes.insert(hash);
            
            // Limit saved hash count to avoid memory leak
            if (recentOverviewHashes.size() > 50)
            {
                recentOverviewHashes.clear();
            }
        }
        
        // Check for duplicate initialization information
        if (message.contains("Initialize semantic channel:") || 
            message.contains("PhysicalChannelMapper: Map channel"))
        {
            auto hash = generateMessageHash(message);
            if (recentInitHashes.find(hash) != recentInitHashes.end())
            {
                duplicateInitCount++;
                if (duplicateInitCount % 20 == 0)  // Every 20 duplicates, output a reminder
                {
                    logInternal("... Init info repeated " + juce::String(duplicateInitCount) + " times, omitted", LogLevel::INFO);
                }
                return true;
            }
            recentInitHashes.insert(hash);
            
            // 限制保存的hash数量
            if (recentInitHashes.size() > 100)
            {
                recentInitHashes.clear();
            }
        }
        
        return false;
    }
    
    // Generate message hash for duplicate detection
    std::size_t generateMessageHash(const juce::String& message) const
    {
        return std::hash<std::string>{}(message.toStdString());
    }
    
    // 记录消息用于重复检测
    void recordMessageForDuplicateDetection(const juce::String& message, LogLevel level)
    {
        // Currently handled in isDuplicateMessage, reserved for extension
    }
    
    // Get log level name
    juce::String getLogLevelName(LogLevel level) const
    {
        switch (level)
        {
            case LogLevel::CRITICAL: return "CRITICAL";
            case LogLevel::IMPORTANT: return "IMPORTANT";
            case LogLevel::INFO: return "INFO";
            case LogLevel::DETAIL: return "DETAIL";
            case LogLevel::VERBOSE: return "VERBOSE";
            default: return "UNKNOWN";
        }
    }
    
    // Get log level prefix
    juce::String getLogLevelPrefix(LogLevel level) const
    {
        switch (level)
        {
            case LogLevel::CRITICAL: return " [!]";
            case LogLevel::IMPORTANT: return " [*]";
            case LogLevel::INFO: return "";
            case LogLevel::DETAIL: return " [D]";
            case LogLevel::VERBOSE: return " [V]";
            default: return "";
        }
    }
    
    bool isInitialized;
    juce::File logFile;
    std::unique_ptr<std::ofstream> logStream;
    LogLevel currentLogLevel;
    
    // Duplicate detection related
    std::unordered_set<std::size_t> recentOverviewHashes;
    std::unordered_set<std::size_t> recentInitHashes;
    int duplicateOverviewCount = 0;
    int duplicateInitCount = 0;
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(DebugLogger)
};

// Forward declarations to avoid circular includes
class MonitorControllerMaxAudioProcessor;
enum class PluginRole;

// Convenience macro definitions - supports hierarchical log levels
#define VST3_DBG(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().log(oss.str()); \
    } while(0)

#define VST3_DBG_CRITICAL(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().logCritical(oss.str()); \
    } while(0)

#define VST3_DBG_IMPORTANT(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().logImportant(oss.str()); \
    } while(0)

#define VST3_DBG_INFO(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().logInfo(oss.str()); \
    } while(0)

#define VST3_DBG_DETAIL(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().logDetail(oss.str()); \
    } while(0)

#define VST3_DBG_VERBOSE(message) \
    do { \
        DBG(message); \
        std::ostringstream oss; \
        oss << message; \
        DebugLogger::getInstance().logVerbose(oss.str()); \
    } while(0)

// Role-aware debug macros with role prefix (defined where needed)
// Use these macros in PluginProcessor and PluginEditor files
// They will be implemented with inline functions to avoid circular includes