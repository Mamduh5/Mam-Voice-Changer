import { useEffect, useRef, useState } from 'react';
import { AudioHealthPanel } from './components/AudioHealthPanel';
import { ApplicationChrome } from './components/ApplicationChrome';
import { PageNavigation, type NavigationPage } from './components/PageNavigation';
import { QuickSetup } from './components/QuickSetup';
import { SettingsDiagnosticsPage } from './components/SettingsDiagnosticsPage';
import { TestPage } from './components/TestPage';
import { UsePage } from './components/UsePage';
import { VoiceLabPage } from './components/VoiceLabPage';
import { useAudioDevices } from './hooks/useAudioDevices';
import { useAudioHealth } from './hooks/useAudioHealth';
import { isBackgroundChromeToggle, useAutoHideChrome } from './hooks/useAutoHideChrome';
import { useAudioParameters } from './hooks/useAudioParameters';
import { useEngineState } from './hooks/useEngineState';
import { usePresets } from './hooks/usePresets';
import { useProductInformation } from './hooks/useProductInformation';
import { useVoiceLab } from './hooks/useVoiceLab';
import { useModelShutdownGuard } from './hooks/useModelShutdownGuard';
import { DESKTOP_RUNTIME_UNAVAILABLE, tauriAudioApi } from './services/tauriAudioApi';
import { defaultAudioParameters } from './types/parameters';
import { isLeavingTest } from './utils/monitoringMode';
import { classifyRecoverableError } from './utils/productDiagnostics';

