import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import type {
  AudioDevice,
  ExternalAudioRoute,
  ExternalAudioRouteCatalog,
  RouteCompatibilityResult,
} from '../types/audio';
import { stoppedStatus } from '../types/engine';
import { defaultAudioParameters } from '../types/parameters';
import { EngineControls } from './EngineControls';
import { PageNavigation } from './PageNavigation';
import { RoutingNotice } from './RoutingNotice';
import { SettingsDiagnosticsPage } from './SettingsDiagnosticsPage';
import { TestPage } from './TestPage';
import { UsePage } from './UsePage';

function device(
  id: string,
  name: string,
  direction: 'input' | 'output',
  isLikelyVirtual: boolean,
  isDefault = false,
): AudioDevice {
  return {
    id,
    name,
    direction,
    isDefault,
    isLikelyVirtual,
    virtualFamily: isLikelyVirtual ? 'studio-route' : null,
    minimumSampleRate: 44_100,
    maximumSampleRate: 48_000,
    commonSampleRates: [44_100, 48_000],
    channelCounts: [2],
  };
}

const input = device('microphone', 'Built-in microphone', 'input', false, true);
const playback = device('virtual-playback', 'Studio Virtual Input', 'output', true);
const capture = device('virtual-capture', 'Studio Virtual Output', 'input', true);
const monitor = device('headphones', 'Headphones', 'output', false, true);
const route: ExternalAudioRoute = {
  routeId: 'external-route',
  displayName: 'Studio Virtual Input -> Studio Virtual Output',
  playbackDevice: playback,
  captureDevice: capture,
  candidateCaptureDevices: [],
  pairingConfidence: 'exact',
  pairingSource: 'knownPattern',
  validationStatus: 'ready',
  compatibility: {
    commonVirtualSampleRates: [44_100, 48_000],
    details: 'Both virtual endpoints advertise 44100 Hz and 48000 Hz.',
  },
  manual: false,
};
const routeCatalog: ExternalAudioRouteCatalog = {
  routes: [route],
  virtualPlaybackDevices: [playback],
  virtualCaptureDevices: [capture],
  unpairedCaptureDevices: [],
  selectedRouteId: route.routeId,
  restorationWarning: null,
};
const readyValidation: RouteCompatibilityResult = {
  routeId: route.routeId,
  readiness: 'ready',
  message: 'Route configuration is ready.',
  negotiatedSampleRate: 48_000,
  captureEndpointAvailable: true,
};
const asyncAction = vi.fn(async () => true);

function useProps() {
  return {
    physicalInputs: [input],
    inputs: [input, capture],
    outputs: [playback, monitor],
    inputId: input.id,
    routes: routeCatalog,
    selectedRoute: route,
    validation: readyValidation,
    draftRouteId: route.routeId,
    draftPlaybackId: playback.id,
    draftCaptureId: capture.id,
    confirmPhysicalEndpoints: false,
    routeBusy: false,
    disabled: false,
    status: stoppedStatus,
    catalog: null,
    presetBusy: false,
    onInputChange: vi.fn(),
    onDraftRouteChange: vi.fn(),
    onDraftPlaybackChange: vi.fn(),
    onDraftCaptureChange: vi.fn(),
    onConfirmPhysicalEndpointsChange: vi.fn(),
    onSaveRoute: asyncAction,
    onDeleteRoute: asyncAction,
    onValidateRoute: vi.fn(),
    onApplyPreset: asyncAction,
    onStart: vi.fn(),
    onStop: vi.fn(),
  };
}

