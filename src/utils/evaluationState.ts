import type { VoiceModelArtifact } from '../types/voiceModel';

export function canApproveArtifact(artifact: VoiceModelArtifact, consentActive: boolean) {
  return (
    consentActive &&
    artifact.approvalStatus === 'unevaluated' &&
    Boolean(artifact.evaluation?.ratings.listeningConfirmed) &&
    Boolean(artifact.evaluation?.clips.some((clip) => clip.successful))
  );
}

export function approvalLabel(artifact: VoiceModelArtifact) {
  const labels: Record<VoiceModelArtifact['approvalStatus'], string> = {
    unevaluated: 'Model unevaluated',
    evaluationInProgress: 'Evaluating',
    approvedForOfflineUse: 'Approved for local offline conversion',
    rejected: 'Rejected',
    disabledByConsent: 'Disabled by consent',
    invalid: 'Artifact invalid',
    missingFiles: 'Artifact missing files',
  };
  return labels[artifact.approvalStatus];
}
