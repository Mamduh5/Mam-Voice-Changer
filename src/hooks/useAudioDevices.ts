import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type {
  ApplicationPage,
  ApplicationSettingsUpdate,
  AudioDevice,
  ReliabilityProfile,
} from '../types/audio';
import { preferredDevice, reconcileSelection } from '../utils/deviceSelection';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export const defaultApplicationSettings: ApplicationSettingsUpdate = {
  selectedInputId: null,
  processedDestinationId: null,
  localMonitorId: null,
  localMonitorEnabled: false,
  reliabilityProfile: 'balanced',
  lastPage: 'use',
};

export function useAudioDevices(enabled = true) {
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [settings, setSettings] = useState(defaultApplicationSettings);
  const [hasLikelyVirtualDestination, setHasLikelyVirtualDestination] = useState(false);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef(false);
  const settingsRef = useRef(defaultApplicationSettings);
  const refreshRevisionRef = useRef(0);
  const pendingSaveRef = useRef<ApplicationSettingsUpdate | null>(null);
  const savingRef = useRef<Promise<void> | null>(null);

  const flushSettings = useCallback(async () => {
    while (pendingSaveRef.current || savingRef.current) {
      if (!savingRef.current) {
        const save = async () => {
          while (pendingSaveRef.current) {
            const next = pendingSaveRef.current;
            pendingSaveRef.current = null;
            try {
              await tauriAudioApi.saveApplicationSettings(next);
              if (activeRef.current && !pendingSaveRef.current) setError(null);
            } catch (cause) {
              if (activeRef.current && !pendingSaveRef.current) {
                setError(`Could not save application settings: ${errorMessage(cause)}`);
              }
            }
          }
        };
        savingRef.current = save();
      }
      const saving = savingRef.current;
      await saving;
      if (savingRef.current === saving) savingRef.current = null;
    }
  }, []);

  const updateSettings = useCallback(
    (changes: Partial<ApplicationSettingsUpdate>) => {
      if (!enabled) return;
      const next = { ...settingsRef.current, ...changes };
      if (!next.localMonitorId) next.localMonitorEnabled = false;
      settingsRef.current = next;
      setSettings(next);
      pendingSaveRef.current = next;
      void flushSettings();
    },
    [enabled, flushSettings],
  );

  const refresh = useCallback(async () => {
    if (!enabled) return;
    const revision = ++refreshRevisionRef.current;
    setLoading(true);
    try {
      while (pendingSaveRef.current || savingRef.current) await flushSettings();
      const devices = await tauriAudioApi.listAudioDevices();
      if (activeRef.current && revision === refreshRevisionRef.current) {
        const inputId = reconcileSelection(devices.selectedInputId ?? '', devices.inputs);
        const destinationId = devices.outputs.some(
          (device) => device.id === devices.processedDestinationId,
        )
          ? (devices.processedDestinationId ?? '')
          : '';
        const monitorId = reconcileSelection(
          devices.localMonitorId ?? preferredDevice(devices.outputs),
          devices.outputs,
        );
        const next: ApplicationSettingsUpdate = {
          selectedInputId: inputId || null,
          processedDestinationId: destinationId || null,
          localMonitorId: monitorId || null,
          localMonitorEnabled: Boolean(devices.localMonitorEnabled && monitorId),
          reliabilityProfile: devices.reliabilityProfile,
          lastPage: devices.lastPage,
        };
        settingsRef.current = next;
        setSettings(next);
        setInputs(devices.inputs);
        setOutputs(devices.outputs);
        setHasLikelyVirtualDestination(devices.hasLikelyVirtualDestination);
        setError(devices.restorationWarning);
      }
    } catch (cause) {
      if (activeRef.current && revision === refreshRevisionRef.current) {
        setError(`Could not refresh audio devices: ${errorMessage(cause)}`);
      }
    } finally {
      if (activeRef.current && revision === refreshRevisionRef.current) setLoading(false);
    }
  }, [enabled, flushSettings]);

  useEffect(() => {
    if (!enabled) return undefined;
    activeRef.current = true;
    const initialRefresh = window.setTimeout(() => void refresh(), 0);
    return () => {
      activeRef.current = false;
      refreshRevisionRef.current += 1;
      pendingSaveRef.current = null;
      window.clearTimeout(initialRefresh);
    };
  }, [enabled, refresh]);

  const setInputId = useCallback(
    (id: string) => updateSettings({ selectedInputId: id || null }),
    [updateSettings],
  );
  const setProcessedDestinationId = useCallback(
    (id: string) => updateSettings({ processedDestinationId: id || null }),
    [updateSettings],
  );
  const setLocalMonitorId = useCallback(
    (id: string) => updateSettings({ localMonitorId: id || null }),
    [updateSettings],
  );
  const setLocalMonitorEnabled = useCallback(
    (enabled: boolean) => updateSettings({ localMonitorEnabled: enabled }),
    [updateSettings],
  );
  const setReliabilityProfile = useCallback(
    (profile: ReliabilityProfile) => updateSettings({ reliabilityProfile: profile }),
    [updateSettings],
  );
  const setLastPage = useCallback(
    (page: ApplicationPage) => updateSettings({ lastPage: page }),
    [updateSettings],
  );

  return {
    inputs,
    outputs,
    inputId: settings.selectedInputId ?? '',
    processedDestinationId: settings.processedDestinationId ?? '',
    localMonitorId: settings.localMonitorId ?? '',
    localMonitorEnabled: settings.localMonitorEnabled,
    reliabilityProfile: settings.reliabilityProfile,
    lastPage: settings.lastPage,
    hasLikelyVirtualDestination,
    setInputId,
    setProcessedDestinationId,
    setLocalMonitorId,
    setLocalMonitorEnabled,
    setReliabilityProfile,
    setLastPage,
    refresh,
    loading: enabled && loading,
    error,
  };
}
