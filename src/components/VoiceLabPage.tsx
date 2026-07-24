import { useState } from 'react';
import { useVoiceDataset } from '../hooks/useVoiceDataset';
import { useVoiceModels } from '../hooks/useVoiceModels';
import { useVoiceProfiles } from '../hooks/useVoiceProfiles';
import type { AudioDevice } from '../types/audio';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';
import type { VoiceLabClipSummary, VoiceLabClipVersion, VoiceLabStatus } from '../types/voiceLab';
import { DeviceSelector } from './DeviceSelector';
import { DspControls } from './DspControls';
import { VoiceDatasetPage } from './voice-dataset/VoiceDatasetPage';
import { VoiceModelPage } from './voice-model/VoiceModelPage';
import { VoiceProfilesPage } from './voice-profile/VoiceProfilesPage';
import { SyntheticAudioNotice } from './voice-model/SyntheticAudioNotice';

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
  chromeHidden: boolean;
  onChromeAutomaticActivity: () => void;
  onChromeNavigationFocus: () => void;
};

function formatDuration(milliseconds: number) {
  return `${(milliseconds / 1_000).toFixed(2)} s`;
}

function formatSampleRate(sampleRate: number | null | undefined) {
  return sampleRate ? `${(sampleRate / 1_000).toFixed(1)} kHz` : 'Unknown rate';
}

function ClipCard({ title, clip }: { title: string; clip: VoiceLabClipSummary }) {
  return (
    <article className="voice-lab-clip">
      <div className="voice-lab-active-clip-heading">
        <span>
          Active clip: <strong>{title}</strong>
        </span>
        <span>{formatDuration(clip.durationMs)}</span>
      </div>
      <div className="voice-lab-waveform" aria-label={`${title} waveform`}>
        {clip.waveform.map((peak, index) => (
          <span key={index} style={{ height: `${Math.max(4, peak * 100)}%` }} />
        ))}
      </div>
    </article>
  );
}

function selectedDevice(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id);
}

