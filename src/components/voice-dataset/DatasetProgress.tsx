import type { VoiceDatasetManifest } from '../../types/voiceDataset';
import { collectionGoalPercent, formatDatasetDuration } from '../../utils/datasetProgress';

export function DatasetProgress({ manifest }: { manifest: VoiceDatasetManifest }) {
  const stats = manifest.statistics;
  const percent = collectionGoalPercent(stats, manifest.profile.collectionGoalMinutes);
  return (
    <section className="card dataset-progress">
      <div className="section-heading">
        <h2>Collection progress</h2>
        <span>More clean speech may improve future model quality</span>
      </div>
      <div className="dataset-metrics">
        <div>
          <strong>{stats.totalTakes}</strong>
          <small>Total takes</small>
        </div>
        <div>
          <strong>{stats.acceptedTakes}</strong>
          <small>Accepted</small>
        </div>
        <div>
          <strong>{stats.pendingTakes}</strong>
          <small>Pending</small>
        </div>
        <div>
          <strong>{stats.rejectedTakes}</strong>
          <small>Rejected</small>
        </div>
        <div>
          <strong>{formatDatasetDuration(stats.acceptedDurationMs)}</strong>
          <small>Accepted recording duration</small>
        </div>
        <div>
          <strong>
            {stats.completedPrompts}/{stats.totalPrompts}
          </strong>
          <small>Prompt coverage</small>
        </div>
        <div>
          <strong>{stats.warningTakes}</strong>
          <small>Warning takes</small>
        </div>
        <div>
          <strong>{stats.failedTakes}</strong>
          <small>Failed takes</small>
        </div>
        <div>
          <strong>{stats.excludedTakes}</strong>
          <small>Excluded</small>
        </div>
      </div>
      {manifest.profile.collectionGoalMinutes && (
        <div className="dataset-goal" aria-label="Collection goal progress">
          <span style={{ width: `${percent}%` }} />
        </div>
      )}
      <small>
        Category coverage: {Object.keys(stats.categoryCoverage).length} categories · Custom takes:{' '}
        {stats.customTakes}. The goal is informational and model-dependent.
      </small>
    </section>
  );
}
