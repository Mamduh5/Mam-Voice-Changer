import { useState } from 'react';
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
import { shouldStopTemporaryTestMonitoring } from './utils/monitoringMode';

type EngineMode = 'use' | 'test' | null;

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
  const [temporaryTestMonitoring, setTemporaryTestMonitoring] = useState(false);
  const [engineMode, setEngineMode] = useState<EngineMode>(null);
  const active = ['running', 'degraded', 'recovering'].includes(engine.status.state);
  const transitioning = ['starting', 'stopping'].includes(engine.status.state);

  const deviceName = (id: string, output = false) =>
    (output ? devices.outputs : devices.inputs).find((device) => device.id === id)?.name ?? '';

  const startUse = () => {
    setEngineMode('use');
    void engine.start({
      inputId: devices.inputId,
      inputName: deviceName(devices.inputId),
      processedDestinationId: devices.processedDestinationId || null,
      processedDestinationName: devices.processedDestinationId
        ? deviceName(devices.processedDestinationId, true)
        : null,
      localMonitorId: devices.localMonitorEnabled ? devices.localMonitorId || null : null,
      localMonitorName:
        devices.localMonitorEnabled && devices.localMonitorId
          ? deviceName(devices.localMonitorId, true)
          : null,
      reliabilityProfile: devices.reliabilityProfile,
    });
  };

  const startTest = () => {
    setEngineMode('test');
    void engine.start({
      inputId: devices.inputId,
      inputName: deviceName(devices.inputId),
      processedDestinationId: null,
      processedDestinationName: null,
      localMonitorId: temporaryTestMonitoring ? devices.localMonitorId || null : null,
      localMonitorName:
        temporaryTestMonitoring && devices.localMonitorId
          ? deviceName(devices.localMonitorId, true)
          : null,
      reliabilityProfile: devices.reliabilityProfile,
    });
  };

  const stop = () => {
    setEngineMode(null);
    void engine.stop();
  };

  const navigate = (nextPage: ApplicationPage) => {
    if (devices.lastPage === 'test' && nextPage !== 'test') {
      setTemporaryTestMonitoring(false);
      if (
        shouldStopTemporaryTestMonitoring(
          devices.lastPage,
          nextPage,
          engineMode,
          engine.status.state,
        )
      ) {
        stop();
      }
    }
    if (
      nextPage === 'test' &&
      engineMode === 'use' &&
      !['stopped', 'error'].includes(engine.status.state)
    ) {
      stop();
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
          monitorId={devices.localMonitorId}
          monitorEnabled={devices.localMonitorEnabled}
          hasLikelyVirtualDestination={devices.hasLikelyVirtualDestination}
          disabled={!desktopRuntimeAvailable}
          status={engine.status}
          catalog={presets.catalog}
          presetBusy={presets.busy}
          onInputChange={devices.setInputId}
          onDestinationChange={devices.setProcessedDestinationId}
          onMonitorDeviceChange={devices.setLocalMonitorId}
          onMonitorEnabledChange={devices.setLocalMonitorEnabled}
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
          temporaryMonitoring={temporaryTestMonitoring}
          routeIsTest={engineMode === 'test'}
          disabled={!desktopRuntimeAvailable}
          status={engine.status}
          parameters={audioParameters.parameters}
          catalog={presets.catalog}
          presetBusy={presets.busy}
          presetActions={presets}
          onInputChange={devices.setInputId}
          onMonitorDeviceChange={devices.setLocalMonitorId}
          onTemporaryMonitoringChange={setTemporaryTestMonitoring}
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
