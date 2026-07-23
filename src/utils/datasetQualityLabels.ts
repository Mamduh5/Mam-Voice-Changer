import type { QualityClassification } from '../types/voiceDataset';

export const qualityLabels: Record<QualityClassification, string> = {
  pass: 'Take passed checks',
  warning: 'Take has warnings',
  fail: 'Take failed checks',
};
