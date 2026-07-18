# Troubleshooting

## The app shows no audio devices

- Confirm the endpoint is enabled in Windows Sound settings.
- Grant desktop-app microphone access in Windows Privacy settings.
- Close exclusive-mode applications, then use **Refresh devices**.
- Restart the app after installing or enabling an audio driver.

## CABLE Input or CABLE Output is missing

- Install VB-CABLE and restart Windows if requested.
- In Windows Sound settings, enable disabled endpoints.
- Remember: CABLE Input appears in the app's output list; CABLE Output appears in the
  receiving application's microphone list.

## No compatible sample rate

The engine intentionally refuses to play mismatched-rate audio because resampling is not
implemented. Open the Advanced format settings for both endpoints and select a common
rate, preferably 48 kHz, then refresh devices.

## Output is silent

- Confirm the engine state is `running` and the input meter moves.
- Confirm the output meter moves and underruns are not increasing rapidly.
- For VB-CABLE, monitor CABLE Output rather than CABLE Input.
- Verify the receiving application has not muted or noise-suppressed its microphone.

## Underruns or overruns increase

- Close CPU-intensive applications.
- Use matching 48 kHz formats.
- Avoid Bluetooth endpoints during initial validation.
- Record the active format, latency estimate, and counter rate in a bug report.

Overflow drops newest samples; underflow writes silence. These policies preserve bounded
latency and prevent stale buffered audio from growing without limit.

## A device was unplugged

The engine should move to `error`, stop both streams, and display a recoverable message.
Reconnect the device or refresh and select an available endpoint, then start again.

## Pitch or presets are unavailable

This is intentional in Milestone 1. The former pitch control was amplitude processing,
not pitch shifting. Effects remain disabled until clean VB-CABLE passthrough is manually
verified.
