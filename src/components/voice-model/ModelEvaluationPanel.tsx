import { useState } from 'react';
import type {
  EvaluationPhrase,
  ManualModelRatings,
  VoiceModelArtifact,
  VoiceModelStatus,
} from '../../types/voiceModel';
import { canApproveArtifact } from '../../utils/evaluationState';

const initialRatings: ManualModelRatings = {
  intelligibility: 3,
  targetSimilarity: 3,
  naturalness: 3,
  stability: 3,
  noiseAndArtifacts: 3,
  notes: null,
  listeningConfirmed: false,
};

export function ModelEvaluationPanel({
  artifact,
  status,
  phrases,
  consentActive,
  busy,
  onSave,
  onApprove,
}: {
  artifact: VoiceModelArtifact | null;
  status: VoiceModelStatus;
  phrases: EvaluationPhrase[];
  consentActive: boolean;
  busy: boolean;
  onSave: (ratings: ManualModelRatings) => Promise<unknown>;
  onApprove: () => Promise<unknown>;
}) {
  const [ratings, setRatings] = useState(initialRatings);
  if (!artifact) return null;
  const update = (changes: Partial<ManualModelRatings>) =>
    setRatings((current) => ({ ...current, ...changes }));
  return (
    <section className="card model-evaluation-panel">
      <div className="section-heading">
        <h2>7. Manual model evaluation</h2>
        <span>Subjective ratings, not biometrics</span>
      </div>
      <p>
        Evaluate project-authored neutral, long, question, number, plosive, sibilant, sustained
        vowel, and pitch-varied phrases. At least one successful synthetic conversion and confirmed
        listening are required before approval.
      </p>
      <div className="evaluation-phrase-pack" aria-label="Local evaluation phrase pack">
        {phrases.map((phrase) => (
          <div key={phrase.phraseId}>
            <strong>{phrase.category}</strong>
            <span>{phrase.text}</span>
          </div>
        ))}
      </div>
      <div className="model-rating-grid">
        {(
          [
            ['intelligibility', 'Intelligibility'],
            ['targetSimilarity', 'Target similarity'],
            ['naturalness', 'Naturalness'],
            ['stability', 'Stability'],
            ['noiseAndArtifacts', 'Noise/artifacts'],
          ] as const
        ).map(([field, label]) => (
          <label key={field}>
            {label} · {ratings[field]}/5
            <input
              type="range"
              min="1"
              max="5"
              value={ratings[field]}
              onChange={(event) => update({ [field]: Number(event.target.value) })}
            />
          </label>
        ))}
      </div>
      <label>
        Evaluation notes
        <textarea
          maxLength={2000}
          value={ratings.notes ?? ''}
          onChange={(event) => update({ notes: event.target.value || null })}
        />
      </label>
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={ratings.listeningConfirmed}
          onChange={(event) => update({ listeningConfirmed: event.target.checked })}
        />
        I listened to the synthetic conversion and completed the manual evaluation.
      </label>
      <div className="voice-lab-actions">
        <button
          type="button"
          disabled={busy || !status.latestConversion || !ratings.listeningConfirmed}
          onClick={() => void onSave(ratings)}
        >
          Save manual evaluation
        </button>
        <button
          type="button"
          className="start"
          disabled={busy || !canApproveArtifact(artifact, consentActive)}
          onClick={() => void onApprove()}
        >
          Approve for offline Voice Lab
        </button>
      </div>
    </section>
  );
}
