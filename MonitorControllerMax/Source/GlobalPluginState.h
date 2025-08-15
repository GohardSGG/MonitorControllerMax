/*
  ==============================================================================
    GlobalPluginState.h
    静态全局状态管理器 - 实现同进程Master-Slave插件通信
    
    v4.0架构核心组件：
    - 线程安全的单例模式
    - Master/Slave角色管理
    - 零延迟状态同步
    - 插件生命周期管理
  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <memory>
#include <mutex>
#include <vector>
#include <map>
#include <algorithm>

// 前向声明
class MonitorControllerMaxAudioProcessor;

// 插件角色定义
enum class PluginRole {
    Standalone = 0,  // 默认独立模式
    Master = 1,      // 主控制模式  
    Slave = 2        // 从属显示模式
};

/**
 * 全局插件状态管理器
 * 管理同进程内所有插件实例的Master-Slave通信
 */
class GlobalPluginState {
private:
    // 🚀 升级：引用计数单例模式 - 确定性生命周期管理
    static std::shared_ptr<GlobalPluginState> instance;
    static std::mutex instanceMutex;
    static std::atomic<bool> shuttingDown; // 🛡️ 关闭状态标志
    static std::atomic<int> refCount; // 🚀 引用计数器，确保安全销毁
    
    // 全局状态存储
    std::map<juce::String, bool> globalSoloStates;
    std::map<juce::String, bool> globalMuteStates;
    
    // v4.1: 全局总线效果状态
    bool globalMonoState = false;
    
    mutable std::mutex stateMutex;
    
    // 🚀 生命周期安全：增强的插件实例管理
    MonitorControllerMaxAudioProcessor* masterPlugin = nullptr;
    std::vector<MonitorControllerMaxAudioProcessor*> slavePlugins;
    std::vector<MonitorControllerMaxAudioProcessor*> waitingSlavePlugins;  // 等待Master的Slave插件
    std::vector<MonitorControllerMaxAudioProcessor*> allPlugins;
    
    // 🛡️ 生命周期跟踪系统 - 双重验证机制
    std::map<MonitorControllerMaxAudioProcessor*, juce::String> pluginIds;  // 插件->唯一ID映射
    std::map<juce::String, MonitorControllerMaxAudioProcessor*> idToPlugin;  // ID->插件映射
    std::set<juce::String> validPluginIds;  // 有效插件ID集合
    std::set<MonitorControllerMaxAudioProcessor*> invalidatedPlugins; // 已失效的插件集合
    std::atomic<uint32_t> cleanupCounter{0};  // 清理操作计数器
    
    mutable std::mutex pluginsMutex;
    
    // 连接日志记录
    std::vector<juce::String> connectionLogs;
    mutable std::mutex logsMutex;
    static const size_t maxLogEntries = 50;
    
    // 🚀 稳定性优化第4步：健壮性监控计数器
    struct HealthMonitor {
        std::atomic<uint32_t> pluginRegistrations{0};      // 插件注册次数
        std::atomic<uint32_t> pluginUnregistrations{0};    // 插件注销次数
        std::atomic<uint32_t> masterPromotions{0};         // Master提升次数
        std::atomic<uint32_t> slaveConnections{0};         // Slave连接次数
        std::atomic<uint32_t> stateChanges{0};             // 状态变化次数
        std::atomic<uint32_t> broadcastCalls{0};           // 广播调用次数
        std::atomic<uint32_t> exceptionsCaught{0};         // 捕获的异常次数
        std::atomic<uint32_t> lockTimeouts{0};             // 锁超时次数
        std::atomic<uint32_t> invalidPluginCleanups{0};   // 无效插件清理次数
        
        // 获取健康报告
        juce::String getHealthReport() const;
    } healthMonitor;

public:
    // 🚀 升级：引用计数单例访问 - 确保生命周期安全
    static std::shared_ptr<GlobalPluginState> getInstance();
    
