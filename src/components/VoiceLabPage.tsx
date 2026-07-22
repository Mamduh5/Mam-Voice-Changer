import { useState } from 'react';
import { useVoiceDataset } from '../hooks/useVoiceDataset';
import type { AudioDevice } from '../types/audio';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';
import type { VoiceLabClipSummary, VoiceLabClipVersion, VoiceLabStatus } from '../types/voiceLab';
import { DeviceSelector } from './DeviceSelector';
import { DspControls } from './DspControls';
import { VoiceDatasetPage } from './voice-dataset/VoiceDatasetPage';

type Props = {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  defaultInputId: string;
  defaultOutputId: string;
  disabled: boolean;
  liveActive: boolean;
  parameters: AudioParameters;
  status: VoiceLabStatus;
  catalog: PresetCatalog | null;
  busy: boolean;
  renderStale: boolean;
  onParametersChange: (changes: Partial<AudioParameters>) => void;
  onApplyPreset: (parameters: AudioParameters) => void;
  onRecord: (inputId: string, inputName: string) => Promise<boolean>;
  onStopRecording: () => Promise<boolean>;
  onImport: () => Promise<boolean>;
  onRender: () => Promise<boolean>;
  onPreview: (
    version: VoiceLabClipVersion,
    outputId: string,
    outputName: string,
    looping: boolean,
  ) => Promise<boolean>;
  onStopPreview: () => Promise<boolean>;
  onStopAudio: () => Promise<boolean>;
  onSavePreset: (name: string, parameters: AudioParameters) => Promise<boolean>;
  onApplyLive: (parameters: AudioParameters) => Promise<boolean>;
  onExport: (version: VoiceLabClipVersion) => Promise<boolean>;
  onClear: () => Promise<boolean>;
};

function formatDuration(milliseconds: number) {
  return `${(milliseconds / 1_000).toFixed(2)} s`;
}

function ClipCard({ title, clip }: { title: string; clip: VoiceLabClipSummary | null }) {
  return (
    <article className="voice-lab-clip">
      <div className="section-heading">
        <h3>{title}</h3>
        <span>{clip ? formatDuration(clip.durationMs) : 'Empty'}</span>
      </div>
      <div className="voice-lab-waveform" aria-label={`${title} waveform`}>
        {(clip?.waveform ?? Array.from({ length: 32 }, () => 0)).map((peak, index) => (
          <span key={index} style={{ height: `${Math.max(4, peak * 100)}%` }} />
        ))}
      </div>
      {clip && (
        <small>
          {clip.sourceName} · {clip.sampleRate.toLocaleString()} Hz ·{' '}
          {clip.channels === 1 ? 'mono' : 'stereo'}
        </small>
      )}
    </article>
  );
}

function selectedDevice(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id);
}

