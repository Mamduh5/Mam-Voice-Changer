import type { AudioParameters } from './parameters';

export type Preset = {
  id: string;
  name: string;
  parameters: AudioParameters;
  builtIn: boolean;
};

export type PresetCatalog = {
  schemaVersion: number;
  presets: Preset[];
  selectedPresetId: string | null;
  activeParameters: AudioParameters;
};
