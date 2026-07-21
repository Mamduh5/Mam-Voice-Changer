import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export function usePresets(
  enabled: boolean,
  beginMutation: () => Promise<AudioParameters>,
  finishMutation: (parameters?: AudioParameters) => void,
) {
  const [catalog, setCatalog] = useState<PresetCatalog | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busyRef = useRef(false);
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    if (!enabled) {
      return () => {
        mountedRef.current = false;
      };
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
          setError(`Unable to load presets: ${errorMessage(cause)}`);
        }
      }
    };
    void load();

    return () => {
      active = false;
      mountedRef.current = false;
    };
  }, [enabled]);

  const execute = useCallback(
    async (
      label: string,
      operation: (parameters: AudioParameters) => Promise<PresetCatalog>,
      appliesParameters: boolean,
    ) => {
      if (!enabled || busyRef.current) {
        return false;
      }

      busyRef.current = true;
      setBusy(true);
      let mutationActive = false;
      try {
        const synchronizedParameters = await beginMutation();
        mutationActive = true;
        if (!mountedRef.current) {
          return false;
        }

        const next = await operation(synchronizedParameters);
        finishMutation(appliesParameters ? next.activeParameters : undefined);
        mutationActive = false;
        if (!mountedRef.current) {
          return false;
        }

        setCatalog(next);
        setError(null);
        return true;
      } catch (cause) {
        if (mountedRef.current) {
          setError(`${label} failed: ${errorMessage(cause)}`);
        }
        return false;
      } finally {
        if (mutationActive) {
          finishMutation();
        }
        busyRef.current = false;
        if (mountedRef.current) {
          setBusy(false);
        }
      }
    },
    [beginMutation, enabled, finishMutation],
  );

  const save = useCallback(
    (name: string, parameters: AudioParameters) => {
      void parameters;
      return execute(
        'Save preset',
        (synchronizedParameters) => tauriAudioApi.savePreset(name, synchronizedParameters),
        true,
      );
    },
    [execute],
  );

  const rename = useCallback(
    (id: string, name: string) =>
      execute('Rename preset', () => tauriAudioApi.renamePreset(id, name), false),
    [execute],
  );

  const saveVoiceLab = useCallback(
    (name: string, parameters: AudioParameters) =>
      execute(
        'Save Voice Lab preset',
        () => tauriAudioApi.saveVoiceLabPreset(name, parameters),
        false,
      ),
    [execute],
  );

  const duplicate = useCallback(
    (id: string) => execute('Duplicate preset', () => tauriAudioApi.duplicatePreset(id), true),
    [execute],
  );

  const remove = useCallback(
    (id: string) => execute('Delete preset', () => tauriAudioApi.deletePreset(id), true),
    [execute],
  );

  const apply = useCallback(
    (id: string) => execute('Apply preset', () => tauriAudioApi.applyPreset(id), true),
    [execute],
  );

  const reset = useCallback(
    () => execute('Reset preset', () => tauriAudioApi.resetPreset(), true),
    [execute],
  );

  return { catalog, busy, error, save, saveVoiceLab, rename, duplicate, remove, apply, reset };
}
