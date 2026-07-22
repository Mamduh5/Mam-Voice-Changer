import { useState } from 'react';
import type { UpdateVoiceProfileRequest, VoiceDatasetManifest } from '../../types/voiceDataset';

export function VoiceProfileEditor({
  manifest,
  busy,
  onUpdate,
  onDelete,
  onRepair,
}: {
  manifest: VoiceDatasetManifest;
  busy: boolean;
  onUpdate: (id: string, request: UpdateVoiceProfileRequest) => Promise<boolean>;
  onDelete: (id: string) => Promise<boolean>;
  onRepair: (id: string) => Promise<boolean>;
}) {
  const [name, setName] = useState(manifest.profile.displayName);
  const [description, setDescription] = useState(manifest.profile.description ?? '');
  const [goal, setGoal] = useState(String(manifest.profile.collectionGoalMinutes ?? ''));
  return (
    <section className="card dataset-profile-editor">
      <div className="section-heading">
        <h2>Voice profile</h2>
        <span>
          Consent {manifest.consent.consentConfirmed ? 'confirmed' : 'required'} · schema v
          {manifest.schemaVersion}
        </span>
      </div>
      <div className="dataset-form-grid">
        <label>
          Display name
          <input value={name} maxLength={80} onChange={(event) => setName(event.target.value)} />
        </label>
        <label>
          Collection goal minutes
          <input
            type="number"
            min="1"
            max="600"
            value={goal}
            onChange={(event) => setGoal(event.target.value)}
          />
        </label>
        <label className="dataset-wide">
          Description
          <textarea
            maxLength={500}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
      </div>
      <div className="voice-lab-actions">
        <button
          type="button"
          disabled={busy || !name.trim()}
          onClick={() =>
            void onUpdate(manifest.profile.id, {
              displayName: name,
              description: description.trim() || null,
              primaryLanguage: manifest.profile.primaryLanguage,
              localeTag: manifest.profile.localeTag,
              collectionGoalMinutes: goal ? Number(goal) : null,
            })
          }
        >
          Save profile details
        </button>
        <button type="button" disabled={busy} onClick={() => void onRepair(manifest.profile.id)}>
          Conservative repair
        </button>
        <button
          type="button"
          className="danger-outline"
          disabled={busy}
          onClick={() => {
            if (
              window.confirm(
                'Delete profile and all recordings? Raw recordings, derived recordings, manifest, and consent metadata will be deleted. Exported copies outside the application cannot be deleted automatically.',
              )
            )
              void onDelete(manifest.profile.id);
          }}
        >
          Delete profile and all recordings
        </button>
      </div>
      <small>
        Consent cannot be unchecked while recordings remain. Revoke consent by deleting this
        profile. Deletion does not claim cryptographic erasure.
      </small>
    </section>
  );
}
