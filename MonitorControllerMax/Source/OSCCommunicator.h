#pragma once

#include <JuceHeader.h>
#include <memory>
#include <atomic>

// Forward declarations
class SemanticChannelState;
class PhysicalChannelMapper;
class MonitorControllerMaxAudioProcessor;

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
// 🚀 稳定性修复：从RealtimeCallback改为MessageLoopCallback
// 避免在实时线程中触发UI更新，严格遵循JUCE线程模型
// 🚀 v4.2: 增加Timer支持实现批量发送优化
class OSCCommunicator : public juce::OSCReceiver::Listener<juce::OSCReceiver::MessageLoopCallback>,
                       public juce::Timer
{
public:
    OSCCommunicator();
    ~OSCCommunicator();
    
    // 设置processor指针用于角色日志
    void setProcessor(MonitorControllerMaxAudioProcessor* processor);

    // 初始化和关闭
    bool initialize();
    void shutdown();
    bool isConnected() const;

    // 发送状态到外部设备
    void sendSoloState(const juce::String& channelName, bool state);
    void sendMuteState(const juce::String& channelName, bool state);
    
    // v4.1: 发送Master总线状态到外部设备
    void sendMasterVolume(float volumePercent);
    void sendMasterDim(bool dimState);
    void sendMasterLowBoost(bool lowBoostState);
    void sendMasterMute(bool masterMuteState);
    void sendMasterMono(bool monoState);
    
    // 状态反馈机制 - 广播所有当前状态
    void broadcastAllStates(const SemanticChannelState& semanticState, 
                           const PhysicalChannelMapper& physicalMapper);
    
    // OSC接收处理 (来自juce::OSCReceiver::Listener)
    void oscMessageReceived(const juce::OSCMessage& message) override;
    
    // 设置状态更新回调 (用于接收外部OSC控制时更新语义状态)
    std::function<void(const juce::String& action, const juce::String& channelName, bool state)> onExternalStateChange;
    
    // v4.1: Master总线OSC控制回调
    std::function<void(float volumePercent)> onMasterVolumeOSC;
    std::function<void(bool dimState)> onMasterDimOSC;
    std::function<void(bool lowBoostState)> onMasterLowBoostOSC;
    std::function<void(bool masterMuteState)> onMasterMuteOSC;
    std::function<void(bool monoState)> onMasterMonoOSC;

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
    
    // Processor指针用于角色日志
    MonitorControllerMaxAudioProcessor* processorPtr = nullptr;
    
    // 🚀 性能优化：消息队列系统
    struct OSCMessage {
        juce::String address;
        float value;
        int priority;  // 0 = 高优先级, 1 = 中等, 2 = 低优先级
        juce::int64 timestamp;
        
        OSCMessage(const juce::String& addr, float val, int prio = 1) 
            : address(addr), value(val), priority(prio)
            , timestamp(juce::Time::getCurrentTime().toMilliseconds()) {}
    };
    
    mutable std::mutex messageQueueMutex;
    std::vector<OSCMessage> messageQueue;
    
    // 消息合并优化
    std::map<juce::String, size_t> addressToQueueIndex;  // 地址到队列索引的映射
    
    // 🚀 队列处理方法
    void queueOSCMessage(const juce::String& address, float value, int priority = 1);
    void processBatchSend();  // 批量发送处理
    bool sendQueuedMessage(const OSCMessage& msg);  // 返回发送是否成功
    
    // Timer回调 (继承自juce::Timer)
    void timerCallback() override;
    
    // 内部工具方法
    void handleIncomingOSCMessage(const juce::OSCMessage& message);
    void handleMasterBusOSCMessage(const juce::String& address, const juce::OSCMessage& message);  // v4.1: Master总线OSC处理
    juce::String formatOSCAddress(const juce::String& action, const juce::String& channelName) const;
    std::pair<juce::String, juce::String> parseOSCAddress(const juce::String& address) const;
    bool isValidChannelName(const juce::String& channelName) const;
    
    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(OSCCommunicator)
};