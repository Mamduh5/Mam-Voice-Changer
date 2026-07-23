# Audio routing

## External-application signal path

```text
Physical microphone
  -> Mam Voice Changer DSP
  -> virtual playback endpoint
  -> paired virtual capture endpoint
  -> Discord / OBS / browser / streaming application
```

Windows applications select capture endpoints as microphones. Mam Voice Changer
writes processed samples to a playback endpoint, so a compatible third-party
virtual playback/capture pair is required to bridge those samples back into a
Windows capture endpoint. Physical speakers are playback endpoints only and are
never chosen automatically as an external route.

Some products use counterintuitive names. With VB-CABLE, for example, the app
commonly writes to `CABLE Input`, while Discord selects `CABLE Output`. Other
products differ; use the exact paired capture name shown on Use rather than
assuming every endpoint containing `Output` is a microphone.

## Discovery and pairing

Device discovery keeps the raw input/output lists and adds advisory direction,
virtual classification, normalized family, common 44.1/48 kHz rates, and channel
counts. Classification is based on endpoint names exposed by CPAL/Windows and is
not proof that an endpoint is virtual.

Automatic pairing is deliberately conservative:

1. Known complementary names receive `exact / knownPattern` confidence.
2. A unique shared normalized product name receives `high / normalizedName`.
3. The best playback and capture choices must be mutual and unique.
4. Equal candidates, duplicate friendly-name fingerprints, unrelated families,
   missing capture endpoints, and ordinary physical devices are not auto-paired.

Ambiguous candidates remain visible. Select playback and capture manually, review
the warning, and choose **Save external route**. A manual pair is persisted with
`manual / manual` provenance. Selecting a likely physical endpoint is allowed only
after explicit confirmation; this is an advanced escape hatch, not a claim that
physical speakers can feed another application.

Use **Refresh devices** after enabling, disabling, renaming, installing, or
removing audio endpoints. Saved ids are restored first; only one unique friendly
name may be used as a fallback. Ambiguous restoration remains unset.

## Readiness and start behavior

**Validate route** checks that:

- one physical input is selected;
- the saved playback endpoint is present exactly once;
- the paired capture endpoint is still enumerated exactly once;
- discovery is not ambiguous; and
- the physical input and playback output can negotiate a common stream rate.

This check does not continuously open the capture endpoint and does not generate
or save audio. Start using stays disabled until a saved route validates as ready.
Use then opens only the physical input and virtual playback output. Test opens only
the physical input and selected local monitor. They are mutually exclusive and
share the same preset/DSP snapshot.

The engine prefers 48 kHz, falls back to 44.1 kHz, and does not resample endpoints
without a common rate. Align Windows Default Format values when validation reports
an incompatibility.

## Receiving applications and health language

After Use is running, select the displayed paired capture endpoint in the
receiving application's microphone settings and verify its own meter or mic test.
`Playback active` means Mam Voice Changer is writing to the playback endpoint.
`Capture endpoint available` means Windows still enumerates the paired capture
endpoint. Neither proves Discord, OBS, or a browser has selected or consumed it;
the app has no integration API that can truthfully report `Discord connected`.

## Safety and driver boundary

Use headphones for Test and avoid monitoring the virtual capture through speakers;
that can create a feedback loop. Start with low system/headphone volume.

Mam Voice Changer does not install, bundle, or implement a custom Windows virtual
audio driver. Compatibility depends on a separately installed virtual-audio
product and must be confirmed manually for each receiving application.
