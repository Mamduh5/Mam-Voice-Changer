export type AudioParameters = {
  pitchSemitones: number;
  dryWet: number;
  gateEnabled: boolean;
  gateThresholdDb: number;
  inputGainDb: number;
  outputGainDb: number;
  masterCeilingDb: number;
  limiterEnabled: boolean;
  bypass: boolean;
  muted: boolean;
};

export const defaultAudioParameters: AudioParameters = {
  pitchSemitones: 0,
  dryWet: 0.35,
  gateEnabled: false,
  gateThresholdDb: -50,
  inputGainDb: 0,
  outputGainDb: -6,
  masterCeilingDb: -3,
  limiterEnabled: true,
  bypass: false,
  muted: false,
};

