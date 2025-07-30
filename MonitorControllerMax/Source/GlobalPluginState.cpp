#include "GlobalPluginState.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

// 静态成员初始化
std::unique_ptr<GlobalPluginState> GlobalPluginState::instance = nullptr;
std::mutex GlobalPluginState::instanceMutex;

GlobalPluginState& GlobalPluginState::getInstance() {
    std::lock_guard<std::mutex> lock(instanceMutex);
    if (!instance) {
        instance = std::unique_ptr<GlobalPluginState>(new GlobalPluginState());
    }
    return *instance;
}

void GlobalPluginState::registerPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        if (plugin == nullptr) return;
        
        auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
        if (it == allPlugins.end()) {
            allPlugins.push_back(plugin);
            
            // 🚀 稳定性优化：计数器监控
            healthMonitor.pluginRegistrations++;
            
            juce::String logMsg = getCurrentTimeString() + " Plugin registered (ID: " + 
                                 juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                                 ") - Total: " + juce::String(allPlugins.size());
            
            VST3_DBG(logMsg);
            addConnectionLog(logMsg);
        }
    }
    catch (...) {
        // 🚀 稳定性优化：异常处理
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in registerPlugin - continuing safely");
    }
}

void GlobalPluginState::unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        if (plugin == nullptr) return;
        
        // 从所有列表中移除
        auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
        if (it != allPlugins.end()) {
            allPlugins.erase(it);
            
            // 🚀 稳定性优化：计数器监控
            healthMonitor.pluginUnregistrations++;
            
            juce::String logMsg = getCurrentTimeString() + " Plugin unregistered (ID: " + 
                                 juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                                 ") - Remaining: " + juce::String(allPlugins.size());
            
            VST3_DBG(logMsg);
            addConnectionLog(logMsg);
        }
    }
    catch (...) {
        // 🚀 稳定性优化：异常处理
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in unregisterPlugin - continuing safely");
        return;  // 安全退出，避免进一步处理
    }
    
    try {
        // 如果是Master，清除Master状态  
        if (masterPlugin == plugin) {
            masterPlugin = nullptr;
        
        juce::String masterLogMsg = getCurrentTimeString() + " Master plugin unregistered - Master role available";
        VST3_DBG(masterLogMsg);
        addConnectionLog(masterLogMsg);
        
        // 通知所有Slave切换到Standalone
        for (auto* slave : slavePlugins) {
            if (slave != nullptr && slave != plugin) {
                juce::MessageManager::callAsync([slave]() {
                    slave->onMasterDisconnected();
                });
            }
        }
        slavePlugins.clear();
    }
    
    // 如果是Slave，从Slave列表移除
    auto slaveIt = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (slaveIt != slavePlugins.end()) {
        slavePlugins.erase(slaveIt);
        
        juce::String slaveLogMsg = getCurrentTimeString() + " Slave plugin unregistered - Active slaves: " + 
                                  juce::String(slavePlugins.size());
        VST3_DBG(slaveLogMsg);
        addConnectionLog(slaveLogMsg);
    }
    
    // 如果在等待列表中，也要移除
    auto waitingIt = std::find(waitingSlavePlugins.begin(), waitingSlavePlugins.end(), plugin);
    if (waitingIt != waitingSlavePlugins.end()) {
        waitingSlavePlugins.erase(waitingIt);
        
        juce::String waitingLogMsg = getCurrentTimeString() + " Waiting slave plugin unregistered - Waiting slaves: " + 
                                    juce::String(waitingSlavePlugins.size());
        VST3_DBG(waitingLogMsg);
        addConnectionLog(waitingLogMsg);
        }
    }
    catch (...) {
        // 🚀 稳定性优化：异常处理
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in unregisterPlugin Master/Slave cleanup - continuing safely");
    }
}

