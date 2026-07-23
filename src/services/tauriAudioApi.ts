import { invoke, isTauri } from '@tauri-apps/api/core';
import type {
  ApplicationSettingsUpdate,
  AudioDeviceList,
  ExternalAudioRouteCatalog,
  ReliabilityProfile,
  RouteCompatibilityResult,
  SaveExternalAudioRouteRequest,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';
import type { VoiceLabClipVersion, VoiceLabStatus } from '../types/voiceLab';
import type {
  CreateVoiceProfileRequest,
  DatasetExportOptions,
  PromptPack,
  PromptSelection,
  ReviewTakeRequest,
  SelectedTakeVersion,
  UpdateVoiceProfileRequest,
  VoiceDatasetStatus,
  VoiceProfileSummary,
} from '../types/voiceDataset';
import type {
  BackendCompatibilityProfile,
  BackendValidationStatus,
  ManualListeningQualification,
  ModelBackendSettings,
  QualificationRun,
} from '../types/modelBackend';
import type {
  TrainingConfiguration,
  TrainingJob,
  TrainingPreflightReport,
} from '../types/trainingJob';
import type {
  CreateTrainingSnapshotRequest,
  EvaluationPhrase,
  InferenceConfiguration,
  ModelEvaluationSummary,
  OfflineConversionResult,
  TrainingSnapshot,
  VoiceModelArtifact,
  VoiceModelStatus,
} from '../types/voiceModel';

export type StartAudioRequest =
  | {
      mode: 'use';
      inputId: string;
      inputName: string;
      externalRouteId: string;
      reliabilityProfile: ReliabilityProfile;
    }
  | {
      mode: 'test';
      inputId: string;
      inputName: string;
      monitorId: string;
      monitorName: string;
      reliabilityProfile: ReliabilityProfile;
    };

export const DESKTOP_RUNTIME_UNAVAILABLE = 'Desktop runtime unavailable. Launch with npm run dev.';

function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    return Promise.reject(new Error(DESKTOP_RUNTIME_UNAVAILABLE));
  }

  return invoke<T>(command, args);
}

