# CLAUDE.md

该文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。

## 🚨 **重要编码标准**

**⚡ 全局文件格式要求：所有文件必须保存为 UTF-8 with BOM 格式**

- ✅ **新建文件**: 必须使用 UTF-8 with BOM 编码
- ✅ **修改文件**: 保持原有编码格式，如是UTF-8 with BOM则保持
- ✅ **中文注释**: 确保中文字符正确显示和保存
- ✅ **跨平台兼容**: UTF-8 with BOM保证Windows/Linux一致性

**重要性说明：**
- 防止中文注释乱码问题
- 确保Visual Studio正确识别文件编码
- 保证跨平台开发环境一致性
- 避免编译时的字符编码错误

## 项目概述

该代码库包含用于 REAPER（数字音频工作站）的音频效果，主要专注于 **监听控制器插件**。项目包括两个主要部分：

1.  **JSFX 效果** - 使用 EEL2 脚本语言编写的 REAPER 原生音频效果
2.  **JUCE 音频插件** - 使用 JUCE 框架构建的跨平台 VST3/独立运行的监听控制器

## 构建命令

### JUCE 插件 (MonitorControllerMax)

```bash
# 使用 Visual Studio (Windows) 构建
cd "MonitorControllerMax/Builds/VisualStudio2022"
# 在 Visual Studio 中打开 MonitorControllerMax.sln
# 或从命令行构建：
msbuild MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64

# 构建 Debug 版本
msbuild MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64
```

### JSFX 效果

JSFX 文件 (`.jsfx`) 是基于脚本的，无需编译。它们可以直接加载到 REAPER 中：
- `Monitor Controllor 7.1.4.jsfx` -主监听控制器
- `Monitor Controllor SUB.jsfx` - 超低音控制器
- `Monitor Controllor 7.1.4 AiYue_V1.jsfx` - 扩展版本

## 架构概述

### JUCE 插件架构 (MonitorControllerMax)

该 JUCE 插件为专业监听控制实现了一个复杂的 **主从通信系统**：

**核心组件:**
- `PluginProcessor` - 管理多达26个通道的主音频处理引擎，支持角色化处理
- `PluginEditor` - 动态UI，支持主从模式和角色选择下拉框
- `ConfigManager` - 从 JSON 文件中解析扬声器布局配置
- `InterPluginCommunicator` - 处理主从设置的插件间通信
- `SemanticChannelState` - 语义通道状态管理系统
- `OSCCommunicator` - 双向OSC通信系统（仅主插件和独立插件发送）

**关键设计模式:**
- **基于角色的处理**: 插件可以作为独立(Standalone)、主(Master)或从(Slave)实例运行
- **语义状态系统**: 使用声道名称而非物理通道索引进行状态管理
- **角色化OSC通信**: 只有主插件和独立插件发送OSC消息，从插件只接收状态
- **动态布局适应**: UI和处理逻辑根据当前扬声器配置自动调整
- **主从状态同步**: 主实例通过IPC向从实例广播Solo/Mute状态

**音频处理流程:**
1.  **从插件** (校准前): 应用Solo/Mute滤波，不发送OSC消息
2.  **外部校准软件**: 处理滤波后的音频
3.  **主插件** (校准后): 应用最终处理，负责OSC通信和界面控制

**主从角色分工 (v3.0新架构):**

| 角色 | OSC发送 | OSC接收 | 音频处理 | 界面控制 | 主从同步 |
|------|---------|---------|----------|----------|----------|
| **独立(Standalone)** | ✅ | ✅ | ✅ | ✅ | ❌ |
| **主插件(Master)** | ✅ | ✅ | ✅ | ✅ | ✅发送 |
| **从插件(Slave)** | ❌ | ❌ | ✅ | ✅显示 | ✅接收 |

### 扬声器配置系统

该插件使用 `Source/Config/Speaker_Config.json` 来定义：
- 扬声器布局 (2.0, 2.1, 5.1, 7.1.4, 等)
- 超低音布局 (单超低音, 双超低音, 等)
- 声道到音频接口输出的映射
- 用于 UI 布局的网格位置

