#include "OSCCommunicator.h"
#include "SemanticChannelState.h"
#include "PhysicalChannelMapper.h"
#include "PluginProcessor.h"
#include "DebugLogger.h"

// OSC类专用角色日志宏
#define OSC_DBG_ROLE(message) \
    do { \
        if (processorPtr) { \
            VST3_DBG_ROLE(processorPtr, message); \
        } else { \
            VST3_DBG("[OSC] " + juce::String(message)); \
        } \
    } while(0)

OSCCommunicator::OSCCommunicator()
{
    OSC_DBG_ROLE("OSCCommunicator: Initialize OSC communication system");
    
    // 创建OSC发送和接收组件
    sender = std::make_unique<juce::OSCSender>();
    receiver = std::make_unique<juce::OSCReceiver>();
}

void OSCCommunicator::setProcessor(MonitorControllerMaxAudioProcessor* processor)
{
    processorPtr = processor;
}

OSCCommunicator::~OSCCommunicator()
{
    OSC_DBG_ROLE("OSCCommunicator: Shutdown OSC communication system");
    shutdown();
}

bool OSCCommunicator::initialize()
{
    OSC_DBG_ROLE("OSCCommunicator: Initialize OSC connections");
    
    bool success = true;
    
    // 初始化OSC发送器
    if (sender->connect(TARGET_IP, TARGET_PORT))
    {
        senderConnected.store(true);
        OSC_DBG_ROLE("OSCCommunicator: OSC Sender connected to " + juce::String(TARGET_IP) + ":" + juce::String(TARGET_PORT));
    }
    else
    {
        senderConnected.store(false);
        OSC_DBG_ROLE("OSCCommunicator: Failed to connect OSC Sender to " + juce::String(TARGET_IP) + ":" + juce::String(TARGET_PORT));
        success = false;
    }
    
    // 初始化OSC接收器
    if (receiver->connect(RECEIVE_PORT))
    {
        receiver->addListener(this);
        receiverConnected.store(true);
        OSC_DBG_ROLE("OSCCommunicator: OSC Receiver listening on port " + juce::String(RECEIVE_PORT));
    }
    else
    {
        receiverConnected.store(false);
        OSC_DBG_ROLE("OSCCommunicator: Failed to start OSC Receiver on port " + juce::String(RECEIVE_PORT));
        success = false;
    }
    
    isInitialized.store(success);
    
    if (success)
    {
        OSC_DBG_ROLE("OSCCommunicator: OSC communication system initialized successfully");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: OSC communication system initialization failed");
    }
    
    return success;
}

void OSCCommunicator::shutdown()
{
    OSC_DBG_ROLE("OSCCommunicator: Shutdown OSC communication");
    
    if (receiver && receiverConnected.load())
    {
        receiver->removeListener(this);
        receiver->disconnect();
        receiverConnected.store(false);
    }
    
    if (sender && senderConnected.load())
    {
        sender->disconnect();
        senderConnected.store(false);
    }
    
    isInitialized.store(false);
    OSC_DBG_ROLE("OSCCommunicator: OSC communication shutdown complete");
}

bool OSCCommunicator::isConnected() const
{
    return isInitialized.load() && senderConnected.load();
}

void OSCCommunicator::sendSoloState(const juce::String& channelName, bool state)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    juce::String address = formatOSCAddress("Solo", channelName);
    float value = state ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Solo state - " + address + " = " + juce::String(value));
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Solo state - " + address);
    }
}

void OSCCommunicator::sendMuteState(const juce::String& channelName, bool state)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    juce::String address = formatOSCAddress("Mute", channelName);
    float value = state ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Mute state - " + address + " = " + juce::String(value));
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Mute state - " + address);
    }
}

void OSCCommunicator::sendMasterVolume(float volumePercent)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    // v4.1: 发送Master Volume状态 (地址: /Monitor/Master/Volume)
    juce::String address = "/Monitor/Master/Volume";

    // 关键修改：将内部的 0-100 百分比转换为OSC标准的 0.0-1.0 范围
    float oscValue = volumePercent / 100.0f;
    oscValue = juce::jlimit(0.0f, 1.0f, oscValue); // 确保值在0.0和1.0之间
    
    if (sender->send(address, oscValue))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Master Volume - " + address + " = " + juce::String(oscValue) + " (from " + juce::String(volumePercent) + "%)");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Master Volume - " + address);
    }
}

void OSCCommunicator::sendMasterDim(bool dimState)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    // v4.1: 发送Master Dim状态 (地址: /Monitor/Master/Dim)
    juce::String address = "/Monitor/Master/Dim";
    float value = dimState ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Master Dim - " + address + " = " + juce::String(value) + " (" + (dimState ? "ON" : "OFF") + ")");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Master Dim - " + address);
    }
}

