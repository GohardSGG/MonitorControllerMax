# Custom Widgets and Visualizations

> **Relevant source files**
> * [plugins/crisp/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/editor.rs)
> * [plugins/diopser/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs)
> * [plugins/examples/gain_gui_vizia/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs)
> * [plugins/spectral_compressor/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs)

This page documents how to create custom widgets and visualizations for NIH-plug GUIs using the VIZIA framework. Custom widgets extend beyond the built-in `ParamSlider`, `ParamButton`, and `GenericUi` components to provide specialized controls and visual feedback. For general editor concepts and the `Editor` trait, see [Editor System Overview](/robbert-vdh/nih-plug/4.1-editor-system-overview). For VIZIA adapter basics and built-in widgets, see [Vizia Integration](/robbert-vdh/nih-plug/4.2-vizia-integration).

## Widget Architecture Overview

Custom widgets in NIH-plug follow VIZIA's `View` trait pattern. Widgets are created during the GUI build phase and can:

* Bind to parameter data through `Arc<dyn Params>` and `ParamPtr`
* React to user input events (mouse, keyboard)
* Render custom graphics using VIZIA's Canvas API
* Observe shared state through `Arc<Mutex<T>>` or `Arc<AtomicCell<T>>`

```mermaid
flowchart TD

BuildPhase["Editor Build Phase<br>create_vizia_editor()"]
CustomWidget["Custom Widget::new()"]
ViewImpl["View Trait Implementation"]
ParamsArc["Arc<DiopserParams>"]
ParamPtr["ParamPtr (e.g., filter_frequency)"]
SharedState["Arc<Mutex<SpectrumOutput>>"]
AtomicState["Arc<AtomicF32> (e.g., sample_rate)"]
ParamWidget["Parameter Control Widgets<br>(XyPad, SafeModeButton)"]
VisWidget["Visualization Widgets<br>(SpectrumAnalyzer, PeakMeter)"]
MouseEvent["on_mouse_down/move/up"]
GuiContext["GuiContext::begin_set_parameter"]
ParamUpdate["Parameter value update"]
DrawMethod["View::draw()"]
Canvas["Canvas API"]
CustomGraphics["Custom graphics rendering"]

ParamsArc --> CustomWidget
ParamPtr --> ParamWidget
SharedState --> VisWidget
AtomicState --> VisWidget
ViewImpl --> ParamWidget
ViewImpl --> VisWidget
ParamWidget --> MouseEvent
VisWidget --> DrawMethod

subgraph Rendering ["Rendering"]
    DrawMethod
    Canvas
    CustomGraphics
    DrawMethod --> Canvas
    Canvas --> CustomGraphics
end

subgraph subGraph3 ["User Interaction"]
    MouseEvent
    GuiContext
    ParamUpdate
    MouseEvent --> GuiContext
    GuiContext --> ParamUpdate
end

subgraph subGraph2 ["Widget Types"]
    ParamWidget
    VisWidget
end

subgraph subGraph1 ["Data Flow"]
    ParamsArc
    ParamPtr
    SharedState
    AtomicState
end

subgraph subGraph0 ["Widget Creation"]
    BuildPhase
    CustomWidget
    ViewImpl
    BuildPhase --> CustomWidget
    CustomWidget --> ViewImpl
end
```

**Sources:** [plugins/diopser/src/editor.rs L1-L237](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L1-L237)

 [plugins/spectral_compressor/src/editor.rs L1-L250](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L1-L250)

## Parameter Control Widgets

Parameter control widgets provide specialized interfaces for modifying plugin parameters. They use `GuiContext` to update parameters in a thread-safe, host-compatible manner.

### XyPad Widget

The XyPad provides two-dimensional parameter control, commonly used for frequency and resonance in Diopser. It maps mouse position to two separate parameters simultaneously.

