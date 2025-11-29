# Crisp Plugin

> **Relevant source files**
> * [plugins/crisp/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/editor.rs)
> * [plugins/crisp/src/lib.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs)
> * [plugins/diopser/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/editor.rs)
> * [plugins/diopser/src/lib.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/diopser/src/lib.rs)
> * [plugins/examples/gain_gui_vizia/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/examples/gain_gui_vizia/src/editor.rs)
> * [plugins/spectral_compressor/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/spectral_compressor/src/editor.rs)

This page provides a detailed walkthrough of the Crisp plugin, demonstrating ring modulation with filtered noise. Crisp layers the input signal with a ring modulated copy using noise as the modulator, with extensive filtering options to shape the character of the effect.

For information about other example plugins demonstrating different NIH-plug features, see [Simple Examples](/robbert-vdh/nih-plug/5.1-simple-examples), [Diopser Plugin](/robbert-vdh/nih-plug/5.2-diopser-plugin), and [Spectral Compressor Plugin](/robbert-vdh/nih-plug/5.3-spectral-compressor-plugin).

## Overview

Crisp is a creative audio effect that adds high-frequency content to low-frequency signals through ring modulation. The plugin generates filtered noise and uses it to ring-modulate the input signal, creating a bright, crispy top end. The effect includes multiple modulation modes, stereo handling options, and a comprehensive filter chain for precise tonal control.

**Key Features:**

* Three ring modulation modes (full waveform, positive half, negative half)
* Deterministic PRNG-based noise generation for reproducible bouncing
* Cascaded biquad filters for input and noise signal shaping
* Mono/stereo noise source options
* Sample-accurate automation support
* Wet-only output mode for parallel processing

Sources: [plugins/crisp/src/lib.rs L1-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L1-L59)

## Plugin Architecture

### Main Plugin Structure

```mermaid
flowchart TD

NumChannels["NUM_CHANNELS = 2"]
MaxBlockSize["MAX_BLOCK_SIZE = 64"]
InitialSeed["INITIAL_PRNG_SEED"]
AmountMult["AMOUNT_GAIN_MULTIPLIER = 2.0"]
Crisp["Crisp struct"]
Params["CrispParams"]
Editor["nih_plug_vizia Editor"]
SampleRate["sample_rate: f32"]
PRNG["prng: Pcg32iState"]
RMInputLPF["rm_input_lpf[2]: Biquad<f32>"]
NoiseHPF["noise_hpf[2]: Biquad<f32>"]
NoiseLPF["noise_lpf[2]: Biquad<f32>"]
Amount["amount: FloatParam"]
Mode["mode: EnumParam<Mode>"]
StereoMode["stereo_mode: EnumParam<StereoMode>"]
FilterParams["Filter Parameters<br>(6 FloatParams)"]
OutputGain["output_gain: FloatParam"]
WetOnly["wet_only: BoolParam"]

Params --> Amount
Params --> Mode
Params --> StereoMode
Params --> FilterParams
Params --> OutputGain
Params --> WetOnly

subgraph Parameters ["Parameters"]
    Amount
    Mode
    StereoMode
    FilterParams
    OutputGain
    WetOnly
end

subgraph subGraph1 ["Crisp Plugin"]
    Crisp
    Params
    Editor
    Crisp --> Params
    Crisp --> Editor
    Crisp --> SampleRate
    Crisp --> PRNG
    Crisp --> RMInputLPF
    Crisp --> NoiseHPF
    Crisp --> NoiseLPF

subgraph subGraph0 ["Audio Processing State"]
    SampleRate
    PRNG
    RMInputLPF
    NoiseHPF
    NoiseLPF
end
end

subgraph Constants ["Constants"]
    NumChannels
    MaxBlockSize
    InitialSeed
    AmountMult
end
```

The `Crisp` struct maintains processing state including the PRNG for noise generation and three pairs of biquad filters (one pair each for input low-pass, noise high-pass, and noise low-pass). The hardcoded `NUM_CHANNELS = 2` allows for potential future SIMD optimization.

