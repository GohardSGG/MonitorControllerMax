## Background Tasks and Async Execution

Relevant source files
- [src/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/context.rs)
- [src/prelude.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs)
- [src/wrapper/clap/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs)
- [src/wrapper/standalone/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs)
- [src/wrapper/standalone/wrapper.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs)
- [src/wrapper/vst3/context.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs)

This page documents NIH-plug's asynchronous task execution system for operations that must run outside the real-time audio processing thread. The framework provides a type-safe mechanism for scheduling background tasks and GUI updates without blocking audio processing.

## Task Execution Architecture

NIH-plug's async execution model centers around two key types: `P::BackgroundTask` (an associated type defined by the plugin) and `TaskExecutor<P>` (a closure that executes these tasks). The framework provides multiple execution contexts that can schedule tasks on different threads.

#### Task Execution Flow

```
definesprovides via task_executor()runs directlyschedule_background()schedule_gui()wrapsqueuesexecutes via MainThreadExecutorcontainsPlugin Trait ImplementationP::BackgroundTask
(Associated Type)TaskExecutor<P>
(Closure: FnMut(P::BackgroundTask))InitContext::execute()ProcessContext::execute_background()ProcessContext::execute_gui()AsyncExecutor
(execute_background, execute_gui)OsEventLoop<Task<P>, Wrapper>Task<P> Enum
(PluginTask, ParameterValuesChanged)
```

