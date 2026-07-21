import type { AudioRoutePurpose, EngineStatus } from '../types/engine';

type Props = {
  status: EngineStatus;
  purpose: AudioRoutePurpose;
  canStart: boolean;
  startLabel: string;
  stopLabel: string;
  description: string;
  onStart: () => void;
  onStop: () => void;
};

export function EngineControls({
  status,
  purpose,
  canStart,
  startLabel,
  stopLabel,
  description,
  onStart,
  onStop,
}: Props) {
  const ownsRoute = status.routePurpose === purpose;
  const active = ownsRoute && ['running', 'degraded', 'recovering'].includes(status.state);
  const busy = status.state === 'starting' || status.state === 'stopping';
  const routeName = purpose === 'use' ? 'Use route' : 'Test route';
  const otherRouteName = status.routePurpose === 'use' ? 'Use route' : 'Test route';
  const routeMessage = ownsRoute
    ? status.message
    : status.routePurpose
      ? `${otherRouteName} is active`
      : 'Not active';

  return (
    <section className="transport card">
      <div>
        <strong>
          {routeName}: {routeMessage}
        </strong>
        <small>{description}</small>
      </div>
      <button
        className={active ? 'stop' : 'start'}
        disabled={busy || (!active && !canStart)}
        onClick={active ? onStop : onStart}
      >
        {busy ? 'Please wait...' : active ? stopLabel : startLabel}
      </button>
    </section>
  );
}
