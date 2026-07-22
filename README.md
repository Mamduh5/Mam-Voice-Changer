# Mam Voice Changer

Mam Voice Changer is a Windows 10/11 x64 desktop application built with Tauri 2,
React, TypeScript, Rust, and CPAL. It captures a physical microphone, applies a
local real-time DSP chain, and sends the result to a saved external route's
virtual playback endpoint. The paired Windows capture endpoint is shown for
selection in Discord, OBS, a browser, or another receiving application. An
independent local monitor is available only on Test and defaults off.

![Desktop interface](docs/screenshots/milestone-1-ui.png)

## Current implementation

- Windows input/output device enumeration and selection
- Conservative virtual playback/capture discovery, explicit manual pairing, and route readiness
- Common sample-rate negotiation and normalized `f32` processing
- Separate Use, Test, Voice Lab, and Settings & Diagnostics pages
- One DSP worker with independent bounded processed-destination and monitor rings
- Low latency, Balanced, and Reliable buffering/prefill profiles
- Short-dropout concealment, staged stream recovery, and detailed location-specific counters
- Input gain, 20 Hz high-pass filtering, an optional soft speech expander, and output gain
- Signalsmith Stretch pitch shifting with formant compensation and independent
  formant shift
- Pitch-latency-aligned dry/wet mixing and bypass
- Warmth (200 Hz low shelf) and brightness (4 kHz high shelf)
- Linked 5 ms lookahead master limiter with a configurable digital ceiling
- Final smoothed mute stage
- Atomic live parameter snapshots, meters, counters, and latency estimates
- Versioned preset persistence in Tauri's application-data directory
- Three read-only built-in presets (`Natural`, `Warm tone`, and `Bright tone`) plus
  user presets created from the complete live DSP parameter snapshot
- Preset apply, save, rename, duplicate, delete, and reset workflows; the selected
  preset is restored at startup, and reset selects `Natural`
- An isolated, memory-bounded Voice Lab for recording or importing up to 15 seconds,
  offline rendering through the existing DSP, latency-aligned A/B preview and loop,
  non-selecting preset development, explicit live apply, and explicit WAV export
- A separate persistent Voice Dataset workspace under Voice Lab with explicit
  consent, original project prompts, dry bounded recording, WAV import and
  canonical PCM24 mono 48 kHz conversion, deterministic quality reports,
  manual accept/reject/redo, non-destructive silence trimming, progress,
  local physical-output preview, deletion/recovery, and explicit directory export
- A separate **Voice Lab → Models** workspace for consent-bound immutable snapshots,
  optional manually configured Seed-VC child-process training, versioned and
  hash-validated model artifacts, offline synthetic conversion, manual evaluation,
  and approval for offline Voice Lab comparison only
- Browser-safe frontend boundary when Vite is opened outside Tauri

Built-in presets may be applied or duplicated, but they cannot be renamed or
deleted. Saving always creates and selects a user preset. Deleting the selected
user preset falls back to `Natural`.

## Validation status

### Automated coverage present

Device-independent Rust tests cover presets, application-settings migration,
routing fan-out, reliability profiles, the expander, concealment, counters, and
bounded recovery policy. Dataset tests cover consent gates, versioned storage,
safe paths, hashes, canonical WAV ingestion, quality heuristics, review statistics,
export filtering, deletion, and health reporting. Frontend tests cover device selection,
application pages, monitoring safety, navigation, diagnostics, and Dataset states. These are descriptions of test
coverage, not a claim that the commands below passed in the current checkout.

### Manual validation completed

On 2026-07-18, the Tauri debug executable launched, the React interface rendered,
and the available Realtek input/output endpoints were enumerated. The exact scope
and limitations of that session are recorded in the
[manual test plan](docs/manual-test-plan.md).

### Manual validation still required

Preset and Dataset workflows across a real application restart, continuous monitored audio,
virtual-device routing, repeated start/stop and disconnection recovery,
long-duration stability, and Discord/OBS/TikTok Live Studio compatibility remain pending. A
planned compatibility milestone is manual validation work, not evidence that the
corresponding application features are absent.

