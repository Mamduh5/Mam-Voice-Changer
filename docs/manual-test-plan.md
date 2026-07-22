# Manual Windows test plan

Automated checks do not establish audible quality, live endpoint behavior, feedback safety, or third-party compatibility. Run this plan on Windows with conservative levels and record the devices, formats, profile, duration, counter changes, and audible observations for every completed section.

## A. Raw microphone baseline

1. Make a Windows Sound Recorder recording directly from the physical microphone.
2. Test the same raw microphone directly in Discord, without Mam Voice Changer in the route.
3. For both, record quiet, normal, and loud speech plus phrase beginnings and endings.
4. Note cuts, missing syllables, clicks, or discontinuities that already exist at the physical microphone.

## B. External-route setup matrix

### No virtual device

1. Launch with only a Realtek or comparable physical microphone and speakers.
2. Refresh devices and confirm no playback/capture pair is invented.
3. Confirm **Start using** remains disabled with actionable installation guidance.
4. Open Test and confirm headphone testing remains usable independently.

### Clear virtual pair

1. Install or enable one compatible virtual playback/capture pair and refresh.
2. Confirm both endpoints and one conservative automatic route appear.
3. Save and validate the route; record confidence, source, and negotiated rate.
4. Start Use and select the displayed capture endpoint in Discord.
5. Run Discord's microphone test and confirm the processed voice reaches it.
6. Stop Use and confirm Discord no longer receives the processed voice.

### Manual pair and persistence

1. Choose **Manual playback/capture pair**, select both sides, and save.
2. If deliberately testing a likely physical side, confirm the warning and record
   why; otherwise verify physical choices are not silently accepted.
3. Relaunch and confirm both sides restore uniquely with `manual / manual` provenance.
4. Create duplicate friendly names where practical and confirm ambiguous restore
   stays unset instead of selecting either endpoint.

### Route failure cases

1. Disable the paired capture while stopped, refresh, and verify `missingCapture`.
2. Disable capture while Use is active; confirm health does not claim end-to-end
   delivery, then stop/refresh and verify the missing state.
3. Disable the playback endpoint and verify route-specific recovery is bounded.
4. Rename or remove either side and confirm only unique friendly-name restoration.
5. Change Windows Default Format values to remove the shared rate, then verify an
   actionable incompatible-rate result; restore 48 kHz or 44.1 kHz afterward.

## C. Use page

1. Launch the app and confirm Use has no local-monitor control and the Use route is stopped.
2. Confirm Use displays playback, paired capture, readiness, and receiving-app instructions but no local-monitor control.
3. Save and validate a complete external route; incomplete, ambiguous, or incompatible routes must keep Start disabled.
4. Press **Start using**, then **Stop using**. Confirm meters move and Use-route state changes are clear.
5. Switch Natural, Warm tone, Bright tone, Old Lady, and a saved preset.
6. Confirm Use never opens a speaker/headphone talkback stream.
7. Navigate to Settings & Diagnostics while Use is active and confirm the Use route continues.
8. Navigate into and back out of Test while Use is active without pressing **Start hearing test**; confirm Use continues.

## D. Test page

1. Put on headphones before starting monitoring.
2. Open Test and confirm monitoring remains off until **Start hearing test** is pressed, with no extra enable checkbox.
3. Compare bypass and processed output, then compare presets and DSP controls.
4. Press **Stop test**, restart, then leave Test while running and confirm monitoring stops.
5. Repeat while the engine is starting or recovering and confirm monitoring does not remain active afterward.
6. Never perform this section through open speakers; stop immediately if feedback starts.

## E. Reliability profiles

For Low latency, Balanced, and Reliable:

1. Stop the engine, choose the profile, and restart.
2. Record negotiated callback sizes, ring capacities implied by the profile, prefill target/actual, and estimated latency.
3. Compare perceived latency, audible cuts, callback gaps, ring overflows/underruns, DSP deadline misses, and concealed frames.
4. Run each relevant profile for 30 minutes and record min/current/max ring fill. Look for a steady long-term trend that could indicate device-clock drift.

## F. Weak microphone behavior

1. Compare Gate disabled with the speech expander enabled.
2. Test quiet syllables, phrase beginnings/endings, and normal background noise.
3. Listen for chopping, pumping, clicks, swallowed consonants, and excessive noise lift.
4. Confirm quiet speech becomes smoothly attenuated rather than hard-zeroed.

## G. Recovery

1. While running, disable and re-enable the selected microphone.
2. Unplug and reconnect applicable USB input, destination, and monitor devices.
3. Change the Windows default endpoint; verify the stored identifier remains authoritative and unique friendly-name restoration is conservative.
4. Exercise sleep/wake.
5. Confirm exact errors are visible, restart count is bounded, staged recovery does not loop tightly, and Stop works during recovery.
6. Fail the Test monitor and confirm the Test route enters bounded `recovering`, then `running` or `error`.
7. Fail the Use destination and confirm it enters bounded `recovering`, then `running` or `error`.

## H. Receiving applications

1. Test only when a real Windows capture endpoint is paired with the selected virtual playback endpoint.
2. Test Discord, OBS, browser microphone selection, and one additional available streaming or communication app.
3. In each application, select the exact paired capture endpoint displayed on Use and run its own meter or microphone test.
4. Record application version, virtual-device product/version, endpoint names,
   sample rates, reliability profile, latency estimate, underruns, concealed
   frames, recovery events, and audible observations.
5. Do not mark direct routing as passed when only speakers/headphones are available or merely because the capture endpoint is enumerated.

## I. Shared settings and persistence regression

1. Exercise preset apply/save/rename/duplicate/delete/reset and restart persistence.
2. Confirm the Old Lady Age Character, Breathiness, and Tremor controls still work.
3. Confirm mute affects both route purposes, bypass remains latency-aligned, and limiter ceiling remains respected.
4. Change DSP controls in Test, start Use, and confirm Use uses the same snapshot.
5. Apply a preset in Use, open Test, and confirm the same preset and controls render.
6. Relaunch after saving app settings. Confirm page, input, playback/capture pair,
   manual source, monitor device, and profile restore, but neither route auto-starts.
7. Exercise migrated v1/v2/v3, malformed, and future-version application settings;
   capture stays unset when migration is ambiguous and neither route auto-starts.

## J. Voice Lab Phase 1

1. With Use and Test stopped, record short mono/stereo microphone clips at both supported rates where the device permits. Confirm recording stops at 15 seconds and Clear releases the clip.
2. Import PCM 16/24/32-bit and 32-bit-float WAV files at 44.1/48 kHz. Confirm unsupported rates, channel counts, encodings, and clips longer than 15 seconds produce actionable errors.
3. Render Natural and Old Lady settings. Compare original and processed playback through headphones, replay each, enable loop before starting preview, and verify A/B timing is aligned.
4. Change Lab controls and confirm Test/Use controls and sound do not change. Apply an existing preset to Lab and confirm the live selected preset does not change.
5. Save the Lab configuration as a new user preset. Confirm it appears in the catalog while the prior live selected preset and live parameters remain unchanged across restart.
6. Press **Apply to live settings** and only then confirm Test/Use receive the complete Lab parameter snapshot.
7. Export original and processed WAV files to explicit paths. Re-import each and confirm duration, channels, rate, and audible content. Cancel each dialog once and confirm no file is created.
8. Start Use or Test and confirm Lab record, render, and preview are unavailable. Start a Lab record or preview and confirm Use/Test cannot start until it stops.
9. Navigate away during recording and preview. Confirm Lab audio stops, the source remains available on return, and Clear drops both original and processed buffers.
10. Confirm no model download, voice-cloning, training, embedding, neural inference, cloud request, or realtime AI conversion occurs.

## K. Voice Dataset Capture Phase 2

Run with a consenting test speaker and headphones. Record the exact microphone,
preview output, file formats, profile health, quality values, and any filesystem
errors. Do not mark audible quality passed without listening.

1. Launch the application.
2. Open **Voice Lab → Dataset**.
3. Create a profile.
4. Confirm consent is required.
5. Confirm recording cannot begin before consent.
6. Select the physical microphone.
7. Record a normal prompted phrase.
8. Stop and review it.
9. Listen through headphones.
10. Inspect waveform and quality measurements.
11. Accept the take.
12. Record a deliberately clipped take.
13. Confirm the clipping warning.
14. Reject or redo it.
15. Record a take with long leading silence.
16. Test automatic trim.
17. Compare raw and trimmed versions.
18. Accept the trimmed version.
19. Import a stereo 44.1 kHz WAV.
20. Confirm canonical mono 48 kHz PCM24 conversion and pending status.
21. Import the same WAV again.
22. Confirm exact-duplicate detection.
23. Navigate away during recording.
24. Confirm unfinished audio is discarded.
25. Return to Dataset.
26. Confirm finalized takes remain.
27. Start Test.
28. Confirm Dataset recording is blocked.
29. Stop Test.
30. Start Dataset preview.
31. Confirm Use and Voice Lab audio are blocked.
32. Export the accepted dataset.
33. Inspect manifest, consent, prompt pack, README, and relative audio paths.
34. Confirm rejected, pending, excluded, and recorded-consent takes are excluded by default.
35. Restart the application.
36. Confirm the profile and accepted takes restore.
37. Delete one take.
38. Delete the complete profile.
39. Confirm managed files are removed or an exact partial-deletion error is shown.
40. Confirm exported copies remain and the UI explains that they require separate deletion.

Additional cases: no microphone; microphone removal during recording; preview-device
removal; forced ring overflow; a sub-one-second take; excessive background noise;
a WAV with DC offset; invalid/compressed/empty WAV; corrupt manifest; missing take
file; interrupted `.tmp`/`.bak` write; partial deletion under an external file lock;
unsupported future schema; a custom non-ASCII prompt; optional recorded consent and
separate deletion; failed-take override acknowledgement; import batch bound; export
cancellation/failure cleanup; and a long collection session with many takes.

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

- All live/audio sections A-I above for this external-routing phase.
- Old Lady listening and full preset/application-settings persistence interaction.
- Thirty-minute profile runs and clock-drift trend collection.
- Discord, OBS, browser, and another receiving-app route when a real virtual pair is available.
- Voice Lab microphone capture, Windows open/save dialogs, audible A/B alignment and quality,
  looping, explicit export/re-import, live isolation, and memory-clear behavior in section J.
- All Voice Dataset hardware, audible, dialog, restart, device-removal, export inspection,
  partial-deletion, and long-session cases in section K. Phase 2 automated checks use generated
  audio/metadata only and are not evidence that these manual cases passed.