void OSCCommunicator::sendMasterLowBoost(bool lowBoostState)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    // v4.1: 发送Master Low Boost状态 (地址: /Monitor/Master/Effect/Low_Boost)
    juce::String address = "/Monitor/Master/Effect/Low_Boost";
    float value = lowBoostState ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Master Low Boost - " + address + " = " + juce::String(value) + " (" + (lowBoostState ? "ON" : "OFF") + ")");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Master Low Boost - " + address);
    }
}

void OSCCommunicator::sendMasterMute(bool masterMuteState)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    // v4.1: 发送Master Mute状态 (地址: /Monitor/Master/Mute)
    juce::String address = "/Monitor/Master/Mute";
    float value = masterMuteState ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Master Mute - " + address + " = " + juce::String(value) + " (" + (masterMuteState ? "ON" : "OFF") + ")");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Master Mute - " + address);
    }
}

void OSCCommunicator::sendMasterMono(bool monoState)
{
    // 检查连接状态
    if (!isConnected())
    {
        return;
    }
    
    // v4.1: 发送Master Mono状态 (地址: /Monitor/Master/Effect/Mono)
    juce::String address = "/Monitor/Master/Effect/Mono";
    float value = monoState ? 1.0f : 0.0f;
    
    if (sender->send(address, value))
    {
        OSC_DBG_ROLE("OSCCommunicator: Sent Master Mono - " + address + " = " + juce::String(value) + " (" + (monoState ? "ON" : "OFF") + ")");
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Failed to send Master Mono - " + address);
    }
}

void OSCCommunicator::broadcastAllStates(const SemanticChannelState& semanticState, 
                                        const PhysicalChannelMapper& physicalMapper)
{
    if (!isConnected())
    {
        OSC_DBG_ROLE("OSCCommunicator: Cannot broadcast - not connected");
        return;
    }
    
    OSC_DBG_ROLE("OSCCommunicator: Broadcasting all current states");
    
    // 获取当前活跃的语义通道
    auto activeChannels = physicalMapper.getActiveSemanticChannels();
    
    for (const auto& channelName : activeChannels)
    {
        // 发送Solo状态
        bool soloState = semanticState.getSoloState(channelName);
        sendSoloState(channelName, soloState);
        
        // 发送Mute状态 (使用基本Mute状态，不是最终状态)
        bool muteState = semanticState.getMuteState(channelName);
        sendMuteState(channelName, muteState);
    }
    
    OSC_DBG_ROLE("OSCCommunicator: Broadcast complete - " + juce::String(activeChannels.size()) + " channels");
}

void OSCCommunicator::oscMessageReceived(const juce::OSCMessage& message)
{
    handleIncomingOSCMessage(message);
}

void OSCCommunicator::handleIncomingOSCMessage(const juce::OSCMessage& message)
{
    juce::String address = message.getAddressPattern().toString();
    
    // v4.1: 处理Master总线消息
    if (address == "/Monitor/Master/Dim" || address == "/Monitor/Master/Volume" || address == "/Monitor/Master/Effect/Low_Boost" || address == "/Monitor/Master/Mute" || address == "/Monitor/Master/Effect/Mono")
    {
        handleMasterBusOSCMessage(address, message);
        return;
    }
    
    // 常规通道消息处理
    // 解析OSC地址
    auto [action, channelName] = parseOSCAddress(address);
    
    if (action.isEmpty() || channelName.isEmpty())
    {
        OSC_DBG_ROLE("OSCCommunicator: Invalid OSC address format - " + address);
        return;
    }
    
    if (!isValidChannelName(channelName))
    {
        OSC_DBG_ROLE("OSCCommunicator: Invalid channel name - " + channelName);
        return;
    }
    
    // 获取值
    if (message.size() < 1)
    {
        OSC_DBG_ROLE("OSCCommunicator: OSC message has no arguments");
        return;
    }
    
    float value = 0.0f;
    if (message[0].isFloat32())
    {
        value = message[0].getFloat32();
    }
    else if (message[0].isInt32())
    {
        value = static_cast<float>(message[0].getInt32());
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: OSC message argument is not numeric");
        return;
    }
    
    bool state = (value > 0.5f);
    
    OSC_DBG_ROLE("OSCCommunicator: Parsed OSC - action:" + action + " channel:" + channelName + " state:" + (state ? "ON" : "OFF"));
    
    // 调用处理函数来更新对应的状态
    if (onExternalStateChange)
    {
        // 传递action类型、通道名和状态值
        onExternalStateChange(action, channelName, state);
    }
}

