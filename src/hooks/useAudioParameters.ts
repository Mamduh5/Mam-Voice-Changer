import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';
import { ParameterSynchronizer } from './parameterSynchronizer';

export function useAudioParameters(enabled = true) {
  const [parameters, setParameters] = useState(defaultAudioParameters);
  const [confirmedParameters, setConfirmedParameters] = useState(defaultAudioParameters);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [persistenceError, setPersistenceError] = useState<string | null>(null);
  const [persistenceReady, setPersistenceReady] = useState(false);

  const [synchronizer] = useState(
    () =>
      new ParameterSynchronizer(defaultAudioParameters, {
        getParameters: tauriAudioApi.getParameters,
        setParameters: tauriAudioApi.setParameters,
        onStateChange: (state) => {
          setParameters(state.parameters);
          setConfirmedParameters(state.confirmedParameters);
          setSyncError(state.error);
        },
      }),
  );

  useEffect(() => {
    if (!enabled) {
      synchronizer.disconnect();
      return undefined;
    }

    let active = true;
    synchronizer.connect();
    void synchronizer.settle().then(() => {
      if (active) setPersistenceReady(true);
    });
    return () => {
      active = false;
      synchronizer.disconnect();
    };
  }, [enabled, synchronizer]);

  useEffect(() => {
    if (
      !enabled ||
      !persistenceReady ||
      syncError !== null ||
      JSON.stringify(parameters) !== JSON.stringify(confirmedParameters)
    ) {
      return undefined;
    }
    const timer = window.setTimeout(() => {
      void tauriAudioApi
        .persistAudioParameters({ ...confirmedParameters })
        .then(() => setPersistenceError(null))
        .catch((cause) =>
          setPersistenceError(`Unable to save audio settings for restart: ${String(cause)}`),
        );
    }, 500);
    return () => window.clearTimeout(timer);
  }, [confirmedParameters, enabled, parameters, persistenceReady, syncError]);

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
  const applySnapshot = useCallback(
    async (next: AudioParameters) => {
      if (!enabled || !synchronizer.update(next)) return false;
      await synchronizer.settle();
      const snapshot = synchronizer.snapshot();
      return (
        snapshot.error === null &&
        JSON.stringify(snapshot.confirmedParameters) === JSON.stringify(next)
      );
    },
    [enabled, synchronizer],
  );

  return {
    parameters,
    update,
    settle,
    beginPresetOperation,
    finishPresetOperation,
    applySnapshot,
    error: syncError ?? persistenceError,
  };
}
