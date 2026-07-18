# Mam Voice Changer

Windows desktop voice changer prototype built with Tauri 2, React, TypeScript, Rust, and CPAL. It captures a physical microphone, applies a lightweight real-time DSP chain, and routes processed sound to an output such as **CABLE Input (VB-Audio Virtual Cable)**.

## Prototype features

- Enumerate Windows input and output audio devices
- Start and stop a live microphone-to-output stream
- Input/output gain, noise gate, pitch character, dry/wet mix, and soft limiting
- Four starter voice presets
- Live input/output level telemetry and underrun diagnostics
- Audio processing stays local; no recording or network upload

## Prerequisites

1. Windows 10/11 x64, Rust stable, Node.js 20+, and Microsoft C++ Build Tools.
2. Install [VB-CABLE](https://vb-audio.com/Cable/) and restart Windows.
3. Run `npm install`, then `npm run tauri dev`.
4. Select your microphone as Input and **CABLE Input** as Output. In Discord/OBS/etc., select **CABLE Output** as the microphone.

## Checks

```powershell
npm run build
cd src-tauri
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

This is an early prototype. Pitch currently uses low-latency resampling and changes duration/timbre; production-quality pitch preservation is planned after the end-to-end audio path is validated.
