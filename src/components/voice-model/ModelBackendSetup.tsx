import { open } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';
import type { ModelBackendSettings, SeedVcBackendConfiguration } from '../../types/modelBackend';
import { backendReadinessLabel } from '../../utils/modelReadiness';

const blank: SeedVcBackendConfiguration = {
  pythonExecutable: '',
  workerPackageDirectory: '',
  seedVcDirectory: '',
  modelConfigurationPath: '',
  pretrainedCheckpointPaths: [],
  outputDirectory: '',
  device: 'cpu',
  precision: 'float32',
};

export function ModelBackendSetup({
  settings,
  readiness,
  message,
  busy,
  onSave,
  onValidate,
}: {
  settings: ModelBackendSettings;
  readiness: Parameters<typeof backendReadinessLabel>[0];
  message: string;
  busy: boolean;
  onSave: (settings: ModelBackendSettings) => Promise<boolean>;
  onValidate: () => Promise<unknown>;
}) {
  const [configuration, setConfiguration] = useState(settings.seedVc ?? blank);
  const update = (changes: Partial<SeedVcBackendConfiguration>) =>
    setConfiguration((current) => ({ ...current, ...changes }));
  const chooseFile = async (field: 'pythonExecutable' | 'modelConfigurationPath') => {
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === 'string') update({ [field]: selected });
  };
  const chooseDirectory = async (
    field: 'workerPackageDirectory' | 'seedVcDirectory' | 'outputDirectory',
  ) => {
    const selected = await open({ multiple: false, directory: true });
    if (typeof selected === 'string') update({ [field]: selected });
  };
  const chooseCheckpoints = async () => {
    const selected = await open({ multiple: true, directory: false });
    const paths = typeof selected === 'string' ? [selected] : selected;
    if (paths?.length) update({ pretrainedCheckpointPaths: paths });
  };

  return (
    <section className="card model-backend-setup">
      <div className="section-heading">
        <h2>2. Configure local model backend</h2>
        <span data-readiness={readiness}>{backendReadinessLabel(readiness)}</span>
      </div>
      <p>{message}</p>
      <div className="model-path-grid">
        <PathRow
          label="Python executable"
          value={configuration.pythonExecutable}
          onChoose={() => void chooseFile('pythonExecutable')}
        />
        <PathRow
          label="Worker package directory"
          value={configuration.workerPackageDirectory}
          onChoose={() => void chooseDirectory('workerPackageDirectory')}
        />
        <PathRow
          label="Seed-VC checkout directory"
          value={configuration.seedVcDirectory}
          onChoose={() => void chooseDirectory('seedVcDirectory')}
        />
        <PathRow
          label="Model configuration"
          value={configuration.modelConfigurationPath}
          onChoose={() => void chooseFile('modelConfigurationPath')}
        />
        <PathRow
          label="Required pretrained checkpoints"
          value={configuration.pretrainedCheckpointPaths.join('; ')}
          onChoose={() => void chooseCheckpoints()}
        />
        <PathRow
          label="Managed output directory"
          value={configuration.outputDirectory}
          onChoose={() => void chooseDirectory('outputDirectory')}
        />
      </div>
      <div className="model-control-grid">
        <label>
          Device
          <select
            value={configuration.device}
            onChange={(event) =>
              update({ device: event.target.value as SeedVcBackendConfiguration['device'] })
            }
          >
            <option value="cpu">CPU</option>
            <option value="cuda">CUDA</option>
            <option value="directMl">DirectML (only when reported)</option>
          </select>
        </label>
        <label>
          Precision
          <select
            value={configuration.precision}
            onChange={(event) =>
              update({ precision: event.target.value as SeedVcBackendConfiguration['precision'] })
            }
          >
            <option value="float32">Float 32</option>
            <option value="float16">Float 16</option>
            <option value="bfloat16">BFloat 16</option>
          </select>
        </label>
      </div>
      <div className="voice-lab-actions">
        <button
          type="button"
          disabled={busy}
          onClick={() => void onSave({ schemaVersion: 1, seedVc: configuration })}
        >
          Save backend configuration
        </button>
        <button type="button" className="start" disabled={busy} onClick={() => void onValidate()}>
          Validate backend
        </button>
      </div>
      <small>
        No Python, packages, Seed-VC source, CUDA, or checkpoints are installed or downloaded by the
        app. Third-party ML code remains untrusted even when it runs locally.
      </small>
    </section>
  );
}

function PathRow({
  label,
  value,
  onChoose,
}: {
  label: string;
  value: string;
  onChoose: () => void;
}) {
  return (
    <div className="model-path-row">
      <span>{label}</span>
      <code title={value}>{value ? 'Selected local path' : 'Not selected'}</code>
      <button type="button" onClick={onChoose}>
        Choose…
      </button>
    </div>
  );
}
