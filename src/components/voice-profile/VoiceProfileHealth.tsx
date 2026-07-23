import type { VoiceProfileSummary } from '../../types/voiceProfile';

const healthLabels = {
  healthy: 'Healthy',
  needsRepair: 'Needs repair',
  missingFiles: 'Missing files',
  orphanedFiles: 'Orphaned files',
  unsupportedSchema: 'Unsupported schema',
  corruptManifest: 'Corrupt manifest',
} as const;

export function VoiceProfileHealth({ summary }: { summary: VoiceProfileSummary }) {
  return (
    <div
      className={`profile-health profile-health-${summary.health}`}
      role={summary.health === 'healthy' ? 'status' : 'alert'}
    >
      <strong>Profile health: {healthLabels[summary.health]}</strong>
      <span>
        {summary.health === 'healthy'
          ? 'Consent and managed Dataset metadata are readable.'
          : 'Dataset and Models remain blocked until this profile is healthy.'}
      </span>
    </div>
  );
}
