import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioDevice } from '../types/audio';
import { reconcileSelection } from '../utils/deviceSelection';

export function useAudioDevices() {
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [inputId, setInputId] = useState('');
  const [outputId, setOutputId] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const devices = await tauriAudioApi.listAudioDevices();
      setInputs(devices.inputs);
      setOutputs(devices.outputs);
      setInputId((current) => reconcileSelection(current, devices.inputs));
      setOutputId((current) => reconcileSelection(current, devices.outputs, true));
      setError(null);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const initialRefresh = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(initialRefresh);
  }, [refresh]);

  return {
    inputs,
    outputs,
    inputId,
    outputId,
    setInputId,
    setOutputId,
    refresh,
    loading,
    error,
  };
}
