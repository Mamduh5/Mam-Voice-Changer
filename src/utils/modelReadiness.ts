import type { BackendReadiness } from '../types/modelBackend';
import type { VoiceDatasetManifest } from '../types/voiceDataset';

export function modelProfileReadiness(manifest: VoiceDatasetManifest | null) {
  if (!manifest) return { ready: false, state: 'noProfile', message: 'Select a Dataset profile.' };
  if (!manifest.consent.consentConfirmed || manifest.consent.revokedAt)
    return {
      ready: false,
      state: 'consentInactive',
      message: 'Active target-speaker consent is required.',
    };
  if (manifest.statistics.acceptedTakes === 0)
    return {
      ready: false,
      state: 'noAcceptedTakes',
      message: 'Accept at least one non-excluded Dataset take.',
    };
  return { ready: true, state: 'ready', message: 'Consent and accepted Dataset audio are ready.' };
}

export function backendReadinessLabel(readiness: BackendReadiness) {
  const labels: Record<BackendReadiness, string> = {
    notConfigured: 'Backend not configured',
    pythonMissing: 'Python missing',
    workerMissing: 'Worker missing',
    backendMissing: 'Seed-VC backend missing',
    checkpointMissing: 'Checkpoint missing',
    configurationInvalid: 'Configuration requires validation',
    protocolMismatch: 'Worker protocol mismatch',
    unsupportedHardware: 'Unsupported hardware selection',
    ready: 'Backend ready',
  };
  return labels[readiness];
}
