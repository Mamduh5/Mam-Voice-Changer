import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';

export function useAudioParameters(enabled = true) {
  const [parameters, setParameters] = useState(defaultAudioParameters);
  const [error, setError] = useState<string | null>(null);
  const currentRef = useRef(defaultAudioParameters);
  const pendingRef = useRef<AudioParameters | null>(null);
  const sendingRef = useRef(false);
  const waitersRef = useRef<Array<() => void>>([]);

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    let active = true;
    const load = async () => {
      try {
        const current = await tauriAudioApi.getParameters();
        if (active) {
          currentRef.current = current;
          setParameters(current);
          setError(null);
        }
      } catch (cause) {
        if (active) {
          setError(String(cause));
        }
      }
    };
    void load();
    return () => {
      active = false;
    };
  }, [enabled]);

  const flush = useCallback(async () => {
    if (sendingRef.current) {
      await new Promise<void>((resolve) => {
        waitersRef.current.push(resolve);
      });
      return;
    }

    sendingRef.current = true;
    try {
      while (pendingRef.current) {
        const next = pendingRef.current;
        pendingRef.current = null;
        try {
          await tauriAudioApi.setParameters(next);
          setError(null);
        } catch (cause) {
          setError(String(cause));
        }
      }
    } finally {
      sendingRef.current = false;
      const waiters = waitersRef.current.splice(0);
      waiters.forEach((resolve) => resolve());
    }
  }, []);

  const settle = useCallback(async () => {
    while (pendingRef.current || sendingRef.current) {
      await flush();
    }
  }, [flush]);

  const update = useCallback(
    (changes: Partial<AudioParameters>) => {
      if (!enabled) {
        return;
      }

      const next = { ...currentRef.current, ...changes };
      currentRef.current = next;
      pendingRef.current = next;
      setParameters(next);
      void flush();
    },
    [enabled, flush],
  );

  const replace = useCallback((next: AudioParameters) => {
    pendingRef.current = null;
    currentRef.current = next;
    setParameters(next);
    setError(null);
  }, []);

  return { parameters, update, replace, settle, error };
}
