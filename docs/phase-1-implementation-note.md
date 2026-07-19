# Phase 1 implementation note

Phase 1 is limited to cleanup and correctness. The vocal-aging DSP work remains
gated until the validation commands at the end of this note pass.

## Planned change set

- `README.md`, `docs/prototype-scope.md`, `docs/manual-test-plan.md`,
  `docs/architecture.md`, `docs/audio-routing.md`, and
  `docs/Mam-Voice-Changer-Tech-Stack-and-Structure.md`: reconcile current preset,
  routing, validation, and roadmap claims with the implementation.
- `src/hooks/parameterSynchronizer.ts` and
  `src/hooks/parameterSynchronizer.test.ts`: add and test the coalesced,
  backend-confirmed parameter synchronization state machine.
- `src/hooks/useAudioParameters.ts`: connect React state to the synchronizer and
  stop in-flight work from updating an unmounted component.
- `src/hooks/useAudioDevices.ts`, `src/services/tauriAudioApi.ts`,
  `src/types/audio.ts`, `src/utils/deviceSelection.ts`, and
  `src/utils/deviceSelection.test.ts`: restore persisted selections, save user
  changes, expose restoration/fallback details, and cover conservative fallback.
- `src/hooks/useEngineState.ts` and `src/App.tsx`: keep independent recoverable
  errors visible, clear polling errors after recovery, and prevent preset
  operations from racing interactive parameter changes.
- `src-tauri/src/config/application_settings.rs` and
  `src-tauri/src/config/mod.rs`: add a validated versioned settings document,
  conservative selection resolution, and atomic persistence with focused tests.
- `src-tauri/src/state/app_state.rs`, `src-tauri/src/commands/devices.rs`, and
  `src-tauri/src/lib.rs`: load settings during application setup and expose
  resolved device selection plus settings updates through Tauri commands, outside
  every audio callback.

No dependency, audio callback, DSP processor, preset-schema, or audio-engine
architecture changes are part of Phase 1.

## Validation record

Baseline and completion results will be recorded here with the exact commands.
Automated checks do not establish audible quality or hardware routing behavior.

