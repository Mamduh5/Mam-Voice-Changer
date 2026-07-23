import { open } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import { emptyVoiceDatasetStatus, type VoiceDatasetStatus } from '../types/voiceDataset';
import type { VoiceModelStatus } from '../types/voiceModel';
import type {
  CreateVoiceProfileRequest,
  UpdateVoiceProfileRequest,
  VoiceProfileSummary,
} from '../types/voiceProfile';

function errorMessage(cause: unknown) {
  if (cause && typeof cause === 'object' && 'message' in cause) return String(cause.message);
  return cause instanceof Error ? cause.message : String(cause);
}

function canRestore(summary: VoiceProfileSummary | undefined) {
  return Boolean(summary && !['unsupportedSchema', 'corruptManifest'].includes(summary.health));
}

export function validateSelectedProfileId(
  profiles: VoiceProfileSummary[],
  candidate: string | null,
) {
  if (!candidate) return null;
  const summary = profiles.find(({ profile }) => profile.id === candidate);
  return canRestore(summary) ? candidate : null;
}

export function clearDeletedProfileSelection(current: string | null, deletedId: string) {
  return current === deletedId ? null : current;
}

export function useVoiceProfiles(enabled: boolean, modelStatus?: VoiceModelStatus) {
  const [profiles, setProfiles] = useState<VoiceProfileSummary[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [status, setStatus] = useState<VoiceDatasetStatus>(emptyVoiceDatasetStatus);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = useRef(false);

  const acceptStatus = useCallback((next: VoiceDatasetStatus) => {
    if (!active.current) return;
    setStatus(next);
    setError(next.lastError?.message ?? null);
    setSelectedProfileId(next.currentProfileId);
  }, []);

  const refreshProfiles = useCallback(async () => {
    const nextProfiles = await tauriAudioApi.listVoiceProfiles();
    if (!active.current) return nextProfiles;
    setProfiles(nextProfiles);
    setSelectedProfileId((current) => {
      return validateSelectedProfileId(nextProfiles, current);
    });
    return nextProfiles;
  }, []);

  useEffect(() => {
    active.current = enabled;
    if (!enabled) return undefined;
    let cancelled = false;
    void Promise.all([tauriAudioApi.listVoiceProfiles(), tauriAudioApi.getVoiceDatasetStatus()])
      .then(([nextProfiles, nextStatus]) => {
        if (cancelled) return;
        setProfiles(nextProfiles);
        const restoredId = validateSelectedProfileId(nextProfiles, nextStatus.currentProfileId);
        if (restoredId && nextStatus.manifest?.profile.id === restoredId) {
          setSelectedProfileId(restoredId);
          setStatus(nextStatus);
        } else {
          setSelectedProfileId(null);
          setStatus(emptyVoiceDatasetStatus);
        }
        setError(nextStatus.lastError?.message ?? null);
      })
      .catch((cause: unknown) => !cancelled && setError(errorMessage(cause)))
      .finally(() => !cancelled && setBusy(false));
    return () => {
      cancelled = true;
      active.current = false;
    };
  }, [enabled]);

  const run = useCallback(
    async (
      operation: () => Promise<VoiceDatasetStatus>,
      options: { refresh?: boolean; deletedId?: string } = {},
    ) => {
      setBusy(true);
      try {
        const nextStatus = await operation();
        if (!active.current) return false;
        setStatus(nextStatus);
        setError(nextStatus.lastError?.message ?? null);
        setSelectedProfileId((current) => {
          if (options.deletedId) {
            return clearDeletedProfileSelection(current, options.deletedId);
          }
          return nextStatus.currentProfileId;
        });
        if (options.refresh) await refreshProfiles();
        return true;
      } catch (cause) {
        if (active.current) setError(errorMessage(cause));
        return false;
      } finally {
        if (active.current) setBusy(false);
      }
    },
    [refreshProfiles],
  );

  const selectProfile = useCallback(
    (profileId: string) => {
      const summary = profiles.find(({ profile }) => profile.id === profileId);
      if (!summary) {
        setError('The selected voice profile no longer exists.');
        setSelectedProfileId(null);
        setStatus(emptyVoiceDatasetStatus);
        return Promise.resolve(false);
      }
      if (!canRestore(summary)) {
        setError(
          `Voice profile "${summary.profile.displayName}" cannot be active while its health is ${summary.health}. Repair it from Profiles first.`,
        );
        setSelectedProfileId(null);
        setStatus(emptyVoiceDatasetStatus);
        return Promise.resolve(false);
      }
      return run(() => tauriAudioApi.readVoiceProfile(profileId));
    },
    [profiles, run],
  );
  const createProfile = useCallback(
    (request: CreateVoiceProfileRequest) =>
      run(() => tauriAudioApi.createVoiceProfile(request), { refresh: true }),
    [run],
  );
  const updateProfile = useCallback(
    (profileId: string, request: UpdateVoiceProfileRequest) =>
      run(() => tauriAudioApi.updateVoiceProfile(profileId, request), { refresh: true }),
    [run],
  );
  const repairProfile = useCallback(
    (profileId: string) =>
      run(() => tauriAudioApi.repairVoiceProfile(profileId), { refresh: true }),
    [run],
  );
  const deleteProfile = useCallback(
    (profileId: string) =>
      run(() => tauriAudioApi.deleteVoiceProfile(profileId), {
        refresh: true,
        deletedId: profileId,
      }),
    [run],
  );
  const exportDataset = useCallback(async () => {
    if (!selectedProfileId) {
      setError('Select a voice profile before exporting its Dataset.');
      return false;
    }
    const destination = await open({ multiple: false, directory: true });
    if (typeof destination !== 'string') return false;
    setBusy(true);
    try {
      await tauriAudioApi.exportVoiceDataset(destination, {
        includeRejected: false,
        includeRawMasters: false,
      });
      if (active.current) setError(null);
      return true;
    } catch (cause) {
      if (active.current) setError(errorMessage(cause));
      return false;
    } finally {
      if (active.current) setBusy(false);
    }
  }, [selectedProfileId]);

  const selectedSummary = profiles.find(({ profile }) => profile.id === selectedProfileId) ?? null;
  const datasetSummary = status.manifest?.statistics ?? null;
  const modelSummary = useMemo(() => {
    if (!selectedProfileId || !modelStatus) {
      return { snapshots: 0, artifacts: 0, activeTraining: false };
    }
    return {
      snapshots: modelStatus.snapshots.filter(
        (snapshot) => snapshot.profileId === selectedProfileId,
      ).length,
      artifacts: modelStatus.artifacts.filter(
        (artifact) => artifact.profileId === selectedProfileId,
      ).length,
      activeTraining: modelStatus.activeTrainingJob?.profileId === selectedProfileId,
    };
  }, [modelStatus, selectedProfileId]);

  return {
    profiles,
    selectedProfileId,
    selectedSummary,
    status,
    manifest: status.manifest,
    consentActive: Boolean(
      status.manifest?.consent.consentConfirmed && !status.manifest.consent.revokedAt,
    ),
    datasetSummary,
    modelSummary,
    busy,
    error,
    selectProfile,
    createProfile,
    updateProfile,
    repairProfile,
    deleteProfile,
    exportDataset,
    refreshProfiles,
    acceptStatus,
  };
}
