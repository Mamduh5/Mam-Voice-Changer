# Troubleshooting

## Desktop runtime unavailable

`npm run dev:web` intentionally opens a frontend-only page and disables native
audio controls. Launch the desktop application with `npm run dev`.

## No audio devices

- Enable the endpoint in Windows Sound settings.
- Grant desktop-app microphone access in Windows Privacy settings.
- Close exclusive-mode applications and refresh devices.
- Restart the app after changing an audio driver.

## No external route is available

- Install or enable a compatible third-party Windows virtual audio pair.
- Select **Refresh devices** on Use after any driver or Windows Sound change.
- Confirm that both one playback endpoint and one capture endpoint are enabled.
- Do not select physical speakers merely to make Start using available; they cannot
  appear as another application's microphone.

## CABLE Input or CABLE Output is missing

- Install VB-CABLE and restart Windows if requested.
- Enable disabled endpoints in Windows Sound settings.
- Refresh devices, save CABLE Input as playback with CABLE Output as capture, then
  select CABLE Output as the receiving application's microphone.

Other products may use different or reversed-looking names. Follow endpoint
direction and the paired capture name shown by the app.

## Pairing is ambiguous or a saved endpoint disappeared

- Review all candidate endpoints instead of accepting a same-vendor guess.
- Choose playback and capture manually, then save the route.
- If either side is likely physical, confirm the advanced warning only when that
  choice is deliberate.
- Duplicate friendly names produce the same app fingerprint and are not restored
  automatically. Disable the duplicate or rename/configure the endpoints in the
  device software where possible, then refresh.

## No compatible sample rate

The engine does not silently resample. Configure both endpoints to a common rate,
preferably 48 kHz, then refresh devices.

The static route summary compares the virtual pair's advertised 44.1/48 kHz
support. Final validation negotiates the physical input against the playback side.
No success message means resampling occurred; this prototype does not resample.

## Playback is active but Discord or OBS is silent

- Select the exact paired capture endpoint shown on Use as the application's mic.
- Check the receiving application's input meter or microphone test.
- Confirm it is not muted and that application microphone permission is enabled.
- Avoid exclusive-mode ownership by another application.
- Remember that `Capture endpoint available` proves enumeration only, not that the
  receiving application selected or consumed it.

## The transformed voice is harsh or tiring

- Lower Windows/headphone monitoring volume first.
- Return pitch and formant to 0 semitones.
- Keep dry/wet near the conservative 35% default and increase gradually.
- Return warmth and brightness to 0 dB.
- Leave output gain at -6 dB and the master limiter enabled at -3 dBFS.
- Disable the noise gate while isolating artifacts.

Large pitch/formant combinations can sound synthetic even when digital samples
stay below the limiter ceiling. A dBFS ceiling does not measure sound-pressure
level at your ears and cannot make feedback or loud headphones safe.

## Output is silent

- Confirm the engine is running and mute is off.
- Confirm the input meter moves.
- Check that the saved playback/capture pair is still ready and that the receiving
  application selected the capture side.
- Inspect underrun/overrun counters.
- Temporarily use 0 semitone pitch/formant and 0% wet to isolate routing from
  transformation.

## Underruns or overruns increase

- Close CPU-intensive applications.
- Prefer matching 48 kHz endpoint formats.
- Avoid Bluetooth endpoints during initial diagnosis.
- Record the active format, reported DSP latency, and counter rate.

Overflow drops newest input samples; underflow writes silence so buffered latency
cannot grow without bound.

## Signalsmith does not compile

Use the Rust MSVC toolchain and install Microsoft C++ Build Tools. The vendored
backend is compiled statically and does not require a Signalsmith DLL or libclang.
Do not replace a failed native build with mocked pitch or formant results.

