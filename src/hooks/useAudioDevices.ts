import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioDevice } from '../types/audio';
import { reconcileSelection } from '../utils/deviceSelection';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export function useAudioDevices(enabled = true) {
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [inputId, setInputId] = useState('');
  const [outputId, setOutputId] = useState('');
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef(false);
  const inputIdRef = useRef('');
  const outputIdRef = useRef('');
  const refreshRevisionRef = useRef(0);
  const pendingSaveRef = useRef<{ inputId: string; outputId: string } | null>(null);
  const savingRef = useRef<Promise<void> | null>(null);

  const flushSelection = useCallback(async () => {
    while (pendingSaveRef.current || savingRef.current) {
      if (!savingRef.current) {
        const save = async () => {
          while (pendingSaveRef.current) {
            const selection = pendingSaveRef.current;
            pendingSaveRef.current = null;
            try {
              await tauriAudioApi.saveAudioDeviceSelection(selection.inputId, selection.outputId);
              if (activeRef.current && !pendingSaveRef.current) {
                setError(null);
              }
            } catch (cause) {
              if (activeRef.current && !pendingSaveRef.current) {
                setError(`Could not save the selected audio devices: ${errorMessage(cause)}`);
              }
            }
          }
        };
        savingRef.current = save();
      }

      const saving = savingRef.current;
      await saving;
      if (savingRef.current === saving) {
        savingRef.current = null;
      }
    }
  }, []);

  const queueSelection = useCallback(
    (nextInputId: string, nextOutputId: string) => {
      if (!enabled || !nextInputId || !nextOutputId) {
        return;
      }
      pendingSaveRef.current = { inputId: nextInputId, outputId: nextOutputId };
      void flushSelection();
    },
    [enabled, flushSelection],
  );

  const refresh = useCallback(async () => {
    if (!enabled) {
      return;
    }

    const revision = ++refreshRevisionRef.current;
    setLoading(true);
    try {
      while (pendingSaveRef.current || savingRef.current) {
        await flushSelection();
      }
      const devices = await tauriAudioApi.listAudioDevices();
      if (activeRef.current && revision === refreshRevisionRef.current) {
        const nextInputId = reconcileSelection(devices.selectedInputId ?? '', devices.inputs);
        const nextOutputId = reconcileSelection(
          devices.selectedOutputId ?? '',
          devices.outputs,
          true,
        );
        inputIdRef.current = nextInputId;
        outputIdRef.current = nextOutputId;
        setInputs(devices.inputs);
        setOutputs(devices.outputs);
        setInputId(nextInputId);
        setOutputId(nextOutputId);
        setError(devices.restorationWarning);
      }
    } catch (cause) {
      if (activeRef.current && revision === refreshRevisionRef.current) {
        setError(`Could not refresh audio devices: ${errorMessage(cause)}`);
      }
    } finally {
      if (activeRef.current && revision === refreshRevisionRef.current) {
        setLoading(false);
      }
    }
  }, [enabled, flushSelection]);

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    activeRef.current = true;
    const initialRefresh = window.setTimeout(() => void refresh(), 0);
    return () => {
      activeRef.current = false;
      refreshRevisionRef.current += 1;
      pendingSaveRef.current = null;
      window.clearTimeout(initialRefresh);
    };
  }, [enabled, refresh]);

  const selectInput = useCallback(
    (nextInputId: string) => {
      inputIdRef.current = nextInputId;
      setInputId(nextInputId);
      queueSelection(nextInputId, outputIdRef.current);
    },
    [queueSelection],
  );

  const selectOutput = useCallback(
    (nextOutputId: string) => {
      outputIdRef.current = nextOutputId;
      setOutputId(nextOutputId);
      queueSelection(inputIdRef.current, nextOutputId);
    },
    [queueSelection],
  );

  return {
    inputs,
    outputs,
    inputId,
    outputId,
    setInputId: selectInput,
    setOutputId: selectOutput,
    refresh,
    loading: enabled && loading,
    error,
  };
}
