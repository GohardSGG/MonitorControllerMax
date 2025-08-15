#include "GlobalPluginState.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

// 静态成员初始化
std::unique_ptr<GlobalPluginState> GlobalPluginState::instance = nullptr;
std::mutex GlobalPluginState::instanceMutex;
std::atomic<bool> GlobalPluginState::shuttingDown{false}; // 🛡️ 关闭状态标志

GlobalPluginState& GlobalPluginState::getInstance() {
    std::lock_guard<std::mutex> lock(instanceMutex);
    
    // 🛡️ 关闭检查：防止在程序退出时创建新实例
    if (shuttingDown.load()) {
        static GlobalPluginState dummyInstance; // 安全的哑对象
        return dummyInstance;
    }
    
    if (!instance) {
        instance = std::unique_ptr<GlobalPluginState>(new GlobalPluginState());
    }
    return *instance;
}

// 🛡️ 显式关闭机制
void GlobalPluginState::shutdown() {
    std::lock_guard<std::mutex> lock(instanceMutex);
    
    shuttingDown.store(true);
    
    if (instance) {
        // 清理所有插件引用
        {
            std::lock_guard<std::mutex> pluginsLock(instance->pluginsMutex);
            instance->allPlugins.clear();
            instance->slavePlugins.clear();
            instance->waitingSlavePlugins.clear();
            instance->masterPlugin = nullptr;
            
            // 🚀 清理生命周期系统
            instance->pluginIds.clear();
            instance->idToPlugin.clear();
            instance->validPluginIds.clear();
            instance->invalidatedPlugins.clear();
        }
        
        // 清理状态数据
        {
            std::lock_guard<std::mutex> stateLock(instance->stateMutex);
            instance->globalSoloStates.clear();
            instance->globalMuteStates.clear();
        }
        
        // 清理日志
        {
            std::lock_guard<std::mutex> logsLock(instance->logsMutex);
            instance->connectionLogs.clear();
        }
        
        instance.reset();
    }
}

bool GlobalPluginState::isShuttingDown() {
    return shuttingDown.load();
}

//==============================================================================
// 🚀 生命周期安全：增强的插件管理实现

juce::String GlobalPluginState::generateUniquePluginId(MonitorControllerMaxAudioProcessor* plugin) {
    if (!plugin) return juce::String();
    
    // 生成基于时间戳和内存地址的唯一ID
    auto timestamp = juce::Time::getCurrentTime().toMilliseconds();
    auto address = reinterpret_cast<juce::pointer_sized_int>(plugin);
    return juce::String("Plugin_") + juce::String(timestamp) + "_" + juce::String::toHexString(address);
}

bool GlobalPluginState::isPluginValid(MonitorControllerMaxAudioProcessor* plugin) const {
    if (!plugin) return false;
    
    // 检查插件是否在有效ID集合中
    auto it = pluginIds.find(plugin);
    if (it != pluginIds.end()) {
        return validPluginIds.count(it->second) > 0;
    }
    
    return false;
}

bool GlobalPluginState::isPluginSafeToAccess(MonitorControllerMaxAudioProcessor* plugin) const {
    if (!plugin) return false;
    
    // 双重安全检查：1) 不在已失效列表中 2) 在有效ID集合中
    if (invalidatedPlugins.count(plugin) > 0) {
        return false; // 已明确标记为失效
    }
    
    return isPluginValid(plugin);
}

void GlobalPluginState::invalidatePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    if (!plugin) return;
    
    // 双重标记：1) 从有效ID集合移除 2) 加入失效集合
    auto it = pluginIds.find(plugin);
    if (it != pluginIds.end()) {
        validPluginIds.erase(it->second);
        invalidatedPlugins.insert(plugin);
        VST3_DBG("Plugin invalidated: " + it->second);
    }
}

void GlobalPluginState::removeFromAllLists(MonitorControllerMaxAudioProcessor* plugin) {
    if (!plugin) return;
    
    // 从所有列表中安全移除插件
    auto removeFromVector = [plugin](std::vector<MonitorControllerMaxAudioProcessor*>& vec) {
        vec.erase(std::remove(vec.begin(), vec.end(), plugin), vec.end());
    };
    
    removeFromVector(allPlugins);
    removeFromVector(slavePlugins);
    removeFromVector(waitingSlavePlugins);
    
    if (masterPlugin == plugin) {
        masterPlugin = nullptr;
    }
}

