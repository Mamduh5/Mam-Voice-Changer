import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { emptyBackendSettings, type ModelBackendSettings } from '../types/modelBackend';
import type {
  BackendCompatibilityProfile,
  ManualListeningQualification,
} from '../types/modelBackend';
import type { TrainingConfiguration } from '../types/trainingJob';
import {
  emptyVoiceModelStatus,
  type CreateTrainingSnapshotRequest,
  type EvaluationPhrase,
  type InferenceConfiguration,
  type ModelEvaluationSummary,
  type VoiceModelStatus,
} from '../types/voiceModel';

function errorMessage(cause: unknown) {
  if (cause && typeof cause === 'object' && 'message' in cause) return String(cause.message);
  return cause instanceof Error ? cause.message : String(cause);
}

export function useVoiceModels(enabled: boolean) {
  const [status, setStatus] = useState<VoiceModelStatus>(emptyVoiceModelStatus);
  const [settings, setSettings] = useState<ModelBackendSettings>(emptyBackendSettings);
  const [evaluationPhrases, setEvaluationPhrases] = useState<EvaluationPhrase[]>([]);
  const [compatibilityProfiles, setCompatibilityProfiles] = useState<BackendCompatibilityProfile[]>(
    [],
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = useRef(false);

  const refresh = useCallback(async () => {
    const next = await tauriAudioApi.getVoiceModelStatus();
    if (active.current) {
      setStatus(next);
      setError(next.lastError?.message ?? null);
    }
    return next;
  }, []);

  useEffect(() => {
    active.current = enabled;
    if (!enabled) return undefined;
    let cancelled = false;
    void Promise.all([
      tauriAudioApi.readModelBackendConfiguration(),
      tauriAudioApi.listVoiceModelEvaluationPhrases(),
      tauriAudioApi.listBackendCompatibilityProfiles(),
      refresh(),
    ])
      .then(([configuration, phrases, profiles]) => {
        if (!cancelled) {
          setSettings(configuration);
          setEvaluationPhrases(phrases);
          setCompatibilityProfiles(profiles);
        }
      })
      .catch((cause: unknown) => !cancelled && setError(errorMessage(cause)));
    const timer = window.setInterval(() => void refresh().catch(() => undefined), 750);
    return () => {
      cancelled = true;
      active.current = false;
      window.clearInterval(timer);
    };
  }, [enabled, refresh]);

  const run = useCallback(
    async <T>(operation: () => Promise<T>) => {
      setBusy(true);
      try {
        const result = await operation();
        if (active.current) {
          setError(null);
          await refresh();
        }
        return result;
      } catch (cause) {
        if (active.current) setError(errorMessage(cause));
        return null;
      } finally {
        if (active.current) setBusy(false);
      }
    },
    [refresh],
  );

  const saveSettings = useCallback(
    async (next: ModelBackendSettings) => {
      const saved = await run(() => tauriAudioApi.saveModelBackendConfiguration(next));
      if (saved && active.current) setSettings(saved);
      return Boolean(saved);
    },
    [run],
  );

  return {
    status,
    settings,
    evaluationPhrases,
    compatibilityProfiles,
    busy,
    error,
    refresh,
    saveSettings,
    validateBackend: () => run(tauriAudioApi.validateModelBackend),
    repairIndexes: () => run(tauriAudioApi.repairVoiceModelIndexes),
    runQualification: (profileId: string | null, referenceTakeId: string | null) =>
      run(() => tauriAudioApi.runBackendQualification(profileId, referenceTakeId)),
    loadQualificationSmoke: () => run(tauriAudioApi.loadQualificationSmokeIntoVoiceLab),
    cancelQualification: () => run(tauriAudioApi.cancelBackendQualification),
    confirmManualListening: (confirmation: ManualListeningQualification) =>
      run(() => tauriAudioApi.confirmBackendManualListening(confirmation)),
    saveQualificationReport: (destination: string, humanReadable: boolean) =>
      run(() => tauriAudioApi.saveBackendQualificationReport(destination, humanReadable)),
    createSnapshot: (request: CreateTrainingSnapshotRequest) =>
      run(() => tauriAudioApi.createTrainingSnapshot(request)),
    deleteSnapshot: (snapshotId: string) =>
      run(() => tauriAudioApi.deleteTrainingSnapshot(snapshotId)),
    createTrainingPreflight: (
      profileId: string,
      snapshotId: string,
      configuration: TrainingConfiguration,
    ) => run(() => tauriAudioApi.createTrainingPreflight(profileId, snapshotId, configuration)),
    startTraining: (
      profileId: string,
      snapshotId: string,
      configuration: TrainingConfiguration,
      warningsAcknowledged: boolean,
    ) =>
      run(() =>
        tauriAudioApi.startVoiceModelTraining(
          profileId,
          snapshotId,
          configuration,
          warningsAcknowledged,
        ),
      ),
    cancelTraining: () => run(tauriAudioApi.cancelVoiceModelTraining),
    resumeTraining: (profileId: string, jobId: string) =>
      run(() => tauriAudioApi.resumeVoiceModelTraining(profileId, jobId)),
    deleteJob: (jobId: string) => run(() => tauriAudioApi.deleteTrainingJob(jobId)),
    renameArtifact: (artifactId: string, displayName: string) =>
      run(() => tauriAudioApi.renameVoiceModelArtifact(artifactId, displayName)),
    approveArtifact: (profileId: string, artifactId: string) =>
      run(() => tauriAudioApi.approveVoiceModelArtifact(profileId, artifactId)),
    rejectArtifact: (artifactId: string, notes: string | null) =>
      run(() => tauriAudioApi.rejectVoiceModelArtifact(artifactId, notes)),
    deleteArtifact: (artifactId: string) =>
      run(() => tauriAudioApi.deleteVoiceModelArtifact(artifactId)),
    exportArtifact: (artifactId: string, destination: string, licensingAcknowledged: boolean) =>
      run(() =>
        tauriAudioApi.exportVoiceModelPackage(artifactId, destination, licensingAcknowledged),
      ),
    importArtifact: (request: {
      packagePath: string;
      profileId: string;
      activeConsentVersion: string;
      associationConfirmed: boolean;
    }) => run(() => tauriAudioApi.importVoiceModelPackage(request)),
    startConversion: (
      profileId: string,
      artifactId: string,
      configuration: InferenceConfiguration,
    ) => run(() => tauriAudioApi.startOfflineVoiceConversion(profileId, artifactId, configuration)),
    startEvaluationConversion: (
      profileId: string,
      artifactId: string,
      configuration: InferenceConfiguration,
    ) =>
      run(() => tauriAudioApi.startModelEvaluationConversion(profileId, artifactId, configuration)),
    cancelConversion: () => run(tauriAudioApi.cancelOfflineVoiceConversion),
    loadConversion: (resultId: string) =>
      run(() => tauriAudioApi.loadOfflineConversionIntoVoiceLab(resultId)),
    clearConversion: () => run(tauriAudioApi.clearOfflineConversionResult),
    saveEvaluation: (profileId: string, artifactId: string, evaluation: ModelEvaluationSummary) =>
      run(() => tauriAudioApi.saveModelEvaluationRatings(profileId, artifactId, evaluation)),
  };
}
