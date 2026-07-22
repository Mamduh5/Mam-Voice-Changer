import { useMemo, useState } from 'react';
import type { useVoiceDataset } from '../../hooks/useVoiceDataset';
import type { AudioDevice } from '../../types/audio';
import type { DatasetTake } from '../../types/voiceDataset';
import type { DatasetTakeFilter } from '../../utils/datasetNavigation';
import { ConsentPanel } from './ConsentPanel';
import { DatasetImportExportPanel } from './DatasetImportExportPanel';
import { DatasetProgress } from './DatasetProgress';
import { DatasetTakeList } from './DatasetTakeList';
import { PromptRecorder } from './PromptRecorder';
import { TakeReview } from './TakeReview';
import { VoiceProfileEditor } from './VoiceProfileEditor';
import { VoiceProfileList } from './VoiceProfileList';

type DatasetHook = ReturnType<typeof useVoiceDataset>;

export function VoiceDatasetPage({
  dataset,
  inputs,
  outputs,
  defaultInputId,
  defaultOutputId,
  disabled,
  liveActive,
}: {
  dataset: DatasetHook;
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  defaultInputId: string;
  defaultOutputId: string;
  disabled: boolean;
  liveActive: boolean;
}) {
  const [filter, setFilter] = useState<DatasetTakeFilter>('all');
  const [selectedTakeId, setSelectedTakeId] = useState<string | null>(null);
  const manifest = dataset.status.manifest;
  const selectedTake = useMemo<DatasetTake | null>(() => {
    const takes = manifest?.takes ?? [];
    return (
      takes.find((take) => take.id === selectedTakeId) ??
      takes.find((take) => take.reviewStatus === 'pending') ??
      takes[0] ??
      null
    );
  }, [manifest, selectedTakeId]);
  const promptSelection = {
    promptId: dataset.status.currentPromptId,
    customPromptText:
      dataset.status.currentPromptCategory === 'custom' ? dataset.status.currentPromptText : null,
  };
  const blocked = disabled || liveActive;

  return (
    <div className="page-stack voice-dataset-page">
      <section className="card voice-lab-intro">
        <div>
          <p className="eyebrow">Persistent local collection workspace</p>
          <h2>Voice Dataset Capture</h2>
          <p>
            Consented prompted dry recordings → objective and heuristic quality analysis → manual
            review → accepted local dataset → explicit export.
          </p>
        </div>
        <span className="bounded-label">20 seconds per prompted take · PCM24 mono 48 kHz</span>
      </section>
      <div className="dataset-safety" role="status">
        The speaker must consent. Recording is never automatic or hidden. This phase does not clone
        a voice, train a model, run inference, upload audio, or make a voice “ready.”
      </div>
      {dataset.error && (
        <div className="error" role="alert">
          <strong>Voice Dataset:</strong> {dataset.error}
        </div>
      )}
      {!dataset.profiles.length && !manifest && (
        <div className="dataset-empty">
          <h3>No local voice profiles</h3>
          <p>Create a profile only after the target speaker explicitly consents.</p>
        </div>
      )}
      <VoiceProfileList
        profiles={dataset.profiles}
        currentId={dataset.status.currentProfileId}
        busy={dataset.busy}
        onSelect={dataset.selectProfile}
      />
      {!manifest && <ConsentPanel busy={dataset.busy} onCreate={dataset.createProfile} />}
      {manifest && (
        <>
          <VoiceProfileEditor
            key={manifest.profile.id}
            manifest={manifest}
            busy={dataset.busy}
            onUpdate={dataset.updateProfile}
            onDelete={dataset.deleteProfile}
            onRepair={dataset.repairProfile}
          />
          <DatasetProgress manifest={manifest} />
          {dataset.prompts && (
            <PromptRecorder
              inputs={inputs}
              defaultInputId={defaultInputId}
              prompts={dataset.prompts}
              status={dataset.status}
              busy={dataset.busy}
              blocked={blocked}
              onSelectPrompt={dataset.selectPrompt}
              onRecord={dataset.record}
              onStop={dataset.stopRecording}
              onDiscard={dataset.discardRecording}
            />
          )}
          <DatasetImportExportPanel
            busy={dataset.busy}
            selection={promptSelection}
            onImport={dataset.importWavs}
            onExport={dataset.exportDataset}
          />
          <DatasetTakeList
            takes={manifest.takes}
            filter={filter}
            selectedId={selectedTake?.id ?? null}
            onFilter={setFilter}
            onSelect={setSelectedTakeId}
          />
          {selectedTake && (
            <TakeReview
              key={selectedTake.id}
              take={selectedTake}
              profileId={manifest.profile.id}
              outputs={outputs}
              defaultOutputId={defaultOutputId}
              status={dataset.status}
              busy={dataset.busy}
              onReview={dataset.reviewTake}
              onAutoTrim={dataset.autoTrim}
              onSetTrim={dataset.setTrim}
              onApplyTrim={dataset.applyTrim}
              onResetTrim={dataset.resetTrim}
              onPreview={dataset.preview}
              onPause={dataset.pausePreview}
              onStop={dataset.stopPreview}
              onDelete={dataset.deleteTake}
            />
          )}
        </>
      )}
    </div>
  );
}
