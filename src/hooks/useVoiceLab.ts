import { open, save } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { AudioParameters } from '../types/parameters';
import {
  emptyVoiceLabStatus,
  type VoiceLabClipVersion,
  type VoiceLabStatus,
} from '../types/voiceLab';

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export function useVoiceLab(enabled: boolean, liveParameters: AudioParameters) {
  const [parameters, setParameters] = useState<AudioParameters>({ ...liveParameters });
  const [status, setStatus] = useState<VoiceLabStatus>(emptyVoiceLabStatus);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [renderStale, setRenderStale] = useState(false);
  const initialized = useRef(false);
  const active = useRef(false);

  const initialize = useCallback((next: AudioParameters) => {
    if (initialized.current) return;
    initialized.current = true;
    setParameters({ ...next });
  }, []);

  useEffect(() => {
    active.current = enabled;
    if (!enabled) return undefined;
    let cancelled = false;
    let refreshRunning = false;
    const refresh = async () => {
      if (refreshRunning) return;
      refreshRunning = true;
      try {
        const next = await tauriAudioApi.getVoiceLabStatus();
        if (!cancelled) {
          setStatus(next);
          if (next.lastError) setError(next.lastError);
        }
      } catch (cause) {
        if (!cancelled) setError(`Voice Lab status failed: ${errorMessage(cause)}`);
      } finally {
        refreshRunning = false;
      }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 200);
    return () => {
      cancelled = true;
      active.current = false;
      window.clearInterval(timer);
      void tauriAudioApi.stopVoiceLabAudio().catch(() => undefined);
    };
  }, [enabled]);

  const run = useCallback(async (operation: () => Promise<VoiceLabStatus>) => {
    setBusy(true);
    try {
      const next = await operation();
      if (active.current) {
        setStatus(next);
        setError(next.lastError);
      }
      return true;
    } catch (cause) {
      if (active.current) setError(errorMessage(cause));
      return false;
    } finally {
      if (active.current) setBusy(false);
    }
  }, []);

  const updateParameters = useCallback(
    (changes: Partial<AudioParameters>) => {
      setParameters((current) => ({ ...current, ...changes }));
      if (status.processed) setRenderStale(true);
    },
    [status.processed],
  );

  const applyPreset = useCallback(
    (next: AudioParameters) => {
      setParameters({ ...next });
      if (status.processed) setRenderStale(true);
    },
    [status.processed],
  );

  const record = useCallback(
    (inputId: string, inputName: string) =>
      run(() => tauriAudioApi.startVoiceLabCapture(inputId, inputName)),
    [run],
  );
  const stopRecording = useCallback(async () => {
    const success = await run(tauriAudioApi.stopVoiceLabCapture);
    if (success) setRenderStale(false);
    return success;
  }, [run]);
  const importWav = useCallback(async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'WAV audio', extensions: ['wav'] }],
    });
    if (typeof path !== 'string') return false;
    const success = await run(() => tauriAudioApi.importVoiceLabWav(path));
    if (success) setRenderStale(false);
    return success;
  }, [run]);
  const render = useCallback(async () => {
    const success = await run(() => tauriAudioApi.renderVoiceLab(parameters));
    if (success) setRenderStale(false);
    return success;
  }, [parameters, run]);
  const preview = useCallback(
    (version: VoiceLabClipVersion, outputId: string, outputName: string, looping: boolean) =>
      run(() => tauriAudioApi.startVoiceLabPreview(version, outputId, outputName, looping)),
    [run],
  );
  const stopPreview = useCallback(() => run(tauriAudioApi.stopVoiceLabPreview), [run]);
  const stopAudio = useCallback(() => run(tauriAudioApi.stopVoiceLabAudio), [run]);
  const exportWav = useCallback(async (version: VoiceLabClipVersion) => {
    const path = await save({
      defaultPath: `mam-voice-lab-${version}.wav`,
      filters: [{ name: 'WAV audio', extensions: ['wav'] }],
    });
    if (!path) return false;
    setBusy(true);
    try {
      await tauriAudioApi.exportVoiceLabWav(version, path);
      if (active.current) setError(null);
      return true;
    } catch (cause) {
      if (active.current) setError(errorMessage(cause));
      return false;
    } finally {
      if (active.current) setBusy(false);
    }
  }, []);
  const clear = useCallback(async () => {
    const success = await run(tauriAudioApi.clearVoiceLab);
    if (success) setRenderStale(false);
    return success;
  }, [run]);

  return {
    parameters,
    status,
    busy,
    error,
    renderStale,
    initialize,
    updateParameters,
    applyPreset,
    record,
    stopRecording,
    importWav,
    render,
    preview,
    stopPreview,
    stopAudio,
    exportWav,
    clear,
  };
}
