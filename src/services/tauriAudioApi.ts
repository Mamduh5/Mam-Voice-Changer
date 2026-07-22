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
};
