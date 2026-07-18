import { useCallback, useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioDevice } from '../types/audio';

function preferredDevice(devices: AudioDevice[], preferCable = false): string {
  if (preferCable) {
    const cable = devices.find((device) =>
      device.name.toLowerCase().includes('cable input'),
    );
    if (cable) return cable.id;
  }
  return devices.find((device) => device.isDefault)?.id ?? devices[0]?.id ?? '';
}

function reconcileSelection(
  selected: string,
  devices: AudioDevice[],
  preferCable = false,
): string {
  return devices.some((device) => device.id === selected)
    ? selected
    : preferredDevice(devices, preferCable);
}

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
    void refresh();
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
