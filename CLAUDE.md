# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository contains audio effects for REAPER (Digital Audio Workstation), primarily focused on **monitor controller plugins**. The project consists of two main parts:

1. **JSFX Effects** - Native REAPER audio effects written in EEL2 scripting language
2. **JUCE Audio Plugin** - Cross-platform VST3/Standalone monitor controller built with JUCE framework

## Build Commands

### JUCE Plugin (MonitorControllerMax)
```bash
# Build using Visual Studio (Windows)
cd "MonitorControllerMax/Builds/VisualStudio2022"
# Open MonitorControllerMax.sln in Visual Studio
# Or build from command line:
msbuild MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64

# Build Debug version
msbuild MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64
```

### JSFX Effects
JSFX files (`.jsfx`) are script-based and don't require compilation. They can be directly loaded into REAPER:
- `Monitor Controllor 7.1.4.jsfx` - Main monitor controller
- `Monitor Controllor SUB.jsfx` - Subwoofer controller  
- `Monitor Controllor 7.1.4 AiYue_V1.jsfx` - Extended version

## Architecture Overview

### JUCE Plugin Architecture (MonitorControllerMax)

The JUCE plugin implements a sophisticated **master-slave communication system** for professional monitor control:

**Core Components:**
- `PluginProcessor` - Main audio processing engine that manages up to 26 channels
- `PluginEditor` - Dynamic UI that adapts to speaker configurations
- `ConfigManager` - Parses speaker layout configurations from JSON
- `InterPluginCommunicator` - Handles inter-plugin communication for master-slave setup

**Key Design Patterns:**
- **Role-based Processing**: Plugins can operate as standalone, master, or slave instances
- **Dynamic Parameter Management**: Parameters are generated based on loaded speaker configurations
- **State Synchronization**: Master instance controls slave instances via IPC
- **UI-driven Logic**: Complex state changes are handled in UI callbacks, not parameter change events

**Audio Processing Flow:**
1. **Slave Plugin** (pre-calibration): Applies mute/solo filtering to raw audio
2. **External Calibration Software**: Processes the filtered audio
3. **Master Plugin** (post-calibration): Applies final mute/solo/gain processing

### Speaker Configuration System

The plugin uses `Source/Config/Speaker_Config.json` to define:
- Speaker layouts (2.0, 2.1, 5.1, 7.1.4, etc.)
- Sub layouts (Single Sub, Dual Sub, etc.)  
- Channel mapping to audio interface outputs
- Grid positions for UI layout

**Layout Structure:**
```json
{
  "Speaker": {
    "7.1.4": {
      "L": 1, "R": 5, "C": 3, "LFE": 13,
      "LR": 21, "RR": 25,
      "LTF": 17, "RTF": 19, "LTR": 23, "RTR": 27
    }
  },
  "Sub": {
    "Single Sub": { "SUB M": 9 },
    "Dual Sub": { "SUB L": 9, "SUB R": 11 }
  }
}
```

### File Organization

```
MonitorControllerMax/
├── Source/
│   ├── PluginProcessor.h/cpp     # Main audio processor
│   ├── PluginEditor.h/cpp        # Dynamic UI implementation  
│   ├── ConfigManager.h/cpp       # Configuration parsing
│   ├── ConfigModels.h            # Data structures
│   ├── InterPluginCommunicator.h/cpp  # IPC system
│   └── Config/
│       └── Speaker_Config.json   # Speaker layout definitions
├── Builds/VisualStudio2022/      # Visual Studio project files
└── JuceLibraryCode/              # Auto-generated JUCE code
```

## Development Workflow

### 🚀 Claude Code 自动开发标准流程

**重要说明：大部分情况下，Claude Code应该遵循以下开发流程：**

1. **主要开发模式：快速Debug独立程序编译**
   - 使用Debug独立程序进行日常开发和功能验证
   - 避免在开发过程中进行完整的Release构建
   - 专注于快速迭代和功能实现

2. **自动化错误处理**
   - 实时监控编译日志，立即修复编译错误
   - 确保代码在快速Debug编译中不报错
   - 维护代码质量，避免引入潜在问题

3. **最终构建策略**
   - 开发完成后，由人工进行最终的完整编译
   - 确保生产版本的质量和稳定性
   - 避免在开发过程中的构建复杂度

**开发优先级：**
- ✅ 快速Debug独立程序编译（用于功能验证）
- ✅ 自动错误检测和修复
- ✅ 代码质量保证
- ⚠️ 避免不必要的Release构建

### 🔄 Git版本控制和渐进式开发流程

**核心原则：小步快跑，稳健迭代**

Claude Code在进行自动开发时必须严格遵循以下Git工作流程：

