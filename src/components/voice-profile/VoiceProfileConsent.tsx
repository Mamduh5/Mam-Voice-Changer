import { useState } from 'react';
import {
  VOICE_DATASET_CONSENT_VERSION,
  type CreateVoiceProfileRequest,
} from '../../types/voiceProfile';

export function VoiceProfileConsent({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (request: CreateVoiceProfileRequest) => Promise<boolean>;
}) {
  const [expanded, setExpanded] = useState(false);
  const [displayName, setDisplayName] = useState('');
  const [description, setDescription] = useState('');
  const [language, setLanguage] = useState('English');
  const [localeTag, setLocaleTag] = useState('en-US');
  const [goal, setGoal] = useState('10');
  const [consent, setConsent] = useState(false);

  if (!expanded) {
    return (
      <button
        type="button"
        className="start profile-create-trigger"
        onClick={() => setExpanded(true)}
      >
        Create profile
      </button>
    );
  }
  return (
    <section className="card profile-create-panel" aria-labelledby="create-profile-heading">
      <div className="section-heading">
        <h2 id="create-profile-heading">Create profile</h2>
        <button type="button" disabled={busy} onClick={() => setExpanded(false)}>
          Cancel
        </button>
      </div>
      <p>
        The target speaker must consent to deliberate recording and private local use. Profile
        metadata is a product safeguard, not legal verification.
      </p>
      <div className="dataset-form-grid">
        <label>
          Display name
          <input
            value={displayName}
            maxLength={80}
            onChange={(event) => setDisplayName(event.target.value)}
          />
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
            value={localeTag}
            maxLength={32}
            onChange={(event) => setLocaleTag(event.target.value)}
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
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={consent}
          onChange={(event) => setConsent(event.target.checked)}
        />
        I explicitly confirm that the target speaker consented to creation and private use of this
        local voice profile.
      </label>
      <button
        type="button"
        className="start"
        disabled={busy || !consent || !displayName.trim() || !language.trim()}
        onClick={async () => {
          const created = await onCreate({
            displayName,
            description: description.trim() || null,
            primaryLanguage: language,
            localeTag: localeTag.trim() || null,
            collectionGoalMinutes: goal ? Number(goal) : null,
            consentConfirmed: consent,
            confirmedByUser: consent,
            consentVersion: VOICE_DATASET_CONSENT_VERSION,
            consentNotes: null,
          });
          if (created) {
            setDisplayName('');
            setDescription('');
            setConsent(false);
            setExpanded(false);
          }
        }}
      >
        Create consenting profile
      </button>
    </section>
  );
}
