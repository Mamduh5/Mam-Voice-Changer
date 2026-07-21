import { describe, expect, it } from 'vitest';
import { defaultApplicationSettings } from '../hooks/useAudioDevices';
import { isLeavingTest } from './monitoringMode';

describe('temporary monitoring safety', () => {
  it('does not persist a monitor-enabled setting and starts on Use', () => {
    expect(defaultApplicationSettings).not.toHaveProperty('localMonitorEnabled');
    expect(defaultApplicationSettings.lastPage).toBe('use');
  });

  it('requests a conditional Test stop whenever Test is left', () => {
    expect(isLeavingTest('test', 'use')).toBe(true);
    expect(isLeavingTest('test', 'diagnostics')).toBe(true);
  });

  it('does not request a Test stop for navigation elsewhere', () => {
    expect(isLeavingTest('use', 'test')).toBe(false);
    expect(isLeavingTest('diagnostics', 'use')).toBe(false);
  });
});