**布局结构:**
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

### 文件组织

```
MonitorControllerMax/
├── Source/
│   ├── PluginProcessor.h/cpp          # 主音频处理器和OSC回调
│   ├── PluginProcessor_StateSync.cpp  # 主从状态同步实现
│   ├── PluginEditor.h/cpp             # 动态UI和角色选择器
│   ├── ConfigManager.h/cpp            # 配置解析
│   ├── ConfigModels.h                 # 数据结构
│   ├── InterPluginCommunicator.h/cpp  # IPC 系统
│   ├── SemanticChannelState.h/cpp     # 语义状态管理
│   ├── SemanticChannelButton.h        # 语义UI组件
│   ├── OSCCommunicator.h/cpp          # OSC双向通信
│   ├── PhysicalChannelMapper.h/cpp    # 物理通道映射
│   ├── DebugLogger.h                  # VST3调试日志系统
│   └── Config/
│       └── Speaker_Config.json        # 扬声器布局定义
├── Builds/VisualStudio2022/           # Visual Studio 项目文件
├── JuceLibraryCode/                   # 自动生成的 JUCE 代码
└── Debug/                             # Claude Code 自动化工具套件
    ├── claude_auto_build.sh           # 一键编译运行脚本
    ├── README.md                      # 工具使用说明
    └── logs.txt                       # 构建和运行日志
```

## 开发工作流

### 🚀 Claude Code 自动开发标准流程

**重要说明：大部分情况下，Claude Code应该遵循以下开发流程：**

1.  **主要开发模式：快速Debug独立程序编译**
    *   使用Debug独立程序进行日常开发和功能验证
    *   避免在开发过程中进行完整的Release构建
    *   专注于快速迭代和功能实现

2.  **自动化错误处理**
    *   实时监控编译日志，立即修复编译错误
    *   确保代码在快速Debug编译中不报错
    *   维护代码质量，避免引入潜在问题

3.  **最终构建策略**
    *   开发完成后，由人工进行最终的完整编译
    *   确保生产版本的质量和稳定性
    *   避免在开发过程中的构建复杂度

**开发优先级 (v2.5 更新)：**
- ✅ **一键开发**：编译并自动启动程序（最高优先级）
- ✅ **快速迭代**：直接在启动的程序中测试功能
- ✅ **智能启动**：自动路径转换和程序启动
- ✅ **自动错误检测**：详细的编译错误分析和建议
- ✅ **代码质量保证**：完整的状态验证和日志管理
- ⚠️ **避免不必要的Release构建**

### 🛠️ Claude Code 统一自动化工具套件 (v2.5 现代化版本)

**⚡ 首要规则：使用现代化的一键开发工具套件**

Claude Code 必须使用位于 `/Debug/` 目录的统一自动化工具套件，现已升级为 v2.5 版本，提供完美的一键开发体验：

### 🔧 VST3调试解决方案 (v3.0新增)

**解决VST3插件调试输出问题的完整方案：**

#### 核心问题
- **Standalone版本**：有调试输出但音频设备通道限制
- **VST3版本**：音频功能完整但无法查看调试输出

#### 解决方案 - VST3调试日志系统
项目已实现完整的VST3调试日志系统，位于 `Source/DebugLogger.h`：

**关键特性：**
- ✅ **双重输出**：所有`VST3_DBG()`同时输出到控制台和文件
- ✅ **实时日志**：VST3插件运行时自动记录到文件
- ✅ **时间戳**：精确的毫秒级时间戳便于调试
- ✅ **自动初始化**：插件加载时自动创建日志系统

**日志文件位置：**
```
%TEMP%\MonitorControllerMax_Debug.log
实际路径：C:\Users\[用户名]\AppData\Local\Temp\MonitorControllerMax_Debug.log
```

