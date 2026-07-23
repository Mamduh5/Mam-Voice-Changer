import type { VoiceModelStatus } from '../types/voiceModel';

export function useOfflineConversion(status: VoiceModelStatus) {
  return {
    active: status.activeInference,
    result: status.latestConversion,
    synthetic: status.latestConversion?.synthetic === true,
  };
}
