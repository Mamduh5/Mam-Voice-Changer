# Use and Test route-purpose implementation note

## Current coupling

The previous frontend stored `engineMode` and temporary-monitor state, while one generic backend start request accepted nullable destination and monitor fields. Use could opt into a local monitor, Test required both a checkbox and Start, persisted settings contained `localMonitorEnabled`, and navigation inferred what to stop from frontend-only state.

## Route-purpose model

`StartAudioRequest` is now a tagged `use`/`test` union in TypeScript and Rust. Use contains one input plus one required processed destination; Test contains one input plus one required monitor. The backend status publishes the active route purpose, and route-specific controls use **Start using** / **Stop using** and **Start hearing test** / **Stop test**.

## Shared parameter model

Both start variants reuse the existing controller-owned `ParameterState`. Presets, Old Lady controls, gate/expander, gains, limiter, bypass, and mute therefore remain one authoritative backend snapshot. The existing parameter synchronizer and preset-operation reconciliation remain shared at `App` scope, so page navigation does not fork parameter state.

## Navigation rules

Use is not stopped by navigation. Whenever the user leaves Test, the frontend requests `stop_test_route`; the backend stops only an active or recovering Test request and treats an active Use request as a no-op. This also covers a Test start that is still in flight without relying on frontend route-intent state.

## Settings migration

Application-settings schema v3 removes `localMonitorEnabled` while retaining the selected monitor device for the next explicit Test. Schema v2 is migrated by discarding the enable flag and preserving device selections, reliability profile, and last page. Schema v1 continues to migrate its input and destination. No route auto-start state is persisted.

## Backend validation

Serde rejects unknown or cross-purpose fields. Use requests cannot contain monitor fields and require nonblank input/destination identifiers and names. Test requests cannot contain processed-destination fields and require nonblank input/monitor identifiers and names. Stream construction matches the tagged variant and opens exactly one output role.