Sources: [src/wrapper/standalone/wrapper.rs 30-91](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L30-L91) [src/wrapper/standalone/wrapper.rs 154-173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L154-L173) [src/prelude.rs 17-22](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs#L17-L22)

## The AsyncExecutor Type

When a plugin creates its editor, it receives an `AsyncExecutor` that provides two execution paths:

| Field | Type | Purpose |
| --- | --- | --- |
| `execute_background` | `Arc<dyn Fn(P::BackgroundTask) + Send + Sync>` | Schedules task on background thread pool |
| `execute_gui` | `Arc<dyn Fn(P::BackgroundTask) + Send + Sync>` | Schedules task on GUI/main thread |

The `AsyncExecutor` is constructed by the wrapper during editor initialization and wraps the wrapper's task scheduling methods:

```
receivesexecute_background closure callsexecute_gui closure callsPlugin::editor(AsyncExecutor)AsyncExecutor { execute_background, execute_gui }Wrapper::schedule_background()Wrapper::schedule_gui()OsEventLoop::schedule_background()OsEventLoop::schedule_gui()
```

Sources: [src/wrapper/standalone/wrapper.rs 265-282](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L265-L282) [src/prelude.rs 20](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs#L20-L20) </old\_str> <new\_str>

## Background Tasks and Async Execution

This page documents NIH-plug's asynchronous task execution system for operations that must run outside the real-time audio processing thread. The framework provides a type-safe mechanism for scheduling background tasks and GUI updates without blocking audio processing.

## Task Execution Architecture

NIH-plug's async execution model centers around two key types: `P::BackgroundTask` (an associated type defined by the plugin) and `TaskExecutor<P>` (a closure that executes these tasks). The framework provides multiple execution contexts that can schedule tasks on different threads.

#### Task Execution Flow

```
definesprovides via task_executor()runs directlyschedule_background()schedule_gui()wrapsqueuesexecutes via MainThreadExecutorcontainsPlugin Trait ImplementationP::BackgroundTask
(Associated Type)TaskExecutor<P>
(Closure: FnMut(P::BackgroundTask))InitContext::execute()ProcessContext::execute_background()ProcessContext::execute_gui()AsyncExecutor
(execute_background, execute_gui)OsEventLoop<Task<P>, Wrapper>Task<P> Enum
(PluginTask, ParameterValuesChanged)
```

Sources: [src/wrapper/standalone/wrapper.rs 30-91](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L30-L91) [src/wrapper/standalone/wrapper.rs 154-173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L154-L173) [src/prelude.rs 17-22](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs#L17-L22)

## The AsyncExecutor Type

When a plugin creates its editor, it receives an `AsyncExecutor` that provides two execution paths:

| Field | Type | Purpose |
| --- | --- | --- |
| `execute_background` | `Arc<dyn Fn(P::BackgroundTask) + Send + Sync>` | Schedules task on background thread pool |
| `execute_gui` | `Arc<dyn Fn(P::BackgroundTask) + Send + Sync>` | Schedules task on GUI/main thread |

The `AsyncExecutor` is constructed by the wrapper during editor initialization and wraps the wrapper's task scheduling methods:

```
receivesexecute_background closure callsexecute_gui closure callsPlugin::editor(AsyncExecutor)AsyncExecutor { execute_background, execute_gui }Wrapper::schedule_background()Wrapper::schedule_gui()OsEventLoop::schedule_background()OsEventLoop::schedule_gui()
```

Sources: [src/wrapper/standalone/wrapper.rs 265-282](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L265-L282) [src/prelude.rs 20](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs#L20-L20)

## The BackgroundTask Associated Type

Every plugin defines a `P::BackgroundTask` associated type that must implement `Send + 'static`. This type represents all tasks that can be executed asynchronously outside the audio thread.

#### BackgroundTask Type System

Sources: [src/prelude.rs 40](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/prelude.rs#L40-L40) [src/wrapper/standalone/wrapper.rs 185](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L185-L185)

## Task Execution Contexts

NIH-plug provides different contexts for executing tasks depending on where the plugin code is running:

#### Context Methods Comparison

| Context | Method | Execution | Thread Safety |
| --- | --- | --- | --- |
| `InitContext` | `execute(task)` | Immediate, blocks caller | Runs on plugin's `TaskExecutor` |
| `ProcessContext` | `execute_background(task)` | Scheduled, non-blocking | Queued to background thread pool |
| `ProcessContext` | `execute_gui(task)` | Scheduled, non-blocking | Queued to GUI/main thread |
| `AsyncExecutor` | `execute_background(task)` | Scheduled, non-blocking | Queued to background thread pool |
| `AsyncExecutor` | `execute_gui(task)` | Scheduled, non-blocking | Queued to GUI/main thread |

#### Context Implementation Details

```
Task Queue and ExecutionProcessContext (Scheduled Execution)InitContext (Immediate Execution)InitContext::execute()task_executor.lock()(task_executor)(task)ProcessContext::execute_background()ProcessContext::execute_gui()Wrapper::schedule_background()Wrapper::schedule_gui()EventLoop::schedule_background()EventLoop::schedule_gui()Task<P>::PluginTask(P::BackgroundTask)MainThreadExecutor::execute()
```

Sources: [src/wrapper/clap/context.rs 81-82](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L81-L82) [src/wrapper/clap/context.rs 100-108](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L100-L108) [src/wrapper/vst3/context.rs 70-71](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L70-L71) [src/wrapper/vst3/context.rs 89-97](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L89-L97) [src/wrapper/standalone/context.rs 44-45](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs#L44-L45) [src/wrapper/standalone/context.rs 62-70](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs#L62-L70)

## Task Scheduling and Event Loops

The wrapper's task scheduling system uses an `OsEventLoop` to manage task execution across threads:

#### Task Scheduling Infrastructure

```
ExecutionEvent Loop MethodsTask EnumWrapper ComponentsPluginTaskParameter*Wrapper<P, B>event_loop: AtomicRefCell<Option<OsEventLoop<Task<P>, Self>>>task_executor: Mutex<TaskExecutor<P>>enum Task<P: Plugin>PluginTask(P::BackgroundTask)ParameterValuesChangedParameterValueChanged(ParamPtr, f32)schedule_background(task) -> boolschedule_gui(task) -> boolMainThreadExecutor::execute()match task(task_executor.lock())(background_task)editor.param_value_changed()
```

Sources: [src/wrapper/standalone/wrapper.rs 48-52](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L48-L52) [src/wrapper/standalone/wrapper.rs 97-107](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L97-L107) [src/wrapper/standalone/wrapper.rs 154-173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L154-L173) [src/wrapper/standalone/wrapper.rs 461-476](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L461-L476)

### Thread Model

NIH-plug's wrappers manage three distinct thread contexts:

1. **Audio Thread**: Real-time audio processing
	- Calls `Plugin::process()` with `ProcessContext`
	- Must not allocate, block, or perform I/O
	- Uses `execute_background()` or `execute_gui()` to schedule async tasks
2. **GUI Thread**: Editor and parameter updates
	- Runs the baseview/egui/vizia event loop
	- Receives tasks scheduled via `execute_gui()` or `AsyncExecutor.execute_gui`
	- Can perform UI-related allocations and I/O
3. **Background Thread Pool**: Long-running operations
	- Executes tasks scheduled via `execute_background()` or `AsyncExecutor.execute_background`
	- Shared across all plugin instances
	- Used for sample loading, file I/O, heavy computation

Sources: [src/wrapper/standalone/wrapper.rs 314-322](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L314-L322)

## Task Queue Implementation

The `OsEventLoop` uses lock-free data structures to enable non-blocking task scheduling from the audio thread:

#### Task Queue Architecture

```
ExecutionScheduling MethodsOsEventLoop Internalspush to queuepush to queue + check threadis_gui_thread = falseis_gui_thread = truetasks: ArrayQueue<Task<P>>executor: Weak<E>is_gui_thread: ThreadIdschedule_background(task)schedule_gui(task)Background Thread PoolGUI/Main Threadexecutor.execute(task, is_gui_thread)
```

The queue capacity is limited to prevent unbounded memory growth. If the queue is full, scheduling returns `false`:

```
// From wrapper code
let task_posted = self.schedule_background(Task::PluginTask(task));
nih_debug_assert!(task_posted, "The task queue is full, dropping task...");
```

Sources: [src/wrapper/standalone/wrapper.rs 461-476](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L461-L476) [src/wrapper/clap/context.rs 101-103](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L101-L103) [src/wrapper/vst3/context.rs 90-92](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/context.rs#L90-L92)

## Parameter Change Communication

Parameter changes from the GUI are communicated to the audio thread via a separate lock-free queue:

#### Parameter Change Flow

Sources: [src/wrapper/standalone/wrapper.rs 26-28](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L26-L28) [src/wrapper/standalone/wrapper.rs 74](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L74-L74) [src/wrapper/standalone/wrapper.rs 408-420](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L408-L420) [src/wrapper/standalone/wrapper.rs 544-558](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L544-L558)

## Real-Time Safety Considerations

### Audio Thread Constraints

The audio thread has strict real-time requirements. During `Plugin::process()`:

- **Forbidden**: Memory allocation, deallocation, I/O operations, blocking locks, system calls
- **Allowed**: Lock-free atomic operations, bounded-time computations, parameter reads
- **Async Operations**: Use `ProcessContext::execute_background()` or `execute_gui()` to delegate non-real-time work

The standalone wrapper uses `process_wrapper()` to wrap the audio callback, which can optionally detect allocations in debug builds.

### Background Task Execution

Tasks executed via `TaskExecutor` or `AsyncExecutor.execute_background`:

- Run on a background thread pool shared across plugin instances
- May allocate memory, perform I/O, block on locks
- Should avoid excessively long operations to maintain responsiveness
- Completed immediately when called from `InitContext::execute()`

### GUI Thread Execution

Tasks executed via `execute_gui()` or `AsyncExecutor.execute_gui`:

- Run on the main/GUI thread (same thread as the editor)
- Used for updating UI state or parameter displays
- Should remain responsive; delegate heavy work to background tasks
- Can safely access editor state and GUI frameworks

Sources: [src/wrapper/standalone/wrapper.rs 503-583](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/wrapper.rs#L503-L583) [src/wrapper/standalone/context.rs 62-70](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/standalone/context.rs#L62-L70) [src/wrapper/clap/context.rs 100-108](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/clap/context.rs#L100-L108)

## Platform-Specific Considerations

NIH-plug handles platform-specific differences in threading models:

### Linux

On Linux, VST3 plugins use a special `RunLoopEventHandler` to execute tasks on the host's GUI thread:

Sources: [src/wrapper/vst3/view.rs 45-49](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/view.rs#L45-L49) [src/wrapper/vst3/view.rs 76-99](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/view.rs#L76-L99) [src/wrapper/vst3/view.rs 469-501](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/vst3/view.rs#L469-L501)

## Buffer Management and Thread Safety

NIH-plug's buffer management system is designed to handle audio data safely across threads:

The `BufferManager` ensures that audio buffers are handled correctly even when multiple threads are involved and that auxiliary buffers are properly zeroed:

Sources: [src/wrapper/util/buffer\_management.rs 11-49](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/util/buffer_management.rs#L11-L49) [src/wrapper/util/buffer\_management.rs 148-156](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/src/wrapper/util/buffer_management.rs#L148-L156) [CHANGELOG.md 299-298](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/CHANGELOG.md#L299-L298)

## Summary

NIH-plug's background task and thread safety system provides a robust foundation for developing audio plugins that can perform non-real-time operations without compromising audio performance. By using the appropriate context methods for executing tasks, plugins can maintain real-time safety in the audio thread while performing necessary operations on background and GUI threads.

<svg id="mermaid-sl8sgwd3y7" width="100%" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 2412 512" style="max-width: 512px;" role="graphics-document document" aria-roledescription="error"><g></g><g><path class="error-icon" d="m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z"></path><path class="error-icon" d="m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z"></path><path class="error-icon" d="m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z"></path><path class="error-icon" d="m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z"></path><path class="error-icon" d="m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z"></path><path class="error-icon" d="m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z"></path><text class="error-text" x="1440" y="250" font-size="150px" style="text-anchor: middle;">Syntax error in text</text> <text class="error-text" x="1250" y="400" font-size="100px" style="text-anchor: middle;">mermaid version 11.6.0</text></g></svg> <svg id="mermaid-n4x5ujhlxk" width="100%" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 2412 512" style="max-width: 512px;" role="graphics-document document" aria-roledescription="error"><g></g><g><path class="error-icon" d="m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z"></path><path class="error-icon" d="m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z"></path><path class="error-icon" d="m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z"></path><path class="error-icon" d="m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z"></path><path class="error-icon" d="m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z"></path><path class="error-icon" d="m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z"></path><text class="error-text" x="1440" y="250" font-size="150px" style="text-anchor: middle;">Syntax error in text</text> <text class="error-text" x="1250" y="400" font-size="100px" style="text-anchor: middle;">mermaid version 11.6.0</text></g></svg> <svg id="mermaid-3aodol2uff9" width="100%" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 2412 512" style="max-width: 512px;" role="graphics-document document" aria-roledescription="error"><g></g><g><path class="error-icon" d="m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z"></path><path class="error-icon" d="m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z"></path><path class="error-icon" d="m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z"></path><path class="error-icon" d="m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z"></path><path class="error-icon" d="m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z"></path><path class="error-icon" d="m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z"></path><text class="error-text" x="1440" y="250" font-size="150px" style="text-anchor: middle;">Syntax error in text</text> <text class="error-text" x="1250" y="400" font-size="100px" style="text-anchor: middle;">mermaid version 11.6.0</text></g></svg> <svg id="mermaid-o0ryg7wq69" width="100%" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 2412 512" style="max-width: 512px;" role="graphics-document document" aria-roledescription="error"><g></g><g><path class="error-icon" d="m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z"></path><path class="error-icon" d="m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z"></path><path class="error-icon" d="m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z"></path><path class="error-icon" d="m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z"></path><path class="error-icon" d="m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z"></path><path class="error-icon" d="m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z"></path><text class="error-text" x="1440" y="250" font-size="150px" style="text-anchor: middle;">Syntax error in text</text> <text class="error-text" x="1250" y="400" font-size="100px" style="text-anchor: middle;">mermaid version 11.6.0</text></g></svg>