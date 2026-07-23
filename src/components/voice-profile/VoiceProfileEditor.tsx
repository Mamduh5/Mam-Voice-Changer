import { useState } from 'react';
import type { ReturnTypeOfUseVoiceProfiles } from './profileTypes';
import { VoiceProfileDeleteDialog } from './VoiceProfileDeleteDialog';
import { VoiceProfileHealth } from './VoiceProfileHealth';
import { VoiceProfileSummary } from './VoiceProfileSummary';

export function VoiceProfileEditor({ profiles }: { profiles: ReturnTypeOfUseVoiceProfiles }) {
  const manifest = profiles.manifest;
  const summary = profiles.selectedSummary;
  const [name, setName] = useState(manifest?.profile.displayName ?? '');
  const [description, setDescription] = useState(manifest?.profile.description ?? '');
  const [language, setLanguage] = useState(manifest?.profile.primaryLanguage ?? '');
  const [locale, setLocale] = useState(manifest?.profile.localeTag ?? '');
  const [goal, setGoal] = useState(String(manifest?.profile.collectionGoalMinutes ?? ''));
  const [deleteOpen, setDeleteOpen] = useState(false);

  if (!manifest || !summary) return null;
  const profileId = manifest.profile.id;
  return (
    <section className="card profile-editor-panel" aria-labelledby="profile-details-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Opaque ID {profileId}</p>
          <h2 id="profile-details-heading">Profile details</h2>
        </div>
        <span>{profiles.consentActive ? 'Consent active' : 'Consent required'}</span>
      </div>
      <VoiceProfileHealth summary={summary} />
      <VoiceProfileSummary profiles={profiles} />
      <div className="dataset-form-grid">
        <label>
          Display name
          <input value={name} maxLength={80} onChange={(event) => setName(event.target.value)} />
        </label>
        <label>
          Primary language
          <input
            value={language}
            maxLength={64}
            onChange={(event) => setLanguage(event.target.value)}
          />
        </label>
        <label>
          Locale
          <input
            value={locale}
            maxLength={32}
            onChange={(event) => setLocale(event.target.value)}
          />
        </label>
        <label>
          Collection goal (minutes)
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
            value={description}
            maxLength={500}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
      </div>
      <details className="advanced-section">
        <summary>Storage and dependency details</summary>
        <dl className="profile-detail-list">
          <div>
            <dt>Managed storage</dt>
            <dd>{(summary.managedStorageBytes / 1024 / 1024).toFixed(2)} MiB</dd>
          </div>
          <div>
            <dt>Dataset schema</dt>
            <dd>v{manifest.schemaVersion}</dd>
          </div>
          <div>
            <dt>Consent version</dt>
            <dd>{manifest.consent.consentVersion}</dd>
          </div>
          <div>
            <dt>Model dependencies</dt>
            <dd>
              {profiles.modelSummary.snapshots} snapshots, {profiles.modelSummary.artifacts}{' '}
              artifacts
            </dd>
          </div>
        </dl>
      </details>
      <div className="workspace-primary-actions" aria-label="Profile actions">
        <button
          type="button"
          className="start"
          disabled={profiles.busy || !name.trim() || !language.trim()}
          onClick={() =>
            void profiles.updateProfile(profileId, {
              displayName: name,
              description: description.trim() || null,
              primaryLanguage: language,
              localeTag: locale.trim() || null,
              collectionGoalMinutes: goal ? Number(goal) : null,
            })
          }
        >
          Save profile
        </button>
        <button
          type="button"
          disabled={profiles.busy}
          onClick={() => void profiles.exportDataset()}
        >
          Export Dataset
        </button>
        <button
          type="button"
          disabled={profiles.busy}
          onClick={() => void profiles.repairProfile(profileId)}
        >
          Repair profile
        </button>
        <button
          type="button"
          className="danger-outline"
          disabled={profiles.busy}
          onClick={() => setDeleteOpen(true)}
        >
          Delete profile
        </button>
      </div>
      <VoiceProfileDeleteDialog
        open={deleteOpen}
        profileName={manifest.profile.displayName}
        busy={profiles.busy}
        onCancel={() => setDeleteOpen(false)}
        onConfirm={() => {
          void profiles.deleteProfile(profileId).finally(() => setDeleteOpen(false));
        }}
      />
    </section>
  );
}
