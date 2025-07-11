#pragma once

#include <JuceHeader.h>
#include <memory>
#include <atomic>

// Forward declarations
class SemanticChannelState;
class PhysicalChannelMapper;

/**
 * OSC通信管理器 - 处理监听控制器的OSC双向通信
 * 
 * 主要功能：
 * 1. 发送状态变化到外部OSC设备
 * 2. 接收外部OSC控制命令
 * 3. 在插件加载时广播所有当前状态
 * 
 * OSC协议格式：
 * 地址: /Monitor/{Action}/{Channel}
 * 值: 1.0f (ON) / 0.0f (OFF)
 * 示例: /Monitor/Solo/L 1.0, /Monitor/Mute/SUB_B 0.0
 * 注：通道名中的空格会自动转换为下划线
 * 
 * 双向同步：
 * - 接收外部控制消息并更新内部状态
 * - 对所有状态变化发送确认反馈
 * - 实现控制器与插件的真正双向状态同步
 */
class OSCCommunicator : public juce::OSCReceiver::Listener<juce::OSCReceiver::RealtimeCallback>
{
public:
    OSCCommunicator();
    ~OSCCommunicator();

    // 初始化和关闭
    bool initialize();
    void shutdown();
    bool isConnected() const;

    // 发送状态到外部设备
    void sendSoloState(const juce::String& channelName, bool state);
    void sendMuteState(const juce::String& channelName, bool state);
    
    // 状态反馈机制 - 广播所有当前状态
    void broadcastAllStates(const SemanticChannelState& semanticState, 
                           const PhysicalChannelMapper& physicalMapper);
    
    // OSC接收处理 (来自juce::OSCReceiver::Listener)
    void oscMessageReceived(const juce::OSCMessage& message) override;
    
    // 设置状态更新回调 (用于接收外部OSC控制时更新语义状态)
    std::function<void(const juce::String& action, const juce::String& channelName, bool state)> onExternalStateChange;

private:
    // OSC通信组件
    std::unique_ptr<juce::OSCSender> sender;
    std::unique_ptr<juce::OSCReceiver> receiver;
    
    // 硬编码配置
    static constexpr const char* TARGET_IP = "127.0.0.1";
    static constexpr int TARGET_PORT = 7444;
    static constexpr int RECEIVE_PORT = 7445;
    
    // 连接状态
    std::atomic<bool> isInitialized{false};
    std::atomic<bool> senderConnected{false};
    std::atomic<bool> receiverConnected{false};
    
    // 内部工具方法
    void handleIncomingOSCMessage(const juce::OSCMessage& message);
    juce::String formatOSCAddress(const juce::String& action, const juce::String& channelName) const;
    std::pair<juce::String, juce::String> parseOSCAddress(const juce::String& address) const;
    bool isValidChannelName(const juce::String& channelName) const;
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(OSCCommunicator)
};