```mermaid
flowchart TD

XyPadNew["xy_pad::XyPad::new()"]
XParamPtr["X-axis ParamPtr<br>(filter_frequency)"]
YParamPtr["Y-axis ParamPtr<br>(filter_resonance)"]
RenormDisplay["Display renormalization<br>closure"]
RenormEvent["Event renormalization<br>closure"]
MouseDown["on_mouse_down"]
MouseMove["on_mouse_move"]
MouseUp["on_mouse_up"]
Position["Calculate normalized<br>position (0.0-1.0)"]
ApplyRenoorm["Apply event<br>renormalization"]
BeginSet["GuiContext::begin_set_parameter"]
SetNormalized["set_normalized_value"]
EndSet["GuiContext::end_set_parameter"]

Position --> ApplyRenoorm
MouseUp --> EndSet

subgraph subGraph2 ["Parameter Update"]
    ApplyRenoorm
    BeginSet
    SetNormalized
    EndSet
    ApplyRenoorm --> BeginSet
    BeginSet --> SetNormalized
end

subgraph subGraph1 ["Mouse Interaction"]
    MouseDown
    MouseMove
    MouseUp
    Position
    MouseDown --> Position
    MouseMove --> Position
end

subgraph subGraph0 ["XyPad Structure"]
    XyPadNew
    XParamPtr
    YParamPtr
    RenormDisplay
    RenormEvent
    XyPadNew --> XParamPtr
    XyPadNew --> YParamPtr
    XyPadNew --> RenormDisplay
    XyPadNew --> RenormEvent
end
```

The XyPad in Diopser demonstrates advanced features:

* **Dual parameter control**: Maps X-axis to `filter_frequency` and Y-axis to `filter_resonance` [plugins/diopser/src/editor.rs L158-L173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L158-L173)
* **Safe mode clamping**: Uses closures to restrict parameter ranges when safe mode is enabled [plugins/diopser/src/editor.rs L163-L170](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L163-L170)
* **Visual overlay**: Rendered on top of the spectrum analyzer in a `ZStack` [plugins/diopser/src/editor.rs L150-L174](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L150-L174)

**Key implementation detail**: The widget stores `Arc<DiopserParams>` and uses `GuiContext` from the editor's `Data` struct to perform parameter updates. Renormalization closures enable dynamic range restrictions without modifying the underlying parameter definition.

**Sources:** [plugins/diopser/src/editor.rs L158-L173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L158-L173)

### RestrictedParamSlider Widget

A specialized slider that dynamically restricts the parameter range based on external state, used in Diopser to clamp filter stages when safe mode is active.

| Feature | Implementation |
| --- | --- |
| Base widget | Extends standard slider behavior |
| Range restriction | Two closures: display renormalization and event renormalization |
| Visual feedback | Shows restricted range while preserving underlying parameter range |
| Thread safety | Uses `SafeModeClamper` to coordinate with processing thread |

The widget is instantiated with display and event renormalization closures [plugins/diopser/src/editor.rs L200-L212](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L200-L212)

:

```yaml
RestrictedParamSlider::new(
    cx,
    Data::params,
    |params| &params.filter_stages,
    { display_renorm_closure },
    { event_renorm_closure },
)
```

**Sources:** [plugins/diopser/src/editor.rs L200-L212](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L200-L212)

### SafeModeButton Widget

A custom toggle button that coordinates state between GUI and audio processing without using a parameter. This demonstrates handling non-parameter state in custom widgets.

```mermaid
flowchart TD

ButtonNew["SafeModeButton::new()"]
Clamper["SafeModeClamper<br>(Arc<AtomicBool>)"]
ViewImpl["View trait<br>implementation"]
GuiThread["GUI Thread<br>Button click"]
AtomicBool["Arc<AtomicBool><br>safe_mode enabled"]
ProcessThread["Process Thread<br>Read atomic value"]
ButtonState["Button pressed state"]
Binding["Lens binding<br>to AtomicBool"]
Redraw["Trigger redraw"]
FilterStages["filter_stages param<br>clamped to max 40"]
FilterFreq["filter_frequency param<br>range restricted"]
UpdateUI["Update dependent<br>widgets"]

AtomicBool --> Binding
AtomicBool --> FilterStages
AtomicBool --> FilterFreq

subgraph subGraph3 ["Parameter Restrictions"]
    FilterStages
    FilterFreq
    UpdateUI
    FilterStages --> UpdateUI
    FilterFreq --> UpdateUI
end

subgraph subGraph2 ["Visual Feedback"]
    ButtonState
    Binding
    Redraw
    Binding --> ButtonState
    ButtonState --> Redraw
end

subgraph subGraph1 ["State Coordination"]
    GuiThread
    AtomicBool
    ProcessThread
    GuiThread --> AtomicBool
    AtomicBool --> ProcessThread
end

subgraph SafeModeButton ["SafeModeButton"]
    ButtonNew
    Clamper
    ViewImpl
    ButtonNew --> Clamper
    Clamper --> ViewImpl
end
```