bool GlobalPluginState::setAsMaster(MonitorControllerMaxAudioProcessor* plugin) {
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        if (plugin == nullptr) return false;
        
        // 检查是否已经有Master
        if (masterPlugin != nullptr && masterPlugin != plugin) {
            juce::String logMsg = getCurrentTimeString() + " Master role denied - Master already exists";
            VST3_DBG(logMsg);
            addConnectionLog(logMsg);
            return false;
        }
        
        // 从Slave列表移除（如果存在）
        auto slaveIt = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
        if (slaveIt != slavePlugins.end()) {
            slavePlugins.erase(slaveIt);
        }
        
        masterPlugin = plugin;
        
        // 🚀 稳定性优化：计数器监控
        healthMonitor.masterPromotions++;
        
        juce::String logMsg = getCurrentTimeString() + " Master role assigned (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + ")";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 将等待中的Slave提升为活跃Slave
        promoteWaitingSlavesToActive();
        
        return true;
    }
    catch (...) {
        // 🚀 稳定性优化：异常处理
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in setAsMaster - returning false safely");
        return false;
    }
}

void GlobalPluginState::removeMaster(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (masterPlugin == plugin) {
        masterPlugin = nullptr;
        
        juce::String logMsg = getCurrentTimeString() + " Master role removed - Role available";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 通知所有Slave自动切换到Standalone
        for (auto* slave : slavePlugins) {
            if (slave != nullptr) {
                juce::MessageManager::callAsync([slave]() {
                    slave->onMasterDisconnected();
                });
            }
        }
        
        // 将活跃的Slave移到等待列表
        for (auto* slave : slavePlugins) {
            if (slave != nullptr) {
                waitingSlavePlugins.push_back(slave);
            }
        }
        slavePlugins.clear();
        
        if (!waitingSlavePlugins.empty()) {
            juce::String waitingLogMsg = getCurrentTimeString() + " Moved " + juce::String(waitingSlavePlugins.size()) + 
                                        " slaves to waiting list - waiting for new Master";
            VST3_DBG(waitingLogMsg);
            addConnectionLog(waitingLogMsg);
        }
    }
}

bool GlobalPluginState::isMasterPlugin(MonitorControllerMaxAudioProcessor* plugin) const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return masterPlugin == plugin;
}

bool GlobalPluginState::addSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (plugin == nullptr) return false;
    
    // 不能将Master设为Slave
    if (plugin == masterPlugin) {
        juce::String logMsg = getCurrentTimeString() + " Slave role denied - Plugin is Master";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        return false;
    }
    
    // 检查是否有Master
    if (masterPlugin == nullptr) {
        // 没有Master，将Slave加入等待列表
        addWaitingSlavePlugin(plugin);
        return true;  // 返回true表示成功加入等待列表
    }
    
    // 有Master，直接加入活跃Slave列表
    // 检查是否已经是Slave
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it == slavePlugins.end()) {
        slavePlugins.push_back(plugin);
        
        juce::String logMsg = getCurrentTimeString() + " Slave role assigned (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Active slaves: " + juce::String(slavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 立即同步Master状态到新Slave
        syncAllStatesToSlave(plugin);
    }
    
    return true;
}

void GlobalPluginState::removeSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    // 从活跃Slave列表移除
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it != slavePlugins.end()) {
        slavePlugins.erase(it);
        
        juce::String logMsg = getCurrentTimeString() + " Slave role removed (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Active slaves: " + juce::String(slavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
    
    // 从等待Slave列表移除
    removeWaitingSlavePlugin(plugin);
}

std::vector<MonitorControllerMaxAudioProcessor*> GlobalPluginState::getSlavePlugins() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return slavePlugins;
}

void GlobalPluginState::setGlobalSoloState(const juce::String& channelName, bool state) {
    std::lock_guard<std::mutex> lock(stateMutex);
    globalSoloStates[channelName] = state;
}

void GlobalPluginState::setGlobalMuteState(const juce::String& channelName, bool state) {
    std::lock_guard<std::mutex> lock(stateMutex);
    globalMuteStates[channelName] = state;
}

