import { useState } from 'react';
import type {
  AudioDevice,
  ExternalAudioRouteCatalog,
  RouteCompatibilityResult,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import { DeviceSelector } from './DeviceSelector';

function endpointLabel(device: AudioDevice) {
  return `${device.name} (${device.direction}; classification: ${
    device.isLikelyVirtual ? 'likely virtual' : 'likely physical'
  })`;
}

export function QuickSetup({
  inputs,
  outputs,
  inputId,
  monitorId,
  unavailableInputName,
  unavailableMonitorName,
  routes,
  draftRouteId,
  draftPlaybackId,
  draftCaptureId,
  routeValidation,
  status,
  disabled,
  onInputChange,
  onMonitorChange,
  onDraftRouteChange,
  onDraftPlaybackChange,
  onDraftCaptureChange,
  onSaveRoute,
  onApplySafeDefaults,
  onStart,
  onClose,
}: {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  monitorId: string;
  unavailableInputName: string | null;
  unavailableMonitorName: string | null;
  routes: ExternalAudioRouteCatalog;
  draftRouteId: string;
  draftPlaybackId: string;
  draftCaptureId: string;
  routeValidation: RouteCompatibilityResult;
  status: EngineStatus;
  disabled: boolean;
  onInputChange: (id: string) => void;
  onMonitorChange: (id: string) => void;
  onDraftRouteChange: (id: string) => void;
  onDraftPlaybackChange: (id: string) => void;
  onDraftCaptureChange: (id: string) => void;
  onSaveRoute: () => Promise<boolean>;
  onApplySafeDefaults: () => Promise<boolean>;
  onStart: () => void;
  onClose: (doNotShowAutomatically: boolean) => void;
}) {
  const [doNotShowAgain, setDoNotShowAgain] = useState(false);
  const [busy, setBusy] = useState<'route' | 'defaults' | null>(null);
  const routeReady = routeValidation.readiness === 'ready';
  const canConfigure = !disabled && ['stopped', 'error'].includes(status.state);
  const canStart = canConfigure && Boolean(inputId) && routeReady;

  const saveRoute = async () => {
    setBusy('route');
    try {
      await onSaveRoute();
    } finally {
      setBusy(null);
    }
  };

  const applyDefaults = async () => {
    setBusy('defaults');
    try {
      await onApplySafeDefaults();
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="setup-backdrop">
      <section
        className="quick-setup card"
        role="dialog"
        aria-modal="true"
        aria-labelledby="quick-setup-title"
      >
        <div className="section-heading">
          <div>
            <h2 id="quick-setup-title">Quick setup</h2>
            <p>Configure a safe local route. Processing never starts automatically.</p>
          </div>
          <button type="button" className="secondary" onClick={() => onClose(doNotShowAgain)}>
            Close
          </button>
        </div>

        <ol className="setup-steps">
          <li>
            <strong>Select the physical microphone.</strong>
            <DeviceSelector
              label="Microphone input"
              value={inputId}
              devices={inputs}
              unavailableName={unavailableInputName}
              disabled={!canConfigure}
              allowEmpty
              onChange={onInputChange}
            />
          </li>
          <li>
            <strong>Select where processed audio should be sent.</strong>
            <p>
              Discord, games, and voice applications normally require a separately installed virtual
              playback/capture cable pair.
            </p>
            <small>
              Endpoint classification is advisory; a name-based match is not proof that a device is
              a virtual cable.
            </small>
            <label>
              Detected candidate pair
              <select
                value={draftRouteId}
                disabled={!canConfigure || busy !== null}
                onChange={(event) => onDraftRouteChange(event.target.value)}
              >
                <option value="">Manual pair</option>
                {routes.routes.map((route) => (
                  <option value={route.routeId} key={route.routeId}>
                    {route.displayName} ({route.pairingConfidence})
                  </option>
                ))}
              </select>
            </label>
            <div className="route-grid">
              <label>
                Processed-output playback endpoint
                <select
                  value={draftPlaybackId}
                  disabled={!canConfigure || busy !== null}
                  onChange={(event) => onDraftPlaybackChange(event.target.value)}
                >
                  <option value="">Select playback endpoint</option>
                  {routes.virtualPlaybackDevices.map((device) => (
                    <option value={device.id} key={`setup-output-${device.id}`}>
                      {endpointLabel(device)}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Target-application capture endpoint
                <select
                  value={draftCaptureId}
                  disabled={!canConfigure || busy !== null}
                  onChange={(event) => onDraftCaptureChange(event.target.value)}
                >
                  <option value="">Select paired capture endpoint</option>
                  {routes.virtualCaptureDevices.map((device) => (
                    <option value={device.id} key={`setup-capture-${device.id}`}>
                      {endpointLabel(device)}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <button
              type="button"
              disabled={!canConfigure || busy !== null || !draftPlaybackId || !draftCaptureId}
              onClick={() => void saveRoute()}
            >
              {busy === 'route' ? 'Saving route...' : 'Save and check route'}
            </button>
            <p className={routeReady ? 'setup-result ready' : 'setup-result'}>
              {routeValidation.message}
            </p>
          </li>
          <li>
            <strong>Select an optional local monitor.</strong>
            <p>Use headphones. Test monitoring stays off until you explicitly start Test.</p>
            <DeviceSelector
              label="Optional Test monitor"
              value={monitorId}
              devices={outputs}
              unavailableName={unavailableMonitorName}
              disabled={!canConfigure}
              allowEmpty
              emptyLabel="No local monitor"
              showOutputClassification
              onChange={onMonitorChange}
            />
          </li>
          <li>
            <strong>Choose safe starting values.</strong>
            <button
              type="button"
              disabled={!canConfigure || busy !== null}
              onClick={() => void applyDefaults()}
            >
              {busy === 'defaults' ? 'Applying...' : 'Apply safe defaults'}
            </button>
          </li>
          <li>
            <strong>Start processing, then configure the target application.</strong>
            <p>
              Select the saved pair&apos;s capture/input side as the microphone in the target
              application.
            </p>
            <button
              type="button"
              className="start"
              disabled={!canStart}
              onClick={() => {
                onClose(doNotShowAgain);
                onStart();
              }}
            >
              Start processing
            </button>
          </li>
        </ol>

        <label className="setup-dismiss">
          <input
            type="checkbox"
            checked={doNotShowAgain}
            onChange={(event) => setDoNotShowAgain(event.target.checked)}
          />
          Do not show this automatically again
        </label>
      </section>
    </div>
  );
}
