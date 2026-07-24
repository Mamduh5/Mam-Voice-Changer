import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import { defaultAudioParameters } from '../types/parameters';
import { DspControls } from './DspControls';

describe('DspControls', () => {
  it('renders consonant preservation and the three vocal-aging controls', () => {
    const markup = renderToStaticMarkup(
      <DspControls parameters={defaultAudioParameters} disabled={false} onChange={vi.fn()} />,
    );

    expect(markup).toContain('Vocal aging');
    expect(markup).toContain('Consonant preservation');
    expect(markup).toContain('Age Character');
    expect(markup).toContain('Breathiness');
    expect(markup).toContain('Tremor');
    expect(markup).toContain('This is not neural voice cloning.');
    expect(markup.match(/type="range"/g)?.length).toBe(13);
  });

  it('disables vocal-aging sliders during preset operations', () => {
    const markup = renderToStaticMarkup(
      <DspControls parameters={defaultAudioParameters} disabled={true} onChange={vi.fn()} />,
    );

    expect(markup.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(3);
  });
});
