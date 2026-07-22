export type VoiceLabClipVersion = 'original' | 'processed';

export type VoiceLabClipSummary = {
  sourceName: string;
  durationMs: number;
  sampleRate: number;
  channels: number;
  frames: number;
  peak: number;
  waveform: number[];
};

export type VoiceLabRenderMetadata = {
  latencyFrames: number;
  blockFrames: number;
};

export type VoiceLabStatus = {
  original: VoiceLabClipSummary | null;
  processed: VoiceLabClipSummary | null;
  renderMetadata: VoiceLabRenderMetadata | null;
  capture: {
    active: boolean;
    droppedFrames: number;
  };
  preview: {
    active: boolean;
    kind: VoiceLabClipVersion | null;
    looping: boolean;
    positionMs: number;
    durationMs: number;
  };
  lastError: string | null;
  processedSynthetic: boolean;
};

export const emptyVoiceLabStatus: VoiceLabStatus = {
  original: null,
  processed: null,
  renderMetadata: null,
  capture: { active: false, droppedFrames: 0 },
  preview: { active: false, kind: null, looping: false, positionMs: 0, durationMs: 0 },
  lastError: null,
  processedSynthetic: false,
};
