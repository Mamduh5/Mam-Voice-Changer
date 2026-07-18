export type AudioParameters = {
  pitchSemitones: number;
  dryWet: number;
  inputGainDb: number;
  outputGainDb: number;
  limiterEnabled: boolean;
  bypass: boolean;
  muted: boolean;
};

export const defaultAudioParameters: AudioParameters = {
  pitchSemitones: 0,
  dryWet: 1,
  inputGainDb: 0,
  outputGainDb: 0,
  limiterEnabled: true,
  bypass: false,
  muted: false,
};