void GlobalPluginState::notifySlavePluginsAboutMasterLoss() {
    // 安全通知所有Slave插件Master已丢失
    std::vector<MonitorControllerMaxAudioProcessor*> validSlaves;
    
    // 收集所有有效的Slave插件
    for (auto* slave : slavePlugins) {
        if (isPluginSafeToAccess(slave)) {
            validSlaves.push_back(slave);
        }
    }
    
    // 通知所有有效的Slave
    for (auto* slave : validSlaves) {
        try {
            juce::MessageManager::callAsync([slave]() {
                if (slave) { // 再次检查空指针
                    slave->onMasterDisconnected();
                }
            });
        } catch (...) {
            healthMonitor.exceptionsCaught++;
            VST3_DBG("Exception notifying slave about master loss");
        }
    }
}

void GlobalPluginState::performSafeCleanup() {
    // 安全的插件清理（双重验证）
    cleanupCounter++;
    
    // 清理已失效的插件
    auto cleanupVector = [this](std::vector<MonitorControllerMaxAudioProcessor*>& vec, const char* listName) {
        size_t originalSize = vec.size();
        vec.erase(
            std::remove_if(vec.begin(), vec.end(),
                [this](MonitorControllerMaxAudioProcessor* plugin) {
                    return !isPluginSafeToAccess(plugin);
                }),
            vec.end()
        );
        
        if (vec.size() != originalSize) {
            VST3_DBG(juce::String("Cleaned ") + juce::String(originalSize - vec.size()) + 
                     " invalid plugins from " + listName);
            healthMonitor.invalidPluginCleanups++;
        }
    };
    
    cleanupVector(allPlugins, "allPlugins");
    cleanupVector(slavePlugins, "slavePlugins");
    cleanupVector(waitingSlavePlugins, "waitingSlavePlugins");
    
    // 检查Master插件
    if (masterPlugin && !isPluginSafeToAccess(masterPlugin)) {
        VST3_DBG("Master plugin became invalid, clearing");
        masterPlugin = nullptr;
        notifySlavePluginsAboutMasterLoss();
    }
}

void GlobalPluginState::registerPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    // 🛡️ 关闭检查：防止在程序退出时操作
    if (shuttingDown.load()) return;
    
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        if (plugin == nullptr) return;
        
        // 🚀 增强安全检查：检查是否已注册或已失效
        if (isPluginValid(plugin)) {
            VST3_DBG("Plugin already registered");
            return;
        }
        
        // 从失效集合中移除（如果存在）
        invalidatedPlugins.erase(plugin);
        
        // 生成唯一ID并注册
        juce::String pluginId = generateUniquePluginId(plugin);
        
        // 添加到所有跟踪系统
        allPlugins.push_back(plugin);
        pluginIds[plugin] = pluginId;
        idToPlugin[pluginId] = plugin;
        validPluginIds.insert(pluginId);
        
        // 定期清理检查（每10个插件注册后）
        if (healthMonitor.pluginRegistrations.load() % 10 == 0) {
            performSafeCleanup();
        }
        
        // Stability optimization: counter monitoring
        healthMonitor.pluginRegistrations++;
        
        juce::String logMsg = getCurrentTimeString() + " Plugin registered (ID: " + pluginId + 
                             ", Ptr: " + juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Total: " + juce::String(allPlugins.size());
        
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 已在上面实现定期清理
    }
    catch (...) {
        // Stability optimization: exception handling
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in registerPlugin - continuing safely");
    }
}

