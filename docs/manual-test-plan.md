# Manual Windows test plan

Automated checks do not establish audible quality, live endpoint behavior, feedback safety, or third-party compatibility. Run this plan on Windows with conservative levels and record the devices, formats, profile, duration, counter changes, and audible observations for every completed section.

## A. Raw microphone baseline

1. Make a Windows Sound Recorder recording directly from the physical microphone.
2. Test the same raw microphone directly in Discord, without Mam Voice Changer in the route.
3. For both, record quiet, normal, and loud speech plus phrase beginnings and endings.
4. Note cuts, missing syllables, clicks, or discontinuities that already exist at the physical microphone.

## B. Use page

1. Launch the app and confirm Use has no local-monitor control and the Use route is stopped.
2. With only physical outputs present, confirm no processed destination is selected automatically and the routing notice explains the limitation.
3. Select a real processed playback destination. Use a virtual playback endpoint when routing to another application.
4. Press **Start using**, then **Stop using**. Confirm meters move and Use-route state changes are clear.
5. Switch Natural, Warm tone, Bright tone, Old Lady, and a saved preset.
6. Confirm Use never opens a speaker/headphone talkback stream.
7. Navigate to Settings & Diagnostics while Use is active and confirm the Use route continues.
8. Navigate into and back out of Test while Use is active without pressing **Start hearing test**; confirm Use continues.

## C. Test page

1. Put on headphones before starting monitoring.
2. Open Test and confirm monitoring remains off until **Start hearing test** is pressed, with no extra enable checkbox.
3. Compare bypass and processed output, then compare presets and DSP controls.
4. Press **Stop test**, restart, then leave Test while running and confirm monitoring stops.
5. Repeat while the engine is starting or recovering and confirm monitoring does not remain active afterward.
6. Never perform this section through open speakers; stop immediately if feedback starts.

## D. Reliability profiles

For Low latency, Balanced, and Reliable:

1. Stop the engine, choose the profile, and restart.
2. Record negotiated callback sizes, ring capacities implied by the profile, prefill target/actual, and estimated latency.
3. Compare perceived latency, audible cuts, callback gaps, ring overflows/underruns, DSP deadline misses, and concealed frames.
4. Run each relevant profile for 30 minutes and record min/current/max ring fill. Look for a steady long-term trend that could indicate device-clock drift.

## E. Weak microphone behavior

1. Compare Gate disabled with the speech expander enabled.
2. Test quiet syllables, phrase beginnings/endings, and normal background noise.
3. Listen for chopping, pumping, clicks, swallowed consonants, and excessive noise lift.
4. Confirm quiet speech becomes smoothly attenuated rather than hard-zeroed.

## F. Recovery

1. While running, disable and re-enable the selected microphone.
2. Unplug and reconnect applicable USB input, destination, and monitor devices.
3. Change the Windows default endpoint; verify the stored identifier remains authoritative and unique friendly-name restoration is conservative.
4. Exercise sleep/wake.
5. Confirm exact errors are visible, restart count is bounded, staged recovery does not loop tightly, and Stop works during recovery.
6. Fail the Test monitor and confirm the Test route enters bounded `recovering`, then `running` or `error`.
7. Fail the Use destination and confirm it enters bounded `recovering`, then `running` or `error`.

## G. Receiving applications

1. Test Discord or OBS only when a real Windows capture endpoint exists, such as the capture side paired with a virtual playback device.
2. Select the virtual playback endpoint as Mam Voice Changer's processed destination.
3. Select the paired Windows capture endpoint in the receiving application.
4. Do not mark direct routing as passed when only speakers/headphones are available. Mam Voice Changer itself is not a registered Windows microphone device.

## Existing effect and persistence regression

1. Exercise preset apply/save/rename/duplicate/delete/reset and restart persistence.
2. Confirm the Old Lady Age Character, Breathiness, and Tremor controls still work.
3. Confirm mute affects both route purposes, bypass remains latency-aligned, and limiter ceiling remains respected.
4. Relaunch after saving app settings. Confirm page, input, destination, monitor device, and profile restore, but neither route auto-starts.
5. Exercise migrated v1/v2, malformed, and future-version application settings; Test monitoring must never auto-start.

## Manual record

### Previously performed on 2026-07-18

- Tauri debug executable launched and the earlier React interface rendered.
- Present Realtek microphone and speaker endpoints were enumerated.
- VB-CABLE was not installed.
- Live passthrough to physical speakers was deliberately not started because of feedback risk.

### Performed for Phase 3 on 2026-07-20

- No live microphone, audible comparison, UI interaction, endpoint-disconnection, sleep/wake, or third-party routing test was performed during this implementation run.
- Automated commands are reported separately in the completion response; they are not evidence of audible improvement.

### Pending

- All Phase 3 sections A-G above.
- Old Lady listening and full preset/application-settings persistence interaction.
- Thirty-minute profile runs and clock-drift trend collection.
- Discord/OBS routing when a real Windows capture endpoint becomes available.
