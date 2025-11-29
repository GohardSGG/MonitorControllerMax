# Plugin Wrappers

> **Relevant source files**
> * [nih_plug_derive/src/lib.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/nih_plug_derive/src/lib.rs)
> * [src/params.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/params.rs)
> * [src/wrapper/clap/wrapper.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs)
> * [src/wrapper/vst3.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3.rs)
> * [src/wrapper/vst3/inner.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs)
> * [src/wrapper/vst3/wrapper.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/wrapper.rs)

## Purpose

Plugin Wrappers are a crucial part of NIH-plug that adapt plugins implementing the core NIH-plug API to different plugin formats (VST3, CLAP) and standalone applications. They serve as the bridge between the plugin's internal implementation and the host environment, handling format-specific interfaces, audio processing, parameter management, and GUI integration.

For detailed information about specific wrapper implementations, see [VST3 Wrapper](/robbert-vdh/nih-plug/3.1-vst3-wrapper), [CLAP Wrapper](/robbert-vdh/nih-plug/3.2-clap-wrapper), and [Standalone Mode](/robbert-vdh/nih-plug/3.3-standalone-wrapper).

## Architecture Overview

The Plugin Wrappers system follows a consistent pattern across all supported formats while accommodating format-specific requirements.

```mermaid
flowchart TD

Plugin["Plugin Trait"]
Params["Params"]
Editor["Editor"]
VST3["VST3Wrapper"]
CLAP["CLAPWrapper"]
Standalone["StandaloneWrapper"]
Context1["Context Objects"]
Context2["Context Objects"]
Context3["Context Objects"]
VST3Host["VST3 Host"]
CLAPHost["CLAP Host"]
OS["Operating System"]

Plugin --> VST3
Plugin --> CLAP
Plugin --> Standalone
Params --> VST3
Params --> CLAP
Params --> Standalone
Editor --> VST3
Editor --> CLAP
Editor --> Standalone
VST3 --> VST3Host
CLAP --> CLAPHost
Standalone --> OS

subgraph subGraph2 ["Plugin Formats"]
    VST3Host
    CLAPHost
    OS
end

subgraph subGraph1 ["NIH-plug Wrappers"]
    VST3
    CLAP
    Standalone
    Context1
    Context2
    Context3
    VST3 --> Context1
    CLAP --> Context2
    Standalone --> Context3
end

subgraph subGraph0 ["Plugin Implementation"]
    Plugin
    Params
    Editor
end
```