void GlobalPluginState::unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    // 🛡️ 关闭检查：允许在关闭时注销插件
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        if (plugin == nullptr) return;
        
        // 🚀 增强安全：立即标记为失效，防止并发访问
        invalidatePlugin(plugin);
        
        // 获取插件ID
        juce::String pluginId;
        auto idIt = pluginIds.find(plugin);
        if (idIt != pluginIds.end()) {
            pluginId = idIt->second;
        }
        
        // 🛡️ 安全移除：使用增强的移除方法
        removeFromAllLists(plugin);
        
        // 清理跟踪系统
        if (!pluginId.isEmpty()) {
            pluginIds.erase(plugin);
            idToPlugin.erase(pluginId);
            validPluginIds.erase(pluginId);
        }
        invalidatedPlugins.insert(plugin); // 确保在失效集合中
        
        // 🚀 增强的Master插件处理
        if (masterPlugin == plugin) {
            VST3_DBG("Master plugin being unregistered (ID: " + pluginId + ")");
            
            // 立即清空Master引用
            masterPlugin = nullptr;
            
            // 安全通知所有Slave
            notifySlavePluginsAboutMasterLoss();
            
            juce::String masterLogMsg = getCurrentTimeString() + " Master plugin unregistered (ID: " + pluginId + ") - Master role available";
            VST3_DBG(masterLogMsg);
            addConnectionLog(masterLogMsg);
            
            // 将所有有效的Slave移到等待列表
            for (auto* slave : slavePlugins) {
                if (isPluginSafeToAccess(slave)) {
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
        
        // 记录成功注销
        healthMonitor.pluginUnregistrations++;
        
        juce::String logMsg = getCurrentTimeString() + " Plugin unregistered (ID: " + pluginId + 
                             ") - Remaining: " + juce::String(allPlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
    }
    catch (...) {
        // Stability optimization: exception handling
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in unregisterPlugin - continuing safely");
    }
}

bool GlobalPluginState::setAsMaster(MonitorControllerMaxAudioProcessor* plugin) {
    try {
        std::lock_guard<std::mutex> lock(pluginsMutex);
        
        // 🚀 增强安全检查
        if (plugin == nullptr || !isPluginSafeToAccess(plugin)) {
            VST3_DBG("Cannot set invalid or unsafe plugin as Master");
            return false;
        }
        
        // 检查是否已经有Master
        if (masterPlugin != nullptr && masterPlugin != plugin) {
            if (isPluginSafeToAccess(masterPlugin)) {
                juce::String logMsg = getCurrentTimeString() + " Master role denied - Master already exists";
                VST3_DBG(logMsg);
                addConnectionLog(logMsg);
                return false;
            } else {
                // 现有Master已无效，清理并通知Slave
                VST3_DBG("Existing Master plugin is invalid, clearing");
                masterPlugin = nullptr;
                notifySlavePluginsAboutMasterLoss();
            }
        }
        
        // 从Slave列表移除（如果存在）
        auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
        if (it != slavePlugins.end()) {
            slavePlugins.erase(it);
        }
        
        masterPlugin = plugin;
        
        // Stability optimization: counter monitoring
        healthMonitor.masterPromotions++;
        
        juce::String pluginId = pluginIds[plugin];
        juce::String logMsg = getCurrentTimeString() + " Master role assigned (ID: " + pluginId + ")";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 将等待中的Slave提升为活跃Slave
        promoteWaitingSlavesToActive();
        
        return true;
    }
    catch (...) {
        // Stability optimization: exception handling
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in setAsMaster - returning false safely");
        return false;
    }
}

void GlobalPluginState::removeMaster(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (masterPlugin == plugin) {
        juce::String pluginId;
        auto idIt = pluginIds.find(plugin);
        if (idIt != pluginIds.end()) {
            pluginId = idIt->second;
        }
        
        masterPlugin = nullptr;
        
        juce::String logMsg = getCurrentTimeString() + " Master role removed (ID: " + pluginId + ") - Role available";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        
        // 通知所有有效的Slave自动切换到Standalone
        std::vector<MonitorControllerMaxAudioProcessor*> validSlaves;
        for (auto* slave : slavePlugins) {
            if (slave != nullptr && isPluginValid(slave)) {
                validSlaves.push_back(slave);
            }
        }
        
        for (auto* slave : validSlaves) {
            juce::MessageManager::callAsync([slave]() {
                slave->onMasterDisconnected();
            });
            waitingSlavePlugins.push_back(slave);
        }
        
        slavePlugins.clear();
        
        if (!validSlaves.empty()) {
            juce::String waitingLogMsg = getCurrentTimeString() + " Moved " + juce::String(validSlaves.size()) + 
                                        " slaves to waiting list - waiting for new Master";
            VST3_DBG(waitingLogMsg);
            addConnectionLog(waitingLogMsg);
        }
    }
}

bool GlobalPluginState::isMasterPlugin(MonitorControllerMaxAudioProcessor* plugin) const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return masterPlugin == plugin && isPluginValid(plugin);
}

bool GlobalPluginState::addSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (plugin == nullptr || !isPluginValid(plugin)) {
        VST3_DBG("Cannot add invalid plugin as Slave");
        return false;
    }
    
    // 不能将Master设为Slave
    if (plugin == masterPlugin) {
        juce::String logMsg = getCurrentTimeString() + " Slave role denied - Plugin is Master";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        return false;
    }
    
    // 检查是否有有效的Master
    if (masterPlugin == nullptr || !isPluginValid(masterPlugin)) {
        // 没有有效Master，将Slave加入等待列表
        addWaitingSlavePlugin(plugin);
        return true;
    }
    
    // 有Master，直接加入活跃Slave列表
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it == slavePlugins.end()) {
        slavePlugins.push_back(plugin);
        
        juce::String pluginId = pluginIds[plugin];
        juce::String logMsg = getCurrentTimeString() + " Slave role assigned (ID: " + pluginId + 
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
    
    if (plugin == nullptr) return;
    
    juce::String pluginId;
    auto idIt = pluginIds.find(plugin);
    if (idIt != pluginIds.end()) {
        pluginId = idIt->second;
    }
    
    // 从活跃Slave列表移除
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it != slavePlugins.end()) {
        slavePlugins.erase(it);
        
        juce::String logMsg = getCurrentTimeString() + " Slave role removed (ID: " + pluginId + 
                             ") - Active slaves: " + juce::String(slavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
    
    // 从等待Slave列表移除
    removeWaitingSlavePlugin(plugin);
}

std::vector<MonitorControllerMaxAudioProcessor*> GlobalPluginState::getSlavePlugins() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    std::vector<MonitorControllerMaxAudioProcessor*> validSlaves;
    
    for (auto* slave : slavePlugins) {
        if (slave != nullptr && isPluginValid(slave)) {
            validSlaves.push_back(slave);
        }
    }
    
    return validSlaves;
}

