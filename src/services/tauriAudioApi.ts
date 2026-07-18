import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AudioDeviceList } from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';

export type StartEngineRequest = {
  inputId: string;
  outputId: string;
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
  startEngine: (request: StartEngineRequest) => invokeDesktop<void>('start_engine', { request }),
  stopEngine: () => invokeDesktop<void>('stop_engine'),
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
