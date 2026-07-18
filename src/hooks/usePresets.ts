import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export function usePresets(
  enabled: boolean,
  beforeMutation: () => Promise<void>,
  onParametersApplied: (parameters: AudioParameters) => void,
) {
  const [catalog, setCatalog] = useState<PresetCatalog | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busyRef = useRef(false);

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    let active = true;
    const load = async () => {
      try {
        const next = await tauriAudioApi.listPresets();
        if (active) {
          setCatalog(next);
          setError(null);
        }
      } catch (cause) {
        if (active) {
          setError(errorMessage(cause));
        }
      }
    };
    void load();

    return () => {
      active = false;
    };
  }, [enabled]);

  const execute = useCallback(
    async (operation: () => Promise<PresetCatalog>, appliesParameters: boolean) => {
      if (!enabled || busyRef.current) {
        return false;
      }

      busyRef.current = true;
      setBusy(true);
      try {
        await beforeMutation();
        const next = await operation();
        setCatalog(next);
        if (appliesParameters) {
          onParametersApplied(next.activeParameters);
        }
        setError(null);
        return true;
      } catch (cause) {
        setError(errorMessage(cause));
        return false;
      } finally {
        busyRef.current = false;
        setBusy(false);
      }
    },
    [beforeMutation, enabled, onParametersApplied],
  );

  const save = useCallback(
    (name: string, parameters: AudioParameters) =>
      execute(() => tauriAudioApi.savePreset(name, parameters), true),
    [execute],
  );

  const rename = useCallback(
    (id: string, name: string) => execute(() => tauriAudioApi.renamePreset(id, name), false),
    [execute],
  );

  const duplicate = useCallback(
    (id: string) => execute(() => tauriAudioApi.duplicatePreset(id), true),
    [execute],
  );

  const remove = useCallback(
    (id: string) => execute(() => tauriAudioApi.deletePreset(id), true),
    [execute],
  );

  const apply = useCallback(
    (id: string) => execute(() => tauriAudioApi.applyPreset(id), true),
    [execute],
  );

  const reset = useCallback(() => execute(() => tauriAudioApi.resetPreset(), true), [execute]);

  return { catalog, busy, error, save, rename, duplicate, remove, apply, reset };
}