Sources: [src/wrapper/vst3/wrapper.rs L50-L52](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/wrapper.rs#L50-L52)

 [src/wrapper/clap/wrapper.rs L103-L257](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L103-L257)

 [src/wrapper/standalone/wrapper.rs L30-L91](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L30-L91)

## Common Wrapper Components

Despite differences in implementation, all wrappers share common structural elements:

| Component | Purpose |
| --- | --- |
| Wrapper struct | Main container for plugin instance and state |
| Context objects | Provide plugin with interfaces for initialization, processing, and GUI |
| Parameter mapping | Translate between NIH-plug parameters and format-specific representations |
| Event handling | Manage MIDI, note, and parameter change events |
| Buffer management | Convert between host's audio buffers and plugin's expected format |
| Task execution | Coordinate tasks across audio, GUI, and background threads |
| State management | Serialize and deserialize plugin state |

Sources: [src/wrapper/vst3/inner.rs L30-L140](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs#L30-L140)

 [src/wrapper/clap/wrapper.rs L103-L257](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L103-L257)

 [src/wrapper/standalone/wrapper.rs L30-L91](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L30-L91)

## Plugin Lifecycle Flow

The following diagram illustrates how wrappers mediate between the host and plugin throughout the plugin lifecycle:

```mermaid
sequenceDiagram
  participant Host (DAW/OS)
  participant Plugin Wrapper
  participant NIH-plug Plugin

  Host (DAW/OS)->>Plugin Wrapper: Initialize
  Plugin Wrapper->>NIH-plug Plugin: initialize(audio_io_layout, buffer_config, init_context)
  NIH-plug Plugin-->>Plugin Wrapper: Return initialization status
  Plugin Wrapper-->>Host (DAW/OS): Return status
  Host (DAW/OS)->>Plugin Wrapper: Set Processing State (Activate)
  Plugin Wrapper->>NIH-plug Plugin: reset()
  Host (DAW/OS)->>Plugin Wrapper: Process Audio
  Plugin Wrapper->>NIH-plug Plugin: process(buffer, aux, process_context)
  NIH-plug Plugin-->>Plugin Wrapper: Return ProcessStatus
  Plugin Wrapper-->>Host (DAW/OS): Return status
  Host (DAW/OS)->>Plugin Wrapper: Parameter Change
  Plugin Wrapper->>NIH-plug Plugin: Update parameter value
  Plugin Wrapper->>NIH-plug Plugin: Notify editor (Task::ParameterValueChanged)
  Host (DAW/OS)->>Plugin Wrapper: Open Editor
  Plugin Wrapper->>NIH-plug Plugin: editor(AsyncExecutor)
  NIH-plug Plugin-->>Plugin Wrapper: Return Editor
  Plugin Wrapper-->>Host (DAW/OS): Return View implementation
  Host (DAW/OS)->>Plugin Wrapper: Save State
  Plugin Wrapper->>NIH-plug Plugin: Serialize parameters and persistent fields
  NIH-plug Plugin-->>Plugin Wrapper: Return serialized state
  Plugin Wrapper-->>Host (DAW/OS): Return state data
  Host (DAW/OS)->>Plugin Wrapper: Load State
  Plugin Wrapper->>NIH-plug Plugin: Deserialize state
  Plugin Wrapper->>NIH-plug Plugin: Update parameters
  Plugin Wrapper-->>Host (DAW/OS): Return status
  Host (DAW/OS)->>Plugin Wrapper: Set Processing State (Deactivate)
  Plugin Wrapper->>NIH-plug Plugin: deactivate()
```

Sources: [src/wrapper/vst3/wrapper.rs L359-L409](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/wrapper.rs#L359-L409)

 [src/wrapper/clap/wrapper.rs L570-L602](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L570-L602)

 [src/wrapper/standalone/wrapper.rs L275-L301](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L275-L301)

## Context Objects

Each wrapper provides context objects that expose format-specific capabilities to the plugin:

```mermaid
flowchart TD

InitContext["WrapperInitContext<br>(initialization phase)"]
ProcessContext["WrapperProcessContext<br>(audio processing phase)"]
GuiContext["WrapperGuiContext<br>(editor interaction)"]
PluginInit["Plugin::initialize()"]
PluginProcess["Plugin::process()"]
EditorInterface["Editor functions"]
Host1["Host"]
Host2["Host"]
Host3["Host"]

InitContext --> PluginInit
ProcessContext --> PluginProcess
GuiContext --> EditorInterface
InitContext --> Host1
ProcessContext --> Host2
GuiContext --> Host3

subgraph subGraph1 ["Plugin Interface"]
    PluginInit
    PluginProcess
    EditorInterface
end

subgraph subGraph0 ["Wrapper Context Objects"]
    InitContext
    ProcessContext
    GuiContext
end
```

Sources: [src/wrapper/vst3/inner.rs L356-L382](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs#L356-L382)

 [src/wrapper/clap/wrapper.rs L730-L756](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L730-L756)

 [src/wrapper/standalone/context.rs L11-L162](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs#L11-L162)

## Parameter System Integration

The wrappers bridge between NIH-plug's parameter system and host-specific parameter representations:

```mermaid
flowchart TD

ParamTrait["Param Trait"]
ParamsTrail["Params Trait"]
Params["Plugin's Params Implementation"]
ParamPtr["ParamPtr (internal reference)"]
ParamMaps["Parameter Maps:<br>- param_by_hash<br>- param_id_by_hash<br>- param_id_to_hash<br>- param_ptr_to_hash"]
EventQueue["Parameter Event Queue"]
HostParams["Host Parameter System"]
Automation["Automation & Modulation"]
GUI["Host GUI"]

ParamPtr --> ParamMaps
EventQueue --> HostParams
Automation --> ParamMaps
GUI --> ParamMaps
ParamMaps --> ParamPtr

subgraph Host ["Host"]
    HostParams
    Automation
    GUI
    HostParams --> Automation
    HostParams --> GUI
end

subgraph Wrapper ["Wrapper"]
    ParamMaps
    EventQueue
    ParamMaps --> EventQueue
end

subgraph NIH-plug ["NIH-plug"]
    ParamTrait
    ParamsTrail
    Params
    ParamPtr
    ParamTrait --> Params
    ParamsTrail --> Params
    Params --> ParamPtr
end
```

Sources: [src/wrapper/vst3/inner.rs L122-L139](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs#L122-L139)

 [src/wrapper/clap/wrapper.rs L193-L214](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L193-L214)

 [src/params.rs L76-L192](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/params.rs#L76-L192)

## Event Handling

Wrappers manage several types of event flows:

1. **MIDI and Note Events**: From host to plugin and back
2. **Parameter Changes**: From host to plugin and from plugin's GUI to host
3. **Task Execution**: For background processing and GUI updates

```mermaid
flowchart TD

Plugin["Plugin"]
HostParam["Host Parameter Change"]
ParamQueue["Parameter Event Queue"]
PluginParam["Plugin Parameter"]
ParamNotify["Parameter Change Notification"]
PluginGUI["Plugin GUI"]
GUIChange["GUI Parameter Change"]
HostParamOut["Host Parameter Update"]
BackgroundTask["Background Task"]
TaskQueue["Task Queue"]
ExecuteTask["Execute Task"]
GUITask["GUI Task"]
MainThreadQueue["Main Thread Queue"]
ExecuteGUI["Execute on GUI Thread"]
HostNoteIn["Host MIDI/Note Input"]
InputEvents["Input Event Queue"]
PluginProcess["Plugin::process()"]
OutputEvents["Output Event Queue"]
HostNoteOut["Host MIDI/Note Output"]

subgraph subGraph3 ["Event Flows"]
    PluginGUI --> GUITask

subgraph Tasks ["Tasks"]
    Plugin
    BackgroundTask
    TaskQueue
    ExecuteTask
    GUITask
    MainThreadQueue
    ExecuteGUI
    Plugin --> BackgroundTask
    BackgroundTask --> TaskQueue
    TaskQueue --> ExecuteTask
    GUITask --> MainThreadQueue
    MainThreadQueue --> ExecuteGUI
end

subgraph subGraph1 ["Parameter Events"]
    HostParam
    ParamQueue
    PluginParam
    ParamNotify
    PluginGUI
    GUIChange
    HostParamOut
    HostParam --> ParamQueue
    ParamQueue --> PluginParam
    PluginParam --> ParamNotify
    ParamNotify --> PluginGUI
    PluginGUI --> GUIChange
    GUIChange --> HostParamOut
end

subgraph subGraph0 ["Note Events"]
    HostNoteIn
    InputEvents
    PluginProcess
    OutputEvents
    HostNoteOut
    HostNoteIn --> InputEvents
    InputEvents --> PluginProcess
    PluginProcess --> OutputEvents
    OutputEvents --> HostNoteOut
end
end
```

Sources: [src/wrapper/vst3/inner.rs L91-L109](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs#L91-L109)

 [src/wrapper/clap/wrapper.rs L142-L282](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L142-L282)

## Export Macros

Each wrapper provides a macro for exporting plugins in the respective format:

```mermaid
flowchart TD

Plugin["Plugin Implementation"]
Export["Export Macros"]
VST3Export["nih_export_vst3!()"]
CLAPExport["nih_export_clap!()"]
StandaloneExport["nih_export_standalone!()"]
VST3Entry["VST3 Entry Points:<br>- GetPluginFactory()<br>- ModuleEntry()/bundleEntry()<br>- ModuleExit()/bundleExit()"]
CLAPEntry["CLAP Entry Points:<br>- clap_entry<br>- clap_init()<br>- clap_deinit()<br>- clap_create_plugin_factory()"]
StandaloneEntry["Standalone Main:<br>- main()<br>- run_standalone()"]

Plugin --> Export
Export --> VST3Export
Export --> CLAPExport
Export --> StandaloneExport
VST3Export --> VST3Entry
CLAPExport --> CLAPEntry
StandaloneExport --> StandaloneEntry
```

Sources: [src/wrapper/vst3.rs L20-L244](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3.rs#L20-L244)

 [src/wrapper/standalone/wrapper.rs L305-L393](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L305-L393)

## Wrapper-specific Features

Each wrapper implementation has unique characteristics:

### VST3 Wrapper

* Implements multiple VST3 interfaces: `IComponent`, `IEditController`, `IAudioProcessor`, etc.
* Uses COM-style reference counting
* Manages parameter change queues with sample-accurate automation
* Handles note expression controllers for extended MIDI expression

Sources: [src/wrapper/vst3/wrapper.rs L41-L702](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/wrapper.rs#L41-L702)

 [src/wrapper/vst3/inner.rs L30-L579](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/inner.rs#L30-L579)

### CLAP Wrapper

* Implements CLAP's extensible plugin interface
* Supports polyphonic modulation and voice management
* Handles parameter gestures and automation
* Provides remote control pages for hardware controllers
* Implements CLAP-specific GUI integration

Sources: [src/wrapper/clap/wrapper.rs L103-L788](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L103-L788)

### Standalone Wrapper

* Uses platform-specific audio backends
* Creates a window for the plugin's GUI
* Simulates a host environment for testing
* Manages audio thread separate from the GUI thread
* Provides simplified parameter management

Sources: [src/wrapper/standalone/wrapper.rs L30-L620](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L30-L620)

 [src/wrapper/standalone/context.rs L11-L162](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs#L11-L162)

## Parameter Translation Tables

The wrappers translate between NIH-plug's parameter system and format-specific parameter representations:

| NIH-plug Parameter Type | VST3 Representation | CLAP Representation | Standalone Representation |
| --- | --- | --- | --- |
| `FloatParam` | `ParameterInfo` with flags | `clap_param_info` | Internal parameter |
| `IntParam` | `ParameterInfo` with step count | `clap_param_info` with steps | Internal parameter |
| `BoolParam` | `ParameterInfo` with step count = 1 | `clap_param_info` with flags | Internal parameter |
| `EnumParam` | `ParameterInfo` with step count | `clap_param_info` with steps | Internal parameter |

Sources: [src/wrapper/vst3/wrapper.rs L525-L590](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/wrapper.rs#L525-L590)

 [src/wrapper/clap/wrapper.rs L193-L214](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/wrapper.rs#L193-L214)

 [src/params.rs L27-L54](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/params.rs#L27-L54)

## Conclusion

The Plugin Wrappers system is a central component of NIH-plug that enables plugin compatibility across multiple formats. By implementing format-specific interfaces while presenting a consistent API to plugins, wrappers allow developers to focus on their plugin's functionality rather than the intricacies of each plugin format.