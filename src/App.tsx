import { useState } from 'react';
import { PageNavigation, type NavigationPage } from './components/PageNavigation';
import { SettingsDiagnosticsPage } from './components/SettingsDiagnosticsPage';
import { TestPage } from './components/TestPage';
import { UsePage } from './components/UsePage';
import { VoiceLabPage } from './components/VoiceLabPage';
import { useAudioDevices } from './hooks/useAudioDevices';
import { useAudioParameters } from './hooks/useAudioParameters';
import { useEngineState } from './hooks/useEngineState';
import { usePresets } from './hooks/usePresets';
import { useVoiceLab } from './hooks/useVoiceLab';
import { useModelShutdownGuard } from './hooks/useModelShutdownGuard';
import { DESKTOP_RUNTIME_UNAVAILABLE, tauriAudioApi } from './services/tauriAudioApi';
import { isLeavingTest } from './utils/monitoringMode';

export default function App() {
  useModelShutdownGuard();
  const [voiceLabOpen, setVoiceLabOpen] = useState(false);
  const desktopRuntimeAvailable = tauriAudioApi.isDesktopRuntimeAvailable();
  const devices = useAudioDevices(desktopRuntimeAvailable);
  const engine = useEngineState(desktopRuntimeAvailable);
  const audioParameters = useAudioParameters(desktopRuntimeAvailable);
  const presets = usePresets(
    desktopRuntimeAvailable,
    audioParameters.beginPresetOperation,
    audioParameters.finishPresetOperation,
  );
  const voiceLab = useVoiceLab(voiceLabOpen && desktopRuntimeAvailable, audioParameters.parameters);
  const active = ['running', 'degraded', 'recovering'].includes(engine.status.state);
  const transitioning = ['starting', 'stopping'].includes(engine.status.state);

  const deviceName = (id: string, output = false) =>
    (output ? devices.outputs : devices.physicalInputs).find((device) => device.id === id)?.name ??
    '';

  const startUse = () => {
    void engine.start({
      mode: 'use',
      inputId: devices.inputId,
      inputName: deviceName(devices.inputId),
      externalRouteId: devices.selectedRoute?.routeId ?? '',
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

  const activePage: NavigationPage = voiceLabOpen ? 'voiceLab' : devices.lastPage;

  const navigate = (nextPage: NavigationPage) => {
    if (isLeavingTest(devices.lastPage, nextPage === 'voiceLab' ? 'use' : nextPage)) {
      void engine.stopTestRoute();
    }
    if (nextPage === 'voiceLab') {
      voiceLab.initialize(audioParameters.parameters);
      setVoiceLabOpen(true);
    } else {
      setVoiceLabOpen(false);
      devices.setLastPage(nextPage);
    }
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
    if (voiceLabOpen && voiceLab.error)
      errors.push({ id: 'voice-lab', label: 'Voice Lab', message: voiceLab.error });
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
            <p>Local Windows routing and an isolated offline Voice Lab</p>
          </div>
        </div>
        <span className={active ? 'live' : 'idle'}>
          {active ? 'ACTIVE' : engine.status.state.toUpperCase()}
        </span>
      </header>

      <div className="navigation-row">
        <PageNavigation page={activePage} onNavigate={navigate} />
        <button
          type="button"
          className="refresh"
          disabled={!desktopRuntimeAvailable || active || transitioning || devices.loading}
          onClick={() => void devices.refresh()}
        >
          {devices.loading ? 'Refreshing...' : 'Refresh devices'}
        </button>
      </div>

      {!voiceLabOpen && devices.lastPage === 'use' && (
        <UsePage
          physicalInputs={devices.physicalInputs}
          inputs={devices.inputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          routes={devices.externalRoutes}
          selectedRoute={devices.selectedRoute}
          validation={devices.routeValidation}
          draftRouteId={devices.draftRouteId}
          draftPlaybackId={devices.draftPlaybackId}
          draftCaptureId={devices.draftCaptureId}
          confirmPhysicalEndpoints={devices.confirmPhysicalEndpoints}
          routeBusy={devices.routeBusy}
          disabled={!desktopRuntimeAvailable}
          status={engine.status}
          catalog={presets.catalog}
          presetBusy={presets.busy}
          onInputChange={devices.setInputId}
          onDraftRouteChange={devices.setDraftRouteId}
          onDraftPlaybackChange={devices.setDraftPlaybackId}
          onDraftCaptureChange={devices.setDraftCaptureId}
          onConfirmPhysicalEndpointsChange={devices.setConfirmPhysicalEndpoints}
          onSaveRoute={devices.saveExternalRoute}
          onDeleteRoute={devices.deleteExternalRoute}
          onValidateRoute={devices.validateSelectedRoute}
          onApplyPreset={presets.apply}
          onStart={startUse}
          onStop={stop}
        />
      )}
      {!voiceLabOpen && devices.lastPage === 'test' && (
        <TestPage
          inputs={devices.physicalInputs}
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
      {!voiceLabOpen && devices.lastPage === 'diagnostics' && (
        <SettingsDiagnosticsPage
          inputs={devices.inputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          monitorId={devices.localMonitorId}
          selectedRoute={devices.selectedRoute}
          routeValidation={devices.routeValidation}
          reliabilityProfile={devices.reliabilityProfile}
          status={engine.status}
          disabled={!desktopRuntimeAvailable}
          onReliabilityProfileChange={devices.setReliabilityProfile}
        />
      )}
      {voiceLabOpen && (
        <VoiceLabPage
          inputs={devices.physicalInputs}
          outputs={devices.outputs}
          defaultInputId={devices.inputId}
          defaultOutputId={devices.localMonitorId}
          disabled={!desktopRuntimeAvailable}
          liveActive={engine.status.state !== 'stopped'}
          parameters={voiceLab.parameters}
          status={voiceLab.status}
          catalog={presets.catalog}
          busy={voiceLab.busy || presets.busy}
          renderStale={voiceLab.renderStale}
          onParametersChange={voiceLab.updateParameters}
          onApplyPreset={voiceLab.applyPreset}
          onRecord={voiceLab.record}
          onStopRecording={voiceLab.stopRecording}
          onImport={voiceLab.importWav}
          onRender={voiceLab.render}
          onPreview={voiceLab.preview}
          onStopPreview={voiceLab.stopPreview}
          onStopAudio={voiceLab.stopAudio}
          onSavePreset={presets.saveVoiceLab}
          onApplyLive={audioParameters.applySnapshot}
          onExport={voiceLab.exportWav}
          onClear={voiceLab.clear}
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
