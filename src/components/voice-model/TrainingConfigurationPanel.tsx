import type {
  TrainingConfiguration,
  TrainingPreflightReport,
  TrainingPreset,
} from '../../types/trainingJob';
import { trainingPresets } from '../../types/trainingJob';

export function TrainingConfigurationPanel({
  configuration,
  disabled,
  onChange,
  onStart,
  preflight,
  warningsAcknowledged,
  onWarningsAcknowledged,
  onPreflight,
}: {
  configuration: TrainingConfiguration;
  disabled: boolean;
  onChange: (configuration: TrainingConfiguration) => void;
  onStart: () => Promise<unknown>;
  preflight: TrainingPreflightReport | null;
  warningsAcknowledged: boolean;
  onWarningsAcknowledged: (acknowledged: boolean) => void;
  onPreflight: () => Promise<unknown>;
}) {
  const update = (changes: Partial<TrainingConfiguration>) =>
    onChange({ ...configuration, ...changes });
  return (
    <section className="card model-training-config">
      <div className="section-heading">
        <h2>4. Local fine-tuning configuration and preflight</h2>
        <span>Typed controls only</span>
      </div>
      <div className="model-control-grid">
        <label>
          Conservative preset
          <select
            value={configuration.preset}
            onChange={(event) => onChange(trainingPresets[event.target.value as TrainingPreset])}
          >
            <option value="quickExperiment">Quick experiment</option>
            <option value="balancedFineTune">Balanced fine-tune</option>
            <option value="extendedFineTune">Extended fine-tune</option>
          </select>
        </label>
        <label>
          Maximum steps
          <input
            type="number"
            min="10"
            max="100000"
            value={configuration.maximumSteps}
            onChange={(event) => update({ maximumSteps: Number(event.target.value) })}
          />
        </label>
        <label>
          Save interval
          <input
            type="number"
            min="1"
            value={configuration.saveInterval}
            onChange={(event) => update({ saveInterval: Number(event.target.value) })}
          />
        </label>
        <label>
          Batch size
          <input
            type="number"
            min="1"
            max="64"
            value={configuration.batchSize}
            onChange={(event) => update({ batchSize: Number(event.target.value) })}
          />
        </label>
        <label>
          Worker count
          <input
            type="number"
            min="0"
            max="16"
            value={configuration.workerCount}
            onChange={(event) => update({ workerCount: Number(event.target.value) })}
          />
        </label>
        <label>
          Random seed
          <input
            type="number"
            min="0"
            value={configuration.randomSeed}
            onChange={(event) => update({ randomSeed: Number(event.target.value) })}
          />
        </label>
      </div>
      <p>More steps do not guarantee better voice quality or target similarity.</p>
      <div className="voice-lab-actions">
        <button type="button" disabled={disabled} onClick={() => void onPreflight()}>
          Review training preflight
        </button>
        <button
          type="button"
          className="start"
          disabled={
            disabled ||
            !preflight?.canStart ||
            (preflight.acknowledgementsRequired.length > 0 && !warningsAcknowledged)
          }
          onClick={() => void onStart()}
        >
          Start local fine-tuning
        </button>
      </div>
      {preflight && (
        <div className="training-preflight" role="region" aria-label="Training preflight report">
          <h3>Training preflight report</h3>
          <div className="model-metrics">
            <span>{preflight.snapshotTakeCount} snapshot takes</span>
            <span>{Math.round(preflight.trainingDurationMs / 1000)}s training audio</span>
            <span>{Math.round(preflight.validationDurationMs / 1000)}s validation audio</span>
            <span>{preflight.estimatedCheckpointCount} estimated checkpoints</span>
            <span>
              Disk estimate {formatBytes(preflight.estimatedDiskMinimumBytes)}â€“
              {formatBytes(preflight.estimatedDiskMaximumBytes)}
            </span>
            <span>Qualification depth: {preflight.qualificationLevel}</span>
            <span>Environment: {preflight.environmentFingerprintStatus}</span>
          </div>
          {preflight.fatalFailures.map((failure) => (
            <div className="error" key={failure}>
              {failure}
            </div>
          ))}
          {preflight.acknowledgementsRequired.length > 0 && (
            <label className="dataset-consent-check">
              <input
                type="checkbox"
                checked={warningsAcknowledged}
                onChange={(event) => onWarningsAcknowledged(event.target.checked)}
              />
              I reviewed and acknowledge: {preflight.acknowledgementsRequired.join(' ')}
            </label>
          )}
          <p>
            Resource estimates are diagnostic ranges and do not guarantee that training will fit.
          </p>
        </div>
      )}
    </section>
  );
}

function formatBytes(value: number) {
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}
