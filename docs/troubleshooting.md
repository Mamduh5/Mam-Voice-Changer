# Troubleshooting

## Desktop runtime unavailable

`npm run dev:web` intentionally opens a frontend-only page and disables native
audio controls. Launch the desktop application with `npm run dev`.

## No audio devices

- Enable the endpoint in Windows Sound settings.
- Grant desktop-app microphone access in Windows Privacy settings.
- Close exclusive-mode applications and refresh devices.
- Restart the app after changing an audio driver.

## CABLE Input or CABLE Output is missing

- Install VB-CABLE and restart Windows if requested.
- Enable disabled endpoints in Windows Sound settings.
- Select CABLE Input as this app's output and CABLE Output as the receiving app's
  microphone.

## No compatible sample rate

The engine does not silently resample. Configure both endpoints to a common rate,
preferably 48 kHz, then refresh devices.

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
- Check that the selected output is monitored by the intended receiving device.
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

