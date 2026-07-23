import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';

const TERMINAL_TRAINING_STATES = new Set(['cancelled', 'completed', 'failed', 'interrupted']);

function delay(milliseconds: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));
}

export function useModelShutdownGuard() {
  useEffect(() => {
    if (!isTauri()) return undefined;
    let disposed = false;
    const unlisten = listen('voice-model-shutdown-blocked', async () => {
      if (
        !window.confirm(
          'Local model work is active. Cancel it safely and close after the worker stops?',
        )
      )
        return;
      try {
        await tauriAudioApi.cancelModelWorkForShutdown();
        for (let attempt = 0; attempt < 75 && !disposed; attempt += 1) {
          const status = await tauriAudioApi.getVoiceModelStatus();
          const trainingActive = Boolean(
            status.activeTrainingJob &&
            !TERMINAL_TRAINING_STATES.has(status.activeTrainingJob.state),
          );
          if (!trainingActive && !status.activeInference) {
            await getCurrentWindow().close();
            return;
          }
          await delay(200);
        }
        if (!disposed)
          window.alert('The model worker is still stopping. Try closing again shortly.');
      } catch (cause) {
        if (!disposed) window.alert(`Could not cancel local model work: ${String(cause)}`);
      }
    });
    return () => {
      disposed = true;
      void unlisten.then((remove) => remove());
    };
  }, []);
}
