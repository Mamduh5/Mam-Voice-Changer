import type { VoiceProfileSummary } from '../../types/voiceProfile';

function bytes(value: number) {
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

export function VoiceProfileList({
  profiles,
  currentId,
  query,
  busy,
  onQuery,
  onSelect,
  onRepair,
}: {
  profiles: VoiceProfileSummary[];
  currentId: string | null;
  query: string;
  busy: boolean;
  onQuery: (query: string) => void;
  onSelect: (id: string) => Promise<boolean>;
  onRepair: (id: string) => Promise<boolean>;
}) {
  const normalized = query.trim().toLocaleLowerCase();
  const filtered = profiles.filter(({ profile }) =>
    [profile.displayName, profile.primaryLanguage, profile.localeTag ?? '']
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalized),
  );
  return (
    <section className="card profile-list-panel" aria-labelledby="profile-list-heading">
      <div className="section-heading">
        <h2 id="profile-list-heading">Voice profiles</h2>
        <span>{profiles.length} local</span>
      </div>
      <label>
        Search profiles
        <input
          type="search"
          value={query}
          placeholder="Name, language, or locale"
          onChange={(event) => onQuery(event.target.value)}
        />
      </label>
      <div className="profile-list" role="listbox" aria-label="Local voice profiles">
        {filtered.map(({ profile, health, managedStorageBytes }) => (
          <div className="profile-list-item" key={profile.id}>
            <button
              type="button"
              role="option"
              aria-selected={currentId === profile.id}
              className={currentId === profile.id ? 'active' : ''}
              disabled={busy || health === 'unsupportedSchema' || health === 'corruptManifest'}
              onClick={() => void onSelect(profile.id)}
            >
              <strong>{profile.displayName}</strong>
              <span>{profile.primaryLanguage}</span>
              <small>
                {health} · {bytes(managedStorageBytes)} managed
              </small>
            </button>
            {health !== 'healthy' && (
              <button
                type="button"
                className="profile-list-repair"
                disabled={busy}
                onClick={() => void onRepair(profile.id)}
              >
                Repair {profile.displayName}
              </button>
            )}
          </div>
        ))}
        {!filtered.length && <p>No profiles match this search.</p>}
      </div>
    </section>
  );
}