#### 1. **每个功能点的开发循环**
```
修改代码 → 快速Debug编译 → 测试功能 → Git提交 → 下一个功能点
```

#### 2. **Git提交策略**
- **小粒度提交：** 每完成一个小的、可验证的功能改进立即提交
- **描述性提交信息：** 使用清晰的中文提交信息描述具体改动
- **状态稳定后提交：** 确保代码能够编译成功且基本功能正常后再提交
- **错误修复后提交：** 修复编译错误或功能问题后立即提交修复版本

#### 3. **具体提交时机**
- ✅ **函数签名修复后** - "修复错误的getParameterName函数签名"
- ✅ **添加新函数声明后** - "添加I/O通道名函数声明"
- ✅ **实现新函数后** - "实现getInputChannelName动态通道名"
- ✅ **编译错误修复后** - "修复编译错误：缺失头文件引用"
- ✅ **功能测试通过后** - "I/O通道名功能测试通过"

#### 4. **错误处理和回滚策略**
- **编译失败时：** 立即分析错误，修复后提交修复版本
- **功能异常时：** 快速定位问题，修复或回滚到上一个稳定版本
- **保持工作记录：** 每个提交都应该是一个可工作的状态点

#### 5. **自动化开发时的安全网**
- **在用户不在电脑前时：** 可以安全地进行多个小步骤的开发
- **每个稳定点都有Git记录：** 确保不会丢失已解决的问题状态
- **回滚能力：** 如果某个改动导致问题，可以快速回到上一个工作状态

#### 6. **提交信息格式**
```
类型: 简短描述

详细说明改动内容和原因（如有必要）

相关: Dev Step.md 步骤X.X
```

**示例提交信息：**
- `修复: 移除错误的getParameterName函数签名`
- `功能: 实现动态I/O通道名getInputChannelName`
- `测试: I/O通道名功能验证通过`
- `修复: 解决编译错误-缺失const限定符`

### 📝 代码注释标准

**重要说明：所有代码修改都必须包含中文注释**

#### 1. **注释语言要求**
- ✅ **必须使用中文注释** - 所有新增和修改的代码都需要中文注释
- ✅ **保持现有英文注释** - 不修改已有的英文注释，除非必要
- ✅ **关键逻辑必须注释** - 复杂的业务逻辑、算法和重要决策点

#### 2. **注释内容要求**
- **函数注释：** 说明函数的目的、参数含义、返回值和特殊行为
- **复杂逻辑注释：** 解释为什么这样实现，而不仅仅是做了什么
- **业务逻辑注释：** 说明与音频处理、参数管理相关的专业概念
- **修改原因注释：** 对于修复或改进，说明修改的原因和背景

#### 3. **注释示例格式**
```cpp
// 动态获取输入通道名称，根据当前音箱布局映射物理通道到逻辑声道名
// channelIndex: 物理通道索引（从0开始）
// 返回: 对应的声道名称（如"LFE"）或默认名称
const String MonitorControllerMaxAudioProcessor::getInputChannelName(int channelIndex) const
{
    // 遍历当前布局中的所有通道配置
    for (const auto& chanInfo : currentLayout.channels)
    {
        // 检查物理通道索引是否匹配布局中的通道索引
        if (chanInfo.channelIndex == channelIndex)
        {
            return chanInfo.name;  // 返回配置文件中定义的声道名称
        }
    }
    
    // 未找到映射时返回默认通道名称
    return "Channel " + String(channelIndex + 1);
}
```

#### 4. **特殊情况的注释要求**
- **JUCE框架相关：** 解释JUCE特有的概念和API使用
- **音频处理逻辑：** 说明采样、缓冲区、实时性等音频概念
- **线程安全：** 标明哪些代码在音频线程中执行
- **性能考虑：** 解释为什么选择某种实现方式

#### 5. **修改现有代码时的注释策略**
- **新增功能：** 完整的中文注释
- **修复问题：** 添加修复原因的注释
- **重构代码：** 说明重构的目的和改进点
- **删除代码：** 保留必要的注释说明删除原因

### Working with Speaker Configurations
1. Edit `Speaker_Config.json` to add new layouts
2. Layouts are automatically loaded and UI adapts dynamically
3. Channel indices in JSON correspond to audio interface outputs

### Adding New Features
1. **Audio Processing**: Modify `PluginProcessor::processBlock()`
2. **UI Components**: Update `PluginEditor::updateLayout()` 
3. **Parameters**: Extend `createParameterLayout()` if needed
4. **Communication**: Modify `InterPluginCommunicator` for cross-instance features

### Testing Master-Slave Setup
1. Load two plugin instances in DAW
2. Click "Link" button on desired master instance  
3. Slave instance UI becomes read-only and mirrors master state
4. Place slave before calibration software, master after

