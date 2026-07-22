import { useState } from 'react';
import type { useVoiceDataset } from '../../hooks/useVoiceDataset';
import type { useVoiceModels } from '../../hooks/useVoiceModels';
import { trainingPresets } from '../../types/trainingJob';
import type { ManualModelRatings } from '../../types/voiceModel';
import { ModelArtifactDetails } from './ModelArtifactDetails';
import { ModelArtifactList } from './ModelArtifactList';
import { ModelBackendSetup } from './ModelBackendSetup';
import { ModelDeletionDialog } from './ModelDeletionDialog';
import { ModelEvaluationPanel } from './ModelEvaluationPanel';
import { OfflineConversionPanel } from './OfflineConversionPanel';
import { SyntheticAudioNotice } from './SyntheticAudioNotice';
import { TrainingConfigurationPanel } from './TrainingConfigurationPanel';
import { TrainingJobPanel } from './TrainingJobPanel';
import { TrainingSnapshotPanel } from './TrainingSnapshotPanel';

type DatasetHook = ReturnType<typeof useVoiceDataset>;
type ModelsHook = ReturnType<typeof useVoiceModels>;

export function VoiceModelPage({
  dataset,
  models,
  hasVoiceLabSource,
  disabled,
}: {
  dataset: DatasetHook;
  models: ModelsHook;
  hasVoiceLabSource: boolean;
  disabled: boolean;
}) {
  const [snapshotSelection, setSnapshotSelection] = useState('');
  const [artifactSelection, setArtifactSelection] = useState('');
  const [pendingArtifactDeletion, setPendingArtifactDeletion] = useState<string | null>(null);
  const [trainingConfiguration, setTrainingConfiguration] = useState(
    trainingPresets.quickExperiment,
  );
  const profileId = dataset.status.manifest?.profile.id ?? null;
  const profileSnapshots = models.status.snapshots.filter(
    (snapshot) => snapshot.profileId === profileId,
  );
  const selectedSnapshotId = profileSnapshots.some(
    (snapshot) => snapshot.snapshotId === snapshotSelection,
  )
    ? snapshotSelection
    : (profileSnapshots[0]?.snapshotId ?? '');
  const profileArtifacts = models.status.artifacts.filter(
    (artifact) => artifact.profileId === profileId,
  );
  const selectedArtifact =
    profileArtifacts.find((artifact) => artifact.artifactId === artifactSelection) ??
    profileArtifacts[0] ??
    null;
  const consentActive = Boolean(
    dataset.status.manifest?.consent.consentConfirmed &&
    !dataset.status.manifest?.consent.revokedAt,
  );
  const busy = disabled || dataset.busy || models.busy;
  const backendReady = models.status.backend.readiness === 'ready';
  const trainingActive = Boolean(
    models.status.activeTrainingJob &&
    !['cancelled', 'completed', 'failed', 'interrupted'].includes(
      models.status.activeTrainingJob.state,
    ),
  );

  const createSnapshot = () => {
    if (!profileId) return Promise.resolve(null);
    return models.createSnapshot({
      profileId,
      minimumAcceptedDurationMs: 30_000,
      validationPercent: 20,
      splitSeed: 13,
    });
  };
  const startTraining = () => {
    if (!profileId || !selectedSnapshotId) return Promise.resolve(null);
    if (
      typeof window !== 'undefined' &&
      !window.confirm(
        'Start local fine-tuning in the configured third-party worker? Training output is synthetic and quality is not guaranteed.',
      )
    )
      return Promise.resolve(null);
    return models.startTraining(profileId, selectedSnapshotId, trainingConfiguration);
  };
  const convert = () => {
    if (!profileId || !selectedArtifact) return Promise.resolve(null);
    const configuration = {
      diffusionSteps: 25,
      f0Conditioning: false,
      pitchAdjustmentSemitones: 0,
      lengthAdjustment: 1,
      device: models.settings.seedVc?.device ?? ('cpu' as const),
      precision: models.settings.seedVc?.precision ?? ('float32' as const),
      referenceTakeIds: [],
    };
    return selectedArtifact.approvalStatus === 'approvedForOfflineUse'
      ? models.startConversion(profileId, selectedArtifact.artifactId, configuration)
      : models.startEvaluationConversion(profileId, selectedArtifact.artifactId, configuration);
  };
  const saveEvaluation = (ratings: ManualModelRatings) => {
    if (!profileId || !selectedArtifact || !models.status.latestConversion)
      return Promise.resolve(null);
    return models.saveEvaluation(profileId, selectedArtifact.artifactId, {
      schemaVersion: 1,
      clips: [
        {
          phraseId: 'user-voice-lab-source',
          phraseLabel: 'Voice Lab evaluation source',
          resultId: models.status.latestConversion.resultId,
          successful: true,
        },
      ],
      ratings,
      completedAt: Date.now().toString(),
    });
  };

  return (
    <div className="page-stack voice-model-page">
      <section className="card voice-lab-intro">
        <div>
          <p className="eyebrow">Local child-process machine-learning workspace</p>
          <h2>Voice Models</h2>
          <p>
            Accepted consenting Dataset → immutable snapshot → isolated local worker → versioned
            artifact → manual evaluation → approved offline Voice Lab conversion.
          </p>
        </div>
        <span className="bounded-label">Offline only · no automatic downloads</span>
      </section>
      <SyntheticAudioNotice />
      {(models.error || dataset.error) && (
        <div className="error" role="alert">
          <strong>Voice Models:</strong> {models.error ?? dataset.error}
        </div>
      )}
      <TrainingSnapshotPanel
        profiles={dataset.profiles}
        manifest={dataset.status.manifest}
        snapshots={models.status.snapshots}
        selectedSnapshotId={selectedSnapshotId}
        busy={busy || trainingActive}
        onSelectProfile={dataset.selectProfile}
        onCreate={createSnapshot}
        onSelectSnapshot={setSnapshotSelection}
        onDelete={models.deleteSnapshot}
      />
      <ModelBackendSetup
        key={JSON.stringify(models.settings)}
        settings={models.settings}
        readiness={models.status.backend.readiness}
        message={models.status.backend.message}
        busy={busy || trainingActive}
        onSave={models.saveSettings}
        onValidate={models.validateBackend}
      />
      <TrainingConfigurationPanel
        configuration={trainingConfiguration}
        disabled={busy || trainingActive || !profileId || !selectedSnapshotId || !backendReady}
        onChange={setTrainingConfiguration}
        onStart={startTraining}
      />
      <TrainingJobPanel
        job={models.status.activeTrainingJob}
        logs={models.status.logs}
        busy={busy}
        onCancel={models.cancelTraining}
        onResume={(jobId) =>
          profileId ? models.resumeTraining(profileId, jobId) : Promise.resolve(null)
        }
        onDelete={models.deleteJob}
      />
      <div className="model-artifact-grid">
        <ModelArtifactList
          artifacts={models.status.artifacts}
          profileId={profileId}
          selectedId={selectedArtifact?.artifactId ?? ''}
          onSelect={setArtifactSelection}
        />
        <ModelArtifactDetails
          key={selectedArtifact?.artifactId ?? 'none'}
          artifact={selectedArtifact}
          busy={busy}
          onRename={models.renameArtifact}
          onReject={models.rejectArtifact}
          onDelete={(artifactId) => {
            setPendingArtifactDeletion(artifactId);
            return Promise.resolve(null);
          }}
        />
      </div>
      <ModelDeletionDialog
        open={pendingArtifactDeletion !== null}
        label="synthetic model"
        onCancel={() => setPendingArtifactDeletion(null)}
        onConfirm={() => {
          if (!pendingArtifactDeletion) return;
          void models
            .deleteArtifact(pendingArtifactDeletion)
            .finally(() => setPendingArtifactDeletion(null));
        }}
      />
      <OfflineConversionPanel
        status={models.status}
        artifact={selectedArtifact}
        hasVoiceLabSource={hasVoiceLabSource}
        busy={busy}
        onConvert={convert}
        onCancel={models.cancelConversion}
        onLoad={models.loadConversion}
        onClear={models.clearConversion}
      />
      <ModelEvaluationPanel
        artifact={selectedArtifact}
        status={models.status}
        phrases={models.evaluationPhrases}
        consentActive={consentActive}
        busy={busy}
        onSave={saveEvaluation}
        onApprove={() =>
          profileId && selectedArtifact
            ? models.approveArtifact(profileId, selectedArtifact.artifactId)
            : Promise.resolve(null)
        }
      />
      <div className="dataset-safety" role="status">
        No realtime model conversion was added. Use, Test, external routes, VB-CABLE, Discord, and
        CPAL live callbacks cannot select neural model output.
      </div>
    </div>
  );
}
