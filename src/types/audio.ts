export type ReliabilityProfile = 'lowLatency' | 'balanced' | 'reliable';
export type ApplicationPage = 'use' | 'test' | 'diagnostics';

export type AudioDevice = {
  id: string;
  name: string;
  isDefault: boolean;
  isLikelyVirtual: boolean;
};

export type AudioDeviceList = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  selectedInputId: string | null;
  processedDestinationId: string | null;
  localMonitorId: string | null;
  localMonitorEnabled: boolean;
  reliabilityProfile: ReliabilityProfile;
  lastPage: ApplicationPage;
  hasLikelyVirtualDestination: boolean;
  restorationWarning: string | null;
};

export type ApplicationSettingsUpdate = {
  selectedInputId: string | null;
  processedDestinationId: string | null;
  localMonitorId: string | null;
  localMonitorEnabled: boolean;
  reliabilityProfile: ReliabilityProfile;
  lastPage: ApplicationPage;
};

export type ActiveStreamFormat = {
  inputSampleRate: number;
  processedDestinationSampleRate: number | null;
  localMonitorSampleRate: number | null;
  inputChannels: number;
  processedDestinationChannels: number | null;
  localMonitorChannels: number | null;
  inputSampleFormat: string;
  processedDestinationSampleFormat: string | null;
  localMonitorSampleFormat: string | null;
  inputBufferFrames: number;
  processedDestinationBufferFrames: number | null;
  localMonitorBufferFrames: number | null;
  dspBlockFrames: number;
};
