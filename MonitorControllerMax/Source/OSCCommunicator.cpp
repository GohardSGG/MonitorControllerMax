#include "OSCCommunicator.h"
#include "SemanticChannelState.h"
#include "PhysicalChannelMapper.h"
#include "DebugLogger.h"

OSCCommunicator::OSCCommunicator()
{
    VST3_DBG("OSCCommunicator: Initialize OSC communication system");
    
    // 创建OSC发送和接收组件
    sender = std::make_unique<juce::OSCSender>();
    receiver = std::make_unique<juce::OSCReceiver>();
}

OSCCommunicator::~OSCCommunicator()
{
    VST3_DBG("OSCCommunicator: Shutdown OSC communication system");
    shutdown();
}

bool OSCCommunicator::initialize()
{
    VST3_DBG("OSCCommunicator: Initialize OSC connections");
    
    bool success = true;
    
    // 初始化OSC发送器
    if (sender->connect(TARGET_IP, TARGET_PORT))
    {
        senderConnected.store(true);
        VST3_DBG("OSCCommunicator: OSC Sender connected to " << TARGET_IP << ":" << TARGET_PORT);
    }
    else
    {
        senderConnected.store(false);
        VST3_DBG("OSCCommunicator: Failed to connect OSC Sender to " << TARGET_IP << ":" << TARGET_PORT);
        success = false;
    }
    
    // 初始化OSC接收器
    if (receiver->connect(RECEIVE_PORT))
    {
        receiver->addListener(this);
        receiverConnected.store(true);
        VST3_DBG("OSCCommunicator: OSC Receiver listening on port " << RECEIVE_PORT);
    }
    else
    {
        receiverConnected.store(false);
        VST3_DBG("OSCCommunicator: Failed to start OSC Receiver on port " << RECEIVE_PORT);
        success = false;
    }
    
    isInitialized.store(success);
    
    if (success)
    {
        VST3_DBG("OSCCommunicator: OSC communication system initialized successfully");
    }
    else
    {
        VST3_DBG("OSCCommunicator: OSC communication system initialization failed");
    }
    
    return success;
}

void OSCCommunicator::shutdown()
{
    VST3_DBG("OSCCommunicator: Shutdown OSC communication");
    
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
    VST3_DBG("OSCCommunicator: OSC communication shutdown complete");
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
        VST3_DBG("OSCCommunicator: Sent Solo state - " << address << " = " << value);
    }
    else
    {
        VST3_DBG("OSCCommunicator: Failed to send Solo state - " << address);
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
        VST3_DBG("OSCCommunicator: Sent Mute state - " << address << " = " << value);
    }
    else
    {
        VST3_DBG("OSCCommunicator: Failed to send Mute state - " << address);
    }
}

void OSCCommunicator::broadcastAllStates(const SemanticChannelState& semanticState, 
                                        const PhysicalChannelMapper& physicalMapper)
{
    if (!isConnected())
    {
        VST3_DBG("OSCCommunicator: Cannot broadcast - not connected");
        return;
    }
    
    VST3_DBG("OSCCommunicator: Broadcasting all current states");
    
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
    
    VST3_DBG("OSCCommunicator: Broadcast complete - " << activeChannels.size() << " channels");
}

void OSCCommunicator::oscMessageReceived(const juce::OSCMessage& message)
{
    handleIncomingOSCMessage(message);
}

void OSCCommunicator::handleIncomingOSCMessage(const juce::OSCMessage& message)
{
    juce::String address = message.getAddressPattern().toString();
    
    VST3_DBG("OSCCommunicator: Received OSC message - " << address);
    
    // 解析OSC地址
    auto [action, channelName] = parseOSCAddress(address);
    
    if (action.isEmpty() || channelName.isEmpty())
    {
        VST3_DBG("OSCCommunicator: Invalid OSC address format - " << address);
        return;
    }
    
    if (!isValidChannelName(channelName))
    {
        VST3_DBG("OSCCommunicator: Invalid channel name - " << channelName);
        return;
    }
    
    // 获取值
    if (message.size() < 1)
    {
        VST3_DBG("OSCCommunicator: OSC message has no arguments");
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
        VST3_DBG("OSCCommunicator: OSC message argument is not numeric");
        return;
    }
    
    bool state = (value > 0.5f);
    
    VST3_DBG("OSCCommunicator: Parsed OSC - action:" << action << " channel:" << channelName << " state:" << (state ? "ON" : "OFF"));
    
    // 调用处理函数来更新对应的状态
    if (onExternalStateChange)
    {
        // 传递action类型、通道名和状态值
        onExternalStateChange(action, channelName, state);
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
    // 验证语义通道名称
    static const std::vector<juce::String> validChannels = {
        "L", "R", "C", "LFE", "LR", "RR",
        "LTF", "RTF", "LTR", "RTR",
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