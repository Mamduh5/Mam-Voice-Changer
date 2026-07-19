# Phase 3 routing and reliability implementation note

## Current implementation

- `src/App.tsx` renders one combined routing, preset, DSP, diagnostics, and transport surface.
- `useAudioDevices` restores one input and one output and falls back to a Windows default output; that can select physical speakers.
- `StartRequest` requires one input and one output. CPAL opens one input stream and one output stream, so processed audio is always played by that output.
- The DSP worker owns one `DspChain`, writes to one bounded output ring, and uses the maximum negotiated callback buffer as its block size.
- Output buffering starts with zero-filled prefill rather than waiting for captured and processed audio.
- CPAL requests 256 frames where fixed buffer sizing is supported. Both rings are bounded to 250 ms.
- `NoiseGate` has hysteresis and smoothed attack/release but closes ordinary quiet speech toward absolute zero and has no hold or configurable attenuation floor.
- Diagnostics expose input overflow, DSP input underrun, DSP output overflow, and output underrun, but not callback gaps, expander attenuation, ring fill, deadline misses, concealment, prefill, restart attempts, or monitor-specific failures.
- Old Lady pitch jitter/tremor, aspiration, spectral aging, limiter placement, preset schema migration, and parameter reconciliation are already present and remain unchanged.

## Files and architecture

The implementation changes the controller, worker, stream callbacks, metrics, stream negotiation, application settings, device commands/types, React state, and the main UI. Cohesive modules are added for reliability profiles and dropout concealment; the existing gate module becomes a speech expander without changing the public Gate controls.

One DSP worker produces one linked, limited signal. It maps complete frames into independent bounded rings for an optional processed destination and optional local monitor. Neither output callback waits for the other. Normal Use requires an explicit processed destination and defaults monitoring off. Test starts a monitor-only route only after an explicit opt-in and stops that temporary route when leaving Test.

Input starts before output consumption. The controller waits for profile-specific processed-ring prefill with a bounded timeout, then starts destination and monitor streams. Output callbacks use bounded last-waveform decay and smooth recovery for short underruns. Main input/destination failures enter a bounded staged recovery policy. A local-monitor failure degrades monitoring without stopping an active main destination; a Test monitor-only failure enters the same bounded recovery path because it is the active route.

Application-settings schema migration separates processed destination and monitor device, persists a reliability profile and last page, and always migrates/defaults monitoring to false. Output classification is advisory across common virtual/loopback metadata; without a likely virtual playback endpoint the UI explains that Discord still needs a real Windows capture endpoint.

Ring-fill and callback timing diagnostics are added now. Adaptive clock correction remains disabled at ratio 1.0 until long-session measurements demonstrate persistent hardware-clock drift; larger buffers are not presented as a permanent drift solution.
