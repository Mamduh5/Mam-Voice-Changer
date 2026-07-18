import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { stoppedStatus, type EngineStatus } from '../types/engine';

export function useEngineState(enabled = true) {
  const [status, setStatus] = useState<EngineStatus>(stoppedStatus);
  const [commandError, setCommandError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    if (!enabled) {
      return;
    }

    try {
      setStatus(await tauriAudioApi.getEngineStatus());
    } catch (cause) {
      setCommandError(String(cause));
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    const initialRefresh = window.setTimeout(() => void refreshStatus(), 0);
    const timer = window.setInterval(() => void refreshStatus(), 250);
    return () => {
      window.clearTimeout(initialRefresh);
      window.clearInterval(timer);
    };
  }, [enabled, refreshStatus]);

  const start = useCallback(
    async (inputId: string, outputId: string) => {
      if (!enabled) {
        return;
      }

      setCommandError(null);
      try {
        await tauriAudioApi.startEngine({ inputId, outputId });
      } catch (cause) {
        setCommandError(String(cause));
      } finally {
        await refreshStatus();
      }
    },
    [enabled, refreshStatus],
  );

  const stop = useCallback(async () => {
    if (!enabled) {
      return;
    }

    setCommandError(null);
    try {
      await tauriAudioApi.stopEngine();
    } catch (cause) {
      setCommandError(String(cause));
    } finally {
      await refreshStatus();
    }
  }, [enabled, refreshStatus]);

  return { status, commandError, start, stop };
}

