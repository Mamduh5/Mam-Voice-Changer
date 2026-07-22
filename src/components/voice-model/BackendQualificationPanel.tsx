import { save } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';
import type {
  BackendCompatibilityProfile,
  ManualListeningQualification,
  QualificationRun,
} from '../../types/modelBackend';

const qualificationSteps = [
  'Select compatibility profile',
  'Select Python executable',
  'Select worker package',
  'Select Seed-VC checkout',
  'Select configuration files',
  'Select required checkpoints',
  'Inspect repository revision',
  'Hash files',
  'Inspect Python packages',
  'Inspect CPU/GPU resources',
  'Run worker handshake',
  'Run backend import test',
  'Run audio smoke test',
  'Optional inference smoke test',
  'Review qualification report',
  'Acknowledge warnings and save',
];

const emptyListening: ManualListeningQualification = {
  syntheticOutputPlayed: false,
  speechIntelligible: false,
  noSevereClipping: false,
  noSevereTruncation: false,
  noSourceTargetMixUp: false,
  syntheticLabelReviewed: false,
  notes: null,
  confirmedAt: null,
};

export function BackendQualificationPanel({
  profiles,
  selectedProfileId,
  referenceTakes,
  qualification,
  active,
  busy,
  onRun,
  onLoadSmoke,
  onCancel,
  onConfirmListening,
  onSaveReport,
  onRepairIndexes,
}: {
  profiles: BackendCompatibilityProfile[];
  selectedProfileId: string;
  referenceTakes: Array<{ id: string; label: string }>;
  qualification: QualificationRun | null;
  active: boolean;
  busy: boolean;
  onRun: (referenceTakeId: string | null) => Promise<unknown>;
  onLoadSmoke: () => Promise<unknown>;
  onCancel: () => Promise<unknown>;
  onConfirmListening: (confirmation: ManualListeningQualification) => Promise<unknown>;
  onSaveReport: (destination: string, humanReadable: boolean) => Promise<unknown>;
  onRepairIndexes: () => Promise<unknown>;
}) {
  const [listening, setListening] = useState(emptyListening);
  const [referenceTakeId, setReferenceTakeId] = useState('');
  const profile = profiles.find((item) => item.profileId === selectedProfileId) ?? null;
  const saveReport = async (humanReadable: boolean) => {
    const destination = await save({
      defaultPath: humanReadable ? 'backend-qualification.txt' : 'backend-qualification.json',
      filters: [
        humanReadable
          ? { name: 'Text report', extensions: ['txt'] }
          : { name: 'JSON report', extensions: ['json'] },
      ],
    });
    if (destination) await onSaveReport(destination, humanReadable);
  };
  const copyReport = async () => {
    if (!qualification || typeof navigator === 'undefined' || !navigator.clipboard) return;
    await navigator.clipboard.writeText(JSON.stringify(qualification, null, 2));
  };
  const listeningReady = Object.entries(listening)
    .filter(([key]) => !['notes', 'confirmedAt'].includes(key))
    .every(([, value]) => value === true);
  return (
    <section className="card backend-qualification-panel">
      <div className="section-heading">
        <h2>3. Backend Qualification</h2>
        <span data-qualification-state={qualification?.state ?? 'notStarted'}>
          {qualification?.state ?? 'Not configured'}
        </span>
      </div>
      <p>
        Qualification is layered and machine-specific. Worker startup alone is not backend
        qualification, and audible quality remains a manual gate.
      </p>
      <ol className="qualification-steps">
        {qualificationSteps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      <div className="support-matrix" role="region" aria-label="Backend support matrix">
        <table>
          <thead>
            <tr>
              <th>Profile</th>
              <th>Revision</th>
              <th>Adapter</th>
              <th>Python</th>
              <th>Devices / precision</th>
              <th>Declared support</th>
              <th>This machine</th>
            </tr>
          </thead>
          <tbody>
            {profiles.map((item) => (
              <tr key={item.profileId}>
                <td>{item.displayName}</td>
                <td>
                  {item.supportedCommitShas.length
                    ? item.supportedCommitShas.join(', ')
                    : 'Unpinned'}
                </td>
                <td>{item.workerAdapterVersion}</td>
                <td>
                  {item.pythonRequirement.minimumInclusive}â€“
                  {item.pythonRequirement.maximumExclusive}
                </td>
                <td>
                  {item.supportedDevices.join(', ')} / {item.supportedPrecisions.join(', ')}
                </td>
                <td>
                  Training {item.capabilities.training ? 'yes' : 'no'} Â· resume{' '}
                  {item.capabilities.resume ? 'yes' : 'no'} Â· offline inference{' '}
                  {item.capabilities.offlineInference ? 'yes' : 'no'}
                </td>
                <td>
                  {qualification?.compatibilityProfileId === item.profileId
                    ? `${qualification.state} / ${qualification.finalLevel}`
                    : 'Not qualified'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {profile?.supportStatus === 'experimental' && (
        <div className="dataset-safety" role="status">
          Experimental profile: no verified Seed-VC commit is pinned. A profile definition is not
          proof that this environment is qualified.
        </div>
      )}
      <label>
        Optional inference smoke reference
        <select
          value={referenceTakeId}
          disabled={busy || active}
          onChange={(event) => setReferenceTakeId(event.target.value)}
        >
          <option value="">Skip inference smoke test</option>
          {referenceTakes.map((take) => (
            <option key={take.id} value={take.id}>
              {take.label}
            </option>
          ))}
        </select>
      </label>
      <small>
        Only an accepted, included take from the active consent profile can be selected. The source
        fixture is project-generated; the output is always labeled synthetic.
      </small>
      <div className="voice-lab-actions">
        <button
          type="button"
          className="start"
          disabled={busy || active || !profile}
          onClick={() => void onRun(referenceTakeId || null)}
        >
          Run layered qualification
        </button>
        <button type="button" disabled={!active} onClick={() => void onCancel()}>
          Cancel qualification
        </button>
        <button type="button" disabled={!qualification} onClick={() => void copyReport()}>
          Copy report
        </button>
        <button type="button" disabled={!qualification} onClick={() => void saveReport(false)}>
          Save JSON report
        </button>
        <button type="button" disabled={!qualification} onClick={() => void saveReport(true)}>
          Save text report
        </button>
        <button type="button" disabled={busy || active} onClick={() => void onRepairIndexes()}>
          Rebuild recovery indexes
        </button>
      </div>
      {qualification && (
        <div className="qualification-report">
          <h3>Qualification report</h3>
          <div className="model-metrics">
            <span>Depth: {qualification.finalLevel}</span>
            <span>Revision: {qualification.repository?.commitSha ?? 'unknown'}</span>
            <span>Checkout: {qualification.repository?.cleanliness ?? 'unknown'}</span>
            <span>Adapter: {qualification.adapterVersion}</span>
            <span>
              Fingerprint: {qualification.environmentFingerprint?.aggregateHash ?? 'pending'}
            </span>
            <span>
              Device:{' '}
              {qualification.environmentFingerprint?.accelerator.selectedDevice ?? 'unknown'}
            </span>
          </div>
          <div className="qualification-checks">
            {qualification.completedChecks.map((check) => (
              <div key={`${check.layer}-${check.code}`} data-check-status={check.status}>
                <strong>{check.label}</strong>
                <span>
                  {check.layer} Â· {check.status}
                </span>
                <p>{check.message}</p>
              </div>
            ))}
          </div>
          {qualification.environmentFingerprint && (
            <div>
              <h3>Relevant Python packages</h3>
              <ul>
                {qualification.environmentFingerprint.packages.map((item) => (
                  <li key={item.package}>
                    {item.package}: {item.version ?? 'missing'}
                  </li>
                ))}
              </ul>
              <h3>Checkpoint identity</h3>
              <ul>
                {qualification.environmentFingerprint.checkpoints.map((item) => (
                  <li key={`${item.role}-${item.displayPath}`}>
                    {item.role}: {item.validationState} Â· SHA-256{' '}
                    {item.contentHash ?? 'unavailable'}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {qualification.resources && (
            <div className="model-metrics">
              <span>Resource risk: {qualification.resources.riskLevel ?? 'unknown'}</span>
              <span>Logical CPUs: {qualification.resources.logicalCpuCount ?? 'unknown'}</span>
              <span>Free disk: {formatBytes(qualification.resources.freeDiskBytes)}</span>
              <span>
                Available RAM: {formatBytes(qualification.resources.availableMemoryBytes)}
              </span>
              <span>Available VRAM: {formatBytes(qualification.resources.availableVramBytes)}</span>
            </div>
          )}
          {qualification.inferenceSmokeResult && (
            <div className="dataset-safety" role="status">
              Synthetic inference smoke WAV: {qualification.inferenceSmokeResult.durationMs} ms,
              peak {qualification.inferenceSmokeResult.peak.toFixed(3)}, clipping{' '}
              {qualification.inferenceSmokeResult.clipping ? 'detected' : 'not detected'}.
              <button type="button" disabled={busy} onClick={() => void onLoadSmoke()}>
                Load synthetic smoke into Voice Lab
              </button>
            </div>
          )}
          {qualification.warnings.map((warning) => (
            <div className="dataset-safety" key={warning}>
              {warning}
            </div>
          ))}
          {qualification.failures.map((failure) => (
            <div className="error" key={failure}>
              {failure}
            </div>
          ))}
          <p>
            Optional inference smoke test:{' '}
            {qualification.finalLevel === 'inferenceGenerated' ? 'generated' : 'pending'}. Manual
            audible quality is never inferred from WAV validation.
          </p>
          <details>
            <summary>Manual listening qualification</summary>
            {(
              [
                ['syntheticOutputPlayed', 'Synthetic output played successfully'],
                ['speechIntelligible', 'Speech is intelligible'],
                ['noSevereClipping', 'No severe clipping'],
                ['noSevereTruncation', 'No severe truncation'],
                ['noSourceTargetMixUp', 'No unexpected source/target mix-up'],
                ['syntheticLabelReviewed', 'Synthetic labeling reviewed'],
              ] as const
            ).map(([field, label]) => (
              <label className="dataset-consent-check" key={field}>
                <input
                  type="checkbox"
                  checked={listening[field]}
                  onChange={(event) =>
                    setListening((current) => ({ ...current, [field]: event.target.checked }))
                  }
                />
                {label}
              </label>
            ))}
            <label>
              Listening notes
              <textarea
                maxLength={2000}
                value={listening.notes ?? ''}
                onChange={(event) =>
                  setListening((current) => ({ ...current, notes: event.target.value || null }))
                }
              />
            </label>
            <button
              type="button"
              disabled={
                busy || !listeningReady || qualification.finalLevel !== 'inferenceGenerated'
              }
              onClick={() =>
                void onConfirmListening({ ...listening, confirmedAt: Date.now().toString() })
              }
            >
              Confirm manual listening gate
            </button>
          </details>
        </div>
      )}
      <small>
        No automatic downloads are permitted. The configured third-party Python code may still be
        capable of network access outside Mam Voice Changer's control.
      </small>
    </section>
  );
}

function formatBytes(value: number | null) {
  if (value === null) return 'unknown';
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}