Sources: [plugins/crisp/src/lib.rs L26-L59](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L26-L59)

 [plugins/crisp/src/lib.rs L131-L144](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L131-L144)

### Parameter Configuration

| Parameter | Type | Range | Purpose |
| --- | --- | --- | --- |
| `amount` | FloatParam | 0.0 - 1.0 | Wet/dry mix (scales up to 2x gain) |
| `mode` | EnumParam | Soggy/Crispy/CrispyNegated | Ring modulation type |
| `stereo_mode` | EnumParam | Mono/Stereo | Noise generation per channel |
| `rm_input_lpf_freq` | FloatParam | 5 - 22000 Hz | Input low-pass cutoff |
| `rm_input_lpf_q` | FloatParam | 0.707 - 10.0 | Input low-pass resonance |
| `noise_hpf_freq` | FloatParam | 5 - 22000 Hz | Noise high-pass cutoff |
| `noise_hpf_q` | FloatParam | 0.707 - 10.0 | Noise high-pass resonance |
| `noise_lpf_freq` | FloatParam | 5 - 22000 Hz | Noise low-pass cutoff |
| `noise_lpf_q` | FloatParam | 0.707 - 10.0 | Noise low-pass resonance |
| `output_gain` | FloatParam | -24 to 0 dB | Final output gain |
| `wet_only` | BoolParam | true/false | Output only the RM signal |

All filter parameters use logarithmic smoothing with 100ms time constant, while `amount` and `output_gain` use linear and logarithmic smoothing (10ms) respectively for responsive control without artifacts.

Sources: [plugins/crisp/src/lib.rs L60-L103](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L60-L103)

 [plugins/crisp/src/lib.rs L146-L294](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L146-L294)

## Processing Pipeline

### Audio Processing Flow

```mermaid
flowchart TD

Input["Input Buffer<br>(stereo)"]
BlockIter["iter_blocks(MAX_BLOCK_SIZE)"]
StereoCheck["stereo_mode"]
MonoPath["Mono Processing"]
StereoPath["Stereo Processing"]
MonoNoise["gen_noise(0)<br>Single PRNG call"]
MonoAmount["amount.smoothed.next()"]
MonoUpdate["maybe_update_filters()"]
MonoRM["do_ring_mod(sample, ch, noise)"]
StereoAmount["amount.smoothed.next()"]
StereoUpdate["maybe_update_filters()"]
StereoNoise["gen_noise(channel_idx)<br>Per-channel PRNG"]
StereoRM["do_ring_mod(sample, ch, noise)"]
RMOutput["rm_outputs buffer"]
WetOnlyCheck["wet_only"]
WetPath["output = rm_output * output_gain"]
MixPath["output = (input + rm_output) * output_gain"]
Output["Output Buffer"]

Input --> BlockIter
BlockIter --> StereoCheck
StereoCheck --> MonoPath
StereoCheck --> StereoPath
MonoPath --> MonoAmount
StereoPath --> StereoAmount
MonoRM --> RMOutput
StereoRM --> RMOutput
RMOutput --> WetOnlyCheck
WetOnlyCheck --> WetPath
WetOnlyCheck --> MixPath
WetPath --> Output
MixPath --> Output

subgraph subGraph1 ["Per-Sample Processing (Stereo)"]
    StereoAmount
    StereoUpdate
    StereoNoise
    StereoRM
    StereoAmount --> StereoUpdate
    StereoUpdate --> StereoNoise
    StereoNoise --> StereoRM
end

subgraph subGraph0 ["Per-Sample Processing (Mono)"]
    MonoNoise
    MonoAmount
    MonoUpdate
    MonoRM
    MonoAmount --> MonoUpdate
    MonoUpdate --> MonoNoise
    MonoNoise --> MonoRM
end
```

The processing pipeline operates on blocks of up to 64 samples to reduce per-sample branching overhead. The key distinction between mono and stereo modes is whether a single noise value is shared across channels (mono) or generated independently per channel (stereo).

Sources: [plugins/crisp/src/lib.rs L355-L416](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L355-L416)

### Ring Modulation Modes

