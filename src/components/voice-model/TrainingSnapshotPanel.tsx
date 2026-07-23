import type { VoiceDatasetManifest } from '../../types/voiceDataset';
import type { TrainingSnapshot } from '../../types/voiceModel';
import { modelProfileReadiness } from '../../utils/modelReadiness';

export function TrainingSnapshotPanel({
  manifest,
  snapshots,
  selectedSnapshotId,
  busy,
  onCreate,
  onSelectSnapshot,
  onDelete,
}: {
  manifest: VoiceDatasetManifest | null;
  snapshots: TrainingSnapshot[];
  selectedSnapshotId: string;
  busy: boolean;
  onCreate: () => Promise<unknown>;
  onSelectSnapshot: (snapshotId: string) => void;
  onDelete: (snapshotId: string) => Promise<unknown>;
}) {
  const readiness = modelProfileReadiness(manifest);
  const profileSnapshots = snapshots.filter(
    (snapshot) => snapshot.profileId === manifest?.profile.id,
  );
  return (
    <section className="card model-snapshot-panel">
      <div className="section-heading">
        <h2>Dataset snapshot</h2>
        <span>{readiness.message}</span>
      </div>
      {manifest && (
        <div className="model-metrics">
          <span>{manifest.statistics.acceptedTakes} accepted takes</span>
          <span>
            {(manifest.statistics.acceptedDurationMs / 60_000).toFixed(1)} accepted minutes
          </span>
          <span>{Object.keys(manifest.statistics.categoryCoverage).length} prompt categories</span>
        </div>
      )}
      <div className="workspace-primary-actions" aria-label="Snapshot actions">
        <button type="button" disabled={busy || !readiness.ready} onClick={() => void onCreate()}>
          Create training snapshot
        </button>
      </div>
      {profileSnapshots.length ? (
        <div className="model-snapshot-list">
          {profileSnapshots.map((snapshot) => (
            <label key={snapshot.snapshotId}>
              <input
                type="radio"
                name="snapshot"
                checked={selectedSnapshotId === snapshot.snapshotId}
                onChange={() => onSelectSnapshot(snapshot.snapshotId)}
              />
              <span>
                {snapshot.takes.length} takes · {(snapshot.totalDurationMs / 60_000).toFixed(1)} min
                · {snapshot.split.trainingTakeCount} train / {snapshot.split.validationTakeCount}{' '}
                validation
              </span>
              <button
                type="button"
                disabled={busy}
                onClick={() => void onDelete(snapshot.snapshotId)}
              >
                Delete snapshot
              </button>
              {snapshot.warnings.map((warning) => (
                <small className="warning" key={warning}>
                  {warning}
                </small>
              ))}
            </label>
          ))}
        </div>
      ) : (
        <p>No immutable training snapshot yet.</p>
      )}
    </section>
  );
}
