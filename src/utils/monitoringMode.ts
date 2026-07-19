import type { EngineState } from '../types/engine';

export function shouldStopTemporaryTestMonitoring(
  currentPage: string,
  nextPage: string,
  engineMode: string | null,
  engineState: EngineState,
) {
  return (
    currentPage === 'test' &&
    nextPage !== 'test' &&
    engineMode === 'test' &&
    !['stopped', 'error'].includes(engineState)
  );
}
