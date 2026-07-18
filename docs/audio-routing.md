# Audio routing

## Prototype signal path

```text
Physical microphone
        |
        v
Mam Voice Changer (clean passthrough in Milestone 1)
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
7. Start passthrough.
8. Choose **CABLE Output** as the microphone in the receiving application.

Use headphones. Routing a live microphone to speakers can create an acoustic feedback
loop, so the app does not automatically select physical speakers for testing.

No Discord, OBS, TikTok Live Studio, browser, or Facebook Live compatibility claim is
made until the corresponding manual test is completed and recorded.
