import { useState } from 'react';
import type { PromptSelection } from '../../types/voiceDataset';

export function DatasetImportExportPanel({
  busy,
  selection,
  onImport,
  onExport,
}: {
  busy: boolean;
  selection: PromptSelection;
  onImport: (selection: PromptSelection) => Promise<boolean>;
  onExport: (options: { includeRejected: boolean; includeRawMasters: boolean }) => Promise<boolean>;
}) {
  const [includeRejected, setIncludeRejected] = useState(false);
  const [includeRaw, setIncludeRaw] = useState(false);
  return (
    <section className="card dataset-transfer">
      <div className="section-heading">
        <h2>Import and explicit export</h2>
        <span>Local only · no upload</span>
      </div>
      <p>
        Import creates a canonical mono 48 kHz PCM24 copy and never changes the source file.
        Imported takes remain pending. Export includes accepted, non-excluded selected WAVs by
        default.
      </p>
      <div className="voice-lab-actions">
        <button type="button" disabled={busy} onClick={() => void onImport(selection)}>
          Import recordings
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void onExport({ includeRejected, includeRawMasters: includeRaw })}
        >
          Export dataset
        </button>
      </div>
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={includeRejected}
          onChange={(event) => setIncludeRejected(event.target.checked)}
        />
        Advanced: include rejected takes
      </label>
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={includeRaw}
          onChange={(event) => setIncludeRaw(event.target.checked)}
        />
        Advanced: export raw masters instead of selected trimmed files
      </label>
      <small>
        Recorded consent audio, pending takes, excluded takes, application settings, presets,
        machine paths, and external routing details are excluded. Exported copies must be deleted
        separately.
      </small>
    </section>
  );
}
