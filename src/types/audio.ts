export type ReliabilityProfile = 'lowLatency' | 'balanced' | 'reliable';
export type ApplicationPage = 'use' | 'test' | 'diagnostics';
export type AudioEndpointDirection = 'input' | 'output';
export type PairingConfidence = 'exact' | 'high' | 'manual' | 'ambiguous' | 'unpaired';
export type PairingSource = 'knownPattern' | 'normalizedName' | 'vendorFamily' | 'manual' | 'none';
export type RouteValidationStatus =
  | 'ready'
  | 'missingCapture'
  | 'ambiguousPair'
  | 'incompatibleFormat'
  | 'physicalConfirmationRequired';
export type RouteReadiness =
  | 'ready'
  | 'missingInput'
  | 'missingPlayback'
  | 'missingCapture'
  | 'ambiguousPair'
  | 'incompatibleFormat'
  | 'deviceUnavailable'
  | 'engineActive';

export type AudioDevice = {
  id: string;
  name: string;
  direction: AudioEndpointDirection;
  isDefault: boolean;
  isLikelyVirtual: boolean;
  virtualFamily: string | null;
  minimumSampleRate: number | null;
  maximumSampleRate: number | null;
  commonSampleRates: number[];
  channelCounts: number[];
};

export type RouteCompatibilityDetails = {
  commonVirtualSampleRates: number[];
  details: string;
};

export type ExternalAudioRoute = {
  routeId: string;
  displayName: string;
  playbackDevice: AudioDevice;
  captureDevice: AudioDevice | null;
  candidateCaptureDevices: AudioDevice[];
  pairingConfidence: PairingConfidence;
  pairingSource: PairingSource;
  validationStatus: RouteValidationStatus;
  compatibility: RouteCompatibilityDetails;
  manual: boolean;
};

export type ExternalAudioRouteCatalog = {
  routes: ExternalAudioRoute[];
  virtualPlaybackDevices: AudioDevice[];
  virtualCaptureDevices: AudioDevice[];
  unpairedCaptureDevices: AudioDevice[];
  selectedRouteId: string | null;
  restorationWarning: string | null;
};

export type RouteCompatibilityResult = {
  routeId: string | null;
  readiness: RouteReadiness;
  message: string;
  negotiatedSampleRate: number | null;
  captureEndpointAvailable: boolean;
};

export type AudioDeviceList = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  selectedInputId: string | null;
  selectedExternalRouteId: string | null;
  externalRoutePlaybackId: string | null;
  externalRouteCaptureId: string | null;
  localMonitorId: string | null;
  reliabilityProfile: ReliabilityProfile;
  lastPage: ApplicationPage;
  hasLikelyVirtualDestination: boolean;
  restorationWarning: string | null;
};

export type ApplicationSettingsUpdate = {
  selectedInputId: string | null;
  localMonitorId: string | null;
  reliabilityProfile: ReliabilityProfile;
  lastPage: ApplicationPage;
};

export type SaveExternalAudioRouteRequest = {
  candidateRouteId: string | null;
  playbackDeviceId: string;
  captureDeviceId: string;
  confirmPhysicalEndpoints: boolean;
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