export default function App() {
  useModelShutdownGuard();
  const chrome = useAutoHideChrome();
  const backgroundPointerStart = useRef<{ x: number; y: number } | null>(null);
  const setupAutoHandled = useRef(false);
  const [voiceLabOpen, setVoiceLabOpen] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const desktopRuntimeAvailable = tauriAudioApi.isDesktopRuntimeAvailable();
  const devices = useAudioDevices(desktopRuntimeAvailable);
  const engine = useEngineState(desktopRuntimeAvailable);
  const audioParameters = useAudioParameters(desktopRuntimeAvailable);
  const product = useProductInformation(desktopRuntimeAvailable);
  const presets = usePresets(
    desktopRuntimeAvailable,
    audioParameters.beginPresetOperation,
    audioParameters.finishPresetOperation,
  );
  const voiceLab = useVoiceLab(voiceLabOpen && desktopRuntimeAvailable, audioParameters.parameters);
  const health = useAudioHealth(engine.status);
  const active = ['running', 'degraded', 'recovering'].includes(engine.status.state);
  const transitioning = ['starting', 'stopping'].includes(engine.status.state);

  useEffect(() => {
    if (
      desktopRuntimeAvailable &&
      !devices.loading &&
      !devices.firstRunSetupDismissed &&
      !setupAutoHandled.current
    ) {
      setupAutoHandled.current = true;
      setSetupOpen(true);
    }
  }, [desktopRuntimeAvailable, devices.firstRunSetupDismissed, devices.loading]);

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

  const applySafeDefaults = () => audioParameters.applySnapshot(defaultAudioParameters);

  const closeSetup = (doNotShowAutomatically: boolean) => {
    setupAutoHandled.current = true;
    setSetupOpen(false);
    if (doNotShowAutomatically) devices.setFirstRunSetupDismissed(true);
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
    if (product.error)
      errors.push({ id: 'product', label: 'Product information', message: product.error });
  }

  return (
    <main
      className="application-background"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          backgroundPointerStart.current = { x: event.clientX, y: event.clientY };
        }
      }}
      onPointerUp={(event) => {
        const start = backgroundPointerStart.current;
        backgroundPointerStart.current = null;
        if (
          start &&
          isBackgroundChromeToggle(
            event.target,
            event.currentTarget,
            Math.hypot(event.clientX - start.x, event.clientY - start.y),
            Boolean(window.getSelection()?.toString()),
          )
        ) {
          chrome.toggleBackground();
        }
      }}
    >
      <div className="application-content">
        {!desktopRuntimeAvailable && (
          <div className="runtime-notice" role="status">
            {DESKTOP_RUNTIME_UNAVAILABLE}
          </div>
        )}
        <ApplicationChrome
          hidden={!chrome.state.visible}
          onAutomaticReveal={chrome.automaticShow}
          onNavigationFocus={chrome.navigationFocus}
          onScheduleAutomaticHide={chrome.scheduleAutomaticHide}
        >
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
            <div className="navigation-actions">
              <button type="button" className="secondary" onClick={() => setSetupOpen(true)}>
                Setup / help
              </button>
              <button
                type="button"
                className="refresh"
                disabled={!desktopRuntimeAvailable || active || transitioning || devices.loading}
                onClick={() => void devices.refresh()}
              >
                {devices.loading ? 'Refreshing...' : 'Refresh devices'}
              </button>
            </div>
          </div>
        </ApplicationChrome>

        {!voiceLabOpen && devices.lastPage === 'use' && (
          <UsePage
            physicalInputs={devices.physicalInputs}
            inputs={devices.inputs}
            outputs={devices.outputs}
            inputId={devices.inputId}
            unavailableInputName={devices.unavailableInputName}
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
            unavailableInputName={devices.unavailableInputName}
            unavailableMonitorName={devices.unavailableLocalMonitorName}
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
            unavailableInputName={devices.unavailableInputName}
            monitorId={devices.localMonitorId}
            unavailableMonitorName={devices.unavailableLocalMonitorName}
            selectedRoute={devices.selectedRoute}
            routeValidation={devices.routeValidation}
            reliabilityProfile={devices.reliabilityProfile}
            status={engine.status}
            parameters={audioParameters.parameters}
            product={product.information}
            lastSuccessfulConfiguration={devices.lastSuccessfulConfiguration}
            inputActivityDetected={health.inputActivityDetected}
            disabled={!desktopRuntimeAvailable}
            changesDisabled={!desktopRuntimeAvailable || active || transitioning || devices.loading}
            onReliabilityProfileChange={devices.setReliabilityProfile}
            onRefresh={devices.refresh}
            onClearClipping={engine.clearClipping}
            onResetSafeDefaults={applySafeDefaults}
            onOpenSetup={() => setSetupOpen(true)}
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
            chromeHidden={!chrome.state.visible}
            onChromeAutomaticActivity={chrome.automaticShow}
            onChromeNavigationFocus={chrome.navigationFocus}
          />
        )}

        {!voiceLabOpen && (
          <AudioHealthPanel
            status={engine.status}
            inputActivityDetected={health.inputActivityDetected}
            onClearClipping={() => void engine.clearClipping()}
          />
        )}

        {errors.map((error) => {
          const recovery = classifyRecoverableError(error.message);
          return (
            <div className="error recoverable-error" role="alert" key={error.id}>
              <div>
                <strong>{error.label}:</strong> {error.message}
                {recovery && <small>Next action: {recovery.action}</small>}
              </div>
              <div className="error-actions">
                {error.id === 'engine-command' && (
                  <button type="button" onClick={engine.clearCommandError}>
                    Dismiss
                  </button>
                )}
                <button
                  type="button"
                  disabled={!desktopRuntimeAvailable || active || transitioning || devices.loading}
                  onClick={() => void devices.refresh()}
                >
                  Refresh devices
                </button>
              </div>
            </div>
          );
        })}
        <footer>
          This app is not a Windows microphone device. Receiving apps require a real capture
          endpoint.
        </footer>
      </div>
      {setupOpen && (
        <QuickSetup
          inputs={devices.physicalInputs}
          outputs={devices.outputs}
          inputId={devices.inputId}
          monitorId={devices.localMonitorId}
          unavailableInputName={devices.unavailableInputName}
          unavailableMonitorName={devices.unavailableLocalMonitorName}
          routes={devices.externalRoutes}
          draftRouteId={devices.draftRouteId}
          draftPlaybackId={devices.draftPlaybackId}
          draftCaptureId={devices.draftCaptureId}
          routeValidation={devices.routeValidation}
          status={engine.status}
          disabled={!desktopRuntimeAvailable}
          onInputChange={devices.setInputId}
          onMonitorChange={devices.setLocalMonitorId}
          onDraftRouteChange={devices.setDraftRouteId}
          onDraftPlaybackChange={devices.setDraftPlaybackId}
          onDraftCaptureChange={devices.setDraftCaptureId}
          onSaveRoute={devices.saveExternalRoute}
          onApplySafeDefaults={applySafeDefaults}
          onStart={() => {
            setSetupOpen(false);
            startUse();
          }}
          onClose={closeSetup}
        />
      )}
    </main>
  );
}