//==============================================================================
// 🚀 生命周期安全：清理机制实现

void GlobalPluginState::cleanupInvalidPlugins() {
    // 此方法假定已持有pluginsMutex锁
    
    // 🚀 增强的清理逻辑：使用双重安全检查
    auto cleanupVector = [this](std::vector<MonitorControllerMaxAudioProcessor*>& container, const juce::String& name) {
        auto oldSize = container.size();
        container.erase(
            std::remove_if(container.begin(), container.end(),
                [this](MonitorControllerMaxAudioProcessor* plugin) {
                    return plugin == nullptr || !isPluginSafeToAccess(plugin);
                }),
            container.end()
        );
        
        auto cleanedCount = oldSize - container.size();
        if (cleanedCount > 0) {
            VST3_DBG("Cleaned " + juce::String(cleanedCount) + " invalid plugins from " + name);
        }
        return cleanedCount;
    };
    
    // 清理各个容器中的无效插件
    auto allCleaned = cleanupVector(allPlugins, "allPlugins");
    auto slavesCleaned = cleanupVector(slavePlugins, "slavePlugins");
    auto waitingCleaned = cleanupVector(waitingSlavePlugins, "waitingSlavePlugins");
    
    // 🛡️ 增强的Master检查
    if (masterPlugin != nullptr && !isPluginSafeToAccess(masterPlugin)) {
        VST3_DBG("Master plugin is unsafe, removing and notifying slaves");
        masterPlugin = nullptr;
        notifySlavePluginsAboutMasterLoss(); // 通知Slave
        allCleaned++;
    }
    
    // 清理已失效插件集合中的旧记录
    size_t invalidatedSizeBefore = invalidatedPlugins.size();
    if (invalidatedSizeBefore > 100) { // 防止集合过大
        invalidatedPlugins.clear();
        VST3_DBG("Cleared oversized invalidated plugins set (" + juce::String(invalidatedSizeBefore) + " entries)");
    }
    
    // 更新健康监控
    auto totalCleaned = allCleaned + slavesCleaned + waitingCleaned;
    if (totalCleaned > 0) {
        healthMonitor.invalidPluginCleanups += totalCleaned;
        VST3_DBG("Total cleaned invalid plugins: " + juce::String(totalCleaned));
    }
}

