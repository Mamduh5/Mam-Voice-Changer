import { useMemo, useState } from 'react';
import type { AudioDevice } from '../../types/audio';
import type { PromptPack, VoiceDatasetStatus } from '../../types/voiceDataset';
import { DeviceSelector } from '../DeviceSelector';

function device(devices: AudioDevice[], id: string) {
  return devices.find((candidate) => candidate.id === id);
}

export function PromptRecorder({
  inputs,
  defaultInputId,
  prompts,
  status,
  busy,
  blocked,
  onSelectPrompt,
  onRecord,
  onStop,
  onDiscard,
}: {
  inputs: AudioDevice[];
  defaultInputId: string;
  prompts: PromptPack;
  status: VoiceDatasetStatus;
  busy: boolean;
  blocked: boolean;
  onSelectPrompt: (selection: {
    promptId: string | null;
    customPromptText: string | null;
  }) => Promise<boolean>;
  onRecord: (id: string, name: string, consent?: boolean) => Promise<boolean>;
  onStop: () => Promise<boolean>;
  onDiscard: () => Promise<boolean>;
}) {
  const [inputSelection, setInputSelection] = useState('');
  const [customPrompt, setCustomPrompt] = useState('');
  const inputId = device(inputs, inputSelection) ? inputSelection : defaultInputId;
  const input = device(inputs, inputId);
  const promptIndex = useMemo(
    () => prompts.prompts.findIndex((prompt) => prompt.id === status.currentPromptId),
    [prompts, status.currentPromptId],
  );
  const selectAt = (index: number) => {
    const prompt = prompts.prompts[(index + prompts.prompts.length) % prompts.prompts.length];
    void onSelectPrompt({ promptId: prompt.id, customPromptText: null });
  };
  const seconds = (status.recording.durationMs / 1_000).toFixed(1);
  return (
    <section className="card dataset-recorder">
      <div className="section-heading">
        <h2>Prompted dry recording</h2>
        <span>
          {status.recording.active
            ? 'Recording — visible and deliberate'
            : status.recording.finalizing
              ? 'Finalizing and analyzing'
              : 'Ready to record'}
        </span>
      </div>
      <p className="dataset-headphone-note">
        Use a physical microphone in a quiet room. Wear headphones for review. No pitch, formant,
        gate, denoise, limiter, or Old Lady processing is applied.
      </p>
      <DeviceSelector
        label="Recording microphone"
        value={inputId}
        devices={inputs}
        disabled={busy || status.recording.active}
        onChange={setInputSelection}
      />
      {!input && (
        <div className="voice-lab-notice">
          No microphone selected. Refresh devices and choose a physical input.
        </div>
      )}
      <article className="dataset-prompt-display">
        <small>
          Prompt {Math.max(1, promptIndex + 1)} of {prompts.prompts.length} ·{' '}
          {status.currentPromptCategory ?? 'custom'}
        </small>
        <blockquote>{status.currentPromptText ?? 'Choose a prompt before recording.'}</blockquote>
        <div className="voice-lab-actions">
          <button
            type="button"
            disabled={busy || status.recording.active}
            onClick={() => selectAt(promptIndex - 1)}
          >
            Previous prompt
          </button>
          <button
            type="button"
            disabled={busy || status.recording.active}
            onClick={() => selectAt(promptIndex + 1)}
          >
            Skip / next prompt
          </button>
        </div>
      </article>
      <div className="dataset-custom-prompt">
        <label>
          Custom prompt text
          <input
            value={customPrompt}
            maxLength={500}
            disabled={busy || status.recording.active}
            onChange={(event) => setCustomPrompt(event.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={!customPrompt.trim() || busy || status.recording.active}
          onClick={() => void onSelectPrompt({ promptId: null, customPromptText: customPrompt })}
        >
          Use custom prompt
        </button>
      </div>
      <div className="dataset-recording-meter" aria-label="Input level">
        <span style={{ width: `${Math.min(100, status.recording.inputLevel * 100)}%` }} />
      </div>
      <div className="dataset-recording-readout">
        <strong>{seconds} s</strong>
        <span>{(status.recording.remainingMs / 1_000).toFixed(1)} s remaining of 20 s maximum</span>
        {status.recording.clipping && (
          <span className="warning">Clipping indicator — lower the microphone level</span>
        )}
        {status.recording.droppedFrames > 0 && (
          <span className="warning">
            Recording overflow: {status.recording.droppedFrames} dropped frame(s)
          </span>
        )}
      </div>
      <div className="workspace-primary-actions" aria-label="Dataset recording actions">
        {!status.recording.active ? (
          <button
            type="button"
            className="start"
            disabled={busy || blocked || !input || !status.currentPromptText}
            onClick={() => input && void onRecord(input.id, input.name)}
          >
            Record phrase
          </button>
        ) : (
          <button type="button" className="stop" onClick={() => void onStop()}>
            Stop recording
          </button>
        )}
        <button
          type="button"
          disabled={busy || blocked || !input || status.recording.active}
          onClick={() => input && void onRecord(input.id, input.name, true)}
        >
          Record optional consent take
        </button>
        <button type="button" disabled={!status.recording.active} onClick={() => void onDiscard()}>
          Discard unfinished take
        </button>
      </div>
      {blocked && (
        <div className="voice-lab-notice">
          Audio device busy. Stop Use, Test, or Voice Lab audio before Dataset recording.
        </div>
      )}
    </section>
  );
}
