/*
  ==============================================================================

    SafeUICallback.h
    Created: 2025-08-01
    Author:  GohardSGG & Claude Code

    UI回调安全化工具类 - 稳定性优化第1天
    
    核心功能：
    - 防止循环引用导致的内存泄漏
    - 自动检测组件生命周期，避免悬空指针访问
    - 全面异常捕获，确保UI回调不会崩溃插件
    - 符合JUCE最佳实践的安全回调模式

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include <functional>
#include <memory>
#include "DebugLogger.h"

//==============================================================================
/**
 * UI回调安全化工具类
 * 
 * 解决的问题：
 * 1. std::function回调中的循环引用 (Component -> Processor -> std::function -> Component)
 * 2. 异步回调中的悬空指针访问 (组件已销毁但回调仍执行)
 * 3. UI回调中的异常导致插件崩溃
 * 4. Timer回调与其他UI操作的竞态条件
 * 
 * 设计原则：
 * - 使用juce::Component::SafePointer防止悬空指针
 * - 全面异常捕获，确保回调安全失败
 * - 提供简洁的API，易于集成现有代码
 * - 零性能开销，适合实时音频应用
 */
class SafeUICallback
{
public:
    //==============================================================================
    /**
     * 创建安全的组件回调函数
     * 
     * @param component 目标组件指针 (自动转换为SafePointer)
     * @param callback 回调函数，接收有效的组件指针
     * @return 安全的std::function，可安全存储和异步调用
     */
    template<typename ComponentType>
    static std::function<void()> create(ComponentType* component, 
                                        std::function<void(ComponentType*)> callback)
    {
        if (component == nullptr || !callback)
        {
            VST3_DBG("SafeUICallback: Invalid component or callback provided");
            return []() {}; // 返回空回调避免崩溃
        }
        
        // 使用SafePointer和弱引用避免循环引用
        auto safePtr = juce::Component::SafePointer<ComponentType>(component);
        
        return [safePtr, callback]()
        {
            // 检查组件是否仍然有效
            if (auto* validComponent = safePtr.getComponent())
            {
                try
                {
                    // 在安全环境中执行回调
                    callback(validComponent);
                }
                catch (const std::exception& e)
                {
                    VST3_DBG("SafeUICallback: Standard exception caught in UI callback: " + juce::String(e.what()));
                }
                catch (...)
                {
                    VST3_DBG("SafeUICallback: Unknown exception caught in UI callback");
                }
            }
            else
            {
                // 组件已销毁，静默忽略回调（这是正常情况）
                // VST3_DBG("SafeUICallback: Component no longer valid, callback skipped");
            }
        };
    }
    
    //==============================================================================
    /**
     * 创建安全的无参数回调函数
     * 
     * @param component 目标组件指针 (用于生命周期检查)
     * @param callback 回调函数，无参数版本
     * @return 安全的std::function，可安全存储和异步调用
     */
    template<typename ComponentType>
    static std::function<void()> createSimple(ComponentType* component, 
                                              std::function<void()> callback)
    {
        if (component == nullptr || !callback)
        {
            VST3_DBG("SafeUICallback: Invalid component or callback provided");
            return []() {}; // 返回空回调避免崩溃
        }
        
        // 使用SafePointer检查组件生命周期
        auto safePtr = juce::Component::SafePointer<ComponentType>(component);
        
        return [safePtr, callback]()
        {
            // 检查组件是否仍然有效
            if (safePtr.getComponent() != nullptr)
            {
                try
                {
                    // 在安全环境中执行回调
                    callback();
                }
                catch (const std::exception& e)
                {
                    VST3_DBG("SafeUICallback: Standard exception caught in simple UI callback: " + juce::String(e.what()));
                }
                catch (...)
                {
                    VST3_DBG("SafeUICallback: Unknown exception caught in simple UI callback");
                }
            }
            // 组件已销毁时静默忽略（正常情况）
        };
    }
    
    //==============================================================================
    /**
     * 创建安全的MessageManager异步调用
     * 
     * 这是对juce::MessageManager::callAsync的安全封装
     * 自动添加组件生命周期检查和异常处理
     * 
     * @param component 目标组件指针
     * @param callback 要异步执行的回调函数
     */
    template<typename ComponentType>
    static void callAsync(ComponentType* component, 
                          std::function<void(ComponentType*)> callback)
    {
        if (component == nullptr || !callback)
        {
            VST3_DBG("SafeUICallback: Invalid component or callback for async call");
            return;
        }
        
        // 创建安全回调并异步执行
        auto safeCallback = create(component, callback);
        juce::MessageManager::callAsync(safeCallback);
    }
    
    //==============================================================================
    /**
     * 创建安全的MessageManager异步调用 (无参数版本)
     * 
     * @param component 目标组件指针 (用于生命周期检查)
     * @param callback 要异步执行的回调函数
     */
    template<typename ComponentType>
    static void callAsyncSimple(ComponentType* component, 
                                std::function<void()> callback)
    {
        if (component == nullptr || !callback)
        {
            VST3_DBG("SafeUICallback: Invalid component or callback for simple async call");
            return;
        }
        
        // 创建安全回调并异步执行
        auto safeCallback = createSimple(component, callback);
        juce::MessageManager::callAsync(safeCallback);
    }

private:
    // 禁止实例化，这是一个纯静态工具类
    SafeUICallback() = delete;
    ~SafeUICallback() = delete;
    SafeUICallback(const SafeUICallback&) = delete;
    SafeUICallback& operator=(const SafeUICallback&) = delete;
};

//==============================================================================
/**
 * 便捷宏定义 - 简化SafeUICallback的使用
 * 
 * 使用示例:
 * SAFE_UI_CALLBACK(this, [](auto* self) {
 *     self->updateUI();
 * });
 */
#define SAFE_UI_CALLBACK(component, callback) \
    SafeUICallback::create(component, callback)

#define SAFE_UI_CALLBACK_SIMPLE(component, callback) \
    SafeUICallback::createSimple(component, callback)

#define SAFE_UI_ASYNC(component, callback) \
    SafeUICallback::callAsync(component, callback)

#define SAFE_UI_ASYNC_SIMPLE(component, callback) \
    SafeUICallback::callAsyncSimple(component, callback)