describe('application pages', () => {
  it('renders a ready external route and precise receiving-app instruction', () => {
    const markup = renderToStaticMarkup(<UsePage {...useProps()} />);

    expect(markup).toContain('External route setup');
    expect(markup).toContain('Virtual playback endpoint');
    expect(markup).toContain('Paired capture endpoint');
    expect(markup).toContain('Studio Virtual Output');
    expect(markup).toContain('Route readiness: Ready');
    expect(markup).toContain('Start using');
    expect(markup).toContain('Shared voice configuration');
    expect(markup).not.toContain('Hear myself');
    expect(markup).not.toContain('Local monitoring');
    expect(markup).not.toContain('Discord connected');
  });

  it('shows no-route guidance and keeps Start using disabled', () => {
    const markup = renderToStaticMarkup(
      <UsePage
        {...useProps()}
        routes={{ ...routeCatalog, routes: [], selectedRouteId: null }}
        selectedRoute={null}
        validation={{
          routeId: null,
          readiness: 'missingPlayback',
          message: 'No virtual audio route is available.',
          negotiatedSampleRate: null,
          captureEndpointAvailable: false,
        }}
        draftRouteId=""
        draftPlaybackId=""
        draftCaptureId=""
      />,
    );

    expect(markup).toContain('No virtual audio route is available');
    expect(markup).toContain('Install or enable a compatible Windows virtual audio device');
    expect(markup).toMatch(/<button class="start" disabled="">Start using<\/button>/);
  });

  it('surfaces ambiguous candidates and supports manual pairing controls', () => {
    const ambiguous = {
      ...route,
      routeId: 'ambiguous',
      captureDevice: null,
      candidateCaptureDevices: [capture, { ...capture, id: 'capture-two' }],
      pairingConfidence: 'ambiguous' as const,
      pairingSource: 'none' as const,
      validationStatus: 'ambiguousPair' as const,
    };
    const markup = renderToStaticMarkup(
      <UsePage
        {...useProps()}
        routes={{ ...routeCatalog, routes: [ambiguous], selectedRouteId: null }}
        selectedRoute={null}
        validation={{
          routeId: ambiguous.routeId,
          readiness: 'ambiguousPair',
          message: 'Multiple capture endpoints are equally plausible. Save a manual pair.',
          negotiatedSampleRate: null,
          captureEndpointAvailable: false,
        }}
        draftRouteId={ambiguous.routeId}
      />,
    );

    expect(markup).toContain('Pairing is ambiguous');
    expect(markup).toContain('Manual playback/capture pair');
    expect(markup).toContain('Save external route');
  });

  it.each([
    ['missingCapture', 'Missing paired capture endpoint'],
    ['incompatibleFormat', 'Incompatible sample rates'],
  ] as const)('renders %s readiness and keeps Start using disabled', (readiness, label) => {
    const markup = renderToStaticMarkup(
      <UsePage
        {...useProps()}
        validation={{
          routeId: route.routeId,
          readiness,
          message: `Focused ${readiness} guidance.`,
          negotiatedSampleRate: null,
          captureEndpointAvailable: readiness === 'incompatibleFormat',
        }}
      />,
    );

    expect(markup).toContain(`Route readiness: ${label}`);
    expect(markup).toContain(`Focused ${readiness} guidance.`);
    expect(markup).toMatch(/<button class="start" disabled="">Start using<\/button>/);
  });

  it('keeps Test direct, local-monitor-only, and blocked while Use is active', () => {
    const baseProps = {
      inputs: [input],
      outputs: [monitor],
      inputId: input.id,
      monitorId: monitor.id,
      disabled: false,
      parameters: defaultAudioParameters,
      catalog: null,
      presetBusy: false,
      presetActions: {
        apply: asyncAction,
        save: asyncAction,
        rename: asyncAction,
        duplicate: asyncAction,
        remove: asyncAction,
        reset: asyncAction,
      },
      onInputChange: vi.fn(),
      onMonitorDeviceChange: vi.fn(),
      onParametersChange: vi.fn(),
      onStart: vi.fn(),
      onStop: vi.fn(),
    };
    const stopped = renderToStaticMarkup(<TestPage {...baseProps} status={stoppedStatus} />);
    const useActive = renderToStaticMarkup(
      <TestPage
        {...baseProps}
        status={{ ...stoppedStatus, state: 'running', routePurpose: 'use' }}
      />,
    );
    const starting = renderToStaticMarkup(
      <TestPage
        {...baseProps}
        status={{
          ...stoppedStatus,
          state: 'starting',
          routePurpose: 'test',
          message: 'Starting Test monitoring',
        }}
      />,
    );
    const running = renderToStaticMarkup(
      <TestPage
        {...baseProps}
        status={{
          ...stoppedStatus,
          state: 'running',
          routePurpose: 'test',
          message: 'Test monitoring is active',
        }}
      />,
    );

    expect(stopped).toContain('Test monitor device');
    expect(stopped).toContain('Start hearing test');
    expect(stopped).toContain('Processing');
    expect(stopped).toContain('Monitoring off');
    expect(stopped).toContain('Use headphones');
    expect(stopped).not.toContain('Enable temporary test monitoring');
    expect(stopped).not.toContain('monitor-toggle prominent');
    expect(stopped).not.toContain('External route setup');
    expect(starting.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(3);
    expect(running).toContain('Stop test');
    expect(useActive).toMatch(/<button class="start" disabled="">Start hearing test<\/button>/);
  });

  it('renders route diagnostics without claiming receiver connectivity', () => {
    const navigation = renderToStaticMarkup(
      <PageNavigation page="diagnostics" onNavigate={vi.fn()} />,
    );
    const diagnostics = renderToStaticMarkup(
      <SettingsDiagnosticsPage
        inputs={[input, capture]}
        outputs={[playback, monitor]}
        inputId={input.id}
        monitorId={monitor.id}
        selectedRoute={route}
        routeValidation={readyValidation}
        reliabilityProfile="reliable"
        status={{ ...stoppedStatus, inputCallbackGaps: 3, concealedFrames: 5 }}
        disabled={false}
        onReliabilityProfileChange={vi.fn()}
      />,
    );

    expect(navigation).toContain('Settings &amp; Diagnostics');
    expect(navigation).toContain('Use');
    expect(navigation).toContain('Test');
    expect(diagnostics).toContain('Expected paired capture endpoint');
    expect(diagnostics).toContain('exact / knownPattern');
    expect(diagnostics).toContain('Capture endpoint available');
    expect(diagnostics).toContain('48000 Hz');
    expect(diagnostics).toContain('Input callback gaps');
    expect(diagnostics).toContain('>3<');
    expect(diagnostics).toContain('Concealed destination frames');
    expect(diagnostics).toContain('>5<');
    expect(diagnostics).toContain('Last stream error');
    expect(diagnostics).toContain('None');
    expect(diagnostics).toContain('does not prove');
    expect(diagnostics).not.toContain('Discord connected');
  });

  it('renders route-specific recovery controls and honest routing notice', () => {
    const recovering = renderToStaticMarkup(
      <EngineControls
        status={{
          ...stoppedStatus,
          state: 'recovering',
          routePurpose: 'use',
          message: 'Recovering audio route',
        }}
        purpose="use"
        canStart={false}
        startLabel="Start using"
        stopLabel="Stop using"
        description="Virtual playback only"
        onStart={vi.fn()}
        onStop={vi.fn()}
      />,
    );
    const notice = renderToStaticMarkup(<RoutingNotice route={route} />);

    expect(recovering).toContain('Stop using');
    expect(notice).toContain('Receiving-application microphone');
    expect(notice).toContain('does not prove');
  });

  it('clears a prior stream error with the next clean status', () => {
    const page = (status: typeof stoppedStatus) =>
      renderToStaticMarkup(
        <SettingsDiagnosticsPage
          inputs={[input, capture]}
          outputs={[playback, monitor]}
          inputId={input.id}
          monitorId={monitor.id}
          selectedRoute={route}
          routeValidation={readyValidation}
          reliabilityProfile="balanced"
          status={status}
          disabled={false}
          onReliabilityProfileChange={vi.fn()}
        />,
      );

    expect(page({ ...stoppedStatus, lastRuntimeError: 'endpoint disconnected' })).toContain(
      'endpoint disconnected',
    );
    expect(page(stoppedStatus)).not.toContain('endpoint disconnected');
  });
});
