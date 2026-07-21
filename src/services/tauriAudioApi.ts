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
  renamePreset: (id: string, name: string) =>
    invokeDesktop<PresetCatalog>('rename_preset', { id, name }),
  duplicatePreset: (id: string) => invokeDesktop<PresetCatalog>('duplicate_preset', { id }),
  deletePreset: (id: string) => invokeDesktop<PresetCatalog>('delete_preset', { id }),
  applyPreset: (id: string) => invokeDesktop<PresetCatalog>('apply_preset', { id }),
  resetPreset: () => invokeDesktop<PresetCatalog>('reset_preset'),
};