```mermaid
flowchart TD

InputSample["Input Sample"]
RMInputLPF["rm_input_lpf[channel].process()"]
FilteredSample["Filtered Sample"]
ModeSwitch["mode.value()"]
SoggyCalc["sample * noise<br>(full waveform)"]
CrispyCalc["sample.max(0.0) * noise<br>(positive half)"]
NegatedCalc["sample.max(0.0) * noise<br>(negative half)"]
RMOutput["RM Output"]

InputSample --> RMInputLPF
RMInputLPF --> FilteredSample
FilteredSample --> ModeSwitch
ModeSwitch --> SoggyCalc
ModeSwitch --> CrispyCalc
ModeSwitch --> NegatedCalc
SoggyCalc --> RMOutput
CrispyCalc --> RMOutput
NegatedCalc --> RMOutput
```

The `do_ring_mod()` method first applies the input low-pass filter, then performs ring modulation according to the selected mode. The `Crispy` and `CrispyNegated` modes use `sample.max(0.0)` to rectify the signal, creating asymmetric modulation that preserves more of the original signal's character.

Sources: [plugins/crisp/src/lib.rs L427-L438](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L427-L438)

## Filter System

### Filter Chain Architecture

```mermaid
flowchart TD

UpdateTrigger["maybe_update_filters()"]
CheckRMInput["rm_input_lpf_freq<br>or _q smoothing?"]
CheckNoiseHP["noise_hpf_freq<br>or _q smoothing?"]
CheckNoiseLP["noise_lpf_freq<br>or _q smoothing?"]
UpdateRMInput["update_rm_input_lpf()"]
UpdateNoiseHP["update_noise_hpf()"]
UpdateNoiseLP["update_noise_lpf()"]
RecalcCoeffs1["BiquadCoefficients::lowpass()"]
RecalcCoeffs2["BiquadCoefficients::highpass()"]
RecalcCoeffs3["BiquadCoefficients::lowpass()"]
InputSignal["Input Audio"]
RMInputLPF["Low-Pass Filter<br>rm_input_lpf"]
FilteredInput["Filtered Input"]
PRNGNoise["PRNG Noise<br>(-1.0 to 1.0)"]
NoiseHPF["High-Pass Filter<br>noise_hpf"]
HPFiltered["HP Filtered"]
NoiseLPF["Low-Pass Filter<br>noise_lpf"]
FilteredNoise["Filtered Noise"]
RingMod["Ring Modulator"]
RMOutput["RM Output"]

FilteredInput --> RingMod
FilteredNoise --> RingMod
RingMod --> RMOutput

subgraph subGraph1 ["Noise Signal Path"]
    PRNGNoise
    NoiseHPF
    HPFiltered
    NoiseLPF
    FilteredNoise
    PRNGNoise --> NoiseHPF
    NoiseHPF --> HPFiltered
    HPFiltered --> NoiseLPF
    NoiseLPF --> FilteredNoise
end

subgraph subGraph0 ["Input Signal Path"]
    InputSignal
    RMInputLPF
    FilteredInput
    InputSignal --> RMInputLPF
    RMInputLPF --> FilteredInput
end

subgraph subGraph2 ["Filter Updates"]
    UpdateTrigger
    CheckRMInput
    CheckNoiseHP
    CheckNoiseLP
    UpdateRMInput
    UpdateNoiseHP
    UpdateNoiseLP
    RecalcCoeffs1
    RecalcCoeffs2
    RecalcCoeffs3
    UpdateTrigger --> CheckRMInput
    UpdateTrigger --> CheckNoiseHP
    UpdateTrigger --> CheckNoiseLP
    CheckRMInput --> UpdateRMInput
    CheckNoiseHP --> UpdateNoiseHP
    CheckNoiseLP --> UpdateNoiseLP
    UpdateRMInput --> RecalcCoeffs1
    UpdateNoiseHP --> RecalcCoeffs2
    UpdateNoiseLP --> RecalcCoeffs3
end
```

The filter chain provides comprehensive tonal shaping:

* **Input LPF**: Removes high frequencies from the input before ring modulation, preventing the effect from becoming pure noise on already bright signals
* **Noise HPF**: Brightens the noise by removing low frequencies
* **Noise LPF**: Further shapes the noise spectrum after high-pass filtering

