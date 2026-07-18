import type { EngineStatus } from '../types/engine';

export function DiagnosticsPanel({ status }: { status: EngineStatus }) {
  const format = status.activeStreamFormat;
  return (
    <section className="card diagnostics">
      <h2>Diagnostics</h2>
      <dl>
        <div>
          <dt>Format</dt>
          <dd>
            {format
              ? `${format.sampleRate / 1000} kHz · ${format.inputSampleFormat} ${format.inputChannels}ch → ${format.outputSampleFormat} ${format.outputChannels}ch`
              : 'Not active'}
          </dd>
        </div>
        <div>
          <dt>DSP latency</dt>
          <dd>{format ? `${status.dspProcessingLatencyMs.toFixed(1)} ms` : '—'}</dd>
        </div>
        <div>
          <dt>Total estimated latency</dt>
          <dd>{format ? `${status.totalEstimatedLatencyMs.toFixed(1)} ms` : '—'}</dd>
        </div>
        <div>
          <dt>Input overruns</dt>
          <dd>{status.inputOverruns}</dd>
        </div>
        <div>
          <dt>Output underruns</dt>
          <dd>{status.outputUnderruns}</dd>
        </div>
        <div>
          <dt>DSP input underruns</dt>
          <dd>{status.dspInputUnderruns}</dd>
        </div>
        <div>
          <dt>DSP output overruns</dt>
          <dd>{status.dspOutputOverruns}</dd>
        </div>
      </dl>
    </section>
  );
}