void GlobalPluginState::performHealthyCleanup() {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    VST3_DBG("Performing healthy cleanup...");
    
    // 清理无效插件
    cleanupInvalidPlugins();
    
    // 清理孤立的ID映射
    std::vector<juce::String> orphanedIds;
    for (const auto& [id, plugin] : idToPlugin) {
        if (plugin == nullptr || !isPluginValid(plugin)) {
            orphanedIds.push_back(id);
        }
    }
    
    for (const auto& id : orphanedIds) {
        auto plugin = idToPlugin[id];
        if (plugin) {
            pluginIds.erase(plugin);
        }
        idToPlugin.erase(id);
        validPluginIds.erase(id);
    }
    
    if (!orphanedIds.empty()) {
        VST3_DBG("Cleaned " + juce::String(orphanedIds.size()) + " orphaned ID mappings");
    }
    
    VST3_DBG("Healthy cleanup completed. Valid plugins: " + juce::String(validPluginIds.size()));
}

//==============================================================================
// 状态同步机制（保持不变，但增加有效性检查）

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
        
        // 🚀 增强安全：只向安全的Slave插件广播
        std::vector<MonitorControllerMaxAudioProcessor*> safeSlaves;
        for (auto* slave : slavePlugins) {
            if (isPluginSafeToAccess(slave)) {
                safeSlaves.push_back(slave);
            } else {
                VST3_DBG("Skipping unsafe slave plugin during broadcast");
            }
        }
        
        if (safeSlaves.empty()) {
            VST3_DBG("No safe slave plugins available for broadcast");
            return;
        }
        
        // Stability optimization: counter monitoring
        healthMonitor.broadcastCalls++;
        
        VST3_DBG("Broadcasting " + action + " " + channelName + " = " + (state ? "true" : "false") + 
                 " to " + juce::String(safeSlaves.size()) + " safe slaves");
        
        // 🛡️ 安全广播：对每个插件再次检查
        for (auto* slave : safeSlaves) {
            if (!isPluginSafeToAccess(slave)) {
                VST3_DBG("Slave plugin became unsafe during broadcast, skipping");
                continue;
            }
            
            try {
                // Direct call to Slave's state receiver - zero latency
                slave->receiveMasterState(channelName, action, state);
            } catch (const std::exception& e) {
                // Stability optimization: record individual Slave communication exceptions
                healthMonitor.exceptionsCaught++;
                VST3_DBG("Error broadcasting to slave: " + juce::String(e.what()));
                
                // 立即标记有问题的插件
                invalidatePlugin(slave);
            } catch (...) {
                // Stability optimization: catch all exception types
                healthMonitor.exceptionsCaught++;
                VST3_DBG("Unknown error broadcasting to slave");
                
                // 立即标记有问题的插件
                invalidatePlugin(slave);
            }
        }
        
        // 定期清理无效插件
        if (healthMonitor.broadcastCalls % 50 == 0) {
            cleanupInvalidPlugins();
        }
    }
    catch (...) {
        // Stability optimization: overall exception handling
        healthMonitor.exceptionsCaught++;
        VST3_DBG("Exception caught in broadcastStateToSlaves - continuing safely");
    }
}