Each filter can be disabled by setting its frequency to the extreme value (HPF to `MIN_FILTER_FREQUENCY`, LPF to `MAX_FILTER_FREQUENCY`), which is displayed as "Disabled" in the UI.

Sources: [plugins/crisp/src/lib.rs L440-L487](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L440-L487)

### Filter Update Strategy

```mermaid
flowchart TD

Process["process() called"]
PerSample["For each sample"]
MaybeUpdate["maybe_update_filters()"]
Check1["is_smoothing()?<br>rm_input_lpf_freq<br>rm_input_lpf_q"]
Check2["is_smoothing()?<br>noise_hpf_freq<br>noise_hpf_q"]
Check3["is_smoothing()?<br>noise_lpf_freq<br>noise_lpf_q"]
Update1["update_rm_input_lpf()"]
Update2["update_noise_hpf()"]
Update3["update_noise_lpf()"]
NextVal1["smoothed.next()"]
NextVal2["smoothed.next()"]
NextVal3["smoothed.next()"]
CalcCoeffs1["BiquadCoefficients::lowpass<br>(sample_rate, freq, q)"]
CalcCoeffs2["BiquadCoefficients::highpass<br>(sample_rate, freq, q)"]
CalcCoeffs3["BiquadCoefficients::lowpass<br>(sample_rate, freq, q)"]
Apply1["for filter in rm_input_lpf:<br>filter.coefficients = coeffs"]
Apply2["for filter in noise_hpf:<br>filter.coefficients = coeffs"]
Apply3["for filter in noise_lpf:<br>filter.coefficients = coeffs"]

Process --> PerSample
PerSample --> MaybeUpdate
MaybeUpdate --> Check1
MaybeUpdate --> Check2
MaybeUpdate --> Check3
Check1 --> Update1
Check2 --> Update2
Check3 --> Update3
Update1 --> NextVal1
Update2 --> NextVal2
Update3 --> NextVal3
NextVal1 --> CalcCoeffs1
NextVal2 --> CalcCoeffs2
NextVal3 --> CalcCoeffs3
CalcCoeffs1 --> Apply1
CalcCoeffs2 --> Apply2
CalcCoeffs3 --> Apply3
```

Filters are updated per-sample when their parameters are smoothing, ensuring smooth transitions without zipper noise. The update methods apply the same coefficients to both channel filters to maintain stereo coherence.

Sources: [plugins/crisp/src/lib.rs L440-L457](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L440-L457)

 [plugins/crisp/src/lib.rs L459-L487](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L459-L487)

## Noise Generation

### PCG PRNG Implementation

Crisp uses a PCG (Permuted Congruential Generator) PRNG for noise generation, providing high-quality pseudo-random numbers with deterministic behavior for reproducible bouncing.

```mermaid
flowchart TD

PRNGState["Pcg32iState<br>(state, inc)"]
NextU32["next_u32()<br>PCG algorithm"]
U32Value["u32 value"]
NextF32["next_f32()<br>Convert to [0,1)"]
Scale["value * 2.0 - 1.0<br>Range: [-1,1)"]
NoiseValue["Noise Sample"]
GenNoise["gen_noise(channel)"]
HPF["noise_hpf[channel]"]
LPF["noise_lpf[channel]"]
FilteredNoise["Filtered Noise"]

PRNGState --> NextU32
NextU32 --> U32Value
U32Value --> NextF32
NextF32 --> Scale
Scale --> NoiseValue

subgraph subGraph0 ["Per-Channel Usage"]
    NextF32
    GenNoise
    HPF
    LPF
    FilteredNoise
    GenNoise --> NextF32
    GenNoise --> HPF
    HPF --> LPF
    LPF --> FilteredNoise
end
```

The PRNG is initialized with a fixed seed (`INITIAL_PRNG_SEED = Pcg32iState::new(69, 420)`) and reset on each `reset()` call, ensuring that bouncing the same audio produces identical results. This determinism is critical for professional production workflows.

**Stereo Handling:**

