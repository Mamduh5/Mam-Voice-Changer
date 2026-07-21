import { PageNavigation } from './components/PageNavigation';
import { SettingsDiagnosticsPage } from './components/SettingsDiagnosticsPage';
import { TestPage } from './components/TestPage';
import { UsePage } from './components/UsePage';
import { useAudioDevices } from './hooks/useAudioDevices';
import { useAudioParameters } from './hooks/useAudioParameters';
import { useEngineState } from './hooks/useEngineState';
import { usePresets } from './hooks/usePresets';
import { DESKTOP_RUNTIME_UNAVAILABLE, tauriAudioApi } from './services/tauriAudioApi';
import type { ApplicationPage } from './types/audio';
import { isLeavingTest } from './utils/monitoringMode';

export default function App() {
  const desktopRuntimeAvailable = tauriAudioApi.isDesktopRuntimeAvailable();
  const devices = useAudioDevices(desktopRuntimeAvailable);
  const engine = useEngineState(desktopRuntimeAvailable);
  const audioParameters = useAudioParameters(desktopRuntimeAvailable);
  const presets = usePresets(
    desktopRuntimeAvailable,
    audioParameters.beginPresetOperation,
    audioParameters.finishPresetOperation,
  );
  const active = ['running', 'degraded', 'recovering'].includes(engine.status.state);
  const transitioning = ['starting', 'stopping'].includes(engine.status.state);

  const deviceName = (id: string, output = false) =>
    (output ? devices.outputs : devices.inputs).find((device) => device.id === id)?.name ?? '';

  const startUse = () => {
    void engine.start({
      mode: 'use',
      inputId: devices.inputId,
      inputName: deviceName(devices.inputId),
      processedDestinationId: devices.processedDestinationId,
      processedDestinationName: deviceName(devices.processedDestinationId, true),
      reliabilityProfile: devices.reliabilityProfile,
    });
  };

  const startTest = () => {
    void engine.start({
      mode: 'test',
      inputId: devices.inputId,
      inputName: deviceName(devices.inputId),
      monitorId: devices.localMonitorId,
      monitorName: deviceName(devices.localMonitorId, true),
      reliabilityProfile: devices.reliabilityProfile,
    });
  };

  const stop = () => {
    void engine.stop();
  };

  const navigate = (nextPage: ApplicationPage) => {
    if (isLeavingTest(devices.lastPage, nextPage)) {
      void engine.stopTestRoute();
    }
    devices.setLastPage(nextPage);
  };

  const errors: Array<{ id: string; label: string; message: string }> = [];
  if (desktopRuntimeAvailable) {
    if (engine.commandError)
      errors.push({ id: 'engine-command', label: 'Engine command', message: engine.commandError });
    if (engine.status.lastRuntimeError)
      errors.push({
        id: 'engine-runtime',
        label: 'Audio runtime',
        message: engine.status.lastRuntimeError,
      });
    if (engine.pollError)
      errors.push({ id: 'engine-status', label: 'Engine status', message: engine.pollError });
    if (devices.error) errors.push({ id: 'devices', label: 'Settings', message: devices.error });
    if (audioParameters.error)
      errors.push({ id: 'parameters', label: 'Audio settings', message: audioParameters.error });
    if (presets.error) errors.push({ id: 'presets', label: 'Presets', message: presets.error });
  }

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
            <p>Local Windows audio routing - no recording or cloud processing</p>
          </div>
        </div>
        <span className={active ? 'live' : 'idle'}>
          {active ? 'ACTIVE' : engine.status.state.toUpperCase()}
        </span>
      </header>

      <div className="navigation-row">
        <PageNavigation page={devices.lastPage} onNavigate={navigate} />
        <button
          type="button"
          className="refresh"
          disabled={!desktopRuntimeAvailable || active || transitioning || devices.loading}
          onClick={() => void devices.refresh()}
        >
          {devices.loading ? 'Refreshing...' : 'Refresh devices'}
        </button>
      </div>

      {devices.lastPage === 'use' && (
        <UsePage
          inputs={devices.inputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          destinationId={devices.processedDestinationId}
          hasLikelyVirtualDestination={devices.hasLikelyVirtualDestination}
          disabled={!desktopRuntimeAvailable}
          status={engine.status}
          catalog={presets.catalog}
          presetBusy={presets.busy}
          onInputChange={devices.setInputId}
          onDestinationChange={devices.setProcessedDestinationId}
          onApplyPreset={presets.apply}
          onStart={startUse}
          onStop={stop}
        />
      )}
      {devices.lastPage === 'test' && (
        <TestPage
          inputs={devices.inputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          monitorId={devices.localMonitorId}
          disabled={!desktopRuntimeAvailable}
          status={engine.status}
          parameters={audioParameters.parameters}
          catalog={presets.catalog}
          presetBusy={presets.busy}
          presetActions={presets}
          onInputChange={devices.setInputId}
          onMonitorDeviceChange={devices.setLocalMonitorId}
          onParametersChange={audioParameters.update}
          onStart={startTest}
          onStop={stop}
        />
      )}
      {devices.lastPage === 'diagnostics' && (
        <SettingsDiagnosticsPage
          inputs={devices.inputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          destinationId={devices.processedDestinationId}
          monitorId={devices.localMonitorId}
          reliabilityProfile={devices.reliabilityProfile}
          status={engine.status}
          disabled={!desktopRuntimeAvailable}
          onReliabilityProfileChange={devices.setReliabilityProfile}
        />
      )}

      {errors.map((error) => (
        <div className="error" role="alert" key={error.id}>
          <strong>{error.label}:</strong> {error.message}
        </div>
      ))}
      <footer>
        This app is not a Windows microphone device. Receiving apps require a real capture endpoint.
      </footer>
    </main>
  );
}
