import type { DatasetStatistics } from '../types/voiceDataset';

export function formatDatasetDuration(milliseconds: number) {
  const minutes = Math.floor(milliseconds / 60_000);
  const seconds = Math.floor((milliseconds % 60_000) / 1_000);
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export function collectionGoalPercent(statistics: DatasetStatistics, goalMinutes: number | null) {
  if (!goalMinutes) return 0;
  return Math.min(100, (statistics.acceptedDurationMs / (goalMinutes * 60_000)) * 100);
}
