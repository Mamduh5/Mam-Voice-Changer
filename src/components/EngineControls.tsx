import type { EngineStatus } from '../types/engine';

type Props = {
  status: EngineStatus;
  canStart: boolean;
  onStart: () => void;
  onStop: () => void;
};

export function EngineControls({ status, canStart, onStart, onStop }: Props) {
  const running = status.state === 'running';
  const busy = status.state === 'starting' || status.state === 'stopping';
  return (
    <section className="transport card">
      <div>
        <strong>{status.message}</strong>
        <small>Milestone 1 · clean microphone passthrough</small>
      </div>
      <button
        className={running ? 'stop' : 'start'}
        disabled={busy || (!running && !canStart)}
        onClick={running ? onStop : onStart}
      >
        {busy ? 'Please wait…' : running ? 'Stop engine' : 'Start passthrough'}
      </button>
    </section>
  );
}
