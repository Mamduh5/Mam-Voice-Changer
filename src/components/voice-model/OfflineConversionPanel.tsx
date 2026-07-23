import { useOfflineConversion } from '../../hooks/useOfflineConversion';
import type { VoiceModelStatus, VoiceModelArtifact } from '../../types/voiceModel';

export function OfflineConversionPanel({
  status,
  artifact,
  hasVoiceLabSource,
  busy,
  onConvert,
  onCancel,
  onLoad,
  onClear,
}: {
  status: VoiceModelStatus;
  artifact: VoiceModelArtifact | null;
  hasVoiceLabSource: boolean;
  busy: boolean;
  onConvert: () => Promise<unknown>;
  onCancel: () => Promise<unknown>;
  onLoad: (resultId: string) => Promise<unknown>;
  onClear: () => Promise<unknown>;
}) {
  const conversion = useOfflineConversion(status);
  const eligible = artifact
    ? ['unevaluated', 'evaluationInProgress', 'approvedForOfflineUse'].includes(
        artifact.approvalStatus,
      )
    : false;
  return (
    <section className="card model-conversion-panel">
      <div className="section-heading">
        <h2>6. Offline synthetic conversion</h2>
        <span>{conversion.active ? 'Converting offline' : 'No realtime inference'}</span>
      </div>
      <p>
        Use the current Voice Lab original clip as source speech. The generated WAV remains local,
        temporary, synthetic, and unavailable to Use, Test, Discord, or external routes.
      </p>
      {!hasVoiceLabSource && (
        <p className="warning">Record or import a Voice Lab source clip first.</p>
      )}
      <div className="voice-lab-actions">
        {!conversion.active ? (
          <button
            type="button"
            className="start"
            disabled={busy || !eligible || !hasVoiceLabSource}
            onClick={() => void onConvert()}
          >
            Convert test phrase
          </button>
        ) : (
          <button type="button" className="stop" onClick={() => void onCancel()}>
            Cancel offline conversion
          </button>
        )}
        {conversion.result && (
          <>
            <button type="button" onClick={() => void onLoad(conversion.result!.resultId)}>
              Load synthetic result in Voice Lab Compare
            </button>
            <button type="button" onClick={() => void onClear()}>
              Delete temporary synthetic output
            </button>
          </>
        )}
      </div>
      {conversion.result && (
        <div className="model-result">
          <strong>Synthetic</strong>
          <span>{(conversion.result.durationMs / 1_000).toFixed(2)} seconds</span>
          <span>Peak {conversion.result.peak.toFixed(3)}</span>
          <span>{conversion.result.referenceTakeIds.length} recorded reference take(s)</span>
        </div>
      )}
    </section>
  );
}
