import type { ActiveStreamFormat } from './audio';

export type EngineState = 'stopped' | 'starting' | 'running' | 'stopping' | 'error';

export type EngineStatus = {
  state: EngineState;
  inputLevel: number;
  outputLevel: number;
  inputOverruns: number;
  outputUnderruns: number;
  estimatedLatencyMs: number;
  activeStreamFormat: ActiveStreamFormat | null;
  lastRuntimeError: string | null;
  message: string;
};

export const stoppedStatus: EngineStatus = {
  state: 'stopped',
  inputLevel: 0,
  outputLevel: 0,
  inputOverruns: 0,
  outputUnderruns: 0,
  estimatedLatencyMs: 0,
  activeStreamFormat: null,
  lastRuntimeError: null,
  message: 'Ready to start',
};
