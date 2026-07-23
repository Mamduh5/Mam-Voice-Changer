import type { DatasetTake } from '../types/voiceDataset';

export type DatasetTakeFilter =
  | 'all'
  | 'pending'
  | 'accepted'
  | 'rejected'
  | 'warning'
  | 'failed'
  | 'excluded'
  | 'imported'
  | 'recorded';

export function filterDatasetTakes(takes: DatasetTake[], filter: DatasetTakeFilter) {
  return takes.filter((take) => {
    switch (filter) {
      case 'pending':
      case 'accepted':
      case 'rejected':
        return take.reviewStatus === filter;
      case 'warning':
        return take.quality.classification === 'warning';
      case 'failed':
        return take.quality.classification === 'fail';
      case 'excluded':
        return take.excludeFromTraining;
      case 'imported':
        return take.source === 'imported';
      case 'recorded':
        return take.source === 'recorded';
      default:
        return true;
    }
  });
}
