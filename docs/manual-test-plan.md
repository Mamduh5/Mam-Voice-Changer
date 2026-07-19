# Manual Windows test plan

Automated checks do not establish audible quality, live endpoint behavior, feedback safety, or third-party compatibility. Run this plan on Windows with conservative levels and record the devices, formats, profile, duration, counter changes, and audible observations for every completed section.

## A. Raw microphone baseline

1. Make a Windows Sound Recorder recording directly from the physical microphone.
2. Test the same raw microphone directly in Discord, without Mam Voice Changer in the route.
3. For both, record quiet, normal, and loud speech plus phrase beginnings and endings.
4. Note cuts, missing syllables, clicks, or discontinuities that already exist at the physical microphone.

## B. Use page

1. Launch the app and confirm **Hear myself** is off and the engine is stopped.
2. With only physical outputs present, confirm no processed destination is selected automatically and the routing notice explains the limitation.
3. Select a real processed playback destination. Use a virtual playback endpoint when routing to another application.
4. Start and stop the engine. Confirm meters move and engine state changes are clear.
5. Switch Natural, Warm tone, Bright tone, Old Lady, and a saved preset.
6. With local monitoring off, confirm no speaker/headphone talkback stream is opened merely for listening.
7. Deliberately enable persistent local monitoring with headphones, then confirm destination and monitor carry the same processed performance.

## C. Test page

1. Put on headphones before enabling monitoring.
2. Open Test and confirm monitoring remains off until the checkbox is explicitly enabled and Start is pressed.
3. Compare bypass and processed output, then compare presets and DSP controls.
4. Leave Test while running and confirm temporary monitoring stops.
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
6. With destination plus monitor active, fail only the monitor and confirm the destination continues in `degraded` state.
7. Fail the main destination and confirm it enters bounded `recovering`, then `running` or `error`.

## G. Receiving applications

1. Test Discord or OBS only when a real Windows capture endpoint exists, such as the capture side paired with a virtual playback device.
2. Select the virtual playback endpoint as Mam Voice Changer's processed destination.
3. Select the paired Windows capture endpoint in the receiving application.
4. Do not mark direct routing as passed when only speakers/headphones are available. Mam Voice Changer itself is not a registered Windows microphone device.

## Existing effect and persistence regression

1. Exercise preset apply/save/rename/duplicate/delete/reset and restart persistence.
2. Confirm the Old Lady Age Character, Breathiness, and Tremor controls still work.
3. Confirm mute affects both destinations, bypass remains latency-aligned, and limiter ceiling remains respected.
4. Relaunch after saving app settings. Confirm page, input, destination, monitor device, and profile restore, but the engine and temporary monitoring do not auto-start.
5. Exercise migrated v1, malformed, and future-version application settings; monitoring must remain off under every unsafe fallback.

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
