import type {
  AudioDevice,
  ExternalAudioRoute,
  ExternalAudioRouteCatalog,
  RouteCompatibilityResult,
  RouteReadiness,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { PresetCatalog } from '../types/presets';
import { CompactPresetSelector } from './CompactPresetSelector';
import { DeviceSelector } from './DeviceSelector';
import { EngineControls } from './EngineControls';
import { LevelMeter } from './LevelMeter';
import { RoutingNotice } from './RoutingNotice';

const readinessLabels: Record<RouteReadiness, string> = {
  ready: 'Ready',
  missingInput: 'Missing physical input',
  missingPlayback: 'Missing virtual playback endpoint',
  missingCapture: 'Missing paired capture endpoint',
  ambiguousPair: 'Pairing is ambiguous',
  incompatibleFormat: 'Incompatible sample rates',
  deviceUnavailable: 'Endpoint unavailable',
  engineActive: 'Another route is active',
};

function reliabilityLabel(status: EngineStatus) {
  const problems =
    status.inputCallbackGaps +
    status.inputRingOverflows +
    status.dspInputUnderruns +
    status.dspProcessingDeadlineMisses +
    status.outputRingUnderruns;
  return problems === 0
    ? 'Stable so far'
    : `${problems} reliability event${problems === 1 ? '' : 's'}`;
}

function endpointLabel(device: AudioDevice) {
  return `${device.name}${device.isLikelyVirtual ? ' - likely virtual' : ' - likely physical'}`;
}

type Props = {
  physicalInputs: AudioDevice[];
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  routes: ExternalAudioRouteCatalog;
  selectedRoute: ExternalAudioRoute | null;
  validation: RouteCompatibilityResult;
  draftRouteId: string;
  draftPlaybackId: string;
  draftCaptureId: string;
  confirmPhysicalEndpoints: boolean;
  routeBusy: boolean;
  disabled: boolean;
  status: EngineStatus;
  catalog: PresetCatalog | null;
  presetBusy: boolean;
  onInputChange: (id: string) => void;
  onDraftRouteChange: (id: string) => void;
  onDraftPlaybackChange: (id: string) => void;
  onDraftCaptureChange: (id: string) => void;
  onConfirmPhysicalEndpointsChange: (confirmed: boolean) => void;
  onSaveRoute: () => Promise<boolean>;
  onDeleteRoute: () => Promise<boolean>;
  onValidateRoute: () => Promise<void> | void;
  onApplyPreset: (id: string) => Promise<boolean>;
  onStart: () => void;
  onStop: () => void;
};

export function UsePage(props: Props) {
  const routeLocked = !['stopped', 'error'].includes(props.status.state);
  const testActive =
    props.status.routePurpose === 'test' && !['stopped', 'error'].includes(props.status.state);
  const draftPlayback = props.outputs.find((device) => device.id === props.draftPlaybackId);
  const draftCapture = props.inputs.find((device) => device.id === props.draftCaptureId);
  const needsPhysicalConfirmation = Boolean(
    (draftPlayback && !draftPlayback.isLikelyVirtual) ||
    (draftCapture && !draftCapture.isLikelyVirtual),
  );
  const canStart = Boolean(
    props.selectedRoute && props.validation.readiness === 'ready' && !testActive,
  );

  return (
    <div className="page-stack" data-page="use">
      <section className="card routing external-routing">
        <div className="section-heading">
          <h2>Use with external applications</h2>
          <span className="reliability-pill">{reliabilityLabel(props.status)}</span>
        </div>
        <DeviceSelector
          label="Physical input microphone"
          value={props.inputId}
          devices={props.physicalInputs}
          disabled={props.disabled || routeLocked}
          onChange={props.onInputChange}
        />

        <div className="route-setup">
          <h3>External route setup</h3>
          <ol>
            <li>Detect virtual playback and capture endpoints.</li>
            <li>Select or confirm a playback/capture pair.</li>
            <li>Validate compatibility, then start Mam Voice Changer.</li>
            <li>Select the paired capture endpoint in the receiving application.</li>
            <li>Confirm activity using that application's meter or microphone test.</li>
          </ol>
          <label>
            Candidate external route
            <select
              value={props.draftRouteId}
              disabled={props.disabled || routeLocked || props.routeBusy}
              onChange={(event) => props.onDraftRouteChange(event.target.value)}
            >
              <option value="">Manual playback/capture pair</option>
              {props.routes.routes.map((route, index) => (
                <option value={route.routeId} key={`${route.routeId}-${index}`}>
                  {route.displayName} ({route.pairingConfidence})
                </option>
              ))}
            </select>
          </label>
          <div className="route-grid">
            <label>
              Virtual playback endpoint
              <select
                value={props.draftPlaybackId}
                disabled={props.disabled || routeLocked || props.routeBusy}
                onChange={(event) => props.onDraftPlaybackChange(event.target.value)}
              >
                <option value="">Select playback endpoint</option>
                {props.outputs.map((device) => (
                  <option value={device.id} key={`playback-${device.id}-${device.name}`}>
                    {endpointLabel(device)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Paired capture endpoint
              <select
                value={props.draftCaptureId}
                disabled={props.disabled || routeLocked || props.routeBusy}
                onChange={(event) => props.onDraftCaptureChange(event.target.value)}
              >
                <option value="">Select capture endpoint</option>
                {props.inputs.map((device) => (
                  <option value={device.id} key={`capture-${device.id}-${device.name}`}>
                    {endpointLabel(device)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {needsPhysicalConfirmation && (
            <label className="physical-confirmation warning">
              <input
                type="checkbox"
                checked={props.confirmPhysicalEndpoints}
                onChange={(event) => props.onConfirmPhysicalEndpointsChange(event.target.checked)}
              />
              I understand this manual pair contains a likely physical endpoint and may not route
              audio to another application.
            </label>
          )}
          <div className="route-actions">
            <button
              type="button"
              disabled={
                props.disabled ||
                routeLocked ||
                props.routeBusy ||
                !props.draftPlaybackId ||
                !props.draftCaptureId ||
                (needsPhysicalConfirmation && !props.confirmPhysicalEndpoints)
              }
              onClick={() => void props.onSaveRoute()}
            >
              {props.routeBusy ? 'Saving...' : 'Save external route'}
            </button>
            <button
              type="button"
              disabled={props.disabled || routeLocked || props.routeBusy || !props.selectedRoute}
              onClick={() => void props.onDeleteRoute()}
            >
              Clear saved route
            </button>
            <button
              type="button"
              disabled={props.disabled || routeLocked || !props.selectedRoute}
              onClick={() => void props.onValidateRoute()}
            >
              Validate route
            </button>
          </div>
        </div>

        <div
          className={props.validation.readiness === 'ready' ? 'route-health ready' : 'route-health'}
          role="status"
        >
          <strong>Route readiness: {readinessLabels[props.validation.readiness]}</strong>
          <p>{props.validation.message}</p>
          {props.validation.negotiatedSampleRate && (
            <small>
              Negotiated input/playback rate: {props.validation.negotiatedSampleRate} Hz
            </small>
          )}
          <small>
            Capture endpoint available: {props.validation.captureEndpointAvailable ? 'Yes' : 'No'}
          </small>
        </div>
        {props.selectedRoute && (
          <dl className="route-summary">
            <div>
              <dt>Selected external route</dt>
              <dd>{props.selectedRoute.displayName}</dd>
            </div>
            <div>
              <dt>Pairing</dt>
              <dd>
                {props.selectedRoute.pairingConfidence} / {props.selectedRoute.pairingSource}
              </dd>
            </div>
            <div>
              <dt>Virtual formats</dt>
              <dd>{props.selectedRoute.compatibility.details}</dd>
            </div>
          </dl>
        )}
        <RoutingNotice route={props.selectedRoute} />
      </section>

      <div className="grid">
        <CompactPresetSelector
          catalog={props.catalog}
          disabled={props.disabled || props.presetBusy}
          onApply={props.onApplyPreset}
        />
        <section className="card">
          <h2>Shared voice configuration</h2>
          <p>
            Use and Test share the same preset, Old Lady settings, gate/expander, gains, limiter,
            bypass, and mute snapshot.
          </p>
          <h3>Levels</h3>
          <LevelMeter label="Input" value={props.status.inputLevel} />
          <LevelMeter label="Processed output" value={props.status.outputLevel} />
        </section>
      </div>

      <EngineControls
        status={props.status}
        purpose="use"
        canStart={canStart}
        startLabel="Start using"
        stopLabel="Stop using"
        description="Physical microphone to virtual playback only; no local monitor stream"
        onStart={props.onStart}
        onStop={props.onStop}
      />
    </div>
  );
}
