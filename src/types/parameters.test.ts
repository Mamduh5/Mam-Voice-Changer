import { describe, expect, it } from 'vitest';
import { defaultAudioParameters, type AudioParameters } from './parameters';

describe('audio parameter model', () => {
  it('disables every dedicated vocal-aging effect by default', () => {
    expect(defaultAudioParameters.ageCharacter).toBe(0);
    expect(defaultAudioParameters.breathiness).toBe(0);
    expect(defaultAudioParameters.tremor).toBe(0);
  });

  it('round-trips all vocal-aging fields in a complete parameter snapshot', () => {
    const parameters: AudioParameters = {
      ...defaultAudioParameters,
      ageCharacter: 0.78,
      breathiness: 0.48,
      tremor: 0.34,
    };

    expect(JSON.parse(JSON.stringify(parameters))).toEqual(parameters);
  });
});