export const tauriAudioApi = {
  isDesktopRuntimeAvailable: isTauri,
  listAudioDevices: () => invokeDesktop<AudioDeviceList>('list_audio_devices'),
  saveApplicationSettings: (request: ApplicationSettingsUpdate) =>
    invokeDesktop<void>('save_application_settings', { request }),
  listExternalAudioRoutes: () =>
    invokeDesktop<ExternalAudioRouteCatalog>('list_external_audio_routes'),
  saveExternalAudioRoute: (request: SaveExternalAudioRouteRequest) =>
    invokeDesktop<ExternalAudioRouteCatalog>('save_external_audio_route', { request }),
  deleteExternalAudioRoute: () =>
    invokeDesktop<ExternalAudioRouteCatalog>('delete_external_audio_route'),
  validateExternalAudioRoute: (inputId: string, routeId: string) =>
    invokeDesktop<RouteCompatibilityResult>('validate_external_audio_route', {
      inputId,
      routeId,
    }),
  startEngine: (request: StartAudioRequest) => invokeDesktop<void>('start_engine', { request }),
  stopEngine: () => invokeDesktop<void>('stop_engine'),
  stopTestRoute: () => invokeDesktop<void>('stop_test_route'),
  getEngineStatus: () => invokeDesktop<EngineStatus>('get_engine_status'),
  getParameters: () => invokeDesktop<AudioParameters>('get_parameters'),
  setParameters: (parameters: AudioParameters) =>
    invokeDesktop<void>('set_parameters', { parameters }),
  listPresets: () => invokeDesktop<PresetCatalog>('list_presets'),
  savePreset: (name: string, parameters: AudioParameters) =>
    invokeDesktop<PresetCatalog>('save_preset', { name, parameters }),
  saveVoiceLabPreset: (name: string, parameters: AudioParameters) =>
    invokeDesktop<PresetCatalog>('save_voice_lab_preset', { name, parameters }),
  renamePreset: (id: string, name: string) =>
    invokeDesktop<PresetCatalog>('rename_preset', { id, name }),
  duplicatePreset: (id: string) => invokeDesktop<PresetCatalog>('duplicate_preset', { id }),
  deletePreset: (id: string) => invokeDesktop<PresetCatalog>('delete_preset', { id }),
  applyPreset: (id: string) => invokeDesktop<PresetCatalog>('apply_preset', { id }),
  resetPreset: () => invokeDesktop<PresetCatalog>('reset_preset'),
  getVoiceLabStatus: () => invokeDesktop<VoiceLabStatus>('get_voice_lab_status'),
  startVoiceLabCapture: (inputId: string, inputName: string) =>
    invokeDesktop<VoiceLabStatus>('start_voice_lab_capture', { inputId, inputName }),
  stopVoiceLabCapture: () => invokeDesktop<VoiceLabStatus>('stop_voice_lab_capture'),
  importVoiceLabWav: (path: string) =>
    invokeDesktop<VoiceLabStatus>('import_voice_lab_wav', { path }),
  renderVoiceLab: (parameters: AudioParameters) =>
    invokeDesktop<VoiceLabStatus>('render_voice_lab', { parameters }),
  startVoiceLabPreview: (
    version: VoiceLabClipVersion,
    outputId: string,
    outputName: string,
    looping: boolean,
  ) =>
    invokeDesktop<VoiceLabStatus>('start_voice_lab_preview', {
      version,
      outputId,
      outputName,
      looping,
    }),
  stopVoiceLabPreview: () => invokeDesktop<VoiceLabStatus>('stop_voice_lab_preview'),
  stopVoiceLabAudio: () => invokeDesktop<VoiceLabStatus>('stop_voice_lab_audio'),
  exportVoiceLabWav: (version: VoiceLabClipVersion, path: string) =>
    invokeDesktop<void>('export_voice_lab_wav', { version, path }),
  clearVoiceLab: () => invokeDesktop<VoiceLabStatus>('clear_voice_lab'),
  listVoiceProfiles: () => invokeDesktop<VoiceProfileSummary[]>('list_voice_profiles'),
  createVoiceProfile: (request: CreateVoiceProfileRequest) =>
    invokeDesktop<VoiceDatasetStatus>('create_voice_profile', { request }),
  readVoiceProfile: (profileId: string) =>
    invokeDesktop<VoiceDatasetStatus>('read_voice_profile', { profileId }),
  updateVoiceProfile: (profileId: string, request: UpdateVoiceProfileRequest) =>
    invokeDesktop<VoiceDatasetStatus>('update_voice_profile', { profileId, request }),
  deleteVoiceProfile: (profileId: string) =>
    invokeDesktop<VoiceDatasetStatus>('delete_voice_profile', { profileId }),
  getVoiceDatasetStatus: () => invokeDesktop<VoiceDatasetStatus>('get_voice_dataset_status'),
  listDatasetPrompts: () => invokeDesktop<PromptPack>('list_dataset_prompts'),
  selectDatasetPrompt: (selection: PromptSelection) =>
    invokeDesktop<VoiceDatasetStatus>('select_dataset_prompt', { selection }),
  startDatasetRecording: (inputId: string, inputName: string, recordedConsent = false) =>
    invokeDesktop<VoiceDatasetStatus>('start_dataset_recording', {
      inputId,
      inputName,
      recordedConsent,
    }),
  stopDatasetRecording: () => invokeDesktop<VoiceDatasetStatus>('stop_dataset_recording'),
  discardCurrentDatasetTake: () =>
    invokeDesktop<VoiceDatasetStatus>('discard_current_dataset_take'),
  importDatasetWavs: (paths: string[], selection: PromptSelection) =>
    invokeDesktop<VoiceDatasetStatus>('import_dataset_wavs', { paths, selection }),
  reviewDatasetTake: (profileId: string, takeId: string, request: ReviewTakeRequest) =>
    invokeDesktop<VoiceDatasetStatus>('review_dataset_take', { profileId, takeId, request }),
  setDatasetTrim: (takeId: string, startFrame: number, endFrame: number) =>
    invokeDesktop<VoiceDatasetStatus>('set_dataset_trim', { takeId, startFrame, endFrame }),
  autoTrimDatasetTake: (takeId: string) =>
    invokeDesktop<VoiceDatasetStatus>('auto_trim_dataset_take', { takeId }),
  applyDatasetTrim: () => invokeDesktop<VoiceDatasetStatus>('apply_dataset_trim'),
  resetDatasetTrim: (takeId: string) =>
    invokeDesktop<VoiceDatasetStatus>('reset_dataset_trim', { takeId }),
  previewDatasetTake: (
    takeId: string,
    version: SelectedTakeVersion,
    outputId: string,
    outputName: string,
    seekMs = 0,
  ) =>
    invokeDesktop<VoiceDatasetStatus>('preview_dataset_take', {
      takeId,
      version,
      outputId,
      outputName,
      seekMs,
    }),
  pauseDatasetPreview: () => invokeDesktop<VoiceDatasetStatus>('pause_dataset_preview'),
  stopDatasetPreview: () => invokeDesktop<VoiceDatasetStatus>('stop_dataset_preview'),
  deleteDatasetTake: (takeId: string) =>
    invokeDesktop<VoiceDatasetStatus>('delete_dataset_take', { takeId }),
  exportVoiceDataset: (destination: string, options: DatasetExportOptions) =>
    invokeDesktop<string>('export_voice_dataset', { destination, options }),
  repairVoiceProfile: (profileId: string) =>
    invokeDesktop<VoiceDatasetStatus>('repair_voice_profile', { profileId }),
  leaveVoiceDataset: () => invokeDesktop<void>('leave_voice_dataset'),
  clearVoiceDatasetError: () => invokeDesktop<VoiceDatasetStatus>('clear_voice_dataset_error'),
  readModelBackendConfiguration: () =>
    invokeDesktop<ModelBackendSettings>('read_model_backend_configuration'),
  listVoiceModelEvaluationPhrases: () =>
    invokeDesktop<EvaluationPhrase[]>('list_voice_model_evaluation_phrases'),
  listBackendCompatibilityProfiles: () =>
    invokeDesktop<BackendCompatibilityProfile[]>('list_backend_compatibility_profiles'),
  repairVoiceModelIndexes: () => invokeDesktop<unknown>('repair_voice_model_indexes'),
  runBackendQualification: (profileId: string | null, referenceTakeId: string | null) =>
    invokeDesktop<QualificationRun>('run_backend_qualification', {
      profileId,
      referenceTakeId,
    }),
  loadQualificationSmokeIntoVoiceLab: () =>
    invokeDesktop<VoiceLabStatus>('load_qualification_smoke_into_voice_lab'),
  cancelBackendQualification: () => invokeDesktop<void>('cancel_backend_qualification'),
  confirmBackendManualListening: (confirmation: ManualListeningQualification) =>
    invokeDesktop<QualificationRun>('confirm_backend_manual_listening', { confirmation }),
  saveBackendQualificationReport: (destination: string, humanReadable: boolean) =>
    invokeDesktop<void>('save_backend_qualification_report', { destination, humanReadable }),
  saveModelBackendConfiguration: (settings: ModelBackendSettings) =>
    invokeDesktop<ModelBackendSettings>('save_model_backend_configuration', { settings }),
  validateModelBackend: () => invokeDesktop<BackendValidationStatus>('validate_model_backend'),
  getVoiceModelStatus: () => invokeDesktop<VoiceModelStatus>('get_voice_model_status'),
  createTrainingSnapshot: (request: CreateTrainingSnapshotRequest) =>
    invokeDesktop<TrainingSnapshot>('create_training_snapshot', { request }),
  deleteTrainingSnapshot: (snapshotId: string) =>
    invokeDesktop<void>('delete_training_snapshot', { snapshotId }),
  startVoiceModelTraining: (
    profileId: string,
    snapshotId: string,
    configuration: TrainingConfiguration,
    warningsAcknowledged: boolean,
  ) =>
    invokeDesktop<TrainingJob>('start_voice_model_training', {
      profileId,
      snapshotId,
      configuration,
      warningsAcknowledged,
    }),
  createTrainingPreflight: (
    profileId: string,
    snapshotId: string,
    configuration: TrainingConfiguration,
  ) =>
    invokeDesktop<TrainingPreflightReport>('create_training_preflight', {
      profileId,
      snapshotId,
      configuration,
    }),
  cancelVoiceModelTraining: () => invokeDesktop<TrainingJob>('cancel_voice_model_training'),
  resumeVoiceModelTraining: (profileId: string, jobId: string) =>
    invokeDesktop<TrainingJob>('resume_voice_model_training', { profileId, jobId }),
  deleteTrainingJob: (jobId: string) => invokeDesktop<void>('delete_training_job', { jobId }),
  renameVoiceModelArtifact: (artifactId: string, displayName: string) =>
    invokeDesktop<VoiceModelArtifact>('rename_voice_model_artifact', { artifactId, displayName }),
  approveVoiceModelArtifact: (profileId: string, artifactId: string) =>
    invokeDesktop<VoiceModelArtifact>('approve_voice_model_artifact', { profileId, artifactId }),
  rejectVoiceModelArtifact: (artifactId: string, notes: string | null) =>
    invokeDesktop<VoiceModelArtifact>('reject_voice_model_artifact', { artifactId, notes }),
  deleteVoiceModelArtifact: (artifactId: string) =>
    invokeDesktop<void>('delete_voice_model_artifact', { artifactId }),
  exportVoiceModelPackage: (
    artifactId: string,
    destination: string,
    licensingAcknowledged: boolean,
  ) =>
    invokeDesktop<{
      packageId: string;
      outputFile: string;
      fileCount: number;
      totalBytes: number;
      portabilityStatus: VoiceModelArtifact['portabilityStatus'];
    }>('export_voice_model_package', { artifactId, destination, licensingAcknowledged }),
  importVoiceModelPackage: (request: {
    packagePath: string;
    profileId: string;
    activeConsentVersion: string;
    associationConfirmed: boolean;
  }) => invokeDesktop<VoiceModelArtifact>('import_voice_model_package', { request }),
  startOfflineVoiceConversion: (
    profileId: string,
    artifactId: string,
    configuration: InferenceConfiguration,
  ) =>
    invokeDesktop<void>('start_offline_voice_conversion', {
      profileId,
      artifactId,
      configuration,
    }),
  startModelEvaluationConversion: (
    profileId: string,
    artifactId: string,
    configuration: InferenceConfiguration,
  ) =>
    invokeDesktop<void>('start_model_evaluation_conversion', {
      profileId,
      artifactId,
      configuration,
    }),
  cancelOfflineVoiceConversion: () => invokeDesktop<void>('cancel_offline_voice_conversion'),
  readOfflineConversionResult: () =>
    invokeDesktop<OfflineConversionResult | null>('read_offline_conversion_result'),
  loadOfflineConversionIntoVoiceLab: (resultId: string) =>
    invokeDesktop<VoiceLabStatus>('load_offline_conversion_into_voice_lab', { resultId }),
  clearOfflineConversionResult: () => invokeDesktop<void>('clear_offline_conversion_result'),
  saveModelEvaluationRatings: (
    profileId: string,
    artifactId: string,
    evaluation: ModelEvaluationSummary,
  ) =>
    invokeDesktop<VoiceModelArtifact>('save_model_evaluation_ratings', {
      profileId,
      artifactId,
      evaluation,
    }),
  clearVoiceModelError: () => invokeDesktop<void>('clear_voice_model_error'),
  cancelModelWorkForShutdown: () => invokeDesktop<void>('cancel_model_work_for_shutdown'),
};
