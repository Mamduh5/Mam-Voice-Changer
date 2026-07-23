import type { ReturnTypeOfUseVoiceProfiles } from './profileTypes';

export function VoiceProfileWorkspaceHeader({
  profiles,
  workspace,
  onOpenProfiles,
}: {
  profiles: ReturnTypeOfUseVoiceProfiles;
  workspace: 'Dataset' | 'Models';
  onOpenProfiles: () => void;
}) {
  const manifest = profiles.manifest;
  const summary = profiles.selectedSummary;
  const usable = Boolean(
    manifest &&
    summary?.health === 'healthy' &&
    profiles.consentActive &&
    manifest.profile.id === profiles.selectedProfileId,
  );
  return (
    <section
      className="card compact-profile-header"
      data-profile-workspace={workspace.toLowerCase()}
    >
      <div>
        <p className="eyebrow">Shared voice profile</p>
        <h2>{manifest?.profile.displayName ?? `No profile selected for ${workspace}`}</h2>
        {manifest && summary ? (
          <p>
            Consent {profiles.consentActive ? 'active' : 'required'} · Health {summary.health} ·{' '}
            {(manifest.statistics.acceptedDurationMs / 60_000).toFixed(1)} accepted minutes
          </p>
        ) : (
          <p>
            {workspace === 'Dataset'
              ? 'Select or create a voice profile before collecting recordings.'
              : 'Select a consent-active voice profile before creating or using a model.'}
          </p>
        )}
      </div>
      <div className="compact-profile-actions">
        <label>
          Change profile
          <select
            value={profiles.selectedProfileId ?? ''}
            disabled={profiles.busy}
            onChange={(event) =>
              event.target.value && void profiles.selectProfile(event.target.value)
            }
          >
            <option value="">Select a profile</option>
            {profiles.profiles.map(({ profile, health }) => (
              <option key={profile.id} value={profile.id} disabled={health !== 'healthy'}>
                {profile.displayName} · {health}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={onOpenProfiles}>
          Open Profiles
        </button>
      </div>
      {!usable && manifest && (
        <div className="dataset-safety" role="status">
          {workspace} workflow is blocked until consent is active and profile health is Healthy.
        </div>
      )}
    </section>
  );
}
