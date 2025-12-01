# Context System

> **Relevant source files**
> * [src/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/context.rs)
> * [src/prelude.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs)
> * [src/wrapper/clap/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs)
> * [src/wrapper/vst3/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs)

The Context System provides type-safe interfaces for plugins to communicate with the host application across different plugin APIs. NIH-plug defines three primary context traits—`InitContext`, `ProcessContext`, and `GuiContext`—each available at specific points in the plugin lifecycle and providing appropriate operations for that context. This abstraction layer allows plugins to interact with hosts without knowledge of the underlying plugin format (VST3, CLAP, standalone).

For information about parameter management through contexts, see [Parameter System](/robbert-vdh/nih-plug/2.2-parameter-system). For details on the audio processing lifecycle where `ProcessContext` is used, see [Audio Processing Lifecycle](/robbert-vdh/nih-plug/2.5-audio-processing-lifecycle).

## Purpose and Scope

The Context System serves several critical functions:

1. **Lifecycle-Appropriate Operations**: Each context type exposes only the operations that are safe and meaningful at that point in the plugin lifecycle
2. **Thread Safety**: Contexts enforce thread boundaries, with `InitContext` and `GuiContext` for GUI-thread operations and `ProcessContext` for real-time audio thread operations
3. **API Abstraction**: Plugin code uses the same context traits regardless of whether running as VST3, CLAP, or standalone
4. **Host Communication**: Contexts provide methods for notifying the host of state changes (latency, parameter gestures, resize requests) and scheduling background tasks

## Context Type Overview

```mermaid
flowchart TD

InitContext["InitContext"]
ProcessContext["ProcessContext"]
GuiContext["GuiContext"]
RemoteControlsContext["RemoteControlsContext"]
WrapperInitContext["WrapperInitContext<'a, P>"]
WrapperProcessContext["WrapperProcessContext<'a, P>"]
WrapperGuiContext["WrapperGuiContext"]
RemoteControlPages["RemoteControlPages<'a>"]
Vst3InitContext["WrapperInitContext<'a, P>"]
Vst3ProcessContext["WrapperProcessContext<'a, P>"]
Vst3GuiContext["WrapperGuiContext"]
PluginInit["Plugin::initialize()"]
PluginProcess["Plugin::process()"]
EditorSpawn["Editor::spawn()"]

InitContext --> WrapperInitContext
InitContext --> Vst3InitContext
ProcessContext --> WrapperProcessContext
ProcessContext --> Vst3ProcessContext
GuiContext --> WrapperGuiContext
GuiContext --> Vst3GuiContext
RemoteControlsContext --> RemoteControlPages
WrapperInitContext --> PluginInit
Vst3InitContext --> PluginInit
WrapperProcessContext --> PluginProcess
Vst3ProcessContext --> PluginProcess
WrapperGuiContext --> EditorSpawn
Vst3GuiContext --> EditorSpawn

subgraph subGraph3 ["Plugin Usage"]
    PluginInit
    PluginProcess
    EditorSpawn
end

subgraph subGraph2 ["VST3 Wrapper Implementations (src/wrapper/vst3/context.rs)"]
    Vst3InitContext
    Vst3ProcessContext
    Vst3GuiContext
end

subgraph subGraph1 ["CLAP Wrapper Implementations (src/wrapper/clap/context.rs)"]
    WrapperInitContext
    WrapperProcessContext
    WrapperGuiContext
    RemoteControlPages
end

subgraph subGraph0 ["Context Traits (src/context/)"]
    InitContext
    ProcessContext
    GuiContext
    RemoteControlsContext
end
```

