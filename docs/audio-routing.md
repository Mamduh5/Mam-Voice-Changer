# Audio routing

## Prototype signal path

```text
Physical microphone
        |
        v
Mam Voice Changer (latency-aligned bypass or processed signal)
        |
        v
CABLE Input
        |
        v
CABLE Output
        |
        v
Receiving application microphone input
```

`CABLE Input` is a Windows playback/output endpoint, so it belongs in Mam Voice
Changer's output selector. `CABLE Output` is a Windows capture/input endpoint, so it
belongs in Discord, OBS, or another receiving application's microphone selector.

## Setup

1. Install VB-CABLE from VB-Audio and restart Windows if the installer requests it.
2. Open Windows Sound settings and confirm both CABLE endpoints are enabled.
3. Prefer 48 kHz for the physical microphone and CABLE endpoints. Mam Voice Changer
   negotiates a common rate but does not resample incompatible devices.
4. Start Mam Voice Changer and refresh devices.
5. Choose a physical microphone as input.
6. Choose **CABLE Input** as output.
7. Verify the selected output and start the engine. Use bypass for the clean,
   latency-aligned routing comparison.
8. Choose **CABLE Output** as the microphone in the receiving application.

Use headphones. Routing a live microphone to speakers can create an acoustic
feedback loop. When the current output selection is unavailable, the frontend
prefers an endpoint whose friendly name contains `CABLE Input`; if none exists it
falls back to the Windows default output, which may be physical speakers. Always
verify the output selector before starting the engine.

No Discord, OBS, TikTok Live Studio, browser, or Facebook Live compatibility claim is
made until the corresponding manual test is completed and recorded.
