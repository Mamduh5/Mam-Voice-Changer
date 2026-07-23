import type { AudioDevice } from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';
import { DeviceSelector } from './DeviceSelector';
import { DspControls } from './DspControls';
import { EngineControls } from './EngineControls';
import { LevelMeter } from './LevelMeter';
import { PresetControls } from './PresetControls';

type PresetActions = {
  apply: (id: string) => Promise<boolean>;
  save: (name: string, parameters: AudioParameters) => Promise<boolean>;
  rename: (id: string, name: string) => Promise<boolean>;
  duplicate: (id: string) => Promise<boolean>;
  remove: (id: string) => Promise<boolean>;
  reset: () => Promise<boolean>;
};

type Props = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  monitorId: string;
  disabled: boolean;
  status: EngineStatus;
  parameters: AudioParameters;
  catalog: PresetCatalog | null;
  presetBusy: boolean;
  presetActions: PresetActions;
  onInputChange: (id: string) => void;
  onMonitorDeviceChange: (id: string) => void;
  onParametersChange: (changes: Partial<AudioParameters>) => void;
  onStart: () => void;
  onStop: () => void;
};

export function TestPage(props: Props) {
  const routeLocked = !['stopped', 'error'].includes(props.status.state);
  const monitoringActive =
    props.status.routePurpose === 'test' &&
    ['running', 'degraded', 'recovering'].includes(props.status.state);
  const useActive =
    props.status.routePurpose === 'use' && !['stopped', 'error'].includes(props.status.state);
  return (
    <div className="page-stack" data-page="test">
      <section className="card test-warning" role="alert">
        <strong>Feedback warning</strong>
        <p>
          Use headphones to prevent feedback. Monitoring starts only when you press Start hearing
          test.
        </p>
      </section>
      <section className="card routing">
        <div className="section-heading">
          <h2>Test monitoring</h2>
          <span className={monitoringActive ? 'monitoring-on' : 'monitoring-off'}>
            {monitoringActive ? 'Monitoring active' : 'Monitoring off'}
          </span>
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
            label="Test monitor device"
            value={props.monitorId}
            devices={props.outputs}
            disabled={props.disabled || routeLocked}
            showOutputClassification
            onChange={props.onMonitorDeviceChange}
          />
        </div>
      </section>
      <PresetControls
        catalog={props.catalog}
        parameters={props.parameters}
        disabled={props.disabled}
        busy={props.presetBusy}
        onApply={props.presetActions.apply}
        onSave={props.presetActions.save}
        onRename={props.presetActions.rename}
        onDuplicate={props.presetActions.duplicate}
        onDelete={props.presetActions.remove}
        onReset={props.presetActions.reset}
      />
      <div className="grid">
        <section className="card">
          <h2>Test levels</h2>
          <LevelMeter label="Input" value={props.status.inputLevel} />
          <LevelMeter label="Monitor" value={props.status.monitorLevel} />
        </section>
        <DspControls
          parameters={props.parameters}
          disabled={props.disabled || props.presetBusy}
          onChange={props.onParametersChange}
        />
      </div>
      <EngineControls
        status={props.status}
        purpose="test"
        canStart={Boolean(props.inputId && props.monitorId && !useActive)}
        startLabel="Start hearing test"
        stopLabel="Stop test"
        description="Selected local monitor only; stops automatically when you leave Test"
        onStart={props.onStart}
        onStop={props.onStop}
      />
    </div>
  );
}
