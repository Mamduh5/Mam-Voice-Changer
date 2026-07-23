# Phase 4.1 — Preview compatibility, Profiles workspace, and compact Voice Lab UI

## Scope and invariants

Phase 4.1 changes the isolated Voice Lab preview path and the frontend ownership of
voice-profile selection and management. It does not redesign the Use, Test, or
external-route paths; the live `AudioController`; CPAL live stream negotiation;
recovery and reliability behavior; `DspChain` order; Dataset canonical PCM24 mono
48 kHz storage; Dataset manifests; training snapshots; model worker protocol;
artifact packages; or backend qualification state transitions.

Voice Lab original and processed clips remain Rust-owned, bounded, normalized
`f32` buffers. Preview conversion never mutates either source clip and never
changes exported WAV or model-inference files.

## Preview sample-rate conversion

The preview path is:

```text
Rust-owned original or processed AudioClip
  -> selected physical output device
  -> Voice-Lab-only output capability selection
  -> PreparedPreview at the selected output rate
  -> CPAL output callback (copy/channel-map only)
  -> selected physical output
```

A shared offline audio-rate conversion module owns deterministic linear
resampling for finite normalized interleaved `f32` audio. It validates channel
alignment and rates, bounds output frames before allocation, rejects non-finite
input, and clamps finite output to the normalized range. It is intentionally
named and documented as offline linear resampling, not mastering-quality sample
rate conversion.

The module is reusable by Dataset canonical conversion and Voice Lab preview
preparation. Voice Lab WAV import does not invoke it because an imported clip
must retain its original supported 44.1 or 48 kHz representation. Use and Test
never call this module.

## Output-rate negotiation

Voice Lab preview inspects only the selected output device. It does not restore,
substitute, or route to another output.

The selection order is:

1. A supported 48 kHz configuration.
2. The device default rate when that rate is inside a supported configuration.
3. Another rate inside a supported configuration, preferring the rate closest
   to 48 kHz.

For each rate, the selector prefers `f32`, then `i16`, then `u16`, and prefers
stereo, then mono, then the smallest other valid channel count. Unsupported
sample formats are excluded. The configured buffer size remains bounded by the
device-supported range.

This selector is separate from live input/output negotiation. Hardware-free
tests use plain output-capability values to cover selection and unsupported
configurations.

Errors remain explicit at each boundary: selected output unavailable, no
supported output configuration, preview preparation failure, offline resampling
failure, CPAL stream creation failure, CPAL stream start failure, and output
removal/runtime failure.

## Preview-buffer ownership and alignment

`PreparedPreview` owns:

- source clip ID;
- source sample rate;
- selected output sample rate;
- source channel count;
- output-rate frame count;
- the output-rate interleaved samples.

The Voice Lab session owns at most one prepared-preview cache entry. Its key
contains the source clip ID, selected output device ID, output sample rate,
output channel count, and output sample format. Creating/replacing the original,
rendering/replacing the processed clip, changing playback source, changing
output device/configuration, or clearing the session makes the current key
inapplicable; explicit clip/session mutations also clear the cache.

All allocation, validation, and resampling occur before `build_output_stream`
and `Stream::play`. The CPAL data callback only advances a bounded cursor,
maps the prepared mono/stereo frame to output channels, converts the normalized
sample to the selected CPAL sample format, writes silence after completion, and
updates atomics. It does not allocate, block, resample, log, or share mutable
state with live routing.

Original/processed switching carries position by time:

```text
time_seconds = old_preview_frame / old_preview_rate
new_preview_frame = time_seconds * new_preview_rate
```

The result is clamped to the new preview frame count. Loop restart resets to
frame zero in the prepared-rate domain, preventing accumulated conversion error.
Stopping is idempotent.

Preview status reports the clip sample rate, selected output sample rate,
whether resampling is active, output channels, and output sample format.

## Profile service ownership

`useVoiceProfiles` is the single frontend owner of:

- profile list loading;
- opaque selected profile ID;
- create, read/select, update, repair, export, and delete operations;
- consent and health summaries;
- selected manifest/dataset summary;
- model dependency summary supplied by the Models status;
- profile loading/error state.

Selection is never inferred from display name. Restored/backend selection is
accepted only when the opaque ID still exists. Unsupported or corrupt profiles
are not restored as the active Dataset/Models profile. Deletion clears the
shared selected ID and selected manifest before Dataset or Models can render
stale data.