bool GlobalPluginState::getGlobalSoloState(const juce::String& channelName) const {
    std::lock_guard<std::mutex> lock(stateMutex);
    auto it = globalSoloStates.find(channelName);
    return it != globalSoloStates.end() ? it->second : false;
}

bool GlobalPluginState::getGlobalMuteState(const juce::String& channelName) const {
    std::lock_guard<std::mutex> lock(stateMutex);
    auto it = globalMuteStates.find(channelName);
    return it != globalMuteStates.end() ? it->second : false;
}

void GlobalPluginState::broadcastStateToSlaves(const juce::String& channelName, const juce::String& action, bool state) {
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        // 清理无效插件指针
        cleanupInvalidPlugins();
        
        if (slavePlugins.empty()) return;
        
        // 🚀 稳定性优化：计数器监控
        healthMonitor.broadcastCalls++;
        
        VST3_DBG("Broadcasting " + action + " " + channelName + " = " + (state ? "true" : "false") + 
                 " to " + juce::String(slavePlugins.size()) + " slaves");
        
        for (auto* slave : slavePlugins) {
            if (slave != nullptr) {
                try {
                    // 直接调用Slave的状态接收方法 - 零延迟
                    slave->receiveMasterState(channelName, action, state);
                } catch (const std::exception& e) {
                    // 🚀 稳定性优化：记录个别Slave通信异常
                    healthMonitor.exceptionsCaught++;
                    VST3_DBG("Error broadcasting to slave: " + juce::String(e.what()));
                } catch (...) {
                    // 🚀 稳定性优化：捕获所有类型异常
                    healthMonitor.exceptionsCaught++;
                    VST3_DBG("Unknown error broadcasting to slave");
                }
            }
        }
    }
    catch (...) {
        // 🚀 稳定性优化：整体异常处理
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in broadcastStateToSlaves - continuing safely");
    }
}

void GlobalPluginState::syncAllStatesToSlave(MonitorControllerMaxAudioProcessor* slavePlugin) {
    if (slavePlugin == nullptr || masterPlugin == nullptr) return;
    
    std::lock_guard<std::mutex> stateLock(stateMutex);
    
    VST3_DBG("Syncing all Master states to new Slave");
    
    // 同步所有Solo状态
    for (const auto& [channelName, state] : globalSoloStates) {
        try {
            slavePlugin->receiveMasterState(channelName, "solo", state);
        } catch (const std::exception& e) {
            VST3_DBG("Error syncing solo state: " + juce::String(e.what()));
        }
    }
    
    // 同步所有Mute状态
    for (const auto& [channelName, state] : globalMuteStates) {
        try {
            slavePlugin->receiveMasterState(channelName, "mute", state);
        } catch (const std::exception& e) {
            VST3_DBG("Error syncing mute state: " + juce::String(e.what()));
        }
    }
    
    // v4.1: 同步总线效果状态
    try {
        slavePlugin->receiveMasterBusState("mono", globalMonoState);
    } catch (const std::exception& e) {
        VST3_DBG("Error syncing mono state: " + juce::String(e.what()));
    }
}

int GlobalPluginState::getSlaveCount() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return static_cast<int>(slavePlugins.size());
}

int GlobalPluginState::getWaitingSlaveCount() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return static_cast<int>(waitingSlavePlugins.size());
}

