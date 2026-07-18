# Mam Voice Changer

Mam Voice Changer is a Windows 10/11 x64 desktop prototype built with Tauri 2,
React, TypeScript, Rust, and CPAL. It captures a physical microphone, applies a
small real-time DSP chain, and sends the result to a compatible Windows output such
as VB-CABLE's **CABLE Input**.

![Milestone 1 desktop interface](docs/screenshots/milestone-1-ui.png)

## Current status

Implemented and automated-testable:

- Windows input/output device enumeration and refresh
- Stable, order-independent device fingerprints
- Repeated start/stop lifecycle owned by a dedicated audio-engine thread
- Common input/output sample-rate negotiation with 48 kHz preference
- `f32`, `i16`, and `u16` sample conversion through normalized `f32`
- Mono/stereo channel mapping
- Bounded lock-free ring buffering with explicit overflow/underflow behavior
- Input/output meters, counters, estimated latency, active format, and runtime errors
- Recoverable stopped, starting, running, stopping, and error states
- Input and output gain, mute, bypass, 20 Hz high-pass filtering, and soft limiting
- Lock-free live parameter updates through immutable callback snapshots

Not implemented yet:

- Noise gate, preset persistence, or recording
- Genuine pitch transformation or dry/wet mixing
- Resampling between devices with no common sample rate
- Discord, OBS, or TikTok Live Studio compatibility verification

The previous amplitude-scaling control was not pitch shifting and remains removed.
No pitch or preset behavior is simulated in the frontend.

## Prerequisites

- Windows 10 or Windows 11 x64
- Node.js 20 or newer and npm
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime
- [VB-CABLE](https://vb-audio.com/Cable/) for virtual-microphone routing

## Development

```powershell
npm ci
npm run dev
```

Choose a physical microphone as input and **CABLE Input** as output. In the
receiving application, choose **CABLE Output** as its microphone. Use headphones
while testing to avoid acoustic feedback.

Frontend-only development is available with `npm run dev:web`. The production frontend
build is `npm run build`, and a local debug executable can be produced with:

```powershell
npm run tauri -- build --debug --no-bundle --ci
```

## Validation

```powershell
npm ci
npm run lint
npm run format:check
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --debug --no-bundle --ci
```

Tests do not require a physical audio device. Follow
[the manual test plan](docs/manual-test-plan.md) for real routing validation.

## Documentation

- [Architecture](docs/architecture.md)
- [Audio routing](docs/audio-routing.md)
- [Prototype scope](docs/prototype-scope.md)
- [Manual test plan](docs/manual-test-plan.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Authoritative technical specification](docs/Mam-Voice-Changer-Tech-Stack-and-Structure.md)

## Known limitations

- Input and output must expose at least one common sample rate. The engine rejects
  incompatible pairs rather than playing audio at the wrong rate.
- CPAL 0.15 does not expose WASAPI endpoint GUIDs. Device IDs are deterministic
  fingerprints of direction plus Windows friendly name, so a rename changes the ID
  and identical friendly names may be ambiguous.
- The displayed latency is an estimate based on requested device buffers and ring
  prefill, not a measured round-trip latency.
- On ring overflow the newest samples are dropped; on underflow the output is filled
  with silence. Counters report callback blocks where either condition occurred.
- Pitch, formant preservation, presets, and other DSP controls are deliberately absent.
