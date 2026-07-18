# Mam Voice Changer

Mam Voice Changer is a Windows 10/11 x64 desktop application built with Tauri 2,
React, TypeScript, Rust, and CPAL. It captures a physical microphone, applies a
local real-time DSP chain, and sends the result to a selected Windows output such
as VB-CABLE's **CABLE Input**.

![Desktop interface](docs/screenshots/milestone-1-ui.png)

## Current implementation

- Windows input/output device enumeration and selection
- Common sample-rate negotiation and normalized `f32` processing
- Bounded input/output rings and a dedicated fixed-block DSP worker
- Input gain, 20 Hz high-pass filtering, optional noise gate, and output gain
- Signalsmith Stretch pitch shifting with formant compensation and independent
  formant shift
- Pitch-latency-aligned dry/wet mixing and bypass
- Warmth (200 Hz low shelf) and brightness (4 kHz high shelf)
- Linked 5 ms lookahead master limiter with a configurable digital ceiling
- Final smoothed mute stage
- Atomic live parameter snapshots, meters, counters, and latency estimates
- Versioned built-in and user presets stored in the Tauri application-data directory
- Browser-safe frontend boundary when Vite is opened outside Tauri

Not implemented: recording, incompatible-rate resampling, AI voice conversion,
custom virtual drivers, or verified Discord/OBS/TikTok compatibility.

## Conservative defaults

The application starts at 0 semitones pitch/formant, 35% wet, gate off,
0 dB input, -6 dB output, neutral tone controls, a -3 dBFS master ceiling,
limiter on, bypass off, and mute off.

The digital limiter prevents samples from exceeding its configured ceiling while
enabled. It cannot measure headphone volume, speaker output, acoustic feedback,
or safe listening exposure. Start with low Windows/headphone volume, use
headphones, and increase levels gradually.

## Prerequisites

- Windows 10 or Windows 11 x64
- Node.js 20 or newer and npm
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime
- [VB-CABLE](https://vb-audio.com/Cable/) when virtual-microphone routing is needed

Signalsmith Stretch and Signalsmith Linear are vendored under their MIT licenses
and compile statically into the application with MSVC. No Signalsmith DLL,
libclang installation, or runtime download is required.

## Development

```powershell
npm ci
npm run dev
```

`npm run dev` launches the Tauri desktop runtime. `npm run dev:web` intentionally
launches only Vite; native audio controls remain disabled there.

Choose a physical microphone as input and **CABLE Input** as output. In the
receiving application, choose **CABLE Output** as its microphone.

## Compile-time checks

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Runtime and audible behavior require separate manual validation with conservative
monitoring levels. See the [manual test plan](docs/manual-test-plan.md).

## Documentation

- [DSP design and parameters](docs/dsp.md)
- [Architecture](docs/architecture.md)
- [Audio routing](docs/audio-routing.md)
- [Prototype scope](docs/prototype-scope.md)
- [Manual test plan](docs/manual-test-plan.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Authoritative technical specification](docs/Mam-Voice-Changer-Tech-Stack-and-Structure.md)

## Known limitations

- Input and output must expose a common sample rate.
- CPAL 0.15 device IDs are deterministic friendly-name fingerprints, not WASAPI
  endpoint GUIDs.
- Latency is estimated from configured buffers and reported DSP delay; it is not a
  measured acoustic round trip.
- Formant processing is spectral and polyphonic rather than a monophonic PSOLA
  model. Extreme pitch/formant combinations can still sound synthetic.
- Compatibility and subjective listening quality have not been established by
  compile-time checks.

