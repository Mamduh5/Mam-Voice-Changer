# External-routing implementation note

## Current and target behavior

The previous Use route stored one processed playback destination. That was enough
to write samples but not enough to identify the Windows capture endpoint a
receiving application must select. The schema-v4 model now stores a complete
external route: stable route id, playback endpoint, optional capture endpoint,
candidate captures, confidence, source, validation status, compatibility details,
and manual status.

```text
Physical microphone -> Mam Voice Changer -> virtual playback endpoint
-> paired virtual capture endpoint -> receiving application
```

## Pairing and manual fallback

Discovery classifies input and output directions independently, normalizes product
families, recognizes a small set of complementary naming conventions, and accepts
an automatic pair only when the best choice is unique in both directions. It does
not pair unrelated families, tied candidates, duplicate identifiers, or physical
endpoints. The current rules are vendor-neutral and do not use a shared vendor
name alone as evidence that two different product families belong together.

Ambiguous and unpaired playback endpoints remain in the catalog with candidate
capture metadata. The user may save an explicit pair. Likely physical sides need
an additional confirmation. Manual provenance is persisted and restored only when
both endpoint identities resolve uniquely.

## Validation and runtime boundary

Readiness distinguishes missing input, playback, capture, ambiguous pairing,
incompatible format, removed devices, and an already-active route. Validation
enumerates the capture side but does not hold it open. It negotiates the physical
input against the playback endpoint because those are the two CPAL streams Use
actually opens. A ready result is configuration evidence, not an end-to-end signal
measurement.

The public Use start request carries a saved route id, not arbitrary capture text.
Rust resolves the route again and opens only physical input plus virtual playback.
Test remains input plus local monitor. Route tags enforce mutual exclusion and
recovery recreates only the active variant's devices.

## Settings migration

Application settings move from v3 to v4. Input, monitor, reliability profile, and
last page are preserved. The former processed destination becomes a playback
candidate. Capture stays unset unless current discovery finds exactly one safe
pair; ambiguity never substitutes the physical microphone and no route starts
automatically. Current settings persist both endpoint ids and friendly names,
pairing source, and manual status using atomic file replacement.

## Windows and CPAL limitations

CPAL 0.15 exposes friendly names and stream capabilities here, not durable WASAPI
endpoint GUIDs. App ids are direction-scoped FNV fingerprints of friendly names,
so duplicate names are explicitly ambiguous. Naming is not standardized, and
advertised ranges do not prove a third-party virtual transport is functioning.
The engine supports a shared 48 kHz or 44.1 kHz rate and does not resample.

No custom virtual audio driver is included or installed. The app cannot prove a
receiving application is consuming the capture endpoint without a separate
integration or a bounded capture-side signal measurement; neither is implemented
in this phase.