void GlobalPluginState::addWaitingSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    // 此方法在已持有锁的情况下调用
    if (plugin == nullptr) return;
    
    // 从活跃Slave列表移除（如果存在）
    auto activeIt = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (activeIt != slavePlugins.end()) {
        slavePlugins.erase(activeIt);
    }
    
    // 检查是否已在等待列表
    auto waitingIt = std::find(waitingSlavePlugins.begin(), waitingSlavePlugins.end(), plugin);
    if (waitingIt == waitingSlavePlugins.end()) {
        waitingSlavePlugins.push_back(plugin);
        
        juce::String logMsg = getCurrentTimeString() + " Slave added to waiting list (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Waiting slaves: " + juce::String(waitingSlavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
}

void GlobalPluginState::removeWaitingSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    // 此方法在已持有锁的情况下调用
    auto waitingIt = std::find(waitingSlavePlugins.begin(), waitingSlavePlugins.end(), plugin);
    if (waitingIt != waitingSlavePlugins.end()) {
        waitingSlavePlugins.erase(waitingIt);
        
        juce::String logMsg = getCurrentTimeString() + " Slave removed from waiting list (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Waiting slaves: " + juce::String(waitingSlavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
}

void GlobalPluginState::promoteWaitingSlavesToActive() {
    // 此方法在已持有锁的情况下调用
    if (waitingSlavePlugins.empty()) return;
    
    juce::String logMsg = getCurrentTimeString() + " Promoting " + juce::String(waitingSlavePlugins.size()) + 
                         " waiting slaves to active - Master is now available";
    VST3_DBG(logMsg);
    addConnectionLog(logMsg);
    
    // 将所有等待中的Slave提升为活跃Slave
    for (auto* waitingSlave : waitingSlavePlugins) {
        if (waitingSlave != nullptr) {
            slavePlugins.push_back(waitingSlave);
            
            // 同步Master状态到新连接的Slave
            syncAllStatesToSlave(waitingSlave);
            
            // 通知Slave现在已连接到Master
            juce::MessageManager::callAsync([waitingSlave]() {
                waitingSlave->onMasterConnected();
            });
        }
    }
    
    // 清空等待列表
    waitingSlavePlugins.clear();
    
    juce::String finalLogMsg = getCurrentTimeString() + " All waiting slaves promoted - Active slaves: " + 
                              juce::String(slavePlugins.size());
    VST3_DBG(finalLogMsg);
    addConnectionLog(finalLogMsg);
}

bool GlobalPluginState::hasMaster() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return masterPlugin != nullptr;
}

juce::String GlobalPluginState::getConnectionInfo() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    juce::String info;
    
    if (masterPlugin != nullptr) {
        info += "Master: Active";
        if (!slavePlugins.empty()) {
            info += " | Slaves: " + juce::String(slavePlugins.size());
        } else {
            info += " | No Slaves";
        }
    } else {
        info += "No Master";
        if (!waitingSlavePlugins.empty()) {
            info += " | Waiting Slaves: " + juce::String(waitingSlavePlugins.size());
        }
        info += " | Plugins: " + juce::String(allPlugins.size());
    }
    
    return info;
}

MonitorControllerMaxAudioProcessor* GlobalPluginState::getMasterPlugin() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return masterPlugin;
}

void GlobalPluginState::addConnectionLog(const juce::String& message) {
    std::lock_guard<std::mutex> lock(logsMutex);
    
    connectionLogs.push_back(message);
    
    // 限制日志条目数量
    if (connectionLogs.size() > maxLogEntries) {
        connectionLogs.erase(connectionLogs.begin());
    }
}

std::vector<juce::String> GlobalPluginState::getConnectionLogs() const {
    std::lock_guard<std::mutex> lock(logsMutex);
    return connectionLogs;
}

void GlobalPluginState::clearConnectionLogs() {
    std::lock_guard<std::mutex> lock(logsMutex);
    connectionLogs.clear();
}

void GlobalPluginState::cleanupInvalidPlugins() {
    // 移除空指针（这个方法在持有锁的情况下调用）
    slavePlugins.erase(
        std::remove_if(slavePlugins.begin(), slavePlugins.end(),
            [](MonitorControllerMaxAudioProcessor* plugin) {
                return plugin == nullptr;
            }),
        slavePlugins.end()
    );
    
    waitingSlavePlugins.erase(
        std::remove_if(waitingSlavePlugins.begin(), waitingSlavePlugins.end(),
            [](MonitorControllerMaxAudioProcessor* plugin) {
                return plugin == nullptr;
            }),
        waitingSlavePlugins.end()
    );
    
    allPlugins.erase(
        std::remove_if(allPlugins.begin(), allPlugins.end(),
            [](MonitorControllerMaxAudioProcessor* plugin) {
                return plugin == nullptr;
            }),
        allPlugins.end()
    );
}

