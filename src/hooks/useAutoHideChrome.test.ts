import { describe, expect, it } from 'vitest';
import {
  chromeReducer,
  getChromeScrollDirective,
  initialChromeState,
  isBackgroundChromeToggle,
} from './useAutoHideChrome';

describe('getChromeScrollDirective', () => {
  it('keeps chrome visible at the page top and after upward scrolling', () => {
    expect(getChromeScrollDirective(80, 20)).toBe('show');
    expect(getChromeScrollDirective(120, 96)).toBe('show');
  });

  it('hides chrome only after a meaningful downward scroll', () => {
    expect(getChromeScrollDirective(30, 38)).toBe('keep');
    expect(getChromeScrollDirective(30, 48)).toBe('hide');
  });
});

describe('chromeReducer', () => {
  it('keeps a manual hide stable against automatic events', () => {
    const hidden = chromeReducer(initialChromeState, { type: 'BACKGROUND_TOGGLE' });

    expect(hidden).toEqual({ mode: 'manuallyHidden', visible: false });
    expect(chromeReducer(hidden, { type: 'AUTOMATIC_SHOW' })).toEqual(hidden);
    expect(chromeReducer(hidden, { type: 'AUTOMATIC_HIDE' })).toEqual(hidden);
  });

  it('keeps a manual show stable until an explicit future mode change', () => {
    const hidden = chromeReducer(initialChromeState, { type: 'BACKGROUND_TOGGLE' });
    const visible = chromeReducer(hidden, { type: 'BACKGROUND_TOGGLE' });

    expect(visible).toEqual({ mode: 'manuallyVisible', visible: true });
    expect(chromeReducer(visible, { type: 'AUTOMATIC_HIDE' })).toEqual(visible);
  });

  it('reveals chrome for Escape and navigation focus', () => {
    const hidden = chromeReducer(initialChromeState, { type: 'BACKGROUND_TOGGLE' });

    expect(chromeReducer(hidden, { type: 'ESCAPE' })).toEqual({
      mode: 'manuallyVisible',
      visible: true,
    });
    expect(chromeReducer(hidden, { type: 'NAVIGATION_FOCUS' })).toEqual({
      mode: 'manuallyVisible',
      visible: true,
    });
  });
});

describe('isBackgroundChromeToggle', () => {
  it('accepts only a short click on the outer application background', () => {
    const background = new EventTarget();
    const other = new EventTarget();

    expect(isBackgroundChromeToggle(background, background, 0, false)).toBe(true);
    expect(isBackgroundChromeToggle(other, background, 0, false)).toBe(false);
    expect(isBackgroundChromeToggle(background, background, 5, false)).toBe(false);
    expect(isBackgroundChromeToggle(background, background, 0, true)).toBe(false);
  });
});
