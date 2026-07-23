import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type {
  ApplicationPage,
  ApplicationSettingsUpdate,
  AudioDevice,
  ExternalAudioRouteCatalog,
  ReliabilityProfile,
  RouteCompatibilityResult,
} from '../types/audio';
import { preferredDevice, reconcileSelection } from '../utils/deviceSelection';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export const defaultApplicationSettings: ApplicationSettingsUpdate = {
  selectedInputId: null,
  localMonitorId: null,
  reliabilityProfile: 'balanced',
  lastPage: 'use',
};

export const emptyExternalRouteCatalog: ExternalAudioRouteCatalog = {
  routes: [],
  virtualPlaybackDevices: [],
  virtualCaptureDevices: [],
  unpairedCaptureDevices: [],
  selectedRouteId: null,
  restorationWarning: null,
};

const missingRouteValidation: RouteCompatibilityResult = {
  routeId: null,
  readiness: 'missingPlayback',
  message:
    'No virtual audio route is available. Install or enable a compatible Windows virtual audio device, then refresh devices.',
  negotiatedSampleRate: null,
  captureEndpointAvailable: false,
};

export function useAudioDevices(enabled = true) {
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [settings, setSettings] = useState(defaultApplicationSettings);
  const [externalRoutes, setExternalRoutes] = useState(emptyExternalRouteCatalog);
  const [routeValidation, setRouteValidation] = useState(missingRouteValidation);
  const [draftRouteId, setDraftRouteIdState] = useState('');
  const [draftPlaybackId, setDraftPlaybackId] = useState('');
  const [draftCaptureId, setDraftCaptureId] = useState('');
  const [confirmPhysicalEndpoints, setConfirmPhysicalEndpoints] = useState(false);
  const [routeBusy, setRouteBusy] = useState(false);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef(false);
  const settingsRef = useRef(defaultApplicationSettings);
  const routesRef = useRef(emptyExternalRouteCatalog);
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
      settingsRef.current = next;
      setSettings(next);
      pendingSaveRef.current = next;
      void flushSettings();
    },
    [enabled, flushSettings],
  );

  const validateRoute = useCallback(
    async (inputId: string, routeId: string) => {
      if (!enabled || !routeId) {
        if (activeRef.current) setRouteValidation(missingRouteValidation);
        return;
      }
      try {
        const result = await tauriAudioApi.validateExternalAudioRoute(inputId, routeId);
        if (activeRef.current) setRouteValidation(result);
      } catch (cause) {
        if (activeRef.current) {
          setRouteValidation({
            routeId,
            readiness: 'deviceUnavailable',
            message: `Could not validate the external route: ${errorMessage(cause)}`,
            negotiatedSampleRate: null,
            captureEndpointAvailable: false,
          });
        }
      }
    },
    [enabled],
  );

  const applyRouteCatalog = useCallback((catalog: ExternalAudioRouteCatalog) => {
    routesRef.current = catalog;
    setExternalRoutes(catalog);
    const selected = catalog.routes.find((route) => route.routeId === catalog.selectedRouteId);
    const draft = selected ?? catalog.routes[0];
    setDraftRouteIdState(draft?.routeId ?? '');
    setDraftPlaybackId(draft?.playbackDevice.id ?? catalog.virtualPlaybackDevices[0]?.id ?? '');
    setDraftCaptureId(draft?.captureDevice?.id ?? draft?.candidateCaptureDevices[0]?.id ?? '');
    setConfirmPhysicalEndpoints(false);
  }, []);

  const refresh = useCallback(async () => {
    if (!enabled) return;
    const revision = ++refreshRevisionRef.current;
    setLoading(true);
    try {
      while (pendingSaveRef.current || savingRef.current) await flushSettings();
      const [devices, routes] = await Promise.all([
        tauriAudioApi.listAudioDevices(),
        tauriAudioApi.listExternalAudioRoutes(),
      ]);
      if (activeRef.current && revision === refreshRevisionRef.current) {
        const physicalInputs = devices.inputs.filter((device) => !device.isLikelyVirtual);
        const inputId = reconcileSelection(devices.selectedInputId ?? '', physicalInputs);
        const monitorId = reconcileSelection(
          devices.localMonitorId ?? preferredDevice(devices.outputs),
          devices.outputs,
        );
        const next: ApplicationSettingsUpdate = {
          selectedInputId: inputId || null,
          localMonitorId: monitorId || null,
          reliabilityProfile: devices.reliabilityProfile,
          lastPage: devices.lastPage,
        };
        settingsRef.current = next;
        setSettings(next);
        setInputs(devices.inputs);
        setOutputs(devices.outputs);
        applyRouteCatalog(routes);
        setError(
          [devices.restorationWarning, routes.restorationWarning].filter(Boolean).join(' ') || null,
        );
        await validateRoute(inputId, routes.selectedRouteId ?? '');
      }
    } catch (cause) {
      if (activeRef.current && revision === refreshRevisionRef.current) {
        setError(`Could not refresh audio devices: ${errorMessage(cause)}`);
      }
    } finally {
      if (activeRef.current && revision === refreshRevisionRef.current) setLoading(false);
    }
  }, [applyRouteCatalog, enabled, flushSettings, validateRoute]);

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
    (id: string) => {
      updateSettings({ selectedInputId: id || null });
      void validateRoute(id, routesRef.current.selectedRouteId ?? '');
    },
    [updateSettings, validateRoute],
  );
  const setLocalMonitorId = useCallback(
    (id: string) => updateSettings({ localMonitorId: id || null }),
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
  const setDraftRouteId = useCallback((routeId: string) => {
    setDraftRouteIdState(routeId);
    const route = routesRef.current.routes.find((candidate) => candidate.routeId === routeId);
    if (route) {
      setDraftPlaybackId(route.playbackDevice.id);
      setDraftCaptureId(route.captureDevice?.id ?? route.candidateCaptureDevices[0]?.id ?? '');
      setConfirmPhysicalEndpoints(false);
    }
  }, []);

  const saveExternalRoute = useCallback(async () => {
    if (!enabled || !draftPlaybackId || !draftCaptureId) return false;
    setRouteBusy(true);
    try {
      const catalog = await tauriAudioApi.saveExternalAudioRoute({
        candidateRouteId: draftRouteId || null,
        playbackDeviceId: draftPlaybackId,
        captureDeviceId: draftCaptureId,
        confirmPhysicalEndpoints,
      });
      if (!activeRef.current) return false;
      applyRouteCatalog(catalog);
      setError(null);
      await validateRoute(settingsRef.current.selectedInputId ?? '', catalog.selectedRouteId ?? '');
      return true;
    } catch (cause) {
      if (activeRef.current) setError(`Could not save external route: ${errorMessage(cause)}`);
      return false;
    } finally {
      if (activeRef.current) setRouteBusy(false);
    }
  }, [
    applyRouteCatalog,
    confirmPhysicalEndpoints,
    draftCaptureId,
    draftRouteId,
    draftPlaybackId,
    enabled,
    validateRoute,
  ]);

  const deleteExternalRoute = useCallback(async () => {
    if (!enabled) return false;
    setRouteBusy(true);
    try {
      const catalog = await tauriAudioApi.deleteExternalAudioRoute();
      if (!activeRef.current) return false;
      applyRouteCatalog(catalog);
      setRouteValidation(missingRouteValidation);
      setError(null);
      return true;
    } catch (cause) {
      if (activeRef.current) setError(`Could not delete external route: ${errorMessage(cause)}`);
      return false;
    } finally {
      if (activeRef.current) setRouteBusy(false);
    }
  }, [applyRouteCatalog, enabled]);

  const selectedRoute =
    externalRoutes.routes.find((route) => route.routeId === externalRoutes.selectedRouteId) ?? null;

  return {
    inputs,
    physicalInputs: inputs.filter((device) => !device.isLikelyVirtual),
    outputs,
    inputId: settings.selectedInputId ?? '',
    localMonitorId: settings.localMonitorId ?? '',
    reliabilityProfile: settings.reliabilityProfile,
    lastPage: settings.lastPage,
    externalRoutes,
    selectedRoute,
    routeValidation,
    draftRouteId,
    draftPlaybackId,
    draftCaptureId,
    confirmPhysicalEndpoints,
    routeBusy,
    setInputId,
    setLocalMonitorId,
    setReliabilityProfile,
    setLastPage,
    setDraftRouteId,
    setDraftPlaybackId: (id: string) => {
      setDraftRouteIdState('');
      setDraftPlaybackId(id);
    },
    setDraftCaptureId: (id: string) => {
      setDraftRouteIdState('');
      setDraftCaptureId(id);
    },
    setConfirmPhysicalEndpoints,
    saveExternalRoute,
    deleteExternalRoute,
    validateSelectedRoute: () =>
      validateRoute(
        settingsRef.current.selectedInputId ?? '',
        routesRef.current.selectedRouteId ?? '',
      ),
    refresh,
    loading: enabled && loading,
    error,
  };
}
