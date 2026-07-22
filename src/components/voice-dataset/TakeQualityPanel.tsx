import type { TakeQualityReport } from '../../types/voiceDataset';
import { qualityLabels } from '../../utils/datasetQualityLabels';

export function TakeQualityPanel({ report }: { report: TakeQualityReport }) {
  return (
    <div className={`dataset-quality ${report.classification}`}>
      <strong>{qualityLabels[report.classification]}</strong>
      <ul>
        {report.reasons.map((reason) => (
          <li key={reason.code}>
            <code>{reason.code}</code> — {reason.guidance}
          </li>
        ))}
      </ul>
      <dl className="dataset-quality-metrics">
        <div>
          <dt>Peak</dt>
          <dd>{report.peakAmplitude.toFixed(3)}</dd>
        </div>
        <div>
          <dt>RMS</dt>
          <dd>{report.rmsLevel.toFixed(3)}</dd>
        </div>
        <div>
          <dt>Clipped samples</dt>
          <dd>{report.clippedSampleCount}</dd>
        </div>
        <div>
          <dt>DC offset</dt>
          <dd>{report.dcOffset.toFixed(4)}</dd>
        </div>
        <div>
          <dt>Leading silence</dt>
          <dd>{report.leadingSilenceMs} ms</dd>
        </div>
        <div>
          <dt>Trailing silence</dt>
          <dd>{report.trailingSilenceMs} ms</dd>
        </div>
        <div>
          <dt>Active speech estimate</dt>
          <dd>{(report.estimatedActiveSpeechRatio * 100).toFixed(1)}%</dd>
        </div>
        <div>
          <dt>Noise floor estimate</dt>
          <dd>{report.estimatedBackgroundNoiseFloor.toFixed(4)}</dd>
        </div>
        <div>
          <dt>Heuristic SNR</dt>
          <dd>{report.heuristicSignalToNoiseDb.toFixed(1)} dB</dd>
        </div>
        <div>
          <dt>Dropout regions</dt>
          <dd>{report.consecutiveZeroRegions}</dd>
        </div>
        <div>
          <dt>Queue overflow</dt>
          <dd>{report.recordingQueueOverflowCount}</dd>
        </div>
        <div>
          <dt>Callback gaps</dt>
          <dd>{report.callbackGaps}</dd>
        </div>
      </dl>
      <small>
        Active speech, noise floor, and SNR are heuristic estimates. They do not prove speaker
        identity, transcript accuracy, studio quality, or model readiness.
      </small>
    </div>
  );
}