## Key Implementation Details

### Parameter Management
- Parameters are created dynamically based on maximum channel count
- Channel mapping happens at runtime based on active layout
- Unused parameters are automatically bypassed

### State Synchronization  
- Only mute/solo states are synchronized between instances
- Gain/volume parameters remain local to each instance
- Communication uses `juce::InterprocessConnection` for low latency

### UI Behavior
- **Normal Mode**: All controls active
- **Master Mode**: Full control, sends state to slave
- **Slave Mode**: UI locked, displays master state only
- **Solo Logic**: Automatically mutes non-soloed channels with state caching

### Dynamic Host Integration
- `getParameterName()`: Returns layout-aware parameter names ("Mute LFE" vs "Mute 4")
- `getInputChannelName()`/`getOutputChannelName()`: Returns channel-specific names
- `updateHostDisplay()`: Notifies DAW of parameter name changes

## Documentation Resources

### JUCE Framework Deep Dive
The project includes comprehensive JUCE documentation in `Doc/JUCE Wiki/`:

**Core Architecture Understanding:**
- `JUCE-Framework-Overview.md` - Complete framework module relationships and architecture diagrams
- `Audio-Framework.md` - AudioProcessor, AudioDeviceManager, and plugin system details
- `Audio-Plugin-System.md` - VST/AU/AAX plugin format implementations and hosting
- `Component-System.md` - GUI component hierarchy and event handling
- `GUI-Framework.md` - LookAndFeel customization and graphics rendering

**Development Workflow:**
- `CMake-Build-System.md` - Modern build configuration (preferred over Projucer)
- `Projucer.md` - Legacy project management tool
- `Development-Tools.md` - Complete toolchain overview
- `Standalone-Plugin-Applications.md` - Standalone app development

**Advanced Topics:**
- `Core-Systems.md` - Memory management, threading, and data structures
- `String,-ValueTree,-and-File.md` - Data persistence and serialization
- `OpenGL-Integration.md` - Hardware-accelerated graphics
- `Mathematics-and-Geometry.md` - DSP and geometric utilities

### JSFX/EEL2 Scripting Reference
Complete REAPER JSFX programming documentation in `Doc/ReaScript/`:

**Language Fundamentals:**
- `Introduction.txt` - JSFX file structure and basic syntax
- `Basic code reference.txt` - EEL2 language essentials, operators, and memory management
- `Special Variables.txt` - Built-in variables for audio processing

**Audio & MIDI:**
- `MIDI.txt` - MIDI message handling and bus support
- `Memory Slider FFT MDCT Functions.txt` - DSP algorithms and audio buffer operations
- `Graphics.txt` - Custom UI drawing and visualization

**Integration:**
- `ReaScript API.txt` - REAPER automation and host interaction
- `File IO and Serialization.txt` - Data persistence in JSFX
- `Strings.txt` - Text processing and manipulation

### Project-Specific Documentation
- `Dev.md` - Comprehensive monitor controller architecture and implementation guide
- `Dev Step.md` - Current development roadmap and next implementation steps
- `Juce插件开发详细指南_.md` - Detailed JUCE plugin development guide in Chinese

## Technical Notes

### JUCE Best Practices (From Documentation Analysis)
- **Modern Workflow**: Use CMake build system over Projucer for professional development
- **Audio Safety**: Follow real-time audio constraints in `processBlock()`
- **Parameter Management**: Use AudioProcessorValueTreeState for thread-safe parameter handling
- **Cross-Platform**: Abstract platform-specific code behind JUCE interfaces
- **Memory Management**: Prefer RAII and smart pointers for resource management

### JSFX Development
- **Language**: EEL2 scripting with C-like syntax but dynamic typing
- **Integration**: Direct REAPER integration, no compilation needed
- **Features**: Real-time audio processing, MIDI handling, custom graphics
- **Memory**: ~8M local + ~1M global shared memory space
- **UI**: Vector-based custom drawing with immediate-mode graphics

### Performance Considerations
- **Audio Thread Safety**: Never allocate/deallocate in `processBlock()`
- **Channel Mapping**: Physical channel iteration with logical channel mapping
- **State Synchronization**: Minimize communication overhead between instances
- **UI Updates**: Timer-based to avoid blocking audio processing
- **Memory Access**: Use proper alignment for SIMD operations

### Platform Support
- **Primary**: Windows with Visual Studio 2022 project
- **Cross-Platform**: JUCE codebase supports macOS/Linux via CMake
- **JSFX**: REAPER-specific (Windows/macOS/Linux)
- **Plugin Formats**: VST3, AU, AAX, Standalone