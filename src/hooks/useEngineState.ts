import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { StartEngineRequest } from '../services/tauriAudioApi';
import { stoppedStatus, type EngineStatus } from '../types/engine';

export function useEngineState(enabled = true) {
  const [status, setStatus] = useState<EngineStatus>(stoppedStatus);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [pollError, setPollError] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const refreshPromiseRef = useRef<Promise<void> | null>(null);

  const refreshStatus = useCallback((): Promise<void> => {
    if (!enabled) {
      return Promise.resolve();
    }
    if (refreshPromiseRef.current) {
      return refreshPromiseRef.current;
    }

    const request = tauriAudioApi
      .getEngineStatus()
      .then((nextStatus) => {
        if (mountedRef.current) {
          setStatus(nextStatus);
          setPollError(null);
        }
      })
      .catch((cause) => {
        if (mountedRef.current) {
          setPollError(`Unable to refresh engine status: ${String(cause)}`);
        }
      })
      .finally(() => {
        if (refreshPromiseRef.current === request) {
          refreshPromiseRef.current = null;
        }
      });
    refreshPromiseRef.current = request;
    return request;
  }, [enabled]);

  useEffect(() => {
    mountedRef.current = enabled;
    if (!enabled) {
      return () => {
        mountedRef.current = false;
      };
    }

    const initialRefresh = window.setTimeout(() => void refreshStatus(), 0);
    const timer = window.setInterval(() => void refreshStatus(), 250);
    return () => {
      mountedRef.current = false;
      window.clearTimeout(initialRefresh);
      window.clearInterval(timer);
    };
  }, [enabled, refreshStatus]);

  const start = useCallback(
    async (request: StartEngineRequest) => {
      if (!enabled) {
        return;
      }

      setCommandError(null);
      try {
        await tauriAudioApi.startEngine(request);
      } catch (cause) {
        if (mountedRef.current) {
          setCommandError(`Unable to start the audio engine: ${String(cause)}`);
        }
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
      if (mountedRef.current) {
        setCommandError(`Unable to stop the audio engine: ${String(cause)}`);
      }
    } finally {
      await refreshStatus();
    }
  }, [enabled, refreshStatus]);

  return { status, commandError, pollError, start, stop };
}
