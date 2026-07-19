import type { ActiveStreamFormat, ReliabilityProfile } from './audio';

export type EngineState =
  'stopped' | 'starting' | 'running' | 'degraded' | 'recovering' | 'stopping' | 'error';

export type EngineStatus = {
  state: EngineState;
  inputLevel: number;
  outputLevel: number;
  monitorLevel: number;
  reliabilityProfile: ReliabilityProfile;
  inputCallbackGaps: number;
  inputRingOverflows: number;
  expanderAttenuatedFrames: number;
  dspInputUnderruns: number;
  dspProcessingDeadlineMisses: number;
  destinationRingOverflows: number;
  monitorRingOverflows: number;
  outputRingUnderruns: number;
  monitorOutputUnderruns: number;
  outputCallbackGaps: number;
  monitorCallbackGaps: number;
  concealedFrames: number;
  monitorConcealedFrames: number;
  streamRestartCount: number;
  currentInputRingFillFrames: number;
  minimumInputRingFillFrames: number;
  maximumInputRingFillFrames: number;
  currentOutputRingFillFrames: number;
  maximumOutputRingFillFrames: number;
  currentMonitorRingFillFrames: number;
  maximumMonitorRingFillFrames: number;
  maximumDspProcessingTimeMs: number;
  startupPrefillTargetFrames: number;
  startupPrefillAchievedFrames: number;
  startupPrefillTimedOut: boolean;
  clockDriftCorrectionRatio: number;
  minimumClockDriftCorrectionRatio: number;
  maximumClockDriftCorrectionRatio: number;
  estimatedLatencyMs: number;
  dspProcessingLatencyMs: number;
  totalEstimatedLatencyMs: number;
  activeStreamFormat: ActiveStreamFormat | null;
  lastRuntimeError: string | null;
  message: string;
};

export const stoppedStatus: EngineStatus = {
  state: 'stopped',
  inputLevel: 0,
  outputLevel: 0,
  monitorLevel: 0,
  reliabilityProfile: 'balanced',
  inputCallbackGaps: 0,
  inputRingOverflows: 0,
  expanderAttenuatedFrames: 0,
  dspInputUnderruns: 0,
  dspProcessingDeadlineMisses: 0,
  destinationRingOverflows: 0,
  monitorRingOverflows: 0,
  outputRingUnderruns: 0,
  monitorOutputUnderruns: 0,
  outputCallbackGaps: 0,
  monitorCallbackGaps: 0,
  concealedFrames: 0,
  monitorConcealedFrames: 0,
  streamRestartCount: 0,
  currentInputRingFillFrames: 0,
  minimumInputRingFillFrames: 0,
  maximumInputRingFillFrames: 0,
  currentOutputRingFillFrames: 0,
  maximumOutputRingFillFrames: 0,
  currentMonitorRingFillFrames: 0,
  maximumMonitorRingFillFrames: 0,
  maximumDspProcessingTimeMs: 0,
  startupPrefillTargetFrames: 0,
  startupPrefillAchievedFrames: 0,
  startupPrefillTimedOut: false,
  clockDriftCorrectionRatio: 1,
  minimumClockDriftCorrectionRatio: 1,
  maximumClockDriftCorrectionRatio: 1,
  estimatedLatencyMs: 0,
  dspProcessingLatencyMs: 0,
  totalEstimatedLatencyMs: 0,
  activeStreamFormat: null,
  lastRuntimeError: null,
  message: 'Ready to start',
};