`useVoiceDataset(selectedProfileId)` owns recording, import, review, trimming,
take preview, and Dataset export workflow state only. It does not list, create,
edit, repair, or delete profiles. `useVoiceModels` owns model operations and
filters work by the same selected profile ID; it does not own another profile
selection.

The existing Rust `DatasetStorage`, `profiles.json`, manifest, consent, and
profile CRUD commands remain authoritative. Profile deletion continues to
disable/cancel dependent model use before managed Dataset deletion.

## Dataset and Models dependencies

Dataset receives the shared profile service and selected ID. It displays a
compact profile header with consent, health, collection progress, a change
selector, and an Open Profiles action. With no selection it shows only the
instruction to select or create a profile and an Open Profiles action.

Models receives the same service and selected ID. It displays consent, Dataset
health, accepted duration, a change selector, and Open Profiles. Snapshot,
training, artifacts, inference, evaluation, qualification, and portability keep
their current backend commands and opaque profile association.

Profile changes select the corresponding Rust Dataset manifest. Dataset polling
updates the shared selected manifest. Deletion clears selection and therefore
invalidates both Dataset and Models views without cancelling unrelated completed
data. Leaving Models does not cancel training.

## Navigation structure

Top-level navigation remains:

- Use
- Test
- Voice Lab
- Settings

Voice Lab has a sticky secondary tablist:

- Compare
- Profiles
- Dataset
- Models

Only the active subsection is mounted. Switching away from Compare retains the
existing temporary Voice Lab audio cleanup. Switching away from Dataset calls
the existing leave command, stopping unfinished recording/take preview while
preserving finalized takes. Switching away from Models does not issue training
cancellation.

## Layout strategy

All Voice Lab subsections use one primary page scroll.

- Compare uses a two-column source/clip area and DSP/render/preview area.
- Profiles uses a searchable list/create column and selected detail column.
- Dataset uses a sticky profile/progress/filter/take-list sidebar and a prompt,
  recording, review, quality, trim, and transfer main column.
- Models uses a sticky profile/backend/snapshot/artifact navigation column and a
  selected workflow/detail main column.

Primary action regions are sticky above the viewport bottom and include bottom
padding so they never cover content. An action appears once; sticky presentation
does not create duplicate focus targets.

Normal workflow summaries stay visible. Advanced DSP, clip metadata, render
diagnostics, technical quality measurements, trimming values, environment and
package fingerprints, full qualification checks, logs, hashes, licensing, and
import provenance use native `details` disclosure elements and default closed.
Consent requirements and errors never depend on an advanced disclosure.

## Responsive behavior

At wide desktop widths the workspace uses two-column master-detail grids and
sticky sidebars. At medium widths the grids reduce minimum column sizes and
secondary navigation remains horizontally available. At 800 px and below the
workspace becomes one column, sidebars stop sticking, and tabs scroll inside
their own contained row. Tables use contained horizontal scrolling and lists
remain cards. The page itself has no intentional horizontal overflow.

At narrow widths the primary action region is a compact sticky bottom region,
with enough page padding to keep the final content visible. Controls wrap or
become full width. The design targets 1440×900, 1280×720, 1024×768, and 800×600
without requiring full-screen use.

## Accessibility behavior

The secondary navigation uses `tablist`, `tab`, `aria-selected`, and roving
keyboard focus semantics. Active subsection panels have `tabpanel` relationships.
Native disclosures expose `aria-expanded` behavior. Profile list selections are
real buttons with selected state. Errors keep `role="alert"` and status summaries
use text in addition to color.

The profile deletion confirmation is a labelled modal dialog. It traps Tab focus,
closes on Escape, and restores focus to the invoking control. Sticky actions are
the original controls rather than cloned controls. Existing accessible device
labels and waveform labels remain.

## Regression and acceptance boundaries

Automated tests can prove pure rate selection, offline conversion, buffer/cache
behavior, time-based seek conversion, loop boundaries, finite output,
channel mapping, idempotent stop, component ownership, shared profile selection,
subsection exclusivity, disclosure defaults, and responsive class structure.

Automated tests do not prove audible playback, Realtek driver compatibility,
microphone capture, VB-CABLE/receiving-app behavior, keyboard behavior in the
installed WebView, or visual fit at physical window sizes. The reported Realtek
issue remains manual-pending until a 44.1 kHz original and processed clip are
heard through the reported device with diagnostics showing the negotiated output
rate.
