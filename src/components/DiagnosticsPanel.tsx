import type { EngineStatus } from '../types/engine';

function value(value: number | null | undefined, unit = '') {
  return value == null ? 'N/A' : `${value}${unit}`;
}

export function DiagnosticsPanel({ status }: { status: EngineStatus }) {
  const format = status.activeStreamFormat;
  const items: Array<[string, string | number]> = [
    ['Reliability profile', status.reliabilityProfile],
    [
      'Input format',
      format
        ? `${format.inputSampleRate} Hz - ${format.inputChannels} ch - ${format.inputSampleFormat}`
        : 'Not active',
    ],
    [
      'Destination format',
      format
        ? `${value(format.processedDestinationSampleRate, ' Hz')} - ${value(format.processedDestinationChannels, ' ch')}`
        : 'N/A',
    ],
    [
      'Monitor format',
      format
        ? `${value(format.localMonitorSampleRate, ' Hz')} - ${value(format.localMonitorChannels, ' ch')}`
        : 'N/A',
    ],
    ['Input callback buffer', value(format?.inputBufferFrames, ' frames')],
    ['Destination callback buffer', value(format?.processedDestinationBufferFrames, ' frames')],
    ['Monitor callback buffer', value(format?.localMonitorBufferFrames, ' frames')],
    ['DSP block', value(format?.dspBlockFrames, ' frames')],
    ['DSP latency', `${status.dspProcessingLatencyMs.toFixed(1)} ms`],
    ['Total estimated latency', `${status.totalEstimatedLatencyMs.toFixed(1)} ms`],
    ['Input callback gaps', status.inputCallbackGaps],
    ['Input ring overflows', status.inputRingOverflows],
    ['Expander-attenuated frames', status.expanderAttenuatedFrames],
    ['DSP input underruns', status.dspInputUnderruns],
    ['DSP deadline misses', status.dspProcessingDeadlineMisses],
    ['Destination ring overflows', status.destinationRingOverflows],
    ['Monitor ring overflows', status.monitorRingOverflows],
    ['Destination underruns', status.outputRingUnderruns],
    ['Monitor underruns', status.monitorOutputUnderruns],
    ['Destination callback gaps', status.outputCallbackGaps],
    ['Monitor callback gaps', status.monitorCallbackGaps],
    ['Concealed destination frames', status.concealedFrames],
    ['Concealed monitor frames', status.monitorConcealedFrames],
    ['Stream restart attempts', status.streamRestartCount],
    [
      'Input ring current / min / max',
      `${status.currentInputRingFillFrames} / ${status.minimumInputRingFillFrames} / ${status.maximumInputRingFillFrames}`,
    ],
    [
      'Destination ring current / max',
      `${status.currentOutputRingFillFrames} / ${status.maximumOutputRingFillFrames}`,
    ],
    [
      'Monitor ring current / max',
      `${status.currentMonitorRingFillFrames} / ${status.maximumMonitorRingFillFrames}`,
    ],
    ['Maximum DSP block time', `${status.maximumDspProcessingTimeMs.toFixed(3)} ms`],
    [
      'Startup prefill target / actual',
      `${status.startupPrefillTargetFrames} / ${status.startupPrefillAchievedFrames}`,
    ],
    ['Startup prefill timeout', status.startupPrefillTimedOut ? 'Yes' : 'No'],
    [
      'Clock-drift correction',
      `${status.clockDriftCorrectionRatio.toFixed(6)}x (observation only)`,
    ],
    [
      'Observed correction min / max',
      `${status.minimumClockDriftCorrectionRatio.toFixed(6)}x / ${status.maximumClockDriftCorrectionRatio.toFixed(6)}x`,
    ],
    ['Last stream error', status.lastRuntimeError ?? 'None'],
  ];
  return (
    <section className="card diagnostics">
      <div className="section-heading">
        <h2>Pipeline diagnostics</h2>
        <span className="filter-label">Polled at a bounded rate</span>
      </div>
      <dl>
        {items.map(([label, itemValue]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{itemValue}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