**快速查看调试日志：**
1. 按 `Win + R` 输入 `%TEMP%`
2. 找到 `MonitorControllerMax_Debug.log`
3. 用记事本打开即可实时查看VST3调试输出

**开发工作流：**
```
1. 在REAPER中加载VST3插件
2. 执行需要调试的操作
3. 打开日志文件查看详细调试信息
4. 基于日志信息进行代码调整
5. 重新编译并重复测试
```

**代码实现：**
- 所有关键模块已更新使用`VST3_DBG()`替代`DBG()`
- StateManager、PluginProcessor、PluginEditor全面支持
- 自动记录状态转换、UI更新、参数变化等关键事件

#### 🚀 核心命令 (推荐使用顺序)

```bash
# 进入工具目录
cd "/mnt/c/REAPER/Effects/Masking Effects/Debug"

# 🚀 一键开发 - 编译并运行 (最常用，强烈推荐)
./claude_auto_build.sh run

# 快速启动 - 仅运行已有程序 (快速测试)
./claude_auto_build.sh start

# 状态检查 - 随时查看构建状态
./claude_auto_build.sh status

# 日常编译 - 仅编译验证 (传统模式)
./claude_auto_build.sh quick

# 问题排查 - 完整Debug编译
./claude_auto_build.sh full

# 发布准备 - Release编译 (最终阶段)
./claude_auto_build.sh release

# 清理环境
./claude_auto_build.sh clean
```

#### 🎯 Claude Code 现代化开发流程 (v2.5)

**最简单的一键开发流程 (强烈推荐)：**
```
1. 修改代码 (Visual Studio)
2. ./claude_auto_build.sh run  (一键编译并运行)
3. 直接测试功能
4. Git 提交稳定状态
5. 重复循环
```

**Windows PowerShell 用户：**
```powershell
cd "C:\REAPER\Effects\Masking Effects\Debug"

# 一键开发
wsl ./claude_auto_build.sh run

# 查看状态
wsl ./claude_auto_build.sh status
```

#### 🔄 现代化开发循环 (v2.5 一键流程)

**新的一键开发循环 (推荐)：**
```
1. 修改代码
   ↓
2. ./claude_auto_build.sh run  (一键编译并运行)
   ↓
3. 直接在启动的程序中测试功能
   ↓
4. Git提交稳定状态
   ↓
5. 重复循环
```

**传统分步开发循环 (备选)：**
```
1. 修改代码
   ↓
2. ./claude_auto_build.sh quick  (仅编译验证)
   ↓
3. ./claude_auto_build.sh start  (启动程序测试)
   ↓
4. Git提交稳定状态
   ↓
5. 重复循环
```

#### 🚀 工具套件特性 (v2.5 现代化版本)

**新增核心功能：**
- ✅ **一键开发**: `run` 命令实现编译+启动一体化
- ✅ **智能启动**: 自动路径转换和程序启动
- ✅ **后台运行**: 程序启动后命令行依然可用
- ✅ **测试建议**: 自动显示功能测试要点

**传统功能增强：**
- ✅ **直接MSBuild**: 避免PowerShell编码问题，更可靠
- ✅ **智能编译**: 根据场景自动选择最佳编译策略
- ✅ **错误检测**: 自动分析编译错误并提供建议
- ✅ **状态验证**: 验证构建产物的正确性
- ✅ **彩色输出**: 清晰的状态指示 (成功/警告/错误)
- ✅ **日志管理**: 自动保存和分析编译日志
- ✅ **跨环境**: 支持 WSL 和 Windows PowerShell

#### 🎯 Claude Code 使用约定 (v2.5)

**优先级顺序 (按使用频率)：**
1. **永远优先使用 `run` 模式** - 一键编译并运行，最高效的开发方式
2. **快速测试使用 `start` 模式** - 仅启动程序，无需重新编译
3. **状态检查使用 `status` 模式** - 了解当前构建状态和文件信息
4. **编译验证使用 `quick` 模式** - 仅编译不启动，传统验证方式
5. **编译失败时使用 `full` 模式** - 完整清理重建解决问题
6. **遵循小步快跑** - 每个稳定状态及时Git提交
7. **避免手动编译** - 统一使用自动化工具确保一致性