void GlobalPluginState::syncAllStatesToSlave(MonitorControllerMaxAudioProcessor* slavePlugin) {
    if (slavePlugin == nullptr || !isPluginValid(slavePlugin)) return;
    if (masterPlugin == nullptr || !isPluginValid(masterPlugin)) return;
    
    std::lock_guard<std::mutex> stateLock(stateMutex);
    
    VST3_DBG("Syncing all Master states to new Slave");
    
    // 同步所有Solo状态
    for (const auto& [channelName, state] : globalSoloStates) {
        try {
            slavePlugin->receiveMasterState(channelName, "solo", state);
        } catch (const std::exception& e) {
            VST3_DBG("Error syncing solo state: " + juce::String(e.what()));
            invalidatePlugin(slavePlugin);
            return;
        }
    }
    
    // 同步所有Mute状态
    for (const auto& [channelName, state] : globalMuteStates) {
        try {
            slavePlugin->receiveMasterState(channelName, "mute", state);
        } catch (const std::exception& e) {
            VST3_DBG("Error syncing mute state: " + juce::String(e.what()));
            invalidatePlugin(slavePlugin);
            return;
        }
    }
    
    // v4.1: 同步总线效果状态
    try {
        slavePlugin->receiveMasterBusState("mono", globalMonoState);
    } catch (const std::exception& e) {
        VST3_DBG("Error syncing mono state: " + juce::String(e.what()));
        invalidatePlugin(slavePlugin);
    }
}

//==============================================================================
// 其他现有方法的增强实现

int GlobalPluginState::getSlaveCount() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    int validCount = 0;
    for (auto* slave : slavePlugins) {
        if (slave != nullptr && isPluginValid(slave)) {
            validCount++;
        }
    }
    return validCount;
}

int GlobalPluginState::getWaitingSlaveCount() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    int validCount = 0;
    for (auto* slave : waitingSlavePlugins) {
        if (slave != nullptr && isPluginValid(slave)) {
            validCount++;
        }
    }
    return validCount;
}

void GlobalPluginState::addWaitingSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    // 此方法在已持有锁的情况下调用
    if (plugin == nullptr || !isPluginValid(plugin)) return;
    
    // 从活跃Slave列表移除（如果存在）
    auto activeIt = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (activeIt != slavePlugins.end()) {
        slavePlugins.erase(activeIt);
    }
    
    // 检查是否已在等待列表
    auto waitingIt = std::find(waitingSlavePlugins.begin(), waitingSlavePlugins.end(), plugin);
    if (waitingIt == waitingSlavePlugins.end()) {
        waitingSlavePlugins.push_back(plugin);
        
        juce::String pluginId = pluginIds[plugin];
        juce::String logMsg = getCurrentTimeString() + " Slave added to waiting list (ID: " + pluginId + 
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
        
        juce::String pluginId;
        auto idIt = pluginIds.find(plugin);
        if (idIt != pluginIds.end()) {
            pluginId = idIt->second;
        }
        
        juce::String logMsg = getCurrentTimeString() + " Slave removed from waiting list (ID: " + pluginId + 
                             ") - Waiting slaves: " + juce::String(waitingSlavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
}

void GlobalPluginState::promoteWaitingSlavesToActive() {
    // 此方法在已持有锁的情况下调用
    if (waitingSlavePlugins.empty()) return;
    
    std::vector<MonitorControllerMaxAudioProcessor*> validWaitingSlaves;
    
    // 收集有效的等待Slave
    for (auto* slave : waitingSlavePlugins) {
        if (slave != nullptr && isPluginValid(slave)) {
            validWaitingSlaves.push_back(slave);
        }
    }
    
    if (validWaitingSlaves.empty()) {
        waitingSlavePlugins.clear();
        return;
    }
    
    juce::String logMsg = getCurrentTimeString() + " Promoting " + juce::String(validWaitingSlaves.size()) + 
                         " waiting slaves to active - Master is now available";
    VST3_DBG(logMsg);
    addConnectionLog(logMsg);
    
    // 将所有等待中的Slave提升为活跃Slave
    for (auto* waitingSlave : validWaitingSlaves) {
        slavePlugins.push_back(waitingSlave);
        
        // 同步Master状态到新连接的Slave
        syncAllStatesToSlave(waitingSlave);
        
        // 通知Slave现在已连接到Master
        juce::MessageManager::callAsync([waitingSlave]() {
            waitingSlave->onMasterConnected();
        });
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
    return masterPlugin != nullptr && isPluginValid(masterPlugin);
}

juce::String GlobalPluginState::getConnectionInfo() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    juce::String info;
    
    if (masterPlugin != nullptr && isPluginValid(masterPlugin)) {
        info += "Master: Active";
        auto validSlaveCount = getSlaveCount();
        if (validSlaveCount > 0) {
            info += " | Slaves: " + juce::String(validSlaveCount);
        } else {
            info += " | No Slaves";
        }
    } else {
        info += "No Master";
        auto validWaitingCount = getWaitingSlaveCount();
        if (validWaitingCount > 0) {
            info += " | Waiting Slaves: " + juce::String(validWaitingCount);
        }
        info += " | Valid Plugins: " + juce::String(validPluginIds.size());
    }
    
    return info;
}

MonitorControllerMaxAudioProcessor* GlobalPluginState::getMasterPlugin() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    if (masterPlugin != nullptr && isPluginValid(masterPlugin)) {
        return masterPlugin;
    }
    return nullptr;
}

