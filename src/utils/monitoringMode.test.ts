import { describe, expect, it } from 'vitest';
import { defaultApplicationSettings } from '../hooks/useAudioDevices';
import { shouldStopTemporaryTestMonitoring } from './monitoringMode';

describe('temporary monitoring safety', () => {
  it('defaults local monitoring off and starts on Use', () => {
    expect(defaultApplicationSettings.localMonitorEnabled).toBe(false);
    expect(defaultApplicationSettings.lastPage).toBe('use');
  });

  it.each(['starting', 'running', 'degraded', 'recovering', 'stopping'] as const)(
    'stops temporary Test monitoring when leaving during %s',
    (state) => {
      expect(shouldStopTemporaryTestMonitoring('test', 'use', 'test', state)).toBe(true);
    },
  );

  it('does not stop a deliberately separate Use route', () => {
    expect(shouldStopTemporaryTestMonitoring('test', 'use', 'use', 'running')).toBe(false);
  });
});
