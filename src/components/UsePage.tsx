import type { AudioDevice } from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { PresetCatalog } from '../types/presets';
import { CompactPresetSelector } from './CompactPresetSelector';
import { DeviceSelector } from './DeviceSelector';
import { EngineControls } from './EngineControls';
import { LevelMeter } from './LevelMeter';
import { RoutingNotice } from './RoutingNotice';

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

type Props = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  destinationId: string;
  hasLikelyVirtualDestination: boolean;
  disabled: boolean;
  status: EngineStatus;
  catalog: PresetCatalog | null;
  presetBusy: boolean;
  onInputChange: (id: string) => void;
  onDestinationChange: (id: string) => void;
  onApplyPreset: (id: string) => Promise<boolean>;
  onStart: () => void;
  onStop: () => void;
};

export function UsePage(props: Props) {
  const routeLocked = !['stopped', 'error'].includes(props.status.state);
  return (
    <div className="page-stack" data-page="use">
      <section className="card routing">
        <div className="section-heading">
          <h2>Use</h2>
          <span className="reliability-pill">{reliabilityLabel(props.status)}</span>
        </div>
        <div className="route-grid">
          <DeviceSelector
            label="Input microphone"
            value={props.inputId}
            devices={props.inputs}
            disabled={props.disabled || routeLocked}
            onChange={props.onInputChange}
          />
          <DeviceSelector
            label="Processed destination"
            value={props.destinationId}
            devices={props.outputs}
            disabled={props.disabled || routeLocked}
            allowEmpty
            emptyLabel="No processed destination selected"
            showOutputClassification
            onChange={props.onDestinationChange}
          />
        </div>
        {!props.destinationId && (
          <p className="route-requirement" role="status">
            Use requires a processed destination. Install or select a real virtual-audio playback
            endpoint before starting.
          </p>
        )}
        <RoutingNotice hasLikelyVirtualDestination={props.hasLikelyVirtualDestination} />
      </section>

      <div className="grid">
        <CompactPresetSelector
          catalog={props.catalog}
          disabled={props.disabled || props.presetBusy}
          onApply={props.onApplyPreset}
        />
        <section className="card">
          <h2>Levels</h2>
          <LevelMeter label="Input" value={props.status.inputLevel} />
          <LevelMeter label="Processed output" value={props.status.outputLevel} />
        </section>
      </div>

      <EngineControls
        status={props.status}
        purpose="use"
        canStart={Boolean(props.inputId && props.destinationId)}
        startLabel="Start using"
        stopLabel="Stop using"
        description="Processed destination only; no local speaker or headphone monitor"
        onStart={props.onStart}
        onStop={props.onStop}
      />
    </div>
  );
}