* **Mono mode**: Single `gen_noise(0)` call per sample, same noise for both channels
* **Stereo mode**: Separate `gen_noise(channel_idx)` calls, advancing PRNG differently per channel for decorrelated noise

Sources: [plugins/crisp/src/lib.rs L19-L24](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L19-L24)

 [plugins/crisp/src/lib.rs L32-L33](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L32-L33)

 [plugins/crisp/src/lib.rs L340-L353](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L340-L353)

 [plugins/crisp/src/lib.rs L420-L425](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L420-L425)

## Plugin Lifecycle

### Initialization and Reset

```mermaid
flowchart TD

PluginLoad["Plugin Loaded"]
DefaultCtor["Crisp::default()"]
InitState["Initialize state:<br>sample_rate = 1.0<br>prng = INITIAL_PRNG_SEED<br>filters = default"]
HostInit["Host calls initialize()"]
StoreSR["sample_rate = buffer_config.sample_rate"]
UpdateFilters1["update_rm_input_lpf()"]
UpdateFilters2["update_noise_hpf()"]
UpdateFilters3["update_noise_lpf()"]
Ready["Ready to Process"]
ResetCall["reset() called"]
ResetPRNG["prng = INITIAL_PRNG_SEED"]
ResetFilter1["for filter in rm_input_lpf:<br>filter.reset()"]
ResetFilter2["for filter in noise_hpf:<br>filter.reset()"]
ResetFilter3["for filter in noise_lpf:<br>filter.reset()"]

PluginLoad --> DefaultCtor
DefaultCtor --> InitState
InitState --> HostInit
HostInit --> StoreSR
StoreSR --> UpdateFilters1
UpdateFilters1 --> UpdateFilters2
UpdateFilters2 --> UpdateFilters3
UpdateFilters3 --> Ready
Ready --> ResetCall
ResetCall --> ResetPRNG
ResetPRNG --> ResetFilter1
ResetFilter1 --> ResetFilter2
ResetFilter2 --> ResetFilter3
ResetFilter3 --> Ready
```

The initialization sequence ensures filter coefficients are computed based on the actual sample rate before processing begins. The `reset()` method clears filter state and resets the PRNG seed, making bounces deterministic.

Sources: [plugins/crisp/src/lib.rs L324-L353](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L324-L353)

## GUI Implementation

The Crisp editor uses `nih_plug_vizia` with a minimalist design featuring `GenericUi` for automatic parameter layout. The editor is defined in [plugins/crisp/src/editor.rs](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/editor.rs)

### Editor Structure

```mermaid
flowchart TD

Width["400 pixels"]
Height["390 pixels"]
Create["create(params, editor_state)"]
ViziaEditor["create_vizia_editor()"]
BuildData["Data struct<br>(params: Arc<CrispParams>)"]
Layout["VStack layout"]
Title["Label: 'Crisp'<br>(Noto Sans Thin, 30px)"]
ScrollView["ScrollView"]
GenericUI["GenericUi::new(Data::params)"]
AutoParams["Automatic parameter widgets:<br>- Amount slider<br>- Mode enum selector<br>- Stereo Mode enum selector<br>- 6 filter parameter sliders<br>- Output gain slider<br>- Wet Only toggle"]
ResizeHandle["ResizeHandle"]

Create --> ViziaEditor
ViziaEditor --> BuildData
BuildData --> Layout
Layout --> Title
Layout --> ScrollView
ScrollView --> GenericUI
GenericUI --> AutoParams
Layout --> ResizeHandle

subgraph subGraph0 ["Default Size"]
    Width
    Height
end
```

The `GenericUi` widget automatically generates appropriate controls for each parameter type:

* `FloatParam` → Slider with value display
* `EnumParam` → Dropdown or labeled steps
* `BoolParam` → Toggle button

This approach minimizes GUI code while providing full parameter access. The scroll view accommodates the relatively large number of parameters (11 total) within the compact window size.

