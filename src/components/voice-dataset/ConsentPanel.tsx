import { useState } from 'react';
import {
  VOICE_DATASET_CONSENT_VERSION,
  type CreateVoiceProfileRequest,
} from '../../types/voiceDataset';

export function ConsentPanel({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (request: CreateVoiceProfileRequest) => Promise<boolean>;
}) {
  const [displayName, setDisplayName] = useState('');
  const [description, setDescription] = useState('');
  const [language, setLanguage] = useState('English');
  const [localeTag, setLocaleTag] = useState('en-US');
  const [goal, setGoal] = useState('10');
  const [consent, setConsent] = useState(false);

  return (
    <section className="card dataset-consent">
      <div className="section-heading">
        <h2>Create a consenting speaker profile</h2>
        <span>Consent required</span>
      </div>
      <p>
        The target speaker must consent to visible, deliberate recording and private local use.
        Collection does not create a cloned voice. Data stays in managed local storage until you
        explicitly export it, and deleting the profile revokes consent inside this application.
        Future generated speech must never be represented as an authentic recording of the speaker.
        Consent metadata is a product safeguard, not legal verification.
      </p>
      <div className="dataset-form-grid">
        <label>
          Profile display name
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
          Locale tag (optional)
          <input
            value={localeTag}
            maxLength={32}
            onChange={(event) => setLocaleTag(event.target.value)}
          />
        </label>
        <label>
          Collection goal in minutes (informational)
          <input
            type="number"
            min="1"
            max="600"
            value={goal}
            onChange={(event) => setGoal(event.target.value)}
          />
        </label>
        <label className="dataset-wide">
          Description (optional)
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
        onClick={() =>
          void onCreate({
            displayName,
            description: description.trim() || null,
            primaryLanguage: language,
            localeTag: localeTag.trim() || null,
            collectionGoalMinutes: goal ? Number(goal) : null,
            consentConfirmed: consent,
            confirmedByUser: consent,
            consentVersion: VOICE_DATASET_CONSENT_VERSION,
            consentNotes: null,
          })
        }
      >
        Create voice profile
      </button>
    </section>
  );
}