void OSCCommunicator::handleMasterBusOSCMessage(const juce::String& address, const juce::OSCMessage& message)
{
    OSC_DBG_ROLE("OSCCommunicator: Handling Master bus OSC message - " + address);
    
    // 获取值
    if (message.size() < 1)
    {
        OSC_DBG_ROLE("OSCCommunicator: Master OSC message has no arguments");
        return;
    }
    
    float value = 0.0f;
    if (message[0].isFloat32())
    {
        value = message[0].getFloat32();
    }
    else if (message[0].isInt32())
    {
        value = static_cast<float>(message[0].getInt32());
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Master OSC message argument is not numeric");
        return;
    }
    
    // v4.1: 处理Master Volume消息 (/Monitor/Master/Volume)
    if (address == "/Monitor/Master/Volume")
    {
        // 关键修改：将OSC的 0.0-1.0 范围转换为内部使用的 0-100 百分比范围
        float volumePercent = value * 100.0f;
        
        // 限制范围到0-100%
        volumePercent = juce::jlimit(0.0f, 100.0f, volumePercent);
        
        OSC_DBG_ROLE("OSCCommunicator: Received Master Volume OSC - value: " + juce::String(value) + " -> " + juce::String(volumePercent) + "%");
        
        if (onMasterVolumeOSC)
        {
            onMasterVolumeOSC(volumePercent);
        }
    }
    // v4.1: 处理Master Dim消息 (/Monitor/Master/Dim)
    else if (address == "/Monitor/Master/Dim")
    {
        bool dimState = (value > 0.5f);
        
        OSC_DBG_ROLE("OSCCommunicator: Received Master Dim OSC - " + juce::String(dimState ? "ON" : "OFF"));
        
        if (onMasterDimOSC)
        {
            onMasterDimOSC(dimState);
        }
    }
    // v4.1: 处理Master Low Boost消息 (/Monitor/Master/Effect/Low_Boost)
    else if (address == "/Monitor/Master/Effect/Low_Boost")
    {
        bool lowBoostState = (value > 0.5f);
        
        OSC_DBG_ROLE("OSCCommunicator: Received Master Low Boost OSC - " + juce::String(lowBoostState ? "ON" : "OFF"));
        
        if (onMasterLowBoostOSC)
        {
            onMasterLowBoostOSC(lowBoostState);
        }
    }
    // v4.1: 处理Master Mute消息 (/Monitor/Master/Mute)
    else if (address == "/Monitor/Master/Mute")
    {
        bool masterMuteState = (value > 0.5f);
        
        OSC_DBG_ROLE("OSCCommunicator: Received Master Mute OSC - " + juce::String(masterMuteState ? "ON" : "OFF"));
        
        if (onMasterMuteOSC)
        {
            onMasterMuteOSC(masterMuteState);
        }
    }
    // v4.1: 处理Master Mono消息 (/Monitor/Master/Effect/Mono)
    else if (address == "/Monitor/Master/Effect/Mono")
    {
        bool monoState = (value > 0.5f);
        
        OSC_DBG_ROLE("OSCCommunicator: Received Master Mono OSC - " + juce::String(monoState ? "ON" : "OFF"));
        
        if (onMasterMonoOSC)
        {
            onMasterMonoOSC(monoState);
        }
    }
    else
    {
        OSC_DBG_ROLE("OSCCommunicator: Unknown Master bus OSC address - " + address);
    }
}

juce::String OSCCommunicator::formatOSCAddress(const juce::String& action, const juce::String& channelName) const
{
    // 将通道名中的空格替换为下划线
    juce::String sanitizedChannelName = channelName.replaceCharacter(' ', '_');
    return "/Monitor/" + action + "/" + sanitizedChannelName;
}

std::pair<juce::String, juce::String> OSCCommunicator::parseOSCAddress(const juce::String& address) const
{
    // 期望格式: /Monitor/{Action}/{Channel}
    
    if (!address.startsWith("/Monitor/"))
    {
        return {"", ""};
    }
    
    // 移除前缀
    juce::String content = address.substring(9); // 移除"/Monitor/"
    
    // 查找第一个斜杠分隔符
    int slashPos = content.indexOf("/");
    if (slashPos == -1)
    {
        return {"", ""};
    }
    
    juce::String action = content.substring(0, slashPos);
    juce::String channelName = content.substring(slashPos + 1);
    
    // 将下划线替换回空格（如果需要）
    channelName = channelName.replaceCharacter('_', ' ');
    
    return {action, channelName};
}

bool OSCCommunicator::isValidChannelName(const juce::String& channelName) const
{
    // 验证语义通道名称 - 更新为匹配配置文件中的实际通道名
    static const std::vector<juce::String> validChannels = {
        // 主声道
        "L", "R", "C", "LFE", "LR", "RR",
        "LSS", "RSS", "LRS", "RRS",
        "LTF", "RTF", "LTB", "RTB",
        "LBF", "RBF", "LBB", "RBB",
        // SUB通道 (匹配Speaker_Config.json中的名称)
        "SUB F", "SUB B", "SUB L", "SUB R",
        // 旧版SUB通道名（保持兼容性）
        "SUB_L", "SUB_R", "SUB_M"
    };
    
    for (const auto& validChannel : validChannels)
    {
        if (channelName == validChannel)
        {
            return true;
        }
    }
    
    return false;
}