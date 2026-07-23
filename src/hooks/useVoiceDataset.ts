import { open } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import {
  emptyVoiceDatasetStatus,
  type DatasetExportOptions,
  type PromptPack,
  type PromptSelection,
  type ReviewTakeRequest,
  type SelectedTakeVersion,
  type VoiceDatasetStatus,
} from '../types/voiceDataset';

function errorMessage(cause: unknown) {
  if (cause && typeof cause === 'object' && 'message' in cause) return String(cause.message);
  return cause instanceof Error ? cause.message : String(cause);
}

export function useVoiceDataset(
  enabled: boolean,
  selectedProfileId: string | null,
  onStatusChange?: (status: VoiceDatasetStatus) => void,
) {
  const [prompts, setPrompts] = useState<PromptPack | null>(null);
  const [status, setStatus] = useState<VoiceDatasetStatus>(emptyVoiceDatasetStatus);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = useRef(false);

  useEffect(() => {
    active.current = enabled;
    if (!enabled) return undefined;
    let cancelled = false;
    let refreshing = false;
    let firstRefresh = true;
    const refresh = async () => {
      if (refreshing) return;
      refreshing = true;
      try {
        let nextStatus = selectedProfileId
          ? await tauriAudioApi.getVoiceDatasetStatus()
          : emptyVoiceDatasetStatus;
        if (
          selectedProfileId &&
          (nextStatus.currentProfileId !== selectedProfileId ||
            nextStatus.manifest?.profile.id !== selectedProfileId)
        ) {
          nextStatus = await tauriAudioApi.readVoiceProfile(selectedProfileId);
        }
        if (!cancelled) {
          setStatus(nextStatus);
          setError(nextStatus.lastError?.message ?? null);
          onStatusChange?.(nextStatus);
        }
        if (firstRefresh) {
          const nextPrompts = await tauriAudioApi.listDatasetPrompts();
          if (!cancelled) {
            setPrompts(nextPrompts);
          }
          firstRefresh = false;
        }
      } catch (cause) {
        if (!cancelled) setError(errorMessage(cause));
      } finally {
        refreshing = false;
      }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 350);
    return () => {
      cancelled = true;
      active.current = false;
      window.clearInterval(timer);
      void tauriAudioApi.leaveVoiceDataset().catch(() => undefined);
    };
  }, [enabled, onStatusChange, selectedProfileId]);

  const run = useCallback(
    async (operation: () => Promise<VoiceDatasetStatus>) => {
      setBusy(true);
      try {
        const next = await operation();
        if (active.current) {
          setStatus(next);
          setError(next.lastError?.message ?? null);
          onStatusChange?.(next);
        }
        return true;
      } catch (cause) {
        if (active.current) setError(errorMessage(cause));
        return false;
      } finally {
        if (active.current) setBusy(false);
      }
    },
    [onStatusChange],
  );
  const selectPrompt = useCallback(
    (selection: PromptSelection) => run(() => tauriAudioApi.selectDatasetPrompt(selection)),
    [run],
  );
  const record = useCallback(
    (inputId: string, inputName: string, recordedConsent = false) =>
      run(() => tauriAudioApi.startDatasetRecording(inputId, inputName, recordedConsent)),
    [run],
  );
  const stopRecording = useCallback(() => run(tauriAudioApi.stopDatasetRecording), [run]);
  const discardRecording = useCallback(() => run(tauriAudioApi.discardCurrentDatasetTake), [run]);
  const importWavs = useCallback(
    async (selection: PromptSelection) => {
      const selected = await open({
        multiple: true,
        directory: false,
        filters: [{ name: 'WAV audio', extensions: ['wav'] }],
      });
      const paths = typeof selected === 'string' ? [selected] : selected;
      if (!paths?.length) return false;
      return run(() => tauriAudioApi.importDatasetWavs(paths, selection));
    },
    [run],
  );
  const reviewTake = useCallback(
    (profileId: string, takeId: string, request: ReviewTakeRequest) =>
      run(() => tauriAudioApi.reviewDatasetTake(profileId, takeId, request)),
    [run],
  );
  const autoTrim = useCallback(
    (takeId: string) => run(() => tauriAudioApi.autoTrimDatasetTake(takeId)),
    [run],
  );
  const setTrim = useCallback(
    (takeId: string, startFrame: number, endFrame: number) =>
      run(() => tauriAudioApi.setDatasetTrim(takeId, startFrame, endFrame)),
    [run],
  );
  const applyTrim = useCallback(() => run(tauriAudioApi.applyDatasetTrim), [run]);
  const resetTrim = useCallback(
    (takeId: string) => run(() => tauriAudioApi.resetDatasetTrim(takeId)),
    [run],
  );
  const preview = useCallback(
    (
      takeId: string,
      version: SelectedTakeVersion,
      outputId: string,
      outputName: string,
      seekMs = 0,
    ) => run(() => tauriAudioApi.previewDatasetTake(takeId, version, outputId, outputName, seekMs)),
    [run],
  );
  const pausePreview = useCallback(() => run(tauriAudioApi.pauseDatasetPreview), [run]);
  const stopPreview = useCallback(() => run(tauriAudioApi.stopDatasetPreview), [run]);
  const deleteTake = useCallback(
    (takeId: string) => run(() => tauriAudioApi.deleteDatasetTake(takeId)),
    [run],
  );
  const exportDataset = useCallback(async (options: DatasetExportOptions) => {
    const destination = await open({ multiple: false, directory: true });
    if (typeof destination !== 'string') return false;
    setBusy(true);
    try {
      await tauriAudioApi.exportVoiceDataset(destination, options);
      if (active.current) setError(null);
      return true;
    } catch (cause) {
      if (active.current) setError(errorMessage(cause));
      return false;
    } finally {
      if (active.current) setBusy(false);
    }
  }, []);
  return {
    prompts,
    status,
    busy,
    error,
    selectPrompt,
    record,
    stopRecording,
    discardRecording,
    importWavs,
    reviewTake,
    autoTrim,
    setTrim,
    applyTrim,
    resetTrim,
    preview,
    pausePreview,
    stopPreview,
    deleteTake,
    exportDataset,
  };
}
