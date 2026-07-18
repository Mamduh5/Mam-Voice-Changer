import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';

export function useAudioParameters(enabled = true) {
  const [parameters, setParameters] = useState(defaultAudioParameters);
  const [error, setError] = useState<string | null>(null);
  const currentRef = useRef(defaultAudioParameters);
  const pendingRef = useRef<AudioParameters | null>(null);
  const sendingRef = useRef(false);

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
      return;
    }

    sendingRef.current = true;
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
    sendingRef.current = false;
  }, []);

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

  return { parameters, update, error };
}
