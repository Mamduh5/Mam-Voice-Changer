export const VOICE_DATASET_CONSENT_VERSION = 'voice-dataset-consent-v1';

export type ProfileHealth =
  | 'healthy'
  | 'needsRepair'
  | 'missingFiles'
  | 'orphanedFiles'
  | 'unsupportedSchema'
  | 'corruptManifest';

export type VoiceProfileMetadata = {
  id: string;
  displayName: string;
  description: string | null;
  primaryLanguage: string;
  localeTag: string | null;
  collectionGoalMinutes: number | null;
  createdAt: string;
  updatedAt: string;
};

export type VoiceProfileSummary = {
  profile: VoiceProfileMetadata;
  health: ProfileHealth;
  managedStorageBytes: number;
};

export type CreateVoiceProfileRequest = {
  displayName: string;
  description: string | null;
  primaryLanguage: string;
  localeTag: string | null;
  collectionGoalMinutes: number | null;
  consentConfirmed: boolean;
  confirmedByUser: boolean;
  consentVersion: string;
  consentNotes: string | null;
};

export type UpdateVoiceProfileRequest = Omit<
  CreateVoiceProfileRequest,
  'consentConfirmed' | 'confirmedByUser' | 'consentVersion' | 'consentNotes'
>;
