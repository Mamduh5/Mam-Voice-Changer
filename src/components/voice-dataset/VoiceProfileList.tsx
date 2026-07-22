import type { VoiceProfileSummary } from '../../types/voiceDataset';

function bytes(value: number) {
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

export function VoiceProfileList({
  profiles,
  currentId,
  busy,
  onSelect,
}: {
  profiles: VoiceProfileSummary[];
  currentId: string | null;
  busy: boolean;
  onSelect: (id: string) => Promise<boolean>;
}) {
  return (
    <section className="card dataset-profile-list">
      <div className="section-heading">
        <h2>Local voice profiles</h2>
        <span>{profiles.length} profile(s)</span>
      </div>
      <div className="dataset-profile-buttons">
        {profiles.map(({ profile, health, managedStorageBytes }) => (
          <button
            type="button"
            key={profile.id}
            className={currentId === profile.id ? 'active' : ''}
            disabled={busy}
            onClick={() => void onSelect(profile.id)}
          >
            <strong>{profile.displayName}</strong>
            <small>
              {profile.primaryLanguage} · {health} · {bytes(managedStorageBytes)} managed
            </small>
          </button>
        ))}
      </div>
    </section>
  );
}
