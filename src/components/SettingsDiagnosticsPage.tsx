import type {
  AudioDevice,
  ExternalAudioRoute,
  ReliabilityProfile,
  RouteCompatibilityResult,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import { DiagnosticsPanel } from './DiagnosticsPanel';

const profileDetails: Record<ReliabilityProfile, string> = {
  lowLatency: '128-frame request - 80 ms rings - 256-frame prefill - 3 ms concealment',
  balanced: '256-frame request - 250 ms rings - 1024-frame prefill - 6 ms concealment',
  reliable: '512-frame request - 500 ms rings - 2048-frame prefill - 10 ms concealment',
};

function selectedName(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id)?.name ?? 'Not selected';
}

export function SettingsDiagnosticsPage({
  inputs,
  outputs,
  inputId,
  monitorId,
  selectedRoute,
  routeValidation,
  reliabilityProfile,
  status,
  disabled,
  onReliabilityProfileChange,
}: {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  monitorId: string;
  selectedRoute: ExternalAudioRoute | null;
  routeValidation: RouteCompatibilityResult;
  reliabilityProfile: ReliabilityProfile;
  status: EngineStatus;
  disabled: boolean;
  onReliabilityProfileChange: (profile: ReliabilityProfile) => void;
}) {
  const engineActive = !['stopped', 'error'].includes(status.state);
  const playbackActive = engineActive && status.routePurpose === 'use';
  return (
    <div className="page-stack" data-page="diagnostics">
      <section className="card settings-summary">
        <h2>Settings & Diagnostics</h2>
        <dl>
          <div>
            <dt>Input microphone</dt>
            <dd>{selectedName(inputs, inputId)}</dd>
          </div>
          <div>
            <dt>External route</dt>
            <dd>{selectedRoute?.displayName ?? 'Not selected'}</dd>
          </div>
          <div>
            <dt>Local monitor device</dt>
            <dd>{selectedName(outputs, monitorId)}</dd>
          </div>
        </dl>
        <label className="profile-control">
          Reliability profile
          <select
            value={reliabilityProfile}
            disabled={disabled || engineActive}
            onChange={(event) =>
              onReliabilityProfileChange(event.target.value as ReliabilityProfile)
            }
          >
            <option value="lowLatency">Low latency</option>
            <option value="balanced">Balanced</option>
            <option value="reliable">Reliable</option>
          </select>
          <small>{profileDetails[reliabilityProfile]}</small>
          {engineActive && <small>Stop the engine before changing the complete profile.</small>}
        </label>
      </section>
      <section className="card settings-summary route-diagnostics">
        <h2>External-route health</h2>
        <dl>
          <div>
            <dt>Active virtual playback endpoint</dt>
            <dd>
              {playbackActive
                ? `Playback active - ${selectedRoute?.playbackDevice.name ?? 'route metadata unavailable'}`
                : 'Not active'}
            </dd>
          </div>
          <div>
            <dt>Expected paired capture endpoint</dt>
            <dd>{selectedRoute?.captureDevice?.name ?? 'Not paired'}</dd>
          </div>
          <div>
            <dt>Pairing confidence / source</dt>
            <dd>
              {selectedRoute
                ? `${selectedRoute.pairingConfidence} / ${selectedRoute.pairingSource}`
                : 'Unavailable'}
            </dd>
          </div>
          <div>
            <dt>Route readiness</dt>
            <dd>{routeValidation.readiness}</dd>
          </div>
          <div>
            <dt>Capture endpoint available</dt>
            <dd>{routeValidation.captureEndpointAvailable ? 'Yes' : 'No'}</dd>
          </div>
          <div>
            <dt>Negotiated input/playback rate</dt>
            <dd>
              {routeValidation.negotiatedSampleRate
                ? `${routeValidation.negotiatedSampleRate} Hz`
                : 'Not negotiated'}
            </dd>
          </div>
          <div>
            <dt>Last playback error</dt>
            <dd>{status.lastRuntimeError ?? 'None recorded'}</dd>
          </div>
        </dl>
        <p>{routeValidation.message}</p>
        <small>
          Capture availability means Windows still enumerates the endpoint. It does not prove that
          Discord, OBS, or a browser is consuming it.
        </small>
      </section>
      <DiagnosticsPanel status={status} />
      <section className="card clock-drift-note">
        <h2>Device-clock observation</h2>
        <p>
          Ring-fill trends are recorded. Adaptive resampling remains disabled at ratio 1.0 until a
          long session demonstrates persistent input/output clock drift.
        </p>
      </section>
    </div>
  );
}
