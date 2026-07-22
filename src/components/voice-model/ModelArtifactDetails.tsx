import { useState } from 'react';
import type { VoiceModelArtifact } from '../../types/voiceModel';
import { approvalLabel } from '../../utils/evaluationState';

export function ModelArtifactDetails({
  artifact,
  busy,
  onRename,
  onReject,
  onDelete,
}: {
  artifact: VoiceModelArtifact | null;
  busy: boolean;
  onRename: (artifactId: string, name: string) => Promise<unknown>;
  onReject: (artifactId: string, notes: string | null) => Promise<unknown>;
  onDelete: (artifactId: string) => Promise<unknown>;
}) {
  const [name, setName] = useState(artifact?.displayName ?? '');
  if (!artifact) return null;
  return (
    <section className="card model-artifact-details">
      <div className="section-heading">
        <h2>Artifact details</h2>
        <span>{approvalLabel(artifact)}</span>
      </div>
      <label>
        Model display name
        <input value={name} maxLength={80} onChange={(event) => setName(event.target.value)} />
      </label>
      <div className="model-metrics">
        <span>Artifact {artifact.artifactId}</span>
        <span>Snapshot {artifact.snapshotId}</span>
        <span>{artifact.modelFiles.length} hash-validated model files</span>
        <span>Consent {artifact.consentVersion}</span>
      </div>
      <div className="voice-lab-actions">
        <button
          type="button"
          disabled={busy}
          onClick={() => void onRename(artifact.artifactId, name)}
        >
          Rename model
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void onReject(artifact.artifactId, 'Rejected during manual evaluation.')}
        >
          Reject model
        </button>
        <button
          type="button"
          className="danger-outline"
          disabled={busy}
          onClick={() => void onDelete(artifact.artifactId)}
        >
          Delete model
        </button>
      </div>
    </section>
  );
}