export function VoiceLabPage(props: Props) {
  const [section, setSection] = useState<'compare' | 'dataset'>('compare');
  const dataset = useVoiceDataset(section === 'dataset' && !props.disabled);
  const [inputSelection, setInputSelection] = useState('');
  const [outputSelection, setOutputSelection] = useState('');
  const [looping, setLooping] = useState(false);
  const [presetId, setPresetId] = useState('');
  const [presetName, setPresetName] = useState('');

  const inputId = selectedDevice(props.inputs, inputSelection)
    ? inputSelection
    : props.defaultInputId;
  const outputId = selectedDevice(props.outputs, outputSelection)
    ? outputSelection
    : props.defaultOutputId;
  const effectivePresetId = props.catalog?.presets.some((preset) => preset.id === presetId)
    ? presetId
    : (props.catalog?.presets[0]?.id ?? '');
  const input = selectedDevice(props.inputs, inputId);
  const output = selectedDevice(props.outputs, outputId);
  const selectedPreset = props.catalog?.presets.find((preset) => preset.id === effectivePresetId);
  const audioUnavailable =
    props.disabled || props.busy || props.liveActive || props.status.capture.active;
  const previewPosition = props.status.preview.durationMs
    ? Math.min(100, (props.status.preview.positionMs / props.status.preview.durationMs) * 100)
    : 0;

  if (section === 'dataset') {
    return (
      <div className="page-stack">
        <nav className="voice-lab-sections" aria-label="Voice Lab sections">
          <button
            type="button"
            onClick={() => {
              void dataset.stopPreview();
              setSection('compare');
            }}
          >
            Compare
          </button>
          <button type="button" className="active" aria-current="page">
            Dataset
          </button>
        </nav>
        <VoiceDatasetPage
          dataset={dataset}
          inputs={props.inputs}
          outputs={props.outputs}
          defaultInputId={props.defaultInputId}
          defaultOutputId={props.defaultOutputId}
          disabled={props.disabled}
          liveActive={props.liveActive}
        />
      </div>
    );
  }

  return (
    <div className="page-stack voice-lab-page">
      <nav className="voice-lab-sections" aria-label="Voice Lab sections">
        <button type="button" className="active" aria-current="page">
          Compare
        </button>
        <button
          type="button"
          onClick={() => {
            void props.onStopAudio();
            setSection('dataset');
          }}
        >
          Dataset
        </button>
      </nav>
      <section className="card voice-lab-intro">
        <div>
          <p className="eyebrow">Isolated offline workspace</p>
          <h2>Voice Lab</h2>
          <p>
            Capture or import a dry clip, render it through the existing Mam DSP, and compare it
            without changing Use or Test.
          </p>
        </div>
        <span className="bounded-label">15 seconds max · memory only</span>
      </section>

      {props.liveActive && (
        <div className="voice-lab-notice" role="status">
          Stop the active Use/Test route to record, render, or preview. Lab editing, import, preset
          save, export, and clear remain isolated.
        </div>
      )}

      <section className="card voice-lab-source">
        <div className="section-heading">
          <h2>1. Dry source</h2>
          <span>{props.status.capture.active ? 'Recording…' : 'Ready'}</span>
        </div>
        <div className="voice-lab-device-grid">
          <DeviceSelector
            label="Recording microphone"
            value={inputId}
            devices={props.inputs}
            disabled={props.disabled || props.busy || props.status.capture.active}
            onChange={setInputSelection}
          />
          <DeviceSelector
            label="Preview output"
            value={outputId}
            devices={props.outputs}
            disabled={props.disabled || props.busy || props.status.preview.active}
            onChange={setOutputSelection}
          />
        </div>
        <div className="voice-lab-actions">
          {!props.status.capture.active ? (
            <button
              type="button"
              className="start"
              disabled={audioUnavailable || !input}
              onClick={() => input && void props.onRecord(input.id, input.name)}
            >
              Record dry sample
            </button>
          ) : (
            <button type="button" className="stop" onClick={() => void props.onStopRecording()}>
              Stop recording
            </button>
          )}
          <button
            type="button"
            disabled={props.disabled || props.busy || props.status.capture.active}
            onClick={() => void props.onImport()}
          >
            Import WAV
          </button>
          <button
            type="button"
            className="danger-outline"
            disabled={
              props.disabled ||
              props.busy ||
              (!props.status.original && !props.status.capture.active)
            }
            onClick={() => void props.onClear()}
          >
            Clear temporary audio
          </button>
        </div>
        {props.status.capture.droppedFrames > 0 && (
          <small className="warning">
            Capture dropped {props.status.capture.droppedFrames} frames. Record again for a clean
            source.
          </small>
        )}
      </section>

      <section className="voice-lab-comparison">
        <ClipCard title="Original" clip={props.status.original} />
        <ClipCard title="Processed" clip={props.status.processed} />
      </section>

      <section className="card voice-lab-transport">
        <div className="section-heading">
          <h2>2. Compare</h2>
          {props.status.preview.active && <span>Playing {props.status.preview.kind}</span>}
        </div>
        <div className="voice-lab-actions">
          <button
            type="button"
            disabled={audioUnavailable || !output || !props.status.original}
            onClick={() =>
              output && void props.onPreview('original', output.id, output.name, looping)
            }
          >
            Play original
          </button>
          <button
            type="button"
            disabled={audioUnavailable || !output || !props.status.processed || props.renderStale}
            onClick={() =>
              output && void props.onPreview('processed', output.id, output.name, looping)
            }
          >
            Play processed
          </button>
          <button
            type="button"
            disabled={!props.status.preview.active}
            onClick={() => void props.onStopPreview()}
          >
            Stop preview
          </button>
          <label className="limiter-toggle">
            <input
              type="checkbox"
              checked={looping}
              disabled={props.status.preview.active}
              onChange={(event) => setLooping(event.target.checked)}
            />
            Loop replay
          </label>
        </div>
        <div className="voice-lab-progress" aria-label="Preview position">
          <span style={{ width: `${previewPosition}%` }} />
        </div>
      </section>

      <section className="card voice-lab-presets">
        <div className="section-heading">
          <h2>3. Lab preset</h2>
          <span>Local until explicitly applied</span>
        </div>
        <div className="voice-lab-preset-grid">
          <label>
            Existing preset
            <select
              value={effectivePresetId}
              disabled={props.disabled || props.busy || !props.catalog}
              onChange={(event) => setPresetId(event.target.value)}
            >
              {(props.catalog?.presets ?? []).map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.name}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            disabled={props.disabled || props.busy || !selectedPreset}
            onClick={() => selectedPreset && props.onApplyPreset(selectedPreset.parameters)}
          >
            Apply preset to Lab
          </button>
          <label>
            New preset name
            <input
              type="text"
              maxLength={64}
              value={presetName}
              disabled={props.disabled || props.busy}
              onChange={(event) => setPresetName(event.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={props.disabled || props.busy || !presetName.trim()}
            onClick={async () => {
              if (await props.onSavePreset(presetName, props.parameters)) setPresetName('');
            }}
          >
            Save as new preset
          </button>
        </div>
      </section>

      <DspControls
        parameters={props.parameters}
        disabled={props.disabled || props.busy}
        onChange={props.onParametersChange}
      />

      <section className="card voice-lab-finish">
        <div className="section-heading">
          <h2>4. Render and publish</h2>
          {props.renderStale && <span className="warning">Processed clip is stale</span>}
        </div>
        <div className="voice-lab-actions">
          <button
            type="button"
            className="start"
            disabled={audioUnavailable || !props.status.original}
            onClick={() => void props.onRender()}
          >
            Render processed
          </button>
          <button
            type="button"
            disabled={props.disabled || props.busy}
            onClick={() => void props.onApplyLive(props.parameters)}
          >
            Apply to live settings
          </button>
          <button
            type="button"
            disabled={props.disabled || props.busy || !props.status.original}
            onClick={() => void props.onExport('original')}
          >
            Export original WAV
          </button>
          <button
            type="button"
            disabled={props.disabled || props.busy || !props.status.processed || props.renderStale}
            onClick={() => void props.onExport('processed')}
          >
            Export processed WAV
          </button>
        </div>
        {props.status.renderMetadata && (
          <small>
            Offline DSP: {props.status.renderMetadata.blockFrames}-frame blocks ·{' '}
            {props.status.renderMetadata.latencyFrames} latency frames aligned
          </small>
        )}
      </section>
    </div>
  );
}
