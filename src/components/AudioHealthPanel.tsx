import type { EngineStatus } from '../types/engine';
import { LevelMeter } from './LevelMeter';

export function AudioHealthPanel({
  status,
  inputActivityDetected,
  onClearClipping,
}: {
  status: EngineStatus;
  inputActivityDetected: boolean | null;
  onClearClipping: () => void;
}) {
  const format = status.activeStreamFormat;
  const clipping = status.inputClipping || status.outputClipping || status.monitorClipping;
  const active = ['running', 'degraded', 'recovering'].includes(status.state);
  return (
    <section className="card audio-health" aria-labelledby="audio-health-heading">
      <div className="section-heading">
        <div>
          <h2 id="audio-health-heading">Audio health</h2>
          <p>
            {active
              ? inputActivityDetected === true
                ? 'Signal is arriving.'
                : inputActivityDetected === false
                  ? 'No input signal detected for several seconds.'
                  : 'Waiting for microphone activity...'
              : 'Start processing to check live signal flow.'}
          </p>
        </div>
        <span className={`health-state ${status.state}`}>{status.state}</span>
      </div>
      <div className="health-meter-grid">
        <LevelMeter label="Input peak" value={status.inputLevel} />
        <LevelMeter
          label={status.routePurpose === 'test' ? 'Monitor peak' : 'Processed-output peak'}
          value={status.routePurpose === 'test' ? status.monitorLevel : status.outputLevel}
        />
      </div>
      {format ? (
        <p className="stream-format">
          {format.inputSampleRate} Hz, {format.inputChannels} input channel(s),{' '}
          {format.processedDestinationChannels ?? format.localMonitorChannels ?? 0} output
          channel(s)
        </p>
      ) : (
        <p className="stream-format">Sample rate and channel count become available after start.</p>
      )}
      {clipping && (
        <div className="clipping-warning" role="alert">
          <div>
            <strong>Clipping detected</strong>
            <p>Lower input/output gain, then clear and test again.</p>
          </div>
          <button type="button" onClick={onClearClipping}>
            Clear clipping
          </button>
        </div>
      )}
    </section>
  );
}
