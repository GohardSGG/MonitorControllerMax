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
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (plugin == nullptr) return;
    
    auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
    if (it == allPlugins.end()) {
        allPlugins.push_back(plugin);
        
        juce::String logMsg = getCurrentTimeString() + " Plugin registered (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Total: " + juce::String(allPlugins.size());
        
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
}

void GlobalPluginState::unregisterPlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (plugin == nullptr) return;
    
    // 从所有列表中移除
    auto it = std::find(allPlugins.begin(), allPlugins.end(), plugin);
    if (it != allPlugins.end()) {
        allPlugins.erase(it);
        
        juce::String logMsg = getCurrentTimeString() + " Plugin unregistered (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Remaining: " + juce::String(allPlugins.size());
        
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
    
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
}

bool GlobalPluginState::setAsMaster(MonitorControllerMaxAudioProcessor* plugin) {
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
    
    juce::String logMsg = getCurrentTimeString() + " Master role assigned (ID: " + 
                         juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + ")";
    VST3_DBG(logMsg);
    addConnectionLog(logMsg);
    
    return true;
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
        slavePlugins.clear();
    }
}

bool GlobalPluginState::isMasterPlugin(MonitorControllerMaxAudioProcessor* plugin) const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return masterPlugin == plugin;
}

bool GlobalPluginState::addSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    if (plugin == nullptr) return false;
    
    // 检查是否有Master
    if (masterPlugin == nullptr) {
        juce::String logMsg = getCurrentTimeString() + " Slave role denied - No Master available";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        return false;
    }
    
    // 不能将Master设为Slave
    if (plugin == masterPlugin) {
        juce::String logMsg = getCurrentTimeString() + " Slave role denied - Plugin is Master";
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
        return false;
    }
    
    // 检查是否已经是Slave
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it == slavePlugins.end()) {
        slavePlugins.push_back(plugin);
        
        juce::String logMsg = getCurrentTimeString() + " Slave role assigned (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Active slaves: " + juce::String(slavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
    
    return true;
}

void GlobalPluginState::removeSlavePlugin(MonitorControllerMaxAudioProcessor* plugin) {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    auto it = std::find(slavePlugins.begin(), slavePlugins.end(), plugin);
    if (it != slavePlugins.end()) {
        slavePlugins.erase(it);
        
        juce::String logMsg = getCurrentTimeString() + " Slave role removed (ID: " + 
                             juce::String::toHexString(reinterpret_cast<juce::pointer_sized_int>(plugin)) + 
                             ") - Active slaves: " + juce::String(slavePlugins.size());
        VST3_DBG(logMsg);
        addConnectionLog(logMsg);
    }
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
    std::lock_guard<std::mutex> lock(pluginsMutex);
    
    // 清理无效插件指针
    cleanupInvalidPlugins();
    
    if (slavePlugins.empty()) return;
    
    VST3_DBG("Broadcasting " + action + " " + channelName + " = " + (state ? "true" : "false") + 
             " to " + juce::String(slavePlugins.size()) + " slaves");
    
    for (auto* slave : slavePlugins) {
        if (slave != nullptr) {
            try {
                // 直接调用Slave的状态接收方法 - 零延迟
                slave->receiveMasterState(channelName, action, state);
            } catch (const std::exception& e) {
                VST3_DBG("Error broadcasting to slave: " + juce::String(e.what()));
            }
        }
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
}

int GlobalPluginState::getSlaveCount() const {
    std::lock_guard<std::mutex> lock(pluginsMutex);
    return static_cast<int>(slavePlugins.size());
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
        info += "No Master | Plugins: " + juce::String(allPlugins.size());
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