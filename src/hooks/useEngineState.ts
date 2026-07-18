import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { stoppedStatus, type EngineStatus } from '../types/engine';

export function useEngineState() {
  const [status, setStatus] = useState<EngineStatus>(stoppedStatus);
  const [commandError, setCommandError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await tauriAudioApi.getEngineStatus());
    } catch (cause) {
      setCommandError(String(cause));
    }
  }, []);

  useEffect(() => {
    const initialRefresh = window.setTimeout(() => void refreshStatus(), 0);
    const timer = window.setInterval(() => void refreshStatus(), 250);
    return () => {
      window.clearTimeout(initialRefresh);
      window.clearInterval(timer);
    };
  }, [refreshStatus]);

  const start = useCallback(
    async (inputId: string, outputId: string) => {
      setCommandError(null);
      try {
        await tauriAudioApi.startEngine({ inputId, outputId });
      } catch (cause) {
        setCommandError(String(cause));
      } finally {
        await refreshStatus();
      }
    },
    [refreshStatus],
  );

  const stop = useCallback(async () => {
    setCommandError(null);
    try {
      await tauriAudioApi.stopEngine();
    } catch (cause) {
      setCommandError(String(cause));
    } finally {
      await refreshStatus();
    }
  }, [refreshStatus]);

  return { status, commandError, start, stop };
}