//==============================================================================
// 其他现有方法...（日志管理、v4.1总线效果、健康监控等保持不变）

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

juce::String GlobalPluginState::getCurrentTimeString() const {
    auto now = juce::Time::getCurrentTime();
    return now.toString(false, true, true, true);  // 包含毫秒
}

//==============================================================================
// v4.1: 总线效果状态管理（保持不变）
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
    
    auto validSlaves = getSlavePlugins();
    
    VST3_DBG("Broadcasting mono state to " + juce::String(validSlaves.size()) + " slaves: " + 
             juce::String(monoState ? "ON" : "OFF"));
    
    for (auto* slave : validSlaves) {
        try {
            slave->receiveMasterBusState("mono", monoState);
        } catch (const std::exception& e) {
            // Stability optimization: monitor bus state broadcast exceptions
            healthMonitor.exceptionsCaught++;
            VST3_DBG("Error broadcasting mono state: " + juce::String(e.what()));
            invalidatePlugin(slave);
        }
    }
}

//==============================================================================
// Stability Optimization Step 4: Health Monitoring System Implementation（保持不变）

juce::String GlobalPluginState::HealthMonitor::getHealthReport() const {
    juce::String report;
    report += "=== GlobalPluginState Health Report ===\n";
    report += "Plugin Registrations: " + juce::String(pluginRegistrations.load()) + "\n";
    report += "Plugin Unregistrations: " + juce::String(pluginUnregistrations.load()) + "\n";
    report += "Master Promotions: " + juce::String(masterPromotions.load()) + "\n";
    report += "Slave Connections: " + juce::String(slaveConnections.load()) + "\n";
    report += "State Changes: " + juce::String(stateChanges.load()) + "\n";
    report += "Broadcast Calls: " + juce::String(broadcastCalls.load()) + "\n";
    report += "Exceptions Caught: " + juce::String(exceptionsCaught.load()) + "\n";
    report += "Lock Timeouts: " + juce::String(lockTimeouts.load()) + "\n";
    report += "Invalid Plugin Cleanups: " + juce::String(invalidPluginCleanups.load()) + "\n";
    
    // Health status assessment
    uint32_t totalExceptions = exceptionsCaught.load();
    uint32_t totalOperations = pluginRegistrations.load() + pluginUnregistrations.load() + 
                              broadcastCalls.load() + stateChanges.load();
    
    if (totalExceptions == 0) {
        report += "Status: EXCELLENT - No exceptions";
    } else if (totalOperations > 0 && (totalExceptions * 100 / totalOperations) < 1) {
        report += "Status: GOOD - Exception rate < 1%";
    } else {
        report += "Status: NEEDS ATTENTION - High exception rate";
    }
    
    return report;
}

juce::String GlobalPluginState::getHealthReport() const {
    return healthMonitor.getHealthReport();
}

void GlobalPluginState::resetHealthCounters() {
    // Reset all health monitoring counters
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