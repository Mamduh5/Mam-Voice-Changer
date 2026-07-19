import type { EngineStatus } from '../types/engine';

type Props = {
  status: EngineStatus;
  canStart: boolean;
  onStart: () => void;
  onStop: () => void;
};

export function EngineControls({ status, canStart, onStart, onStop }: Props) {
  const active = ['running', 'degraded', 'recovering'].includes(status.state);
  const busy = status.state === 'starting' || status.state === 'stopping';
  return (
    <section className="transport card">
      <div>
        <strong>{status.message}</strong>
        <small>Local real-time audio processing</small>
      </div>
      <button
        className={active ? 'stop' : 'start'}
        disabled={busy || (!active && !canStart)}
        onClick={active ? onStop : onStart}
      >
        {busy ? 'Please wait…' : active ? 'Stop engine' : 'Start engine'}
      </button>
    </section>
  );
}