### Deferred functionality

Realtime neural conversion, neural output in Use/Test/external routes, bundled ML
backends or checkpoints, automatic ML downloads/installers, custom virtual audio
drivers, cloud processing, accounts, telemetry, and non-Windows platforms are not
part of the current prototype. Phase 3's optional neural path is local and offline.

## Conservative defaults

The application starts on Use with monitoring off, the Balanced reliability
profile, 0 semitones pitch/formant, 35% wet, Gate/expander off,
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

Offline model work additionally requires a user-prepared Python environment,
Seed-VC checkout, configuration, and pretrained checkpoints. None are bundled,
cloned, downloaded, or installed by Mam Voice Changer. See the
[local model backend setup guide](docs/voice-model-backend-setup.md).

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

On Use, refresh devices and save a detected or manual playback/capture pair. For
VB-CABLE this is commonly **CABLE Input** as Mam Voice Changer's playback endpoint
and **CABLE Output** as the receiving application's microphone. Names vary by
product; follow the paired capture endpoint shown by the app. Without a real
virtual capture endpoint, Test can still use headphones, but Use cannot become a
microphone source for another application.

## Validation commands

```powershell
npm ci
npx tsc --noEmit
npm test
npm run lint
npm run format:check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --debug --no-bundle --ci
```

Command presence does not imply a pass; report actual results from the checkout
being validated.
Runtime and audible behavior require separate manual validation with conservative
monitoring levels.

## Documentation

- [DSP design and parameters](docs/dsp.md)
- [Architecture](docs/architecture.md)
- [Audio routing](docs/audio-routing.md)
- [External-routing implementation note](docs/external-routing-implementation-note.md)
- [Voice Lab Phase 1 design](docs/voice-lab-phase-1-design.md)
- [Voice Dataset Phase 2 design](docs/voice-dataset-phase-2-design.md)
- [Voice Dataset Phase 2 implementation note](docs/voice-dataset-phase-2-implementation-note.md)
- [Voice Model Phase 3 design](docs/voice-model-phase-3-design.md)
- [Local model backend setup](docs/voice-model-backend-setup.md)
- [Model artifact lifecycle](docs/voice-model-artifact-lifecycle.md)
- [Privacy and consent boundary](docs/privacy.md)
- [Prototype scope](docs/prototype-scope.md)
- [Manual test plan](docs/manual-test-plan.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Technical stack, current structure, and roadmap](docs/Mam-Voice-Changer-Tech-Stack-and-Structure.md)

## Known limitations

- Input and output must expose a common sample rate.
- CPAL 0.15 device IDs are deterministic friendly-name fingerprints, not WASAPI
  endpoint GUIDs; duplicate friendly names therefore remain ambiguous.
- Virtual-device classification and automatic pairing use advisory endpoint names.
  Unknown or ambiguous products require a confirmed manual pair.
- The app does not install or bundle a virtual audio driver and cannot prove that a
  receiving application is consuming an enumerated capture endpoint.
- Latency is estimated from configured buffers and reported DSP delay; it is not a
  measured acoustic round trip.
- Formant processing is spectral and polyphonic rather than a monophonic PSOLA
  model. Extreme pitch/formant combinations can still sound synthetic.
- Compatibility and subjective listening quality have not been established by
  compile-time checks.
- Dataset quality and SNR values are heuristics. Dataset collection does not clone
  a voice, train a model, or establish that a profile can reproduce a speaker.
- Dataset files are local plaintext in application-managed storage. Explicit
  exports are outside application management and must be deleted separately.
- Third-party ML code executes locally but is not thereby trusted or sandboxed.
  Seed-VC/PyTorch compatibility, GPU support, training quality, voice similarity,
  and runtime resource fit depend on the user-prepared environment and Dataset.
- Model output is synthetic. Managed models are disabled when profile consent is
  revoked; exported models or audio remain the user's separate responsibility.

