# Manual Windows test plan

Automated tests avoid relying on a particular audio device. Run this plan on Windows
with headphones before marking Milestone 1 manually verified.

## Environment preflight

1. Confirm Windows 10/11 x64.
2. Confirm a physical microphone is enabled.
3. Install or identify VB-CABLE.
4. Confirm **CABLE Input** and **CABLE Output** are present in Windows Sound settings.
5. Set the microphone and both CABLE endpoints to a common format, preferably 48 kHz.

## Milestone 1 acceptance

1. Launch the application.
2. Refresh devices and verify all enabled input/output endpoints appear.
3. Select a physical microphone as input and **CABLE Input** as output.
4. Start passthrough and confirm the engine reaches `running`.
5. Speak and confirm both meters respond.
6. Monitor **CABLE Output** safely and verify continuous, unmodified audio.
7. Stop and start ten times; confirm no stale streams or invalid state.
8. Change devices while stopped, then restart.
9. Disable or disconnect the selected output and confirm the engine enters a recoverable
   error state with a useful message.
10. Refresh devices, select an available output, and restart successfully.
11. Run for 30 minutes and record underruns, overruns, and the latency estimate.

## Later milestone acceptance

Only after Milestone 1 passes, verify mute, bypass, gains, high-pass filtering, gate,
limiter, dry/wet, genuine pitch, reset, and persisted presets. Then test CABLE Output
in Discord and OBS. Record TikTok Live Studio separately.

## Validation performed on 2026-07-18

- Tauri debug executable built and launched successfully.
- React interface rendered successfully; screenshot captured in
  `docs/screenshots/milestone-1-ui.png`.
- WASAPI enumeration displayed the present Realtek microphone and speaker endpoints.
- Windows `Get-PnpDevice -Class AudioEndpoint -PresentOnly` confirmed those same two
  endpoints.
- VB-CABLE was not installed; no CABLE endpoints were available.
- Passthrough was not started against physical speakers because that could create
  acoustic feedback.
- Continuous audio, repeated start/stop, device disconnection, extended runtime,
  Discord, OBS, and TikTok Live Studio remain manually unverified.
