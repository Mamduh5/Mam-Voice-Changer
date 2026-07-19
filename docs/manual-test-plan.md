# Manual Windows test plan

Automated tests avoid relying on a particular audio device. Run this plan on
Windows with headphones before claiming live routing, audible behavior, device
restoration, preset persistence, or third-party compatibility. An implemented
feature remains manually pending until its steps are performed and recorded.

## Environment preflight

1. Confirm Windows 10/11 x64.
2. Confirm a physical microphone is enabled.
3. Install or identify VB-CABLE.
4. Confirm **CABLE Input** and **CABLE Output** are present in Windows Sound settings.
5. Set the microphone and both CABLE endpoints to a common format, preferably 48 kHz.

## Live routing acceptance

1. Launch the application.
2. Refresh devices and verify all enabled input/output endpoints appear.
3. Select a physical microphone as input and **CABLE Input** as output.
4. Enable bypass, start the engine, and confirm it reaches `running`.
5. Speak and confirm both meters respond.
6. Monitor **CABLE Output** safely and verify continuous, unmodified audio.
7. Stop and start ten times; confirm no stale streams or invalid state.
8. Change devices while stopped, then restart.
9. Disable or disconnect the selected output and confirm the engine enters a recoverable
   error state with a useful message.
10. Refresh devices, select an available output, and restart successfully.
11. Run for 30 minutes and record underruns, overruns, and the latency estimate.

## DSP acceptance

With safe monitoring available, verify mute, bypass, both gains, high-pass
filtering, limiting and ceiling changes, pitch, formant shift, dry/wet, gate,
warmth, and brightness while the engine is running. Check conservative and extreme
valid values, then return every control to its default. Listen for clicks,
non-finite failure symptoms, unstable volume, and unexpected channel differences.
These manual checks do not replace the focused device-independent DSP tests.

## Preset acceptance

1. Select each built-in preset and confirm the controls match its parameter snapshot.
2. Change several parameters, save a named preset, and confirm it appears under
   **My presets** and becomes selected.
3. Apply a different preset, return to the saved preset, and confirm every DSP
   parameter is restored.
4. Rename the user preset and confirm the new name persists.
5. Duplicate both a built-in preset and a user preset; confirm each duplicate is a
   newly selected, editable user preset with a unique copy name.
6. Confirm built-in Rename and Delete actions are unavailable.
7. Delete a non-selected user preset and confirm the active parameters do not change.
8. Delete the selected user preset and confirm selection and parameters fall back to
   `Natural`.
9. Change parameters, choose Reset, and confirm `Natural` and its complete default
   snapshot are restored.
10. Close and relaunch the desktop application. Confirm user presets and the last
    selected preset survive the restart and are applied before the engine starts.
11. Trigger a recoverable invalid-name or storage error where practical, then retry
    successfully and confirm the visible error clears.

## Device-selection restoration acceptance

1. Select known input and output endpoints while the engine is stopped, close the
   application, relaunch it, and confirm both selections are restored without
   starting the engine.
2. Confirm restoration by stored identifier when the original endpoints remain.
3. Rename or remove a stored endpoint and verify the conservative friendly-name or
   default-device fallback shown by the UI.
4. With duplicate friendly names, confirm no ambiguous name-only match is selected.
5. Exercise missing, corrupt, and unsupported-version settings documents and confirm
   startup remains usable with a visible, recoverable fallback state.

## Compatibility and extended-run acceptance

After local routing is safe and stable, test VB-CABLE, Discord microphone input, OBS
capture, TikTok Live Studio where available, and a 30-minute continuous run. These
are planned manual compatibility checks, not missing application features. Record
the application versions, selected endpoints, stream format, estimated latency,
overrun/underrun counts, and any audible artifacts.

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
- Preset apply/save/rename/duplicate/delete/reset and restart persistence were not
  exercised and remain manually unverified.
- Device-selection persistence and fallback behavior were not exercised and remain
  manually unverified.

## Automated validation record

The repository contains device-independent frontend and Rust tests, but this file
does not record a complete automated command run for the current checkout. Add
dated commands and actual results only after they are run; do not infer audible or
third-party compatibility from a passing suite.
