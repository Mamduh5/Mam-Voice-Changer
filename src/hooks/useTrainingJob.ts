import type { TrainingJob } from '../types/trainingJob';
import { trainingPhaseLabel, trainingProgressPercent } from '../utils/modelProgress';

export function useTrainingJob(job: TrainingJob | null) {
  const terminal = job
    ? ['cancelled', 'completed', 'failed', 'interrupted'].includes(job.state)
    : true;
  return {
    job,
    active: Boolean(job && !terminal),
    phaseLabel: trainingPhaseLabel(job),
    progressPercent: trainingProgressPercent(job),
  };
}
