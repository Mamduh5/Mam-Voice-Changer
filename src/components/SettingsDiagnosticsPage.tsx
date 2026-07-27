import type {
  AudioDevice,
  ExternalAudioRoute,
  LastSuccessfulConfiguration,
  ReliabilityProfile,
  RouteCompatibilityResult,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';
import type { ProductInformation } from '../types/product';
import { DiagnosticsPanel } from './DiagnosticsPanel';
import { ProductDiagnosticsPanel } from './ProductDiagnosticsPanel';

const profileDetails: Record<ReliabilityProfile, string> = {
  lowLatency: '128-frame request - 80 ms rings - 256-frame prefill - 3 ms concealment',
  balanced: '256-frame request - 250 ms rings - 1024-frame prefill - 6 ms concealment',
  reliable: '512-frame request - 500 ms rings - 2048-frame prefill - 10 ms concealment',
};

function selectedName(devices: AudioDevice[], id: string, unavailableName: string | null) {
  return (
    devices.find((device) => device.id === id)?.name ??
    (unavailableName ? `${unavailableName} (unavailable)` : 'Not selected')
  );
}

function lastConfigurationLabel(configuration: LastSuccessfulConfiguration | null) {
  if (!configuration) return 'No successful local route has been recorded yet.';
  const outputChannels =
    configuration.outputChannels === null ? 'unknown' : String(configuration.outputChannels);
  return `${configuration.mode.toUpperCase()} - ${configuration.inputDeviceName} to ${
    configuration.outputDeviceName
  } - ${configuration.sampleRate ?? 'unknown'} Hz - ${
    configuration.inputChannels ?? 'unknown'
  } in / ${outputChannels} out - ${new Date(configuration.usedAtUnixMs).toLocaleString()}`;
}

export function SettingsDiagnosticsPage({
  inputs,
  outputs,
  inputId,
  unavailableInputName = null,
  monitorId,
  unavailableMonitorName = null,
  selectedRoute,
  routeValidation,
  reliabilityProfile,
  status,
  parameters = defaultAudioParameters,
  product = null,
  lastSuccessfulConfiguration = null,
  inputActivityDetected = null,
  disabled,
  changesDisabled = disabled,
  onReliabilityProfileChange,
  onRefresh = async () => {},
  onClearClipping = async () => {},
  onResetSafeDefaults = async () => false,
  onOpenSetup = () => {},
}: {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  unavailableInputName?: string | null;
  monitorId: string;
  unavailableMonitorName?: string | null;
  selectedRoute: ExternalAudioRoute | null;
  routeValidation: RouteCompatibilityResult;
  reliabilityProfile: ReliabilityProfile;
  status: EngineStatus;
  parameters?: AudioParameters;
  product?: ProductInformation | null;
  lastSuccessfulConfiguration?: LastSuccessfulConfiguration | null;
  inputActivityDetected?: boolean | null;
  disabled: boolean;
  changesDisabled?: boolean;
  onReliabilityProfileChange: (profile: ReliabilityProfile) => void;
  onRefresh?: () => Promise<void>;
  onClearClipping?: () => Promise<void>;
  onResetSafeDefaults?: () => Promise<boolean>;
  onOpenSetup?: () => void;
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
            <dd>{selectedName(inputs, inputId, unavailableInputName)}</dd>
          </div>
          <div>
            <dt>External route</dt>
            <dd>{selectedRoute?.displayName ?? 'Not selected'}</dd>
          </div>
          <div>
            <dt>Local monitor device</dt>
            <dd>{selectedName(outputs, monitorId, unavailableMonitorName)}</dd>
          </div>
          <div>
            <dt>Last successful configuration</dt>
            <dd>{lastConfigurationLabel(lastSuccessfulConfiguration)}</dd>
          </div>
        </dl>
        <label className="profile-control">
          Reliability profile
          <select
            value={reliabilityProfile}
            disabled={changesDisabled}
            onChange={(event) =>
              onReliabilityProfileChange(event.target.value as ReliabilityProfile)
            }
          >
            <option value="lowLatency">Low latency</option>
            <option value="balanced">Balanced</option>
            <option value="reliable">Reliable</option>
          </select>
          <small>{profileDetails[reliabilityProfile]}</small>
          {engineActive && <small>Stop processing before changing the complete profile.</small>}
        </label>
      </section>

      <ProductDiagnosticsPanel
        inputs={inputs}
        outputs={outputs}
        inputId={inputId}
        unavailableInputName={unavailableInputName}
        monitorId={monitorId}
        unavailableMonitorName={unavailableMonitorName}
        selectedRoute={selectedRoute}
        routeValidation={routeValidation}
        status={status}
        parameters={parameters}
        product={product}
        lastSuccessfulConfiguration={lastSuccessfulConfiguration}
        inputActivityDetected={inputActivityDetected}
        disabled={disabled}
        changesDisabled={changesDisabled}
        onRefresh={onRefresh}
        onClearClipping={onClearClipping}
        onResetSafeDefaults={onResetSafeDefaults}
        onOpenSetup={onOpenSetup}
      />

      <section className="card settings-summary route-diagnostics">
        <h2>External-route health</h2>
        <dl>
          <div>
            <dt>Active virtual playback endpoint</dt>
            <dd>
              {playbackActive
                ? `Playback active - ${
                    selectedRoute?.playbackDevice.name ?? 'route metadata unavailable'
                  }`
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
          Discord, OBS, a game, or a browser is consuming it.
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

      <section className="card about-product">
        <h2>About</h2>
        <dl>
          <div>
            <dt>Product</dt>
            <dd>{product?.productName ?? 'Mam Voice Changer'}</dd>
          </div>
          <div>
            <dt>Application version</dt>
            <dd>{product?.applicationVersion ?? 'Unavailable'}</dd>
          </div>
          <div>
            <dt>Audio backend</dt>
            <dd>{product?.backendVersion ?? 'Unavailable'}</dd>
          </div>
          <div>
            <dt>Platform</dt>
            <dd>
              {product ? `${product.operatingSystem} / ${product.architecture}` : 'Unavailable'}
            </dd>
          </div>
        </dl>
        <p>
          Prototype, local-only desktop processing. No audio, telemetry, or diagnostic report is
          uploaded by the application.
        </p>
        <h3>Known limitations</h3>
        <ul>
          <li>Voice quality depends on the source voice, microphone, room, and chosen settings.</li>
          <li>
            Male, female, or age character is not guaranteed; extreme settings can sound artificial.
          </li>
          <li>Noise and speaker monitoring can cause artifacts or feedback; prefer headphones.</li>
          <li>
            Windows application routing is separate from DSP processing and requires a real capture
            endpoint supplied by a compatible virtual audio device.
          </li>
          <li>WORLD is evaluator-only and is never part of the live audio path.</li>
          <li>One-person listening evidence is personal evidence, not general user validation.</li>
        </ul>
      </section>
    </div>
  );
}
