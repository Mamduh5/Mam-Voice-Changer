# Audio routing

## Separate route purposes

```text
Physical microphone
        |
        v
one DSP chain (preset, bypass, limiter, mute)
        |
        +--> processed destination (Use route)
        or
        +--> selected local monitor (Test route)
```

Each tagged start request selects exactly one output purpose. Use requires a processed destination and cannot include a monitor. Test requires a local monitor and cannot include a processed destination. Both variants read the same authoritative DSP parameter snapshot and preset state.

Use never opens a speaker/headphone merely so the user can hear themselves. Test opens a monitor-only route when the user presses **Start hearing test**; there is no separate enable checkbox. Leaving Test invokes a backend conditional stop that stops Test monitoring but leaves a Use route untouched.

## Receiving applications

A physical output endpoint plays through speakers or headphones. Discord, OBS, and similar applications can choose only Windows capture endpoints as microphone inputs. Mam Voice Changer does not register a Windows capture endpoint and this phase does not implement a virtual microphone driver.

With a third-party virtual audio device, the route is commonly:

```text
physical microphone -> Mam Voice Changer -> virtual playback endpoint
virtual capture endpoint -> Discord / OBS microphone input
```

Endpoint classification in the app is advisory and uses available friendly-name metadata; it is not hard-coded to one vendor. If no likely virtual playback endpoint is present, no physical speaker is automatically promoted to the processed destination.

## Safety

Use headphones for Test monitoring. No monitoring-enabled boolean is persisted and the engine never starts automatically after launch. Verify every destination before Start; endpoint labels do not guarantee a feedback-safe physical setup.
