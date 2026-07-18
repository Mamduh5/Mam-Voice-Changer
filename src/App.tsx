import { DeviceSelector } from './components/DeviceSelector';
import { DiagnosticsPanel } from './components/DiagnosticsPanel';
import { DspControls } from './components/DspControls';
import { EngineControls } from './components/EngineControls';
import { LevelMeter } from './components/LevelMeter';
import { PresetControls } from './components/PresetControls';
import { useAudioDevices } from './hooks/useAudioDevices';
import { useAudioParameters } from './hooks/useAudioParameters';
import { useEngineState } from './hooks/useEngineState';
import { usePresets } from './hooks/usePresets';
import { DESKTOP_RUNTIME_UNAVAILABLE, tauriAudioApi } from './services/tauriAudioApi';

export default function App() {
  const desktopRuntimeAvailable = tauriAudioApi.isDesktopRuntimeAvailable();
  const devices = useAudioDevices(desktopRuntimeAvailable);
  const engine = useEngineState(desktopRuntimeAvailable);
  const audioParameters = useAudioParameters(desktopRuntimeAvailable);
  const presets = usePresets(
    desktopRuntimeAvailable,
    audioParameters.settle,
    audioParameters.replace,
  );
  const running = engine.status.state === 'running';
  const busy = engine.status.state === 'starting' || engine.status.state === 'stopping';
  const error = desktopRuntimeAvailable
    ? (engine.commandError ??
      engine.status.lastRuntimeError ??
      devices.error ??
      audioParameters.error ??
      presets.error)
    : null;

  return (
    <main>
      {!desktopRuntimeAvailable && (
        <div className="runtime-notice" role="status">
          {DESKTOP_RUNTIME_UNAVAILABLE}
        </div>
      )}

      <header>
        <div className="brand">
          <span className="logo">M</span>
          <div>
            <h1>Mam Voice Changer</h1>
            <p>Local Windows audio routing · no recording or cloud processing</p>
          </div>
        </div>
        <span className={running ? 'live' : 'idle'}>
          {running ? '● LIVE' : '○ ' + engine.status.state.toUpperCase()}
        </span>
      </header>

      <section className="routing card">
        <div className="section-heading">
          <h2>Audio routing</h2>
          <button
            className="refresh"
            onClick={() => void devices.refresh()}
            disabled={!desktopRuntimeAvailable || running || busy || devices.loading}
          >
            {devices.loading ? 'Refreshing…' : 'Refresh devices'}
          </button>
        </div>
        <div className="route">
          <DeviceSelector
            label="Physical microphone"
            value={devices.inputId}
            devices={devices.inputs}
            disabled={!desktopRuntimeAvailable || running || busy}
            onChange={devices.setInputId}
          />
          <span>→</span>
          <DeviceSelector
            label="Windows output (normally CABLE Input)"
            value={devices.outputId}
            devices={devices.outputs}
            disabled={!desktopRuntimeAvailable || running || busy}
            onChange={devices.setOutputId}
          />
        </div>
      </section>

      <PresetControls
        catalog={presets.catalog}
        parameters={audioParameters.parameters}
        disabled={!desktopRuntimeAvailable}
        busy={presets.busy}
        onApply={presets.apply}
        onSave={presets.save}
        onRename={presets.rename}
        onDuplicate={presets.duplicate}
        onDelete={presets.remove}
        onReset={presets.reset}
      />

      <div className="grid">
        <section className="card">
          <h2>Levels</h2>
          <LevelMeter label="Input" value={engine.status.inputLevel} />
          <LevelMeter label="Output" value={engine.status.outputLevel} />
        </section>
        <DspControls
          parameters={audioParameters.parameters}
          disabled={!desktopRuntimeAvailable}
          onChange={audioParameters.update}
        />
      </div>

      <DiagnosticsPanel status={engine.status} />
      <EngineControls
        status={engine.status}
        canStart={Boolean(desktopRuntimeAvailable && devices.inputId && devices.outputId)}
        onStart={() => void engine.start(devices.inputId, devices.outputId)}
        onStop={() => void engine.stop()}
      />

      {error && (
        <div className="error" role="alert">
          {error}
        </div>
      )}
      <footer>
        For VB-CABLE, choose CABLE Input above and CABLE Output as the microphone in the target
        application.
      </footer>
    </main>
  );
}
