#include "StateManager.h"
#include <iostream>

int main() {
    // 测试Mute记忆保护逻辑
    std::cout << "Testing Mute Memory Protection Logic..." << std::endl;
    
    // 创建StateManager实例
    StateManager stateManager;
    
    // 模拟场景1：有手动Mute的通道
    std::cout << "\n=== Test 1: Manual mute then save ===" << std::endl;
    stateManager.handleChannelMuteClick(0);  // 手动Mute通道0
    stateManager.handleChannelMuteClick(2);  // 手动Mute通道2
    
    // 保存记忆
    stateManager.saveMuteMemoryNow();
    
    // 模拟场景2：尝试用空状态保存（应该被阻止）
    std::cout << "\n=== Test 2: Try to save empty memory (should be blocked) ===" << std::endl;
    
    // 创建一个新的StateManager来模拟空状态
    StateManager emptyStateManager;
    // 这个调用应该被阻止，因为有现有的记忆
    emptyStateManager.saveMuteMemoryNow();
    
    std::cout << "\nTest completed!" << std::endl;
    return 0;
}