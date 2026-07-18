import { invoke } from '@tauri-apps/api/core';
import type { AudioDeviceList } from '../types/audio';
import type { EngineStatus } from '../types/engine';

export type StartEngineRequest = {
  inputId: string;
  outputId: string;
};

export const tauriAudioApi = {
  listAudioDevices: () => invoke<AudioDeviceList>('list_audio_devices'),
  startEngine: (request: StartEngineRequest) =>
    invoke<void>('start_engine', { request }),
  stopEngine: () => invoke<void>('stop_engine'),
  getEngineStatus: () => invoke<EngineStatus>('get_engine_status'),
};