export function VoiceLabPage(props: Props) {
  type VoiceLabSection = 'compare' | 'profiles' | 'dataset' | 'models';
  const [section, setSection] = useState<VoiceLabSection>('compare');
  const models = useVoiceModels(
    (section === 'profiles' || section === 'models') && !props.disabled,
  );
  const profiles = useVoiceProfiles(!props.disabled, models.status);
  const dataset = useVoiceDataset(
    (section === 'dataset' || section === 'models') && !props.disabled,
    profiles.selectedProfileId,
    profiles.acceptStatus,
  );
  const [inputSelection, setInputSelection] = useState('');
  const [outputSelection, setOutputSelection] = useState('');
  const [looping, setLooping] = useState(false);
  const [presetId, setPresetId] = useState('');
  const [presetName, setPresetName] = useState('');
  const [comparisonVersion, setComparisonVersion] = useState<VoiceLabClipVersion>('original');

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
  const activeClip =
    comparisonVersion === 'processed' && props.status.processed
      ? props.status.processed
      : props.status.original;
  const sections = [
    ['compare', 'Compare'],
    ['profiles', 'Profiles'],
    ['dataset', 'Dataset'],
    ['models', 'Models'],
  ] as const;
  const switchSection = (next: VoiceLabSection) => {
    if (section === 'dataset') void dataset.stopPreview();
    if (section === 'compare') void props.onStopAudio();
    setSection(next);
  };
  const sectionNavigation = (
    <nav
      className={`voice-lab-sections application-chrome-secondary${
        props.chromeHidden ? ' application-chrome-secondary--hidden' : ''
      }`}
      data-application-chrome
      aria-label="Voice Lab sections"
      role="tablist"
      onFocusCapture={props.onChromeNavigationFocus}
      onKeyDown={(event) => {
        if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
        event.preventDefault();
        const current = sections.findIndex(([id]) => id === section);
        const nextIndex =
          event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? sections.length - 1
              : (current + (event.key === 'ArrowRight' ? 1 : -1) + sections.length) %
                sections.length;
        switchSection(sections[nextIndex][0]);
        event.currentTarget
          .querySelectorAll<HTMLButtonElement>('[role="tab"]')
          .item(nextIndex)
          .focus();
      }}
    >
      {sections.map(([id, label]) => (
        <button
          type="button"
          role="tab"
          id={`voice-lab-tab-${id}`}
          aria-controls={`voice-lab-panel-${id}`}
          aria-selected={section === id}
          tabIndex={section === id ? 0 : -1}
          key={id}
          className={section === id ? 'active' : ''}
          onClick={() => switchSection(id)}
        >
          {label}
        </button>
      ))}
    </nav>
  );

  if (section === 'profiles') {
    return (
      <div className="page-stack">
        {sectionNavigation}
        <div id="voice-lab-panel-profiles" role="tabpanel" aria-labelledby="voice-lab-tab-profiles">
          <VoiceProfilesPage profiles={profiles} />
        </div>
      </div>
    );
  }

  if (section === 'dataset') {
    return (
      <div className="page-stack">
        {sectionNavigation}
        <div id="voice-lab-panel-dataset" role="tabpanel" aria-labelledby="voice-lab-tab-dataset">
          <VoiceDatasetPage
            dataset={dataset}
            profiles={profiles}
            inputs={props.inputs}
            outputs={props.outputs}
            defaultInputId={props.defaultInputId}
            defaultOutputId={props.defaultOutputId}
            disabled={props.disabled}
            liveActive={props.liveActive}
            onOpenProfiles={() => switchSection('profiles')}
          />
        </div>
      </div>
    );
  }

  if (section === 'models') {
    return (
      <div className="page-stack">
        {sectionNavigation}
        <div id="voice-lab-panel-models" role="tabpanel" aria-labelledby="voice-lab-tab-models">
          <VoiceModelPage
            dataset={dataset}
            profiles={profiles}
            models={models}
            hasVoiceLabSource={Boolean(props.status.original)}
            disabled={props.disabled}
            onOpenProfiles={() => switchSection('profiles')}
          />
        </div>
      </div>
    );
  }

  return (
    <div
      className="page-stack voice-lab-page"
      id="voice-lab-panel-compare"
      role="tabpanel"
      aria-labelledby="voice-lab-tab-compare"
    >
      {sectionNavigation}
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

      <div
        className="workspace-primary-actions voice-lab-primary-actions"
        aria-label="Voice Lab primary actions"
      >
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
          className="start"
          disabled={audioUnavailable || !props.status.original}
          onClick={() => void props.onRender()}
        >
          Render processed
        </button>
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
        <button
          type="button"
          className="danger-outline"
          disabled={
            props.disabled || props.busy || (!props.status.original && !props.status.capture.active)
          }
          onClick={() => void props.onClear()}
        >
          Clear temporary audio
        </button>
      </div>

      <div className="voice-lab-compare-layout">
        <div className="voice-lab-source-column">
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
              <button
                type="button"
                disabled={props.disabled || props.busy || props.status.capture.active}
                onClick={() => void props.onImport()}
              >
                Import WAV
              </button>
            </div>
            {props.status.capture.droppedFrames > 0 && (
              <small className="warning">
                Capture dropped {props.status.capture.droppedFrames} frames. Record again for a
                clean source.
              </small>
            )}
          </section>

          <section className="card voice-lab-comparison">
            <div className="voice-lab-compare-heading">
              <div className="section-heading">
                <h2>2. Compare</h2>
                {props.status.preview.active && <span>Playing {props.status.preview.kind}</span>}
              </div>
              <div className="voice-lab-clip-selector" aria-label="Comparison clip">
                <button
                  type="button"
                  className={comparisonVersion === 'original' ? 'active' : ''}
                  aria-pressed={comparisonVersion === 'original'}
                  disabled={!props.status.original}
                  onClick={() => setComparisonVersion('original')}
                >
                  Original{' '}
                  {props.status.original
                    ? formatDuration(props.status.original.durationMs)
                    : 'Empty'}
                </button>
                <button
                  type="button"
                  className={comparisonVersion === 'processed' ? 'active' : ''}
                  aria-pressed={comparisonVersion === 'processed'}
                  disabled={!props.status.processed}
                  onClick={() => setComparisonVersion('processed')}
                >
                  {props.status.processedSynthetic ? 'Processed · Synthetic' : 'Processed'}{' '}
                  {props.status.processed
                    ? formatDuration(props.status.processed.durationMs)
                    : 'Empty'}
                </button>
              </div>
            </div>
            {comparisonVersion === 'processed' && props.status.processed ? (
              <ClipCard
                title={props.status.processedSynthetic ? 'Processed · Synthetic' : 'Processed'}
                clip={props.status.processed}
              />
            ) : props.status.original ? (
              <ClipCard title="Original" clip={props.status.original} />
            ) : (
              <div className="voice-lab-empty-clip">
                Record or import a dry source to see its waveform.
              </div>
            )}
            {!props.status.processed && (
              <p className="voice-lab-empty-processed">
                Processed is empty until you render the dry source.
              </p>
            )}
            <div className="voice-lab-progress" aria-label="Preview position">
              <span style={{ width: `${previewPosition}%` }} />
            </div>
            <label className="limiter-toggle voice-lab-loop-control">
              <input
                type="checkbox"
                checked={looping}
                disabled={props.status.preview.active}
                onChange={(event) => setLooping(event.target.checked)}
              />
              <span>Loop replay</span>
            </label>
            {(props.status.preview.clipSampleRate || props.status.preview.outputSampleRate) && (
              <div className="preview-metadata-grid" role="status">
                <div className="preview-metadata-item">
                  <span className="metadata-label">Clip rate</span>
                  <strong>{formatSampleRate(props.status.preview.clipSampleRate)}</strong>
                </div>
                <div className="preview-metadata-item">
                  <span className="metadata-label">Output rate</span>
                  <strong>{formatSampleRate(props.status.preview.outputSampleRate)}</strong>
                </div>
                <div className="preview-metadata-item">
                  <span className="metadata-label">Channels / format</span>
                  <strong>
                    {props.status.preview.outputChannels ?? 'Unknown'} /{' '}
                    {props.status.preview.outputSampleFormat ?? 'Unknown'}
                  </strong>
                </div>
                <div className="preview-metadata-item">
                  <span className="metadata-label">Resampling</span>
                  <strong>{props.status.preview.resamplingActive ? 'Active' : 'Inactive'}</strong>
                </div>
              </div>
            )}
            {activeClip && (
              <details className="advanced-section clip-metadata">
                <summary>Clip technical metadata</summary>
                <small>
                  {activeClip.sourceName} · {activeClip.frames.toLocaleString()} frames · peak{' '}
                  {activeClip.peak.toFixed(3)}
                </small>
              </details>
            )}
          </section>
          {props.status.processedSynthetic && <SyntheticAudioNotice />}
        </div>

        <div className="voice-lab-configuration-column">
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

          <details className="card advanced-section compare-advanced">
            <summary>Advanced DSP controls</summary>
            <DspControls
              parameters={props.parameters}
              disabled={props.disabled || props.busy}
              onChange={props.onParametersChange}
            />
          </details>

          <section className="card voice-lab-finish">
            <div className="section-heading">
              <h2>4. Render and publish</h2>
              {props.renderStale && <span className="warning">Processed clip is stale</span>}
            </div>
            <div className="voice-lab-actions">
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
                disabled={
                  props.disabled || props.busy || !props.status.processed || props.renderStale
                }
                onClick={() => void props.onExport('processed')}
              >
                Export processed WAV
              </button>
            </div>
            {props.status.renderMetadata && (
              <details className="advanced-section">
                <summary>Render diagnostics</summary>
                <small>
                  Offline DSP: {props.status.renderMetadata.blockFrames}-frame blocks ·{' '}
                  {props.status.renderMetadata.latencyFrames} latency frames aligned
                </small>
              </details>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
