import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import { defaultAudioParameters } from '../types/parameters';
import { stoppedStatus } from '../types/engine';
import type { AudioDevice } from '../types/audio';
import { PageNavigation } from './PageNavigation';
import { RoutingNotice } from './RoutingNotice';
import { SettingsDiagnosticsPage } from './SettingsDiagnosticsPage';
import { TestPage } from './TestPage';
import { UsePage } from './UsePage';
import { EngineControls } from './EngineControls';

const input: AudioDevice = {
  id: 'microphone',
  name: 'Built-in microphone',
  isDefault: true,
  isLikelyVirtual: false,
};
const destination: AudioDevice = {
  id: 'virtual-output',
  name: 'Virtual playback endpoint',
  isDefault: false,
  isLikelyVirtual: true,
};
const monitor: AudioDevice = {
  id: 'headphones',
  name: 'Headphones',
  isDefault: true,
  isLikelyVirtual: false,
};
const asyncAction = vi.fn(async () => true);

describe('application pages', () => {
  it('renders Use with a processed destination and no local-monitor concepts', () => {
    const markup = renderToStaticMarkup(
      <UsePage
        inputs={[input]}
        outputs={[destination, monitor]}
        inputId={input.id}
        destinationId={destination.id}
        hasLikelyVirtualDestination
        disabled={false}
        status={stoppedStatus}
        catalog={null}
        presetBusy={false}
        onInputChange={vi.fn()}
        onDestinationChange={vi.fn()}
        onApplyPreset={asyncAction}
        onStart={vi.fn()}
        onStop={vi.fn()}
      />,
    );

    expect(markup).toContain('Processed destination');
    expect(markup).toContain('Start using');
    expect(markup).toContain('Processed output');
    expect(markup).not.toContain('Hear myself');
    expect(markup).not.toContain('Local monitoring');
    expect(markup).not.toContain('Local monitor device');
    expect(markup).not.toContain('Headphone output');
  });

  it('renders Test as direct explicit monitoring and locks routing while starting', () => {
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
    const off = renderToStaticMarkup(<TestPage {...baseProps} status={stoppedStatus} />);
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

    expect(off).toContain('Monitoring off');
    expect(off).toContain('Use headphones');
    expect(off).toContain('Test monitor device');
    expect(off).toContain('Start hearing test');
    expect(off).not.toContain('Enable temporary test monitoring');
    expect(off).not.toContain('monitor-toggle prominent');
    expect(off).not.toContain('temporary');
    expect(running).toContain('Stop test');
    expect(starting.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(3);
  });

  it('renders navigation, reliability selection, counters, and the honest no-virtual notice', () => {
    const navigation = renderToStaticMarkup(
      <PageNavigation page="diagnostics" onNavigate={vi.fn()} />,
    );
    const diagnostics = renderToStaticMarkup(
      <SettingsDiagnosticsPage
        inputs={[input]}
        outputs={[destination, monitor]}
        inputId={input.id}
        destinationId={destination.id}
        monitorId={monitor.id}
        reliabilityProfile="reliable"
        status={{ ...stoppedStatus, inputCallbackGaps: 3, concealedFrames: 5 }}
        disabled={false}
        onReliabilityProfileChange={vi.fn()}
      />,
    );
    const notice = renderToStaticMarkup(<RoutingNotice hasLikelyVirtualDestination={false} />);

    expect(navigation).toContain('Use');
    expect(navigation).toContain('Test');
    expect(navigation).toContain('Settings &amp; Diagnostics');
    expect(diagnostics).toContain('Reliable');
    expect(diagnostics).toContain('Input callback gaps');
    expect(diagnostics).toContain('>3<');
    expect(diagnostics).toContain('Concealed destination frames');
    expect(diagnostics).toContain('>5<');
    expect(notice).toContain('Discord can select only Windows capture devices');
    expect(notice).toContain('future driver support');
  });

  it('renders recovery controls and clears a prior stream error with the next clean status', () => {
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
        description="Processed destination only"
        onStart={vi.fn()}
        onStop={vi.fn()}
      />,
    );
    const failed = renderToStaticMarkup(
      <SettingsDiagnosticsPage
        inputs={[input]}
        outputs={[monitor]}
        inputId={input.id}
        destinationId=""
        monitorId={monitor.id}
        reliabilityProfile="balanced"
        status={{ ...stoppedStatus, lastRuntimeError: 'endpoint disconnected' }}
        disabled={false}
        onReliabilityProfileChange={vi.fn()}
      />,
    );
    const cleared = renderToStaticMarkup(
      <SettingsDiagnosticsPage
        inputs={[input]}
        outputs={[monitor]}
        inputId={input.id}
        destinationId=""
        monitorId={monitor.id}
        reliabilityProfile="balanced"
        status={stoppedStatus}
        disabled={false}
        onReliabilityProfileChange={vi.fn()}
      />,
    );

    expect(recovering).toContain('Stop using');
    expect(failed).toContain('endpoint disconnected');
    expect(cleared).not.toContain('endpoint disconnected');
    expect(cleared).toContain('Last stream error');
    expect(cleared).toContain('None');
  });
});
