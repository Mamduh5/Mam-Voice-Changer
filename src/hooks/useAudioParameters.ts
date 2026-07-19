import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';
import { ParameterSynchronizer } from './parameterSynchronizer';

export function useAudioParameters(enabled = true) {
  const [parameters, setParameters] = useState(defaultAudioParameters);
  const [error, setError] = useState<string | null>(null);
  const [synchronizer] = useState(
    () =>
      new ParameterSynchronizer(defaultAudioParameters, {
        getParameters: tauriAudioApi.getParameters,
        setParameters: tauriAudioApi.setParameters,
        onStateChange: (state) => {
          setParameters(state.parameters);
          setError(state.error);
        },
      }),
  );

  useEffect(() => {
    if (!enabled) {
      synchronizer.disconnect();
      return undefined;
    }

    synchronizer.connect();
    return () => {
      synchronizer.disconnect();
    };
  }, [enabled, synchronizer]);

  const update = useCallback(
    (changes: Partial<AudioParameters>) => {
      if (enabled) {
        synchronizer.update(changes);
      }
    },
    [enabled, synchronizer],
  );

  const settle = useCallback(() => synchronizer.settle(), [synchronizer]);
  const beginPresetOperation = useCallback(
    () => synchronizer.beginPresetOperation(),
    [synchronizer],
  );
  const finishPresetOperation = useCallback(
    (next?: AudioParameters) => synchronizer.finishPresetOperation(next),
    [synchronizer],
  );

  return {
    parameters,
    update,
    settle,
    beginPresetOperation,
    finishPresetOperation,
    error,
  };
}
