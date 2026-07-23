import type { TrainingJob } from '../types/trainingJob';

export function trainingPhaseLabel(job: TrainingJob | null) {
  if (!job) return 'Training ready';
  const spaced = job.state.replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`);
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

export function trainingProgressPercent(job: TrainingJob | null) {
  return Math.round(Math.max(0, Math.min(1, job?.overallProgress ?? 0)) * 100);
}

export function backendMetric(value: number | null) {
  return value === null ? 'Not reported' : value.toFixed(5);
}
