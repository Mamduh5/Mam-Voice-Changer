import { DeviceSelector } from './components/DeviceSelector';
import { DiagnosticsPanel } from './components/DiagnosticsPanel';
import { EngineControls } from './components/EngineControls';
import { LevelMeter } from './components/LevelMeter';
import { useAudioDevices } from './hooks/useAudioDevices';
import { useEngineState } from './hooks/useEngineState';
import { DESKTOP_RUNTIME_UNAVAILABLE, tauriAudioApi } from './services/tauriAudioApi';

export default function App() {
  const desktopRuntimeAvailable = tauriAudioApi.isDesktopRuntimeAvailable();
  const devices = useAudioDevices(desktopRuntimeAvailable);
  const engine = useEngineState(desktopRuntimeAvailable);
  const running = engine.status.state === 'running';
  const busy = engine.status.state === 'starting' || engine.status.state === 'stopping';
  const error = desktopRuntimeAvailable
    ? (engine.commandError ?? engine.status.lastRuntimeError ?? devices.error)
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
            <p>Local Windows audio routing Â· no recording or cloud processing</p>
          </div>
        </div>
        <span className={running ? 'live' : 'idle'}>
          {running ? 'â— LIVE' : `â—‹ ${engine.status.state.toUpperCase()}`}
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
            {devices.loading ? 'Refreshingâ€¦' : 'Refresh devices'}
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
          <span>â†’</span>
          <DeviceSelector
            label="Windows output (normally CABLE Input)"
            value={devices.outputId}
            devices={devices.outputs}
            disabled={!desktopRuntimeAvailable || running || busy}
            onChange={devices.setOutputId}
          />
        </div>
      </section>

      <div className="grid">
        <section className="card">
          <h2>Levels</h2>
          <LevelMeter label="Input" value={engine.status.inputLevel} />
          <LevelMeter label="Output" value={engine.status.outputLevel} />
          <p className="hint">
            Output is intentionally unmodified in Milestone 1 so the live routing path can be
            validated before effects are enabled.
          </p>
        </section>
        <section className="card deferred">
          <h2>Voice effects</h2>
          <strong>Pitch and presets are not enabled yet</strong>
          <p>
            The previous pitch control changed amplitude instead of pitch and has been removed.
            Genuine pitch processing will follow clean passthrough validation.
          </p>
          <button disabled>Effects unavailable in Milestone 1</button>
        </section>
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

