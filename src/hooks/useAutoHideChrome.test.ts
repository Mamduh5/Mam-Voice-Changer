import { describe, expect, it } from 'vitest';
import { getChromeScrollDirective } from './useAutoHideChrome';

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
