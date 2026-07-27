# Windows setup

Mam Voice Changer processes a physical microphone locally, but it is not itself a
Windows microphone device. To use the changed voice in Discord, a game, or
another voice application, install a compatible virtual audio cable that exposes
a paired playback endpoint and recording/capture endpoint.

Windows does not necessarily include a general-purpose virtual audio cable as a
built-in Microsoft component. Choose a product you trust, follow its installation
instructions, and restart applications that were open during installation.

## Signal path

```text
Physical microphone
    -> Mam Voice Changer
    -> Virtual audio cable
    -> Discord/game/voice application
```

The two sides of a virtual cable may have counterintuitive names. In Mam Voice
Changer, select the cable's **playback/input side** as the processed output. In
the target application, select the same cable pair's **recording/output side** as
its microphone. Endpoint names vary, so use the paired capture endpoint displayed
by Mam Voice Changer instead of relying on words such as "input" or "output."

## Quick setup

1. Open **Setup / help** and select the physical microphone.
2. Select or save the virtual cable playback/capture pair.
3. Optionally select headphones as the Test monitor. Leave this empty if local
   monitoring is unnecessary.
4. Apply safe defaults if you are unsure which voice values to use.
5. Start **Use** processing. Processing never starts automatically when the app
   opens or when devices are refreshed.
6. In Discord, the game, or another voice application, select the displayed
   paired capture endpoint as its microphone.
7. Speak and check both Mam Voice Changer's input meter and the target
   application's microphone test.

Use headphones for Test. Speakers near an active microphone can create loud
feedback. Begin with a low Windows/headphone volume and increase it gradually.
The app's limiter cannot measure acoustic volume or guarantee safe listening.

## If a device is missing

- Press **Refresh devices** after installing, enabling, disconnecting, or
  reconnecting an audio device.
- If a saved device disappeared, the app marks it unavailable; deliberately
  select a replacement rather than assuming another similarly named device is
  equivalent.
- If the target application cannot see the cable's capture side, close and
  reopen that application, check Windows Sound settings, verify that the virtual
  cable is enabled, and then refresh Mam Voice Changer.
- If a device is already in exclusive use, close the application holding it or
  disable exclusive mode in the relevant Windows device properties, then retry.
- If formats are incompatible, configure the microphone and playback endpoint
  to a shared format, preferably 48 kHz, and refresh.
- If microphone access is denied, enable desktop microphone access in Windows
  Privacy settings.

**Settings & Diagnostics** provides a read-only readiness check, practical next
actions, latched clipping status, the last successful local configuration, and a
diagnostic report you can review before sharing. A ready route proves only that
the endpoints are available and compatible; it does not prove that the target
application selected the capture endpoint.

## Stop or return to the real microphone

Press **Stop using** before changing endpoints or closing the app. Normal
application shutdown also requests a clean audio stop.

To return to the unprocessed microphone, stop Mam Voice Changer and select the
physical microphone again in the target application's audio settings. You may
leave the virtual cable installed for later use.

## Current limitations

- The application is a prototype.
- Transformation quality depends on the source voice, microphone, room, and
  settings.
- Pitch and formant changes do not guarantee convincing male/female or age
  conversion. Extreme settings can sound artificial.
- Noise and feedback reduce intelligibility.
- Virtual-audio routing is separate Windows system configuration.
- WORLD is evaluator-only and is not used in live processing.
- Existing listening evidence came from one person and is not general user
  validation.
