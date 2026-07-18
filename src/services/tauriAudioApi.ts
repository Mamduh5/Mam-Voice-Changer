import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AudioDeviceList } from '../types/audio';
import type { EngineStatus } from '../types/engine';

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
};