**典型的Claude Code工作流：**
```bash
# 开始开发会话
./claude_auto_build.sh status  # 了解当前状态

# 主要开发循环 (重复多次)
./claude_auto_build.sh run     # 编译并运行，测试功能

# 问题排查 (如果需要)
./claude_auto_build.sh full    # 完整重建

# 最终验证
./claude_auto_build.sh release # Release编译确认
```

#### 📁 工具套件文件清单 (v2.5)

这套工具位于 `/Debug/` 目录，现已简化为最核心的文件：

**核心文件 (仅2个)：**
- `claude_auto_build.sh` - Claude Code自动化编译脚本 (核心工具)
  - 直接调用MSBuild，避免编码问题
  - 支持编译、运行、状态检查等完整功能
  - 一键开发体验，编译并自动启动程序
- `README.md` - 完整使用说明和故障排除指南

**技术架构：**
```
WSL/Bash → claude_auto_build.sh → MSBuild → 独立程序自动启动
```

**此工具套件是Claude Code开发的核心，必须严格遵循使用。v2.5版本实现了最简洁、最可靠的架构。**

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

### 使用扬声器配置
1. 编辑 `Speaker_Config.json` 来添加新的布局
2. 布局会自动加载，UI会动态适应
3. JSON 中的通道索引对应于音频接口的输出

### 添加新功能
1. **音频处理**: 修改 `PluginProcessor::processBlock()`
2. **UI 组件**: 更新 `PluginEditor::updateLayout()` 
3. **参数**: 如果需要，扩展 `createParameterLayout()`
4. **通信**: 修改 `InterPluginCommunicator` 以实现跨实例功能

### 测试主从设置 (v3.0新流程)
1. 在 DAW 中加载两个插件实例
2. 在第一个插件中，使用角色选择下拉框选择 "Master"
3. 在第二个插件中，使用角色选择下拉框选择 "Slave"
4. 从插件的 UI 会显示灰色遮罩，显示主插件的状态但不可操作
5. 只有主插件会发送OSC消息，从插件专注于音频处理
6. 将从插件放置在校准软件之前，主插件放置在之后

**角色分工验证:**
- **主插件**: 操作Solo/Mute按钮时发送OSC消息
- **从插件**: 显示相同状态但不发送OSC消息
- **状态同步**: 主插件状态自动同步到从插件

## 关键实现细节

### 参数管理
- 参数根据最大通道数动态创建
- 通道映射在运行时根据活动布局进行
- 未使用的参数会自动绕过

### 状态同步 (v3.0新实现)
- **语义状态同步**: 使用声道名称进行Solo/Mute状态同步
- **角色化处理**: 只有主插件向从插件广播状态
- **增益参数本地化**: 增益/音量参数保持在每个实例本地
- **IPC通信**: 使用 `juce::InterprocessConnection` 实现低延迟同步
- **状态序列化**: 完整的状态序列化/反序列化系统

### UI 行为 (v3.0新架构)
- **独立模式(Standalone)**: 所有控件可操作，发送OSC消息
- **主模式(Master)**: 完全控制，向从实例发送状态，负责OSC通信
- **从模式(Slave)**: UI显示灰色遮罩，只读显示主实例状态，不发送OSC
- **角色选择器**: 下拉框手动选择插件角色，替代自动连接逻辑
- **独奏逻辑**: 自动静音非独奏通道，支持复杂的Solo模式联动

### OSC通信系统 (v3.0关键特性)
- **角色化发送**: 只有主插件和独立插件发送OSC消息
- **从插件限制**: 从插件完全不发送OSC，避免消息重复
- **双向通信**: 支持外部OSC控制和状态广播
- **实时同步**: 状态变化立即通过OSC广播

