export type AudioDevice = {
  id: string;
  name: string;
  isDefault: boolean;
};

export type AudioDeviceList = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
};

export type ActiveStreamFormat = {
  sampleRate: number;
  inputChannels: number;
  outputChannels: number;
  inputSampleFormat: string;
  outputSampleFormat: string;
  inputBufferFrames: number;
  outputBufferFrames: number;
};
