# Mam Voice Changer

## Project Goal

Build a Windows desktop voice changer that:

- Captures audio from a physical microphone.
- Applies real-time voice effects locally.
- Sends processed audio to a selected Windows output device.
- Works with Discord, TikTok Live Studio, OBS, Facebook Live, browsers, and other applications through a virtual audio device such as VB-CABLE.

The first prototype should validate the complete live audio path before attempting a custom Windows virtual audio driver.

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
| Pitch-shifting fallback | SoundTouch through FFI | More advanced pitch shifting if the initial Rust implementation is insufficient |
| Serialization | Serde and serde_json | Presets and application settings |
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
   | Tauri commands and events
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
High-Pass / DC Blocker
    |
    v
Noise Gate
    |
    v
Pitch Transformation
    |
    v
Dry / Wet Mix
    |
    v
Input and Output Gain
    |
    v
Soft Limiter
    |
    v
Selected Output Device
```

The output device will normally be `CABLE Input` from VB-CABLE.

---

## Repository Structure

```text
Mam-Voice-Changer/
├─ README.md
├─ LICENSE
├─ .gitignore
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
│  ├─ prototype-scope.md
│  ├─ manual-test-plan.md
│  └─ troubleshooting.md
│
├─ src/
│  ├─ main.tsx
│  ├─ App.tsx
│  │
│  ├─ components/
│  │  ├─ DeviceSelector.tsx
│  │  ├─ EngineControls.tsx
│  │  ├─ VoiceControls.tsx
│  │  ├─ LevelMeter.tsx
│  │  ├─ PresetSelector.tsx
│  │  ├─ StatusPanel.tsx
│  │  └─ DiagnosticsPanel.tsx
│  │
│  ├─ hooks/
│  │  ├─ useAudioDevices.ts
│  │  ├─ useEngineState.ts
│  │  ├─ useEngineMetrics.ts
│  │  └─ usePresets.ts
│  │
│  ├─ services/
│  │  └─ tauriAudioApi.ts
│  │
│  ├─ types/
│  │  ├─ audio.ts
│  │  ├─ engine.ts
│  │  └─ preset.ts
│  │
│  └─ styles/
│     ├─ global.css
│     └─ app.css
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
│     │  ├─ engine.rs
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
│     │  ├─ bypass.rs
│     │  ├─ high_pass.rs
│     │  ├─ noise_gate.rs
│     │  ├─ pitch.rs
│     │  ├─ dry_wet.rs
│     │  ├─ gain.rs
│     │  └─ limiter.rs
│     │
│     ├─ config/
│     │  ├─ mod.rs
│     │  ├─ model.rs
│     │  ├─ defaults.rs
│     │  ├─ validation.rs
│     │  └─ storage.rs
│     │
│     └─ state/
│        ├─ mod.rs
│        ├─ app_state.rs
│        ├─ engine_state.rs
│        └─ parameter_state.rs
│
└─ tests/
   ├─ README.md
   └─ fixtures/
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

Contains reusable frontend state and Tauri event handling.

Responsibilities:

- Loading audio devices
- Tracking engine state
- Receiving audio metrics
- Managing presets
- Updating parameters

### `src/services`

Contains the typed frontend boundary for Tauri commands.

Example responsibilities:

- `listInputDevices`
- `listOutputDevices`
- `startEngine`
- `stopEngine`
- `updateParameters`
- `savePreset`
- `loadPreset`

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

Contains serializable settings and presets.

Responsibilities:

- Default parameters
- Configuration validation
- Local JSON persistence
- Preset loading and saving
- Missing-device fallback behavior

### `src-tauri/src/state`

Contains shared application state.

Responsibilities:

- Current engine state
- Selected devices
- Current parameter snapshot
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
pub struct AudioParameters {
    pub pitch_semitones: f32,
    pub dry_wet: f32,
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub gate_threshold_db: f32,
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
- Selected sample rate
- Buffer or latency setting

### Voice Section

- Pitch slider
- Dry/wet slider
- Input gain
- Output gain
- Noise-gate threshold
- Limiter toggle

### Runtime Section

- Start
- Stop
- Bypass
- Mute
- Optional monitor toggle
- Reset parameters

### Preset Section

- Preset selector
- Save preset
- Delete preset
- Reset to default

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

### Included

- Windows microphone capture
- Windows output-device streaming
- VB-CABLE routing
- Pitch control
- Noise gate
- Input gain
- Output gain
- Dry/wet mix
- Soft limiter
- Bypass
- Mute
- Device selection
- Presets
- Input and output meters
- Error reporting
- Basic latency metrics
- Discord compatibility testing
- OBS compatibility testing
- TikTok Live Studio routing documentation

### Deferred

- Custom Windows virtual audio driver
- AI voice conversion
- Voice cloning
- Neural inference
- Formant control unless genuinely implemented
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

---

## Prototype Milestones

### Milestone 1: Audio Passthrough

- Enumerate input and output devices.
- Capture microphone audio.
- Send unmodified audio to the selected output.
- Confirm VB-CABLE routing works.

### Milestone 2: Basic DSP

- Add gain.
- Add mute.
- Add bypass.
- Add high-pass filtering.
- Add limiter.

### Milestone 3: Voice Transformation

- Add real-time pitch transformation.
- Add dry/wet control.
- Add noise gate.
- Verify continuous output.

### Milestone 4: Desktop Interface

- Connect all controls to the real engine.
- Add meters and runtime status.
- Add error handling.
- Add preset persistence.

### Milestone 5: Compatibility Validation

- Verify Discord input.
- Verify OBS input.
- Document TikTok Live Studio routing.
- Run a long-duration stability test.

---

## Prototype Success Condition

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

The transformed voice must be audible in Discord with acceptable latency and without application crashes, fake processing, or prerecorded audio.