Sources: [plugins/crisp/src/editor.rs L1-L76](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/editor.rs#L1-L76)

## Technical Implementation Details

### Constants and Configuration

| Constant | Value | Purpose |
| --- | --- | --- |
| `NUM_CHANNELS` | 2 | Stereo processing, hardcoded for potential SIMD |
| `MAX_BLOCK_SIZE` | 64 | Block iteration size for reduced branching |
| `INITIAL_PRNG_SEED` | `(69, 420)` | Fixed seed for deterministic bouncing |
| `AMOUNT_GAIN_MULTIPLIER` | 2.0 | Allows 100% amount to boost above unity |
| `MIN_FILTER_FREQUENCY` | 5.0 Hz | Lower bound for filter cutoffs |
| `MAX_FILTER_FREQUENCY` | 22000.0 Hz | Upper bound (used for "disabled") |

Sources: [plugins/crisp/src/lib.rs L26-L37](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L26-L37)

### Plugin Traits Implementation

```mermaid
flowchart TD

Crisp["Crisp struct"]
Plugin["Plugin trait"]
ClapPlugin["ClapPlugin trait"]
Vst3Plugin["Vst3Plugin trait"]
Constants1["SAMPLE_ACCURATE_AUTOMATION = true"]
AIOL["AUDIO_IO_LAYOUTS<br>(stereo only)"]
Methods["process(), initialize(),<br>reset(), params(), editor()"]
ClapID["CLAP_ID:<br>'nl.robbertvanderhelm.crisp'"]
ClapFeatures["CLAP_FEATURES:<br>AudioEffect, Stereo, Distortion"]
Vst3ID["VST3_CLASS_ID:<br>'CrispPluginRvdH.'"]
Vst3Cats["VST3_SUBCATEGORIES:<br>Fx, Filter, Distortion, Stereo"]
Export["Export Macros"]
ExportClap["nih_export_clap!(Crisp)"]
ExportVst3["nih_export_vst3!(Crisp)"]

Crisp --> Plugin
Crisp --> ClapPlugin
Crisp --> Vst3Plugin
Plugin --> Constants1
Plugin --> AIOL
Plugin --> Methods
ClapPlugin --> ClapID
ClapPlugin --> ClapFeatures
Vst3Plugin --> Vst3ID
Vst3Plugin --> Vst3Cats
Export --> ExportClap
Export --> ExportVst3
```

The plugin is exported for both CLAP and VST3 formats. Setting `SAMPLE_ACCURATE_AUTOMATION = true` enables sample-accurate parameter changes, ensuring precise timing for automation and modulation.

Sources: [plugins/crisp/src/lib.rs L296-L318](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L296-L318)

 [plugins/crisp/src/lib.rs L490-L515](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L490-L515)

### Future SIMD Considerations

The codebase includes comments indicating planned SIMD optimization:

* `NUM_CHANNELS` is hardcoded to 2 for easier SIMD-ification
* `MAX_BLOCK_SIZE` allows processing multiple samples at once
* The `do_ring_mod()` method includes a TODO comment about avoiding branching for SIMD compatibility

The current implementation processes channels sequentially, but the architecture is designed to support future vectorization where both channels could be processed simultaneously using SIMD types like `f32x2`.

Sources: [plugins/crisp/src/lib.rs L26-L29](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L26-L29)

 [plugins/crisp/src/lib.rs L367](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L367-L367)

 [plugins/crisp/src/lib.rs L432](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L432-L432)

## Parameter Smoothing Strategy

All time-varying parameters use smoothers to prevent audio artifacts:

| Parameter Category | Smoothing Type | Time Constant |
| --- | --- | --- |
| Amount | Linear | 10 ms |
| Filter frequencies | Logarithmic | 100 ms |
| Filter Q values | Logarithmic | 100 ms |
| Output gain | Logarithmic | 10 ms |

Logarithmic smoothing is appropriate for frequency and gain parameters as they are perceived logarithmically, while linear smoothing works for the amount parameter which represents a linear mix ratio. The longer smoothing time for filter parameters (100ms) prevents audible filter sweeps while still allowing responsive control.

Sources: [plugins/crisp/src/lib.rs L154-L276](https://github.com/robbert-vdh/nih-plug/blob/28b149ec/plugins/crisp/src/lib.rs#L154-L276)