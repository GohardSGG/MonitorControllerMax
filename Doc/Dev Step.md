

### **项目重构修改计划 (基于 Dev.md V2)**

#### **第零步：项目环境准备**

1.  **添加 `juce_data_structures` 模块**:
    *   **操作**: 打开 `.jucer` 文件，进入 "Modules" 部分，确保 `juce_data_structures` 模块已被添加。
    *   **原因**: 这是使用 `juce::JSON` 解析 `Speaker_Config.json` 的前置条件。
    *   **验证**: 保存 Projucer 项目并重新在IDE中加载后，`#include <juce/juce_data_structures.h>` 不应报错。

#### **第一步：实现配置解析 (新建 `ConfigManager` 类)**

1.  **创建数据模型头文件**:
    *   **操作**: 在 `Source` 目录下创建一个新文件 `ConfigModels.h`。
    *   **内容**: 在此文件中定义 `ChannelInfo` 和 `Layout` 两个 `struct`，如 `Dev.md` 中所述。
    *   **原因**: 将数据模型分离，使代码更清晰。

2.  **创建 `ConfigManager` 类**:
    *   **操作**: 在 `Source` 目录下创建 `ConfigManager.h` 和 `ConfigManager.cpp` 两个新文件。
    *   **`ConfigManager.h`**: 声明 `ConfigManager` 类，包含一个 `loadConfig()` 方法，以及 `getSpeakerLayoutNames()`, `getSubLayoutNames()`, `getLayoutFor()` 等公共接口。它会持有一个解析后的JSON对象和布局数据。
    *   **`ConfigManager.cpp`**: 实现构造函数，在其中调用 `loadConfig()`。实现 `loadConfig()`，使用 `juce::JSON::parse()` 来解析位于特定路径的 `Speaker_Config.json` 文件，并将解析结果存入成员变量。实现各个 `get...()` 方法。
    *   **最后**: 将这四个新文件 (`.h` 和 `.cpp`) 添加到 Projucer 项目的 `Source` 组中并保存。

#### **第二步：重构音频处理器 (后端)**

1.  **修改 `PluginProcessor.h`**:
    *   **操作**: 包含 `"ConfigManager.h"`。添加一个 `ConfigManager` 的成员变量。
    *   **原因**: 使处理器能够访问配置信息。

2.  **修改 `PluginProcessor.cpp`**:
    *   **构造函数**:
        *   **BusesProperties**: 插件的总线大小不再是固定的 `create7point1()`，而是应该通过 `configManager` 找到所有配置中的最大通道数，然后使用 `juce::AudioChannelSet::discreteChannels()` 来设置。
    *   **`createParameterLayout()` 函数**:
        *   **替换循环**: 删除原来 `for (int i = 0; i < numManagedChannels; ++i)` 的循环。
        *   **新逻辑**: 调用 `configManager` 获取所有配置中的最大通道索引。基于这个最大索引进行循环，创建所有可能用到的 Mute/Solo/Gain 参数。
    *   **`parameterChanged()` 和 `processBlock()`**:
        *   **保持不变**: 这两个函数现有的逻辑是基于最大通道数来工作的，因此无需修改。

#### **第三步：重构用户界面 (前端)**

这是改动最大的部分，我们将几乎完全重写 `PluginEditor`。

1.  **修改 `PluginEditor.h`**:
    *   **删除旧成员**: 删除 `globalMuteButton`, `globalSoloButton` 之外的所有 `std::array` 成员变量（用于存按钮、滑块、FlexBox等）。
    *   **新增成员**:
        *   新增一个 `dimButton`。
        *   新增两个 `juce::ComboBox` (`speakerLayoutSelector`, `subLayoutSelector`)。
        *   新增一个 `std::map<int, std::unique_ptr<juce::TextButton>> channelButtons`，用于按需存储通道按钮。
        *   新增一个 `std::map<int, std::unique_ptr<ButtonAttachment>> channelButtonAttachments`。
        *   新增一个 `ConfigManager&` 引用，指向处理器的 `configManager`。
        *   新增一个私有函数 `void updateLayout();`。

2.  **修改 `PluginEditor.cpp`**:
    *   **构造函数**:
        *   **删除旧逻辑**: 删除所有创建16个通道按钮和滑块的循环。
        *   **新增逻辑**:
            *   初始化 `Solo`, `Dim`, `Mute` 三个全局按钮。
            *   初始化两个 `ComboBox`，并使用 `configManager` 的 `get...Names()` 方法填充它们的选项。
            *   为 `ComboBox` 添加 `onChange` 监听器，让它们在被改变时调用 `updateLayout()`。
            *   在构造函数的最后，手动调用一次 `updateLayout()` 来绘制默认界面。
    *   **`resized()` 函数**:
        *   **完全重写**: 删除旧的 `FlexBox` 逻辑。
        *   **新逻辑**:
            *   使用 `juce::Grid` 创建一个 `1行 x 2列` 的主布局（左侧边栏，右侧主网格）。
            *   将 `Solo`, `Dim`, `Mute` 按钮放入左侧边栏的 `GridItem`。
            *   将右上角的 `ComboBox` 和5x5的通道网格放入右侧的 `GridItem`。
            *   **注意**: `resized` 只负责定义网格的“结构”，不负责填充通道按钮。
    *   **实现 `updateLayout()` 函数**:
        *   **核心逻辑**:
            1.  从两个 `ComboBox` 获取当前选择的布局名称。
            2.  调用 `configManager.getLayoutFor()` 获取对应的 `Layout` 数据对象。
            3.  **隐藏并禁用**当前 `channelButtons` `map` 中的所有按钮。
            4.  遍历从`configManager`获取到的 `channels` 列表。
            5.  对于列表中的每个 `channelInfo`：
                *   检查 `channelButtons` `map` 中是否已存在该通道索引的按钮。如果不存在，就 `new` 一个并存入 `map`。
                *   获取该按钮的指针，设置它的文本为 `channelInfo.name` (例如 "L", "C", "LFE")。
                *   使其可见并启用。
                *   **关键**: 创建一个 `juce::GridItem`，并使用 `withGridArea()` 方法，将其精确地放置到 `channelInfo.gridPosition` 指定的5x5网格位置上。
            6.  处理 `SUB` 按钮的特殊逻辑。
