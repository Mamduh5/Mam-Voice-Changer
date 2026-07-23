import { useTrainingJob } from '../../hooks/useTrainingJob';
import type { TrainingJob } from '../../types/trainingJob';
import { backendMetric } from '../../utils/modelProgress';

export function TrainingJobPanel({
  job,
  logs,
  busy,
  onCancel,
  onResume,
  onDelete,
}: {
  job: TrainingJob | null;
  logs: string[];
  busy: boolean;
  onCancel: () => Promise<unknown>;
  onResume: (jobId: string) => Promise<unknown>;
  onDelete: (jobId: string) => Promise<unknown>;
}) {
  const training = useTrainingJob(job);
  return (
    <section className="card model-job-panel">
      <div className="section-heading">
        <h2>4. Training job</h2>
        <span>{training.phaseLabel}</span>
      </div>
      {!job ? (
        <p>No local training job has been started.</p>
      ) : (
        <>
          <div className="voice-lab-progress" aria-label="Training progress">
            <span style={{ width: `${training.progressPercent}%` }} />
          </div>
          <div className="model-metrics">
            <span>{training.progressPercent}% overall</span>
            <span>
              Step {job.currentStep.toLocaleString()} / {job.maximumSteps.toLocaleString()}
            </span>
            <span>Training loss: {backendMetric(job.latestMetrics.trainingLoss)}</span>
            <span>Validation loss: {backendMetric(job.latestMetrics.validationLoss)}</span>
            <span>Learning rate: {backendMetric(job.latestMetrics.learningRate)}</span>
            <span>Latest checkpoint: {job.lastCheckpoint ?? 'None reported'}</span>
          </div>
          <small>
            Loss and resource metrics are backend-reported and do not guarantee similarity.
          </small>
          {job.warnings.map((warning) => (
            <p className="warning" key={warning}>
              {warning}
            </p>
          ))}
          <details className="advanced-section">
            <summary>View full worker logs</summary>
            <div className="model-log" aria-label="Bounded worker logs">
              {logs.slice(-25).map((line, index) => (
                <code key={`${index}-${line}`}>{line}</code>
              ))}
            </div>
          </details>
          <div className="workspace-primary-actions" aria-label="Training job actions">
            {training.active && (
              <button
                type="button"
                className="stop"
                disabled={busy}
                onClick={() => void onCancel()}
              >
                Cancel training
              </button>
            )}
            {job.state === 'interrupted' && (
              <button type="button" disabled={busy} onClick={() => void onResume(job.jobId)}>
                Resume from checkpoint
              </button>
            )}
            {!training.active && (
              <button type="button" disabled={busy} onClick={() => void onDelete(job.jobId)}>
                Delete training job
              </button>
            )}
          </div>
        </>
      )}
    </section>
  );
}
