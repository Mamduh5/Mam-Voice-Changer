export type AudioParameters = {
  inputGainDb: number;
  outputGainDb: number;
  limiterEnabled: boolean;
  bypass: boolean;
  muted: boolean;
};

export const defaultAudioParameters: AudioParameters = {
  inputGainDb: 0,
  outputGainDb: 0,
  limiterEnabled: true,
  bypass: false,
  muted: false,
};