juce::String GlobalPluginState::getCurrentTimeString() const {
    auto now = juce::Time::getCurrentTime();
    return now.toString(false, true, true, true);  // 包含毫秒
}

//==============================================================================
// v4.1: 总线效果状态管理
void GlobalPluginState::setGlobalMonoState(bool monoState) {
    std::lock_guard<std::mutex> lock(stateMutex);
    globalMonoState = monoState;
    
    VST3_DBG("Global mono state set to: " + juce::String(monoState ? "ON" : "OFF"));
}

bool GlobalPluginState::getGlobalMonoState() const {
    std::lock_guard<std::mutex> lock(stateMutex);
    return globalMonoState;
}

void GlobalPluginState::broadcastMonoStateToSlaves(bool monoState) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    VST3_DBG("Broadcasting mono state to " + juce::String(slavePlugins.size()) + " slaves: " + 
             juce::String(monoState ? "ON" : "OFF"));
    
    for (auto* slave : slavePlugins) {
        if (slave) {
            try {
                slave->receiveMasterBusState("mono", monoState);
            } catch (const std::exception& e) {
                // 🚀 稳定性优化：监控总线状态广播异常
                healthMonitor.exceptionsCaught++;
                VST3_DBG("Error broadcasting mono state: " + juce::String(e.what()));
            }
        }
    }
}

//==============================================================================
// 🚀 稳定性优化第4步：健康监控系统实现

juce::String GlobalPluginState::HealthMonitor::getHealthReport() const {
    juce::String report;
    report += "=== GlobalPluginState 健康报告 ===\n";
    report += "插件注册: " + juce::String(pluginRegistrations.load()) + "\n";
    report += "插件注销: " + juce::String(pluginUnregistrations.load()) + "\n";
    report += "Master提升: " + juce::String(masterPromotions.load()) + "\n";
    report += "Slave连接: " + juce::String(slaveConnections.load()) + "\n";
    report += "状态变化: " + juce::String(stateChanges.load()) + "\n";
    report += "广播调用: " + juce::String(broadcastCalls.load()) + "\n";
    report += "异常捕获: " + juce::String(exceptionsCaught.load()) + "\n";
    report += "锁超时: " + juce::String(lockTimeouts.load()) + "\n";
    report += "无效插件清理: " + juce::String(invalidPluginCleanups.load()) + "\n";
    
    // 🚀 健康状态评估
    uint32_t totalExceptions = exceptionsCaught.load();
    uint32_t totalOperations = pluginRegistrations.load() + pluginUnregistrations.load() + 
                              broadcastCalls.load() + stateChanges.load();
    
    if (totalExceptions == 0) {
        report += "状态: ✅ 优秀 - 无异常";
    } else if (totalOperations > 0 && (totalExceptions * 100 / totalOperations) < 1) {
        report += "状态: ⚠️ 良好 - 异常率低于1%";
    } else {
        report += "状态: ❌ 需要关注 - 异常率过高";
    }
    
    return report;
}

juce::String GlobalPluginState::getHealthReport() const {
    return healthMonitor.getHealthReport();
}

void GlobalPluginState::resetHealthCounters() {
    // 🚀 重置所有健康监控计数器
    healthMonitor.pluginRegistrations = 0;
    healthMonitor.pluginUnregistrations = 0;
    healthMonitor.masterPromotions = 0;
    healthMonitor.slaveConnections = 0;
    healthMonitor.stateChanges = 0;
    healthMonitor.broadcastCalls = 0;
    healthMonitor.exceptionsCaught = 0;
    healthMonitor.lockTimeouts = 0;
    healthMonitor.invalidPluginCleanups = 0;
}