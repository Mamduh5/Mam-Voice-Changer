export type AudioParameters = {
  pitchSemitones: number;
  formantShiftSemitones: number;
  dryWet: number;
  ageCharacter: number;
  breathiness: number;
  tremor: number;
  gateEnabled: boolean;
  gateThresholdDb: number;
  inputGainDb: number;
  outputGainDb: number;
  masterCeilingDb: number;
  warmthDb: number;
  brightnessDb: number;
  limiterEnabled: boolean;
  bypass: boolean;
  muted: boolean;
};

export const defaultAudioParameters: AudioParameters = {
  pitchSemitones: 0,
  formantShiftSemitones: 0,
  dryWet: 0.35,
  ageCharacter: 0,
  breathiness: 0,
  tremor: 0,
  gateEnabled: false,
  gateThresholdDb: -50,
  inputGainDb: 0,
  outputGainDb: -6,
  masterCeilingDb: -3,
  warmthDb: 0,
  brightnessDb: 0,
  limiterEnabled: true,
  bypass: false,
  muted: false,
};