**Sources:** [src/context.rs L1-L30](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/context.rs#L1-L30)

 [src/wrapper/clap/context.rs L19-L56](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L19-L56)

 [src/wrapper/vst3/context.rs L15-L55](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L15-L55)

The context system uses generic associated types, with each context parameterized by the plugin type `P` (which must implement `ClapPlugin` or `Vst3Plugin`). This allows contexts to provide type-safe access to plugin-specific features like the `BackgroundTask` associated type.

| Context Trait | Available During | Thread | Primary Purpose |
| --- | --- | --- | --- |
| `InitContext` | `Plugin::initialize()` | GUI/Main | One-time setup, task executor initialization, initial latency reporting |
| `ProcessContext` | `Plugin::process()` | Audio/Real-time | Event handling, transport info, background task scheduling, latency updates |
| `GuiContext` | `Editor::spawn()` and editor lifetime | GUI/Main | Parameter updates with gestures, resize requests, state save/load |
| `RemoteControlsContext` | `ClapPlugin::remote_controls()` | GUI/Main | Define CLAP remote control pages (CLAP-only) |

## InitContext

`InitContext` is provided during the `Plugin::initialize()` call and allows the plugin to perform one-time setup operations that require communication with the host.

### Core Methods

```mermaid
flowchart TD

InitContext["InitContext"]
plugin_api["plugin_api()"]
execute["execute(task: P::BackgroundTask)"]
set_latency["set_latency_samples(samples: u32)"]
set_voice["set_current_voice_capacity(capacity: u32)"]
PluginApi["Returns: PluginApi enum"]
Executor["Executes on main thread immediately"]
Deferred["Deferred until context drop"]
ClapOnly["CLAP only, updates voice info"]

InitContext --> plugin_api
InitContext --> execute
InitContext --> set_latency
InitContext --> set_voice
plugin_api --> PluginApi
execute --> Executor
set_latency --> Deferred
set_voice --> ClapOnly
```

**Sources:** [src/wrapper/clap/context.rs L76-L93](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L76-L93)

 [src/wrapper/vst3/context.rs L65-L82](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L65-L82)

### Method Descriptions

**`plugin_api(&self) -> PluginApi`**  

Returns the current plugin API (`PluginApi::Clap`, `PluginApi::Vst3`, or `PluginApi::Standalone`). The plugin can use this to enable API-specific behavior or display the current API in an about screen.

**`execute(&self, task: P::BackgroundTask)`**  

Executes a background task immediately on the main thread. During initialization, the audio thread is not yet running, so tasks execute synchronously. This is useful for performing I/O operations or other non-real-time work during plugin initialization.

**`set_latency_samples(&self, samples: u32)`**  

Reports the plugin's processing latency to the host. The latency notification is deferred until the `InitContext` is dropped to avoid reentrancy issues where the host might deactivate and reactivate the plugin during the initialization call itself. See [src/wrapper/clap/context.rs L68-L74](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L68-L74)

 and [src/wrapper/vst3/context.rs L57-L63](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L57-L63)

 for the drop implementation.

**`set_current_voice_capacity(&self, capacity: u32)`**  

Reports the current polyphonic voice capacity. This is only supported by CLAP and is a no-op for VST3. Used by polyphonic plugins to inform the host how many simultaneous voices can be active.

### Deferred Request Pattern

Both wrapper implementations use a `PendingInitContextRequests` struct to defer host notifications until the context is dropped:

```mermaid
sequenceDiagram
  participant Plugin
  participant InitContext
  participant PendingInitContextRequests
  participant Wrapper

  Plugin->>InitContext: set_latency_samples(512)
  InitContext->>PendingInitContextRequests: Store in latency_changed Cell
  note over InitContext: No immediate host call
  Plugin->>InitContext: (initialization continues)
  note over Plugin,InitContext: Plugin::initialize() returns
  InitContext->>InitContext: Drop trait called
  InitContext->>PendingInitContextRequests: Check latency_changed
  PendingInitContextRequests-->>InitContext: Some(512)
  InitContext->>Wrapper: set_latency_samples(512)
  Wrapper->>Wrapper: Notify host
```

**Sources:** [src/wrapper/clap/context.rs L30-L36](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L30-L36)

 [src/wrapper/vst3/context.rs L29-L35](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L29-L35)

This pattern prevents the host from making reentrant calls back into the plugin while it's still initializing, which would be difficult to handle safely in Rust without pervasive interior mutability.

## ProcessContext

`ProcessContext` is provided to the `Plugin::process()` method and allows the plugin to interact with MIDI/note events, access transport information, schedule background tasks, and update latency during processing.

### Core Methods

```mermaid
flowchart TD

ProcessContext["ProcessContext"]
plugin_api["plugin_api()"]
execute_bg["execute_background(task: P::BackgroundTask)"]
execute_gui["execute_gui(task: P::BackgroundTask)"]
transport["transport()"]
next_event["next_event()"]
send_event["send_event(event: PluginNoteEvent"]
set_latency["set_latency_samples(samples: u32)"]
set_voice["set_current_voice_capacity(capacity: u32)"]
BgQueue["Queues to background thread pool"]
GuiQueue["Queues to GUI thread"]
TransportInfo["Returns: &Transport"]
InputEvents["Pops from input_events_guard"]
OutputEvents["Pushes to output_events_guard"]

ProcessContext --> plugin_api
ProcessContext --> execute_bg
ProcessContext --> execute_gui
ProcessContext --> transport
ProcessContext --> next_event
ProcessContext --> send_event
ProcessContext --> set_latency
ProcessContext --> set_voice
execute_bg --> BgQueue
execute_gui --> GuiQueue
transport --> TransportInfo
next_event --> InputEvents
send_event --> OutputEvents
```

**Sources:** [src/wrapper/clap/context.rs L95-L130](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L95-L130)

 [src/wrapper/vst3/context.rs L84-L119](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L84-L119)

### Method Descriptions

**`plugin_api(&self) -> PluginApi`**  

Same as `InitContext::plugin_api()`.

**`execute_background(&self, task: P::BackgroundTask)`**  

Schedules a task to run on a background thread pool. This allows the plugin to perform non-real-time work (file I/O, network requests, expensive computations) without blocking the audio thread. The task is queued using lock-free data structures and will be executed asynchronously. If the queue is full, the task is dropped with a debug assertion.

**`execute_gui(&self, task: P::BackgroundTask)`**  

Schedules a task to run on the GUI/main thread. This is useful when the audio thread needs to trigger GUI updates or perform operations that require main-thread execution. The task is queued similarly to `execute_background()`.

**`transport(&self) -> &Transport`**  

Returns a reference to the current transport state, including playback position, tempo, time signature, and play/record status. The `Transport` struct is populated from host-provided transport information before the process call.

**`next_event(&mut self) -> Option<PluginNoteEvent<P>>`**  

Retrieves the next MIDI or note event from the input event queue. Events are pre-sorted by sample offset. The plugin should call this method repeatedly, processing events at their designated sample offsets to maintain sample-accurate timing.

**`send_event(&mut self, event: PluginNoteEvent<P>)`**  

Sends a MIDI or note event to the host. The event will be delivered to the host's output event stream, allowing the plugin to generate MIDI notes or control messages.

**`set_latency_samples(&self, samples: u32)`**  

Updates the plugin's latency. Unlike `InitContext::set_latency_samples()`, this directly notifies the host during processing, allowing for dynamic latency changes.

**`set_current_voice_capacity(&self, capacity: u32)`**  

Updates polyphonic voice capacity dynamically. CLAP-only.

### Event Queue Management

The `WrapperProcessContext` holds `AtomicRefMut` guards for both input and output event queues throughout the process call:

```mermaid
flowchart TD

CreateContext["Create WrapperProcessContext"]
LockQueues["Lock input/output_events"]
ProcessAudio["Plugin::process()"]
DropContext["Drop WrapperProcessContext"]
InputGuard["input_events_guard: AtomicRefMut"]
OutputGuard["output_events_guard: AtomicRefMut"]
next_event["next_event()"]
send_event["send_event()"]

LockQueues --> InputGuard
LockQueues --> OutputGuard
ProcessAudio --> next_event
ProcessAudio --> send_event
next_event --> InputGuard
send_event --> OutputGuard

subgraph subGraph0 ["Process Call Scope"]
    CreateContext
    LockQueues
    ProcessAudio
    DropContext
    CreateContext --> LockQueues
    LockQueues --> ProcessAudio
    ProcessAudio --> DropContext
end
```

**Sources:** [src/wrapper/clap/context.rs L41-L46](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L41-L46)

 [src/wrapper/vst3/context.rs L40-L45](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L40-L45)

The guards are acquired once at the start of processing and held throughout, avoiding repeated atomic operations for each event access.

## GuiContext

`GuiContext` is provided to the editor through `Editor::spawn()` and allows the GUI to manipulate parameters, request window resizes, and save/load plugin state.

### Core Methods

```mermaid
flowchart TD

GuiContext["GuiContext"]
plugin_api["plugin_api()"]
request_resize["request_resize()"]
raw_begin["raw_begin_set_parameter(param: ParamPtr)"]
raw_set["raw_set_parameter_normalized(param: ParamPtr, normalized: f32)"]
raw_end["raw_end_set_parameter(param: ParamPtr)"]
get_state["get_state()"]
set_state["set_state(state: PluginState)"]
BeginGesture["Starts parameter automation gesture"]
SetValue["Updates parameter value"]
EndGesture["Ends parameter automation gesture"]
Serialize["Serializes current plugin state"]
Deserialize["Deserializes and applies state"]

GuiContext --> plugin_api
GuiContext --> request_resize
GuiContext --> raw_begin
GuiContext --> raw_set
GuiContext --> raw_end
GuiContext --> get_state
GuiContext --> set_state
raw_begin --> BeginGesture
raw_set --> SetValue
raw_end --> EndGesture
get_state --> Serialize
set_state --> Deserialize
```

**Sources:** [src/wrapper/clap/context.rs L132-L243](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L132-L243)

 [src/wrapper/vst3/context.rs L121-L231](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L121-L231)

### Parameter Gesture Protocol

GUI parameter changes must follow a strict three-step protocol:

1. **`raw_begin_set_parameter(param: ParamPtr)`** - Notifies the host that a gesture is beginning (e.g., user clicked a slider)
2. **`raw_set_parameter_normalized(param: ParamPtr, normalized: f32)`** - Sets the parameter value (can be called multiple times during a gesture)
3. **`raw_end_set_parameter(param: ParamPtr)`** - Notifies the host that the gesture is complete (e.g., user released the slider)

```mermaid
sequenceDiagram
  participant User
  participant GUI
  participant GuiContext
  participant Wrapper
  participant Host

  User->>GUI: Mouse down on slider
  GUI->>GuiContext: raw_begin_set_parameter(param_ptr)
  GuiContext->>Wrapper: Queue BeginGesture event
  Wrapper->>Host: Begin automation gesture
  User->>GUI: Drag slider
  GUI->>GuiContext: raw_set_parameter_normalized(param_ptr, 0.5)
  GuiContext->>Wrapper: Queue SetValue event
  note over Wrapper: Parameter updated atomically
  User->>GUI: Continue dragging
  GUI->>GuiContext: raw_set_parameter_normalized(param_ptr, 0.7)
  GuiContext->>Wrapper: Queue SetValue event
  User->>GUI: Mouse up
  GUI->>GuiContext: raw_end_set_parameter(param_ptr)
  GuiContext->>Wrapper: Queue EndGesture event
  Wrapper->>Host: End automation gesture
```

**Sources:** [src/wrapper/clap/context.rs L143-L234](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L143-L234)

 [src/wrapper/vst3/context.rs L137-L221](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L137-L221)

This protocol allows hosts to record automation correctly, distinguishing between the start and end of user interactions versus continuous parameter changes during the interaction.

### CLAP vs VST3 Implementation Differences

**CLAP Implementation** ([src/wrapper/clap/context.rs L143-L206](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L143-L206)

):

* Uses `queue_parameter_event()` to queue `OutputParamEvent` enum variants
* Events are consumed and sent to the host during the audio callback or via explicit flush
* Parameter values are only updated when the output event is actually written

**VST3 Implementation** ([src/wrapper/vst3/context.rs L137-L199](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L137-L199)

):

* Directly calls `IComponentHandler` methods (`begin_edit`, `perform_edit`, `end_edit`)
* Parameters are updated immediately if not currently processing audio (checked via `is_processing` atomic flag)
* Includes a workaround for DAWs like REAPER that silently stop processing when bypassed

### State Management

**`get_state(&self) -> PluginState`**  

Serializes the current plugin state, including all parameter values and persistent fields, into a `PluginState` object. This state can be saved, transmitted, or used for undo/redo.

**`set_state(&self, state: PluginState)`**  

Deserializes and applies a `PluginState`. The wrapper handles the ping-pong pattern for state updates: the GUI sends the state through a zero-capacity channel, the audio thread applies it between processing cycles, and sends it back for deallocation on the GUI thread.

**Sources:** [src/wrapper/clap/context.rs L236-L242](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L236-L242)

 [src/wrapper/vst3/context.rs L224-L230](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L224-L230)

### Debug Assertions for Parameter Gestures

In debug builds, both wrappers include a `ParamGestureChecker` that validates the gesture protocol is followed correctly:

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Gesture : "raw_end_set_parameter()"
    Gesture --> Idle : "raw_end_set_parameter()"
    Idle --> Error : "raw_set_parameter_normalized()"
    Idle --> Error : "raw_end_set_parameter()"
    Gesture --> Error : "raw_set_parameter_normalized()"
    Error --> [*]
```

**Sources:** [src/wrapper/clap/context.rs L54-L55](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L54-L55)

 [src/wrapper/vst3/context.rs L53-L54](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L53-L54)

If the protocol is violated (e.g., setting a parameter without a gesture, or starting a gesture twice), a debug assertion is triggered to help catch bugs during development.

## RemoteControlsContext (CLAP)

`RemoteControlsContext` is a CLAP-specific trait that allows plugins to define remote control pages for hardware controllers. The plugin calls `ClapPlugin::remote_controls()` during initialization, receiving a `RemoteControlPages` implementation.

### Remote Control Structure

```mermaid
flowchart TD

RemoteControlsContext["RemoteControlsContext"]
add_section["add_section(name, f)"]
Section["RemoteControlsSection"]
add_page["add_page(name, f)"]
Page["RemoteControlsPage"]
add_param["add_param(¶m)"]
add_spacer["add_spacer()"]
Pages["Vec"]
PageStruct["clap_remote_controls_page"]
section_name["section_name: [u8; 256]"]
page_id["page_id: clap_id"]
page_name["page_name: [u8; 256]"]
param_ids["param_ids: [clap_id; 8]"]

RemoteControlsContext --> add_section
add_section --> Section
Section --> add_page
add_page --> Page
Page --> add_param
Page --> add_spacer
Page --> PageStruct
Section --> Pages

subgraph subGraph0 ["CLAP Structure"]
    Pages
    PageStruct
    section_name
    page_id
    page_name
    param_ids
    PageStruct --> section_name
    PageStruct --> page_id
    PageStruct --> page_name
    PageStruct --> param_ids
end
```

**Sources:** [src/wrapper/clap/context.rs L324-L378](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L324-L378)

### Usage Pattern

Plugins define remote control pages using a builder pattern:

```rust
fn remote_controls(&self, context: &mut impl RemoteControlsContext) {
    context.add_section("Main", |section| {
        section.add_page("Page 1", |page| {
            page.add_param(&self.params.gain);
            page.add_param(&self.params.frequency);
            page.add_spacer();  // Empty slot
            page.add_param(&self.params.resonance);
        });
    });
}
```

### Automatic Page Splitting

If a page defines more than 8 parameters (the maximum for a single CLAP remote control page), `RemoteControlPages` automatically splits it into multiple pages:

```mermaid
flowchart TD

AddPage["add_page('Controls', params)"]
CheckCount["params.len() > 8?"]
SinglePage["Create single page"]
Split["Split into chunks of 8"]
CreatePage1["Create 'Controls 1'"]
CreatePage2["Create 'Controls 2'"]
CreatePageN["Create 'Controls N'"]
Chunk1["Params 1-8"]
Chunk2["Params 9-16"]
ChunkN["Params N8+1 to N8+8"]

AddPage --> CheckCount
CheckCount --> SinglePage
CheckCount --> Split
Split --> CreatePage1
Split --> CreatePage2
Split --> CreatePageN
CreatePage1 --> Chunk1
CreatePage2 --> Chunk2
CreatePageN --> ChunkN
```

**Sources:** [src/wrapper/clap/context.rs L337-L352](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L337-L352)

This allows plugins to define logical groupings without worrying about the 8-parameter hardware limitation.

## Wrapper Implementation Details

### Wrapper Structure References

Both CLAP and VST3 wrappers contain wrapper implementations that hold references to the wrapper state:

**CLAP Wrapper** ([src/wrapper/clap/context.rs L25-L28](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L25-L28)

):

```
pub(crate) struct WrapperInitContext<'a, P: ClapPlugin> {
    pub(super) wrapper: &'a Wrapper<P>,
    pub(super) pending_requests: PendingInitContextRequests,
}
```

**VST3 Wrapper** ([src/wrapper/vst3/context.rs L24-L27](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L24-L27)

):

```
pub(crate) struct WrapperInitContext<'a, P: Vst3Plugin> {
    pub(super) inner: &'a WrapperInner<P>,
    pub(super) pending_requests: PendingInitContextRequests,
}
```

The `GuiContext` implementations hold `Arc<Wrapper<P>>` references to allow the context to outlive individual method calls and be safely shared with the GUI thread:

**Sources:** [src/wrapper/clap/context.rs L51-L52](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L51-L52)

 [src/wrapper/vst3/context.rs L50-L51](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L50-L51)

### Parameter Pointer to Hash Mapping

Both wrappers use hash maps to translate `ParamPtr` (raw pointers to parameters) to plugin-API-specific identifiers:

```mermaid
flowchart TD