    // 🔄 兼容性：便利方法，返回引用（为了代码兼容性）
    static GlobalPluginState& getRef() {
        auto inst = getInstance();
        return *inst;
    }
    
    // 🛡️ 生命周期安全管理
    static void shutdown();
    static bool isShuttingDown();
    static void releaseReference(); // 🚀 引用计数释放
    
    // 🚀 构造和析构函数（需要public用于std::shared_ptr）
    GlobalPluginState() = default;
    ~GlobalPluginState() = default;
    
    // 🚀 生命周期安全：增强的插件管理接口
    void registerPlugin(MonitorControllerMaxAudioProcessor* plugin);
    void unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin);
    
    // Master插件管理
    bool setAsMaster(MonitorControllerMaxAudioProcessor* plugin);
    void removeMaster(MonitorControllerMaxAudioProcessor* plugin);
    bool isMasterPlugin(MonitorControllerMaxAudioProcessor* plugin) const;
    
    // Slave插件管理
    bool addSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    void removeSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    std::vector<MonitorControllerMaxAudioProcessor*> getSlavePlugins() const;
    
    // 等待Master的Slave管理
    void addWaitingSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    void removeWaitingSlavePlugin(MonitorControllerMaxAudioProcessor* plugin);
    void promoteWaitingSlavesToActive();  // 当Master可用时，将等待中的Slave提升为活跃
    
    // 状态同步机制
    void setGlobalSoloState(const juce::String& channelName, bool state);
    void setGlobalMuteState(const juce::String& channelName, bool state);
    bool getGlobalSoloState(const juce::String& channelName) const;
    bool getGlobalMuteState(const juce::String& channelName) const;
    
    // 广播机制 - 直接调用，零延迟
    void broadcastStateToSlaves(const juce::String& channelName, const juce::String& action, bool state);
    void syncAllStatesToSlave(MonitorControllerMaxAudioProcessor* slavePlugin);
    
    // v4.1: 总线效果状态管理
    void setGlobalMonoState(bool monoState);
    bool getGlobalMonoState() const;
    void broadcastMonoStateToSlaves(bool monoState);
    
    // 状态查询
    int getSlaveCount() const;
    int getWaitingSlaveCount() const;
    bool hasMaster() const;
    juce::String getConnectionInfo() const;
    MonitorControllerMaxAudioProcessor* getMasterPlugin() const;
    
    // 连接日志管理
    void addConnectionLog(const juce::String& message);
    std::vector<juce::String> getConnectionLogs() const;
    void clearConnectionLogs();
    
    // 🚀 稳定性优化第4步：健康监控接口
    juce::String getHealthReport() const;
    void resetHealthCounters();
    uint32_t getTotalExceptions() const { return healthMonitor.exceptionsCaught.load(); }

private:
    // 防止复制
    GlobalPluginState(const GlobalPluginState&) = delete;
    GlobalPluginState& operator=(const GlobalPluginState&) = delete;
    
    // 🚀 生命周期安全：内部辅助方法
    void cleanupInvalidPlugins();           // 清理失效的插件指针
    void performHealthyCleanup();           // 定期健康检查和清理
    void performSafeCleanup();              // 安全的插件清理（双重验证）
    juce::String generateUniquePluginId(MonitorControllerMaxAudioProcessor* plugin);  // 生成唯一插件ID
    bool isPluginValid(MonitorControllerMaxAudioProcessor* plugin) const;  // 检查插件是否有效
    bool isPluginSafeToAccess(MonitorControllerMaxAudioProcessor* plugin) const; // 双重安全检查
    void invalidatePlugin(MonitorControllerMaxAudioProcessor* plugin);     // 标记插件为无效
    void removeFromAllLists(MonitorControllerMaxAudioProcessor* plugin);   // 从所有列表中安全移除
    void notifySlavePluginsAboutMasterLoss();  // 通知Slave插件Master已丢失
    juce::String getCurrentTimeString() const;
};