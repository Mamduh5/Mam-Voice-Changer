import type { VoiceModelArtifact } from '../../types/voiceModel';
import { approvalLabel } from '../../utils/evaluationState';

export function ModelArtifactList({
  artifacts,
  profileId,
  selectedId,
  onSelect,
}: {
  artifacts: VoiceModelArtifact[];
  profileId: string | null;
  selectedId: string;
  onSelect: (artifactId: string) => void;
}) {
  const visible = artifacts.filter((artifact) => artifact.profileId === profileId);
  return (
    <section className="card model-artifact-list">
      <div className="section-heading">
        <h2>5. Versioned local model artifacts</h2>
        <span>{visible.length} managed</span>
      </div>
      {!visible.length ? (
        <p>No model artifact exists for this profile.</p>
      ) : (
        visible.map((artifact) => (
          <button
            type="button"
            className={artifact.artifactId === selectedId ? 'active' : ''}
            key={artifact.artifactId}
            onClick={() => onSelect(artifact.artifactId)}
          >
            <strong>{artifact.displayName}</strong>
            <span>{approvalLabel(artifact)}</span>
            <small>
              {artifact.backendId} · {artifact.trainingSummary.completedSteps} steps
            </small>
          </button>
        ))
      )}
    </section>
  );
}