ParamPtr["ParamPtr (raw pointer)"]
HashMap["param_ptr_to_hash: HashMap"]
ClapHash["CLAP param hash"]
Vst3Hash["VST3 param ID"]
ClapEvents["CLAP param events"]
Vst3Calls["VST3 IComponentHandler calls"]

ParamPtr --> HashMap
HashMap --> ClapHash
HashMap --> Vst3Hash
ClapHash --> ClapEvents
Vst3Hash --> Vst3Calls
```

**Sources:** [src/wrapper/clap/context.rs L144-L148](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L144-L148)

 [src/wrapper/vst3/context.rs L139-L143](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L139-L143)

This indirection allows the same `ParamPtr` to be used regardless of the plugin format, with the wrapper translating to the appropriate identifier for the current API.

### Thread Safety and Atomic Operations

The context system enforces thread safety through careful design:

| Operation | Thread | Safety Mechanism |
| --- | --- | --- |
| `InitContext::execute()` | Main/GUI | Synchronous execution via `task_executor` mutex |
| `ProcessContext::execute_background()` | Audio | Lock-free task queue (`ArrayQueue`) |
| `ProcessContext::execute_gui()` | Audio | Lock-free task queue (`ArrayQueue`) |
| `ProcessContext::next_event()` | Audio | Holds `AtomicRefMut` guard for duration of process call |
| `GuiContext::raw_set_parameter_*()` | GUI | Atomic parameter storage, lock-free event queue |
| `GuiContext::set_state()` | GUI | Zero-capacity channel with ping-pong pattern |

**Sources:** [src/wrapper/clap/context.rs L81-L82](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L81-L82)

 [src/wrapper/clap/context.rs L100-L107](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L100-L107)

 [src/wrapper/clap/context.rs L143-L206](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L143-L206)

The parameter change queues and background task queues use `crossbeam::queue::ArrayQueue` for lock-free, wait-free operations that are safe to call from the real-time audio thread.

## Context Lifecycle

```mermaid
sequenceDiagram
  participant Host
  participant Wrapper
  participant Plugin

  note over Host,Plugin: Initialization Phase
  Host->>Wrapper: Instantiate plugin
  Wrapper->>Wrapper: Create WrapperInitContext
  Wrapper->>Plugin: initialize(&mut InitContext)
  Plugin->>Plugin: Setup internal state
  Plugin-->>Wrapper: Return
  Wrapper->>Wrapper: Drop InitContext (apply pending requests)
  note over Host,Plugin: Processing Phase
  loop [Every audio buffer]
    Host->>Wrapper: process(audio_buffer, events)
    Wrapper->>Wrapper: Create WrapperProcessContext
    Wrapper->>Wrapper: Lock event queues
    Wrapper->>Plugin: process(&mut Buffer, &mut ProcessContext)
    Plugin->>Plugin: Read events via next_event()
    Plugin->>Plugin: Process audio
    Plugin->>Plugin: Send events via send_event()
    Plugin-->>Wrapper: Return ProcessStatus
    Wrapper->>Wrapper: Drop ProcessContext (release locks)
    Wrapper->>Host: Return status
    note over Host,Plugin: GUI Phase (parallel with processing)
    Host->>Wrapper: Create editor
    Wrapper->>Wrapper: Create WrapperGuiContext (Arc)
    Wrapper->>Plugin: Editor::spawn(GuiContext)
    Plugin->>Plugin: Build GUI
    Plugin->>Plugin: User interacts with control
    Plugin->>Plugin: raw_begin_set_parameter()
    Plugin->>Plugin: raw_set_parameter_normalized()
    Plugin->>Plugin: raw_end_set_parameter()
  end
```

**Sources:** [src/wrapper/clap/context.rs L19-L56](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L19-L56)

 [src/wrapper/vst3/context.rs L15-L55](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L15-L55)

Each context type has a specific lifetime and purpose:

* `InitContext` lives only during `Plugin::initialize()` and is dropped immediately after
* `ProcessContext` is created and destroyed for each `Plugin::process()` call
* `GuiContext` lives for the entire lifetime of the editor, stored in an `Arc` for shared ownership

This lifecycle design ensures that operations are only available when they make sense and are safe to perform, preventing common plugin development mistakes like calling GUI operations from the audio thread or scheduling background tasks before the plugin is fully initialized.