The `SafeModeClamper` wraps an `Arc<AtomicBool>` and provides methods for renormalizing parameter values based on the safe mode state [plugins/diopser/src/editor.rs L56](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L56-L56)

 This allows other widgets (XyPad, RestrictedParamSlider) to query the current restriction state and adjust their behavior accordingly.

**Sources:** [plugins/diopser/src/editor.rs L25-L56](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L25-L56)

 [plugins/diopser/src/editor.rs L120](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L120-L120)

## Visualization Widgets

Visualization widgets display audio analysis data or metering information. They typically read from lock-free data structures populated by the audio processing thread.

### SpectrumAnalyzer Widget

Displays real-time frequency spectrum analysis, used in Diopser to visualize the output signal and show filter response overlays.

```mermaid
flowchart TD

ProcessThread["Audio Process Thread"]
StftHelper["StftHelper<br>overlap-add FFT"]
MagnitudeCalc["Calculate bin<br>magnitudes"]
SpectrumOutput["Arc<Mutex<SpectrumOutput>><br>ring buffer"]
AnalyzerNew["analyzer::SpectrumAnalyzer::new()"]
SpectrumArc["Arc<Mutex<SpectrumOutput>>"]
SampleRate["Arc<AtomicF32><br>current sample rate"]
RenormClosure["Frequency renorm<br>closure"]
DrawMethod["View::draw()"]
LockSpectrum["Lock spectrum data"]
MapBinsToFreq["Map FFT bins<br>to frequency axis"]
DrawSpectrum["Draw spectrum curve"]
DrawOverlay["Draw filter overlay<br>(if applicable)"]

SpectrumOutput --> SpectrumArc
SpectrumArc --> DrawMethod
SampleRate --> DrawMethod
RenormClosure --> DrawMethod

subgraph Rendering ["Rendering"]
    DrawMethod
    LockSpectrum
    MapBinsToFreq
    DrawSpectrum
    DrawOverlay
    DrawMethod --> LockSpectrum
    LockSpectrum --> MapBinsToFreq
    MapBinsToFreq --> DrawSpectrum
    MapBinsToFreq --> DrawOverlay
end

subgraph subGraph1 ["Widget State"]
    AnalyzerNew
    SpectrumArc
    SampleRate
    RenormClosure
    AnalyzerNew --> SpectrumArc
    AnalyzerNew --> SampleRate
    AnalyzerNew --> RenormClosure
end

subgraph subGraph0 ["Data Pipeline"]
    ProcessThread
    StftHelper
    MagnitudeCalc
    SpectrumOutput
    ProcessThread --> StftHelper
    StftHelper --> MagnitudeCalc
    MagnitudeCalc --> SpectrumOutput
end
```

The analyzer is created with shared data from the plugin [plugins/diopser/src/editor.rs L151-L156](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L151-L156)

:

```yaml
analyzer::SpectrumAnalyzer::new(
    cx, 
    Data::spectrum,
    Data::sample_rate,
    { renormalization_closure }
)
```

**Key features:**

* **Lock-free updates**: Audio thread writes to `SpectrumOutput` ring buffer, GUI thread reads without blocking
* **Frequency mapping**: Converts FFT bin indices to frequencies using `sample_rate`
* **Dynamic overlays**: Can display filter response curves or other overlays on top of spectrum
* **Efficient rendering**: Only redraws when new spectrum data is available