### 动态主机集成
- `getParameterName()`: 返回与布局相关的参数名称 (例如 "Mute LFE" vs "Mute 4")
- `getInputChannelName()`/`getOutputChannelName()`: 返回特定于通道的名称
- `updateHostDisplay()`: 通知 DAW 参数名称的更改

## 文档资源

### JUCE 框架深度解析
项目在 `Doc/JUCE Wiki/` 中包含了全面的 JUCE 文档：

**核心架构理解:**
- `JUCE-Framework-Overview.md` - 完整的框架模块关系和架构图
- `Audio-Framework.md` - `AudioProcessor`、`AudioDeviceManager` 和插件系统的详细信息
- `Audio-Plugin-System.md` - VST/AU/AAX 插件格式的实现和宿主
- `Component-System.md` - GUI 组件层次结构和事件处理
- `GUI-Framework.md` - `LookAndFeel` 定制和图形渲染

**开发工作流:**
- `CMake-Build-System.md` - 现代构建配置 (优于 Projucer)
- `Projucer.md` - 旧版项目管理工具
- `Development-Tools.md` - 完整的工具链概述
- `Standalone-Plugin-Applications.md` - 独立应用程序开发

**高级主题:**
- `Core-Systems.md` - 内存管理、线程和数据结构
- `String,-ValueTree,-and-File.md` - 数据持久化和序列化
- `OpenGL-Integration.md` - 硬件加速图形
- `Mathematics-and-Geometry.md` - DSP 和几何工具

### JSFX/EEL2 脚本参考
在 `Doc/ReaScript/` 中有完整的 REAPER JSFX 编程文档：

**语言基础:**
- `Introduction.txt` - JSFX 文件结构和基本语法
- `Basic code reference.txt` - EEL2 语言要点、运算符和内存管理
- `Special Variables.txt` - 用于音频处理的内置变量

**音频 & MIDI:**
- `MIDI.txt` - MIDI 消息处理和总线支持
- `Memory Slider FFT MDCT Functions.txt` - DSP 算法和音频缓冲区操作
- `Graphics.txt` - 自定义 UI 绘制和可视化

**集成:**
- `ReaScript API.txt` - REAPER 自动化和宿主交互
- `File IO and Serialization.txt` - JSFX 中的数据持久化
- `Strings.txt` - 文本处理和操作

### 项目特定文档
- `Dev.md` - 综合的监听控制器架构和实现指南
- `Dev Step.md` - 当前的开发路线图和下一步实现步骤
- `Juce插件开发详细指南_.md` - 详细的中文 JUCE 插件开发指南

## 技术说明

### JUCE 最佳实践 (来自文档分析)
- **现代工作流**: 使用 CMake 构建系统而非 Projucer 进行专业开发
- **音频安全**: 在 `processBlock()` 中遵循实时音频约束
- **参数管理**: 使用 `AudioProcessorValueTreeState` 进行线程安全的参数处理
- **跨平台**: 在 JUCE 接口后面抽象平台特定的代码
- **内存管理**: 优先使用 RAII 和智能指针进行资源管理

### JSFX 开发
- **语言**: EEL2 脚本，具有类似 C 的语法但为动态类型
- **集成**: 直接与 REAPER 集成，无需编译
- **特性**: 实时音频处理、MIDI 处理、自定义图形
- **内存**: 约 8M 本地 + 约 1M 全局共享内存空间
- **UI**: 基于向量的自定义绘图，使用立即模式图形

### 性能考量
- **音频线程安全**: 绝不在 `processBlock()` 中分配/释放内存
- **通道映射**: 物理通道迭代与逻辑通道映射
- **状态同步**: 最小化实例之间的通信开销
- **UI 更新**: 基于计时器以避免阻塞音频处理
- **内存访问**: 为 SIMD 操作使用正确的对齐

### 平台支持
- **主要**: Windows 与 Visual Studio 2022 项目
- **跨平台**: JUCE 代码库通过 CMake 支持 macOS/Linux
- **JSFX**: REAPER 特定 (Windows/macOS/Linux)
- **插件格式**: VST3, AU, AAX, 独立运行