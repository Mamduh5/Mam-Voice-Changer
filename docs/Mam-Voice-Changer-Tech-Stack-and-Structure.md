# Mam Voice Changer

## Project Goal

Build a Windows desktop voice changer that:

- Captures audio from a physical microphone.
- Applies real-time voice effects locally.
- Sends processed audio to a selected Windows output device.
- Targets Discord, TikTok Live Studio, OBS, Facebook Live, browsers, and other applications through a virtual audio device such as VB-CABLE.

Those receiving-application targets require separate manual compatibility tests.
They are goals, not current compatibility claims. The prototype should validate
the complete live audio path before any custom Windows virtual audio driver is
considered.

---

## Target Platform

- Windows 10
- Windows 11
- x64 architecture

Initial development should remain Windows-only.

---

## Recommended Tech Stack

### Desktop Application

| Area | Technology | Responsibility |
|---|---|---|
| Desktop framework | Tauri 2 | Native desktop shell and frontend/backend bridge |
| Frontend | React | Application interface |
| Frontend language | TypeScript | Type-safe UI development |
| Frontend build tool | Vite | Development server and production build |
| Styling | CSS Modules or plain CSS | Lightweight component styling |
| Package manager | npm | Frontend dependencies and scripts |

### Audio Engine

| Area | Technology | Responsibility |
|---|---|---|
| Core language | Rust | Real-time audio engine and native application logic |
| Audio input/output | CPAL | Device enumeration, microphone capture, and output streaming |
| Audio buffering | Ring buffer | Transfer audio safely between input and output streams |
| DSP | Native Rust modules | Gain, gate, filtering, pitch processing, mixing, and limiting |
| Pitch and formant backend | Signalsmith Stretch 1.3.2 through a static C ABI | Formant-aware, stream-length-preserving transformation without a runtime DLL |
| Serialization | Serde and serde_json | Versioned local JSON documents |
| Error handling | thiserror | Typed internal errors |
| Logging | tracing and tracing-subscriber | Structured diagnostics outside audio callbacks |

### Virtual Microphone Routing

Use VB-CABLE for the prototype.

```text
Physical Microphone
        |
        v
Mam Voice Changer
        |
        v
CABLE Input
        |
        v
CABLE Output
        |
        v
Discord / TikTok / OBS / Facebook Live
```

Do not build a custom Windows audio driver during the prototype.

### Testing and Validation

| Area | Technology |
|---|---|
| Rust unit tests | Built-in Rust test framework |
| Frontend tests | Vitest |
| Rust formatting | rustfmt |
| Rust static analysis | Clippy |
| Frontend linting | ESLint |
| Frontend formatting | Prettier |
| Manual audio validation | Discord microphone test, OBS monitoring, and local VB-CABLE routing |

---

## High-Level Architecture

```text
React UI
   |
   | Tauri commands and status polling
   v
Application State
   |
   v
Audio Engine Controller
   |
   +---------------------------+
   |                           |
   v                           v
Input Stream              Output Stream
   |                           ^
   v                           |
Sample Conversion             |
   |                           |
   v                           |
DSP Processing Chain ----------+
```

---

## Audio Pipeline

```text
Microphone Input
    |
    v
Input Sample Conversion
    |
    v
Channel Normalization
    |
    v
Input Gain
    |
    v
High-Pass / DC Blocker
    |
    v
Noise Gate
    |
    v
Formant-Aware Pitch Transformation
    |
    v
Pitch-Latency-Aligned Dry / Wet Mix
    |
    v
Warmth Low Shelf
    |
    v
Brightness High Shelf
    |
    v
Pitch-Latency-Aligned Bypass Crossfade
    |
    v
Output Gain
    |
    v
Linked Lookahead Master Limiter
    |
    v
Final Mute Ramp
    |
    v
Selected Output Device
```

The output device will normally be `CABLE Input` from VB-CABLE.
Bypass skips the gate, pitch/formant transformation, dry/wet mix, and tone EQ.
Input gain and the high-pass filter remain before the bypass tap; output gain, the
master limiter, and final mute remain after it.

---

## Key Repository Structure

This is a current map of the principal source files, not an exhaustive vendor or
generated-file listing.

```text
Mam-Voice-Changer/
├─ README.md
├─ .gitignore
├─ .github/workflows/windows-ci.yml
├─ package.json
├─ package-lock.json
├─ tsconfig.json
├─ vite.config.ts
├─ eslint.config.js
├─ prettier.config.js
├─ index.html
│
├─ docs/
│  ├─ architecture.md
│  ├─ audio-routing.md
│  ├─ dsp.md
│  ├─ prototype-scope.md
│  ├─ manual-test-plan.md
│  ├─ troubleshooting.md
│  └─ Mam-Voice-Changer-Tech-Stack-and-Structure.md
│
├─ src/
│  ├─ main.tsx
│  ├─ App.tsx
│  │
│  ├─ components/
│  │  ├─ DeviceSelector.tsx
│  │  ├─ EngineControls.tsx
│  │  ├─ DspControls.tsx
│  │  ├─ LevelMeter.tsx
│  │  ├─ PresetControls.tsx
│  │  └─ DiagnosticsPanel.tsx
│  │
│  ├─ hooks/
│  │  ├─ useAudioDevices.ts
│  │  ├─ useAudioParameters.ts
│  │  ├─ useEngineState.ts
│  │  └─ usePresets.ts
│  │
│  ├─ services/
│  │  └─ tauriAudioApi.ts
│  │
│  ├─ types/
│  │  ├─ audio.ts
│  │  ├─ engine.ts
│  │  ├─ parameters.ts
│  │  └─ presets.ts
│  │
│  ├─ utils/
│  │  └─ deviceSelection.ts
│  │
│  └─ styles.css
│
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ Cargo.lock
│  ├─ build.rs
│  ├─ tauri.conf.json
│  │
│  ├─ capabilities/
│  │  └─ default.json
│  │
│  └─ src/
│     ├─ main.rs
│     ├─ lib.rs
│     ├─ error.rs
│     │
│     ├─ commands/
│     │  ├─ mod.rs
│     │  ├─ devices.rs
│     │  ├─ engine.rs
│     │  ├─ parameters.rs
│     │  └─ presets.rs
│     │
│     ├─ audio/
│     │  ├─ mod.rs
│     │  ├─ device.rs
│     │  ├─ controller.rs
│     │  ├─ input_stream.rs
│     │  ├─ output_stream.rs
│     │  ├─ stream_config.rs
│     │  ├─ sample_format.rs
│     │  ├─ channel_mapper.rs
│     │  ├─ ring_buffer.rs
│     │  ├─ worker.rs
│     │  └─ metrics.rs
│     │
│     ├─ dsp/
│     │  ├─ mod.rs
│     │  ├─ processor.rs
│     │  ├─ chain.rs
│     │  ├─ high_pass.rs
│     │  ├─ noise_gate.rs
│     │  ├─ pitch.rs
│     │  ├─ signalsmith.rs
│     │  ├─ dry_wet.rs
│     │  ├─ gain.rs
│     │  ├─ tone.rs
│     │  ├─ smoothing.rs
│     │  └─ master_limiter.rs
│     │
│     ├─ config/
│     │  ├─ mod.rs
│     │  └─ presets.rs
│     │
│     └─ state/
│        ├─ mod.rs
│        ├─ app_state.rs
│        ├─ engine_state.rs
│        └─ parameter_state.rs
│
└─ tests/
   └─ README.md
```

---

## Module Responsibilities

### `src/components`

Contains visual UI components only.

Responsibilities:

- Device selection
- Start and stop controls
- Voice parameter controls
- Level meters
- Preset controls
- Runtime status
- Error and diagnostic display

Components should not contain audio-processing logic.

### `src/hooks`

Contains reusable frontend state, polling, and Tauri command coordination.

Responsibilities:

- Loading audio devices
- Tracking engine state
- Receiving audio metrics
- Managing presets
- Updating parameters

### `src/services`

Contains the typed frontend boundary for Tauri commands.

Current responsibilities:

- `listAudioDevices`
- `startEngine`
- `stopEngine`
- `getEngineStatus`
- `getParameters`
- `setParameters`
- `listPresets`
- `savePreset`
- `renamePreset`
- `duplicatePreset`
- `deletePreset`
- `applyPreset`
- `resetPreset`

### `src-tauri/src/commands`

Contains Tauri command handlers.

This layer should:

- Validate frontend requests
- Call application services
- Convert internal errors into frontend-safe errors
- Return serializable data

It should not implement DSP directly.

### `src-tauri/src/audio`

Contains audio-device and stream infrastructure.

Responsibilities:

- Device enumeration
- Input stream creation
- Output stream creation
- Sample conversion
- Channel conversion
- Buffering
- Stream lifecycle
- Device disconnection handling
- Runtime metrics
- Underrun and overrun tracking

### `src-tauri/src/dsp`

Contains pure audio processors.

Responsibilities:

- High-pass filtering
- Noise gating
- Pitch transformation
- Dry/wet mixing
- Gain
- Limiting
- Bypass behavior

DSP modules should be testable without opening a real audio device.

### `src-tauri/src/config`

Contains serializable local configuration. Preset storage is currently isolated in
`config/presets.rs`; unrelated application settings must not be added to
`presets.json`.

Responsibilities:

- Versioned preset documents
- Preset and complete-DSP-snapshot validation
- Atomic local JSON persistence and interrupted-write recovery
- Built-in definitions plus user-preset loading and saving

### `src-tauri/src/state`

Contains shared application state.

Responsibilities:

- Current engine state
- Current parameter snapshot
- Preset-store ownership
- Engine controller ownership
- Safe communication between Tauri commands and the audio engine

---

## Core Rust Interfaces

### Audio Processor

```rust
pub trait AudioProcessor: Send {
    fn prepare(
        &mut self,
        sample_rate: u32,
        channels: usize,
        block_size: usize,
    );

    fn process(&mut self, samples: &mut [f32]);

    fn reset(&mut self);
}
```

### Engine State

```rust
pub enum EngineState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}
```

### Main Audio Parameters

```rust
pub struct DspParameters {
    pub pitch_semitones: f32,
    pub formant_shift_semitones: f32,
    pub dry_wet: f32,
    pub gate_enabled: bool,
    pub gate_threshold_db: f32,
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub master_ceiling_db: f32,
    pub warmth_db: f32,
    pub brightness_db: f32,
    pub limiter_enabled: bool,
    pub bypass: bool,
    pub muted: bool,
}
```

---

## Frontend Screens

The prototype only needs one main window.

### Device Section

- Input-device selector
- Output-device selector
- Refresh devices button

### DSP Section

- Pitch slider
- Formant-shift slider
- Dry/wet slider
- Input gain
- Output gain
- Noise-gate toggle
- Noise-gate threshold
- Warmth
- Brightness
- Master-ceiling control
- Limiter toggle
- Bypass
- Mute

### Runtime Section

- Start
- Stop

### Preset Section

- Preset selector
- Separate built-in and user-preset groups
- Save current parameters as a user preset
- Rename user preset
- Duplicate built-in or user preset
- Delete user preset
- Reset to built-in `Natural`

### Status Section

- Engine state
- Input level
- Output level
- Stream format
- Estimated latency
- Input overruns
- Output underruns
- Last runtime error

---

## Real-Time Audio Rules

The real-time audio callback must not:

- Read or write files
- Enumerate devices
- Call frontend APIs
- Perform network requests
- Block on ordinary mutexes
- Sleep
- Log every block
- Allocate repeatedly when buffers can be preallocated
- Panic on recoverable errors

Use:

- Preallocated audio buffers
- Atomic parameter values
- Immutable parameter snapshots
- Bounded non-blocking channels
- Ring buffers
- Separate diagnostic reporting

---

## Prototype Scope

### Implemented

- Windows microphone capture
- Windows output-device streaming
- Common-rate negotiation and normalized channel mapping
- Bounded rings and a dedicated fixed-block DSP worker
- Pitch and independent formant controls
- Noise gate, input/output gain, dry/wet mix, tone EQ, bypass, and mute
- Linked lookahead master limiter
- Device enumeration and selection
- Versioned built-in and user-preset persistence
- Preset apply, save, rename, duplicate, delete, and reset
- Input and output meters
- Error reporting
- Basic latency metrics

Built-ins can be applied or duplicated but cannot be renamed or deleted. User
presets contain the complete `DspParameters` snapshot, and the selected preset is
restored before audio starts.

### Automated validation coverage present

- Device-independent DSP and state tests
- Preset schema, validation, persistence, selection, operation, and corrupt-file tests
- Frontend device-selection fallback tests

Coverage does not itself assert that the current checkout passed every command,
and synthetic tests do not establish audible behavior.

### Manual validation completed

- Tauri debug executable launch on 2026-07-18
- React interface rendering on 2026-07-18
- Enumeration of the Realtek endpoints present during that session

### Manual validation still required

- Preset workflows and persistence across a real application restart
- Continuous monitored audio and repeated start/stop cycles
- VB-CABLE routing
- Discord, OBS, TikTok Live Studio, browser, and Facebook Live compatibility
- Device-disconnection recovery and long-duration stability
- Subjective voice quality and safe monitoring behavior

### Deferred

- Custom Windows virtual audio driver
- AI voice conversion
- Voice cloning
- Neural inference
- macOS support
- Linux support
- Mobile support
- Cloud processing
- Accounts
- Telemetry
- Plugin system
- Automatic VB-CABLE installation
- Audio recording
- Chat reading
- AI comment filtering

Compatibility validation is pending manual evidence; it is not categorized as a
current implementation failure.

---

## Prototype Milestones

### Milestone 1: Audio Passthrough

- Implemented: enumerate input and output devices.
- Implemented: capture and output stream infrastructure.
- Implemented: latency-aligned bypass for a clean comparison path.
- Manual validation pending: continuous passthrough and VB-CABLE routing.

### Milestone 2: Basic DSP

- Implemented: gain, mute, bypass, high-pass filtering, and the linked master limiter.

### Milestone 3: Voice Transformation

- Implemented: real-time pitch/formant transformation, dry/wet, noise gate, and tone EQ.
- Manual validation pending: continuous monitored output and subjective quality.

### Milestone 4: Desktop Interface

- Implemented: native control integration, meters, runtime status, and visible errors.
- Implemented: versioned preset persistence and complete preset operations.
- Manual validation pending: preset workflows across an actual desktop restart.

### Milestone 5: Compatibility Validation

- Pending: verify Discord input.
- Pending: verify OBS input.
- Pending: verify TikTok Live Studio and other receiving applications.
- Pending: run and record a long-duration stability test.

---

## Prototype Success Condition (manual acceptance target)

The prototype is successful when this path works reliably:

```text
Physical Microphone
    |
    v
Mam Voice Changer
    |
    v
Real-Time Pitch and Audio Processing
    |
    v
CABLE Input
    |
    v
CABLE Output
    |
    v
Discord Microphone Test
```

The transformed voice must be audible in Discord with acceptable latency and
without application crashes, fake processing, or prerecorded audio. This target
has not been established by compile-time or device-independent automated checks;
it remains pending until the corresponding manual test is performed and recorded.