**Sources:** [plugins/diopser/src/editor.rs L151-L156](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L151-L156)

 [plugins/diopser/src/editor.rs L42-L53](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L42-L53)

### PeakMeter Widget

A built-in VIZIA widget used in example plugins to display audio level metering with decay characteristics.

```mermaid
flowchart TD

ProcessThread["Process Thread<br>Calculate peak level"]
AtomicF32["Arc<AtomicF32><br>peak_meter"]
MeterWidget["PeakMeter::new()"]
LensMap["Lens::map()<br>gain_to_db conversion"]
DecayTime["Decay duration<br>600ms"]
ReadAtomic["Ordering::Relaxed read"]
ConvertDB["util::gain_to_db()"]
VisualUpdate["Update meter display"]
DecayEffect["Visual decay<br>over time"]

AtomicF32 --> ReadAtomic
DecayTime --> DecayEffect

subgraph Display ["Display"]
    ReadAtomic
    ConvertDB
    VisualUpdate
    DecayEffect
    ReadAtomic --> ConvertDB
    ConvertDB --> VisualUpdate
end

subgraph subGraph0 ["PeakMeter Usage"]
    ProcessThread
    AtomicF32
    MeterWidget
    LensMap
    DecayTime
    ProcessThread --> AtomicF32
    AtomicF32 --> MeterWidget
    MeterWidget --> LensMap
    LensMap --> DecayTime
end
```

The gain_gui_vizia example demonstrates typical usage [plugins/examples/gain_gui_vizia/src/editor.rs L52-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs#L52-L59)

:

```yaml
PeakMeter::new(
    cx,
    Data::peak_meter.map(|peak_meter| 
        util::gain_to_db(peak_meter.load(Ordering::Relaxed))
    ),
    Some(Duration::from_millis(600)),
)
```

**Implementation details:**

* **Atomic storage**: Uses `AtomicF32` for lock-free reads from GUI thread
* **Lens mapping**: Converts linear gain to dB in the lens for efficient reactive updates
* **Visual decay**: Optional decay parameter creates smooth visual falloff
* **Built-in widget**: Part of `nih_plug_vizia::widgets`, no custom implementation needed

**Sources:** [plugins/examples/gain_gui_vizia/src/editor.rs L52-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs#L52-L59)

 [plugins/examples/gain_gui_vizia/src/editor.rs L14-L16](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs#L14-L16)

### Analyzer Widget (Spectral Compressor)

A more complex visualization that displays frequency-domain compression activity, showing threshold curves, gain reduction, and spectrum data simultaneously.

**Data structure:**

| Component | Type | Purpose |
| --- | --- | --- |
| `analyzer_data` | `Arc<Mutex<triple_buffer::Output<AnalyzerData>>>` | Triple-buffered spectrum and envelope data |
| `sample_rate` | `Arc<AtomicF32>` | Frequency axis calibration |
| Visual layers | Multiple overlays | Threshold curves, gain reduction, input/output spectra |

The analyzer is instantiated with triple-buffered data [plugins/spectral_compressor/src/editor.rs L226-L231](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L226-L231)

:

```yaml
Analyzer::new(
    cx, 
    Data::analyzer_data,
    Data::sample_rate
)
```

**Advanced features:**

* **Triple buffering**: Uses `triple_buffer` crate for lock-free data exchange between audio and GUI threads
* **Multi-layer rendering**: Displays input spectrum, output spectrum, threshold curves, and gain reduction overlays
* **Editor mode integration**: Can be toggled on/off via `EditorMode` enum [plugins/spectral_compressor/src/editor.rs L48-L57](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L48-L57)
* **Dynamic sizing**: Expands GUI width when visible, collapses when hidden [plugins/spectral_compressor/src/editor.rs L74-L79](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L74-L79)

**Sources:** [plugins/spectral_compressor/src/editor.rs L226-L231](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L226-L231)

 [plugins/spectral_compressor/src/editor.rs L48-L57](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L48-L57)

 [plugins/spectral_compressor/src/editor.rs L66-L69](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L66-L69)

## Widget Implementation Patterns

### View Trait Implementation

All custom VIZIA widgets must implement the `View` trait. The minimal implementation requires defining how the widget is built and optionally how it draws custom graphics.

```mermaid
flowchart TD

StructDef["struct MyWidget {<br>  data: Arc<WidgetData>,<br>  state: WidgetState<br>}"]
NewMethod["impl MyWidget {<br>  pub fn new(cx: &mut Context)<br>}"]
ViewImpl["impl View for MyWidget"]
EventMethod["fn event(&mut self, cx, event)<br>Handle user input"]
DrawMethod["fn draw(&self, cx, canvas)<br>Custom rendering"]
BuildTree["Build view tree<br>cx.add_view()"]
Styling["Apply CSS classes<br>.class(), .id()"]
Layout["Layout properties<br>.width(), .height()"]

NewMethod --> ViewImpl
NewMethod --> BuildTree

subgraph subGraph2 ["VIZIA Integration"]
    BuildTree
    Styling
    Layout
    BuildTree --> Styling
    Styling --> Layout
end

subgraph subGraph1 ["View Trait"]
    ViewImpl
    EventMethod
    DrawMethod
    ViewImpl --> EventMethod
    ViewImpl --> DrawMethod
end

subgraph subGraph0 ["Widget Definition"]
    StructDef
    NewMethod
    StructDef --> NewMethod
end
```

**Key methods:**

* `new()`: Constructor that calls `Self { ... }.build(cx)` to add the widget to the view tree
* `event()`: Handles mouse, keyboard, and other events
* `draw()`: Custom canvas rendering for complex graphics

**Sources:** [plugins/diopser/src/editor.rs L158-L173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L158-L173)

 (XyPad instantiation)

### Data Binding with Lens

Widgets observe state changes using VIZIA's `Lens` trait, which provides reactive data binding.

**Common binding patterns:**

| Pattern | Code | Use Case |
| --- | --- | --- |
| Direct parameter | `Data::params` | Access entire `Params` struct |
| Param getter | `Data::params.map(\|p\| &p.gain)` | Specific parameter |
| Atomic value | `Data::peak_meter.map(\|m\| m.load())` | Shared atomic state |
| Nested state | `Data::params.map(\|p\| p.global.clone())` | Nested parameter groups |

The `Data` struct serves as the lens root and must implement `Lens` [plugins/diopser/src/editor.rs L47-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L47-L59)

:

```css
#[derive(Lens, Clone)]
pub(crate) struct Data {
    pub(crate) params: Arc<DiopserParams>,
    pub(crate) sample_rate: Arc<AtomicF32>,
    pub(crate) spectrum: Arc<Mutex<SpectrumOutput>>,
    pub(crate) safe_mode_clamper: SafeModeClamper,
}
```

**Reactive updates**: VIZIA automatically triggers widget redraws when lens-bound data changes, eliminating manual update logic.

**Sources:** [plugins/diopser/src/editor.rs L47-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L47-L59)

 [plugins/spectral_compressor/src/editor.rs L59-L69](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L59-L69)

### Event Handling

Custom widgets handle user input by implementing the `event()` method from the `View` trait.

**Event flow for parameter modification:**

```mermaid
sequenceDiagram
  participant User
  participant Widget
  participant GuiContext
  participant ParamPtr
  participant AudioThread

  User->>Widget: Mouse down at position
  Widget->>Widget: Calculate normalized value (0.0-1.0)
  Widget->>GuiContext: begin_set_parameter(param_ptr)
  GuiContext->>ParamPtr: Mark as touched
  User->>Widget: Mouse drag
  Widget->>Widget: Update normalized value
  Widget->>ParamPtr: set_normalized_value()
  ParamPtr->>AudioThread: Update atomic value
  User->>Widget: Mouse up
  Widget->>GuiContext: end_set_parameter(param_ptr)
  GuiContext->>ParamPtr: Mark as released
```

**Critical methods:**

* `GuiContext::begin_set_parameter(param)`: Notify host that parameter editing started
* `ParamPtr::set_normalized_value(value)`: Update parameter atomically
* `GuiContext::end_set_parameter(param)`: Notify host that parameter editing ended

This sequence ensures:

* Host knows when automation should be overridden
* Parameter changes are thread-safe
* Undo/redo points are created correctly
* Host automation lanes update appropriately

**Sources:** Pattern inferred from [plugins/diopser/src/editor.rs L158-L173](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L158-L173)

### Drawing Custom Graphics

Complex visualizations use VIZIA's Canvas API in the `draw()` method.

**Canvas drawing primitives:**

| Primitive | Method | Usage |
| --- | --- | --- |
| Path | `Path::new()` | Define shapes and curves |
| Stroke | `canvas.stroke_path()` | Draw outlines |
| Fill | `canvas.fill_path()` | Fill shapes |
| Text | `canvas.fill_text()` | Render text labels |
| Transform | `canvas.translate()`, `canvas.scale()` | Coordinate transformations |

**Typical rendering sequence:**

1. Lock shared data (spectrum buffer, analyzer data)
2. Map data coordinates to canvas coordinates
3. Build path with `moveTo()` and `lineTo()` operations
4. Apply paint style (color, stroke width)
5. Render path with `stroke_path()` or `fill_path()`
6. Draw overlays (grid lines, labels, cursors)

**Performance considerations:**

* Minimize data copies during rendering
* Use pre-allocated buffers where possible
* Limit redraw frequency with timers or change detection
* Batch drawing operations to reduce canvas API calls

**Sources:** Pattern inferred from analyzer widget usage in [plugins/diopser/src/editor.rs L151-L156](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L151-L156)

 [plugins/spectral_compressor/src/editor.rs L226-L231](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L226-L231)

### Integration with Plugin Data

Custom widgets integrate with plugin processing through shared state:

```mermaid
flowchart TD

PluginProcess["Plugin::process()"]
StftCalc["STFT/FFT calculation"]
MeterCalc["Peak/RMS calculation"]
SpectrumBuffer["Arc<Mutex<SpectrumOutput>>"]
AtomicMeters["Arc<AtomicF32>"]
TripleBuffer["triple_buffer::Output<T>"]
CustomWidget["Custom Widget"]
DrawLoop["Redraw loop<br>60 FPS typical"]
LockData["Lock/read data"]
Render["Render visualization"]

StftCalc --> SpectrumBuffer
StftCalc --> TripleBuffer
MeterCalc --> AtomicMeters
SpectrumBuffer --> CustomWidget
AtomicMeters --> CustomWidget
TripleBuffer --> CustomWidget

subgraph subGraph2 ["GUI Thread"]
    CustomWidget
    DrawLoop
    LockData
    Render
    CustomWidget --> DrawLoop
    DrawLoop --> LockData
    LockData --> Render
end

subgraph subGraph1 ["Shared State"]
    SpectrumBuffer
    AtomicMeters
    TripleBuffer
end

subgraph subGraph0 ["Plugin Core"]
    PluginProcess
    StftCalc
    MeterCalc
    PluginProcess --> StftCalc
    PluginProcess --> MeterCalc
end
```

**Data sharing strategies:**

1. **Mutex-protected buffers**: Simple but can cause contention. Use for infrequent updates [plugins/diopser/src/editor.rs L53](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L53-L53)
2. **Atomic values**: Best for simple metrics (peak level, sample rate) [plugins/examples/gain_gui_vizia/src/editor.rs L15](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs#L15-L15)
3. **Triple buffering**: Lock-free for complex data structures. Audio thread writes, GUI reads [plugins/spectral_compressor/src/editor.rs L66](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L66-L66)
4. **Ring buffers**: Time-series data with automatic overwrite behavior

**Thread safety guarantees:**

* Audio processing never blocks on GUI locks
* GUI reads are non-blocking or use timeouts
* Data structures are sized to prevent allocation in audio thread
* Updates are atomic or use lock-free primitives

**Sources:** [plugins/diopser/src/editor.rs L47-L57](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs#L47-L57)

 [plugins/spectral_compressor/src/editor.rs L59-L69](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs#L59-L69)

 [plugins/examples/gain_gui_vizia/src/editor.rs L12-L16](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs#L12-L16)