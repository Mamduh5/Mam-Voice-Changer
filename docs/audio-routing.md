# Audio routing

## Separate route purposes

```text
Physical microphone
        |
        v
one DSP chain (preset, bypass, limiter, mute)
        |
        +--> processed destination (normal Use route)
        |
        +--> optional local monitor (off by default)
```

The DSP worker processes each block once and fans the same samples into independent bounded destination and monitor rings. Neither output callback waits for the other. A monitor failure degrades only monitoring when the main destination remains healthy; an input or main-destination failure enters bounded recovery.

Use never opens a speaker/headphone merely so the user can hear themselves. Test opens a monitor-only route only after the user checks the temporary-monitor option and presses Start. Leaving Test clears that temporary choice and requests Stop.

## Receiving applications

A physical output endpoint plays through speakers or headphones. Discord, OBS, and similar applications can choose only Windows capture endpoints as microphone inputs. Mam Voice Changer does not register a Windows capture endpoint and this phase does not implement a virtual microphone driver.

With a third-party virtual audio device, the route is commonly:

```text
physical microphone -> Mam Voice Changer -> virtual playback endpoint
virtual capture endpoint -> Discord / OBS microphone input
```

Endpoint classification in the app is advisory and uses available friendly-name metadata; it is not hard-coded to one vendor. If no likely virtual playback endpoint is present, no physical speaker is automatically promoted to the processed destination.

## Safety

Use headphones for Test monitoring. Local monitoring defaults off on first launch and unsafe settings recovery. The engine never starts automatically after launch. Verify every destination before Start; endpoint labels do not guarantee a feedback-safe physical setup.
