import type { AudioDevice, ReliabilityProfile } from '../types/audio';
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
  destinationId,
  monitorId,
  reliabilityProfile,
  status,
  disabled,
  onReliabilityProfileChange,
}: {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  destinationId: string;
  monitorId: string;
  reliabilityProfile: ReliabilityProfile;
  status: EngineStatus;
  disabled: boolean;
  onReliabilityProfileChange: (profile: ReliabilityProfile) => void;
}) {
  const engineActive = !['stopped', 'error'].includes(status.state);
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
            <dt>Processed destination</dt>
            <dd>{selectedName(outputs, destinationId)}</dd>
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
