import { useState } from 'react';
import type { AudioDevice } from '../../types/audio';
import type {
  DatasetTake,
  ReviewTakeRequest,
  SelectedTakeVersion,
  VoiceDatasetStatus,
} from '../../types/voiceDataset';
import { DeviceSelector } from '../DeviceSelector';
import { TakeQualityPanel } from './TakeQualityPanel';

function findDevice(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id);
}

export function TakeReview({
  take,
  profileId,
  outputs,
  defaultOutputId,
  status,
  busy,
  onReview,
  onAutoTrim,
  onSetTrim,
  onApplyTrim,
  onResetTrim,
  onPreview,
  onPause,
  onStop,
  onDelete,
}: {
  take: DatasetTake;
  profileId: string;
  outputs: AudioDevice[];
  defaultOutputId: string;
  status: VoiceDatasetStatus;
  busy: boolean;
  onReview: (profileId: string, takeId: string, request: ReviewTakeRequest) => Promise<boolean>;
  onAutoTrim: (takeId: string) => Promise<boolean>;
  onSetTrim: (takeId: string, start: number, end: number) => Promise<boolean>;
  onApplyTrim: () => Promise<boolean>;
  onResetTrim: (takeId: string) => Promise<boolean>;
  onPreview: (
    takeId: string,
    version: SelectedTakeVersion,
    outputId: string,
    outputName: string,
    seekMs?: number,
  ) => Promise<boolean>;
  onPause: () => Promise<boolean>;
  onStop: () => Promise<boolean>;
  onDelete: (takeId: string) => Promise<boolean>;
}) {
  const [outputSelection, setOutputSelection] = useState('');
  const [version, setVersion] = useState<SelectedTakeVersion>(take.selectedVersion);
  const [excluded, setExcluded] = useState(take.excludeFromTraining);
  const [notes, setNotes] = useState(take.notes ?? '');
  const [trimStart, setTrimStart] = useState(take.trim?.startFrame ?? 0);
  const [trimEnd, setTrimEnd] = useState(take.trim?.endFrame ?? take.frameCount);
  const outputId = findDevice(outputs, outputSelection) ? outputSelection : defaultOutputId;
  const output = findDevice(outputs, outputId);
  const review = (reviewStatus: ReviewTakeRequest['status']) => {
    const failedAcceptance = reviewStatus === 'accepted' && take.quality.classification === 'fail';
    const acknowledged =
      !failedAcceptance ||
      window.confirm(
        'This take failed automatic checks. Keep it anyway and record a manual override?',
      );
    if (!acknowledged) return;
    void onReview(profileId, take.id, {
      status: reviewStatus,
      excludeFromTraining: excluded,
      notes: notes.trim() || null,
      warningAcknowledged: acknowledged,
      selectedVersion: version,
    });
  };
  return (
    <section className="card dataset-review">
      <div className="section-heading">
        <h2>Review required</h2>
        <span>
          {take.source} · {take.reviewStatus}
        </span>
      </div>
      <blockquote>{take.promptText ?? 'No prompt associated'}</blockquote>
      <div className="voice-lab-waveform" aria-label="Take waveform">
        {take.waveformEnvelope.map((point, index) => (
          <span
            key={index}
            style={{
              height: `${Math.max(4, Math.max(Math.abs(point.minimum), Math.abs(point.maximum)) * 100)}%`,
            }}
          />
        ))}
      </div>
      <DeviceSelector
        label="Physical preview output"
        value={outputId}
        devices={outputs}
        disabled={busy || status.preview.active}
        onChange={setOutputSelection}
      />
      <p>
        Use headphones. Dataset preview never uses the external virtual route, Discord destination,
        or Use output.
      </p>
      <div className="voice-lab-actions">
        <label className="limiter-toggle">
          <input
            type="radio"
            name={`version-${take.id}`}
            checked={version === 'raw'}
            onChange={() => setVersion('raw')}
          />
          Raw
        </label>
        <label className="limiter-toggle">
          <input
            type="radio"
            name={`version-${take.id}`}
            checked={version === 'trimmed'}
            disabled={!take.derivedFile}
            onChange={() => setVersion('trimmed')}
          />
          Trimmed
        </label>
        <button
          type="button"
          disabled={busy || !output}
          onClick={() => output && void onPreview(take.id, version, output.id, output.name)}
        >
          Listen
        </button>
        <button type="button" disabled={!status.preview.active} onClick={() => void onPause()}>
          {status.preview.paused ? 'Resume' : 'Pause'}
        </button>
        <button type="button" disabled={!status.preview.active} onClick={() => void onStop()}>
          Stop
        </button>
      </div>
      {status.preview.takeId === take.id && (
        <input
          aria-label="Preview seek"
          type="range"
          min="0"
          max={Math.max(1, status.preview.durationMs)}
          value={status.preview.positionMs}
          disabled={!output}
          onChange={(event) =>
            output &&
            void onPreview(take.id, version, output.id, output.name, Number(event.target.value))
          }
        />
      )}
      <TakeQualityPanel
        report={version === 'trimmed' && take.trim ? take.trim.derivedQuality : take.quality}
      />
      <div className="dataset-trim-controls">
        <strong>Non-destructive silence trimming</strong>
        <label>
          Start frame
          <input
            type="number"
            min="0"
            max={take.frameCount - 1}
            value={trimStart}
            onChange={(event) => setTrimStart(Number(event.target.value))}
          />
        </label>
        <label>
          End frame
          <input
            type="number"
            min="1"
            max={take.frameCount}
            value={trimEnd}
            onChange={(event) => setTrimEnd(Number(event.target.value))}
          />
        </label>
        <button
          type="button"
          disabled={busy}
          onClick={async () => {
            if (await onAutoTrim(take.id)) void onApplyTrim();
          }}
        >
          Auto-detect trim
        </button>
        <button
          type="button"
          disabled={busy || trimStart >= trimEnd}
          onClick={async () => {
            if (await onSetTrim(take.id, trimStart, trimEnd)) void onApplyTrim();
          }}
        >
          Apply trimmed version
        </button>
        <button
          type="button"
          disabled={busy || !take.derivedFile}
          onClick={() => void onResetTrim(take.id)}
        >
          Reset trimming
        </button>
      </div>
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={excluded}
          onChange={(event) => setExcluded(event.target.checked)}
        />
        Exclude from future training data
      </label>
      <label>
        Review notes
        <textarea
          maxLength={500}
          value={notes}
          onChange={(event) => setNotes(event.target.value)}
        />
      </label>
      <div className="voice-lab-actions">
        <button type="button" className="start" disabled={busy} onClick={() => review('accepted')}>
          Accept take
        </button>
        <button type="button" disabled={busy} onClick={() => review('rejected')}>
          Reject take
        </button>
        <button type="button" disabled={busy} onClick={() => review('needsRedo')}>
          Redo take
        </button>
        <button
          type="button"
          className="danger-outline"
          disabled={busy}
          onClick={() => {
            if (window.confirm('Delete this raw take and any derived trimmed file?'))
              void onDelete(take.id);
          }}
        >
          Delete take
        </button>
      </div>
    </section>
  );
}
