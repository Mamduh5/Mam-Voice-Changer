import type { ReturnTypeOfUseVoiceProfiles } from './profileTypes';

export function VoiceProfileSummary({ profiles }: { profiles: ReturnTypeOfUseVoiceProfiles }) {
  const manifest = profiles.manifest;
  if (!manifest || !profiles.selectedSummary) return null;
  return (
    <section className="profile-summary-grid" aria-label="Selected profile summary">
      <div>
        <span>Consent</span>
        <strong>{profiles.consentActive ? 'Active' : 'Required'}</strong>
      </div>
      <div>
        <span>Accepted Dataset</span>
        <strong>{(manifest.statistics.acceptedDurationMs / 60_000).toFixed(1)} min</strong>
      </div>
      <div>
        <span>Takes</span>
        <strong>{manifest.statistics.totalTakes}</strong>
      </div>
      <div>
        <span>Snapshots</span>
        <strong>{profiles.modelSummary.snapshots}</strong>
      </div>
      <div>
        <span>Models</span>
        <strong>{profiles.modelSummary.artifacts}</strong>
      </div>
      <div>
        <span>Training</span>
        <strong>{profiles.modelSummary.activeTraining ? 'Active' : 'Idle'}</strong>
      </div>
    </section>
  );
}
