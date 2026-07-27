import { describe, expect, it } from 'vitest';
import type { AudioDevice, ExternalAudioRoute } from '../types/audio';
import { stoppedStatus } from '../types/engine';
import { defaultAudioParameters } from '../types/parameters';
import {
  classifyRecoverableError,
  createDiagnosticReport,
  evaluateAudioConfiguration,
} from './productDiagnostics';

const input: AudioDevice = {
  id: 'mic',
  name: 'Microphone',
  direction: 'input',
  isDefault: true,
  isLikelyVirtual: false,
  virtualFamily: null,
  minimumSampleRate: 48_000,
  maximumSampleRate: 48_000,
  commonSampleRates: [48_000],
  channelCounts: [1],
};

const playback: AudioDevice = {
  ...input,
  id: 'playback',
  name: 'Virtual playback',
  direction: 'output',
  isDefault: false,
  isLikelyVirtual: true,
  channelCounts: [2],
};

const route: ExternalAudioRoute = {
  routeId: 'route',
  displayName: 'Saved pair',
  playbackDevice: playback,
  captureDevice: { ...input, id: 'capture', name: 'Virtual capture', isLikelyVirtual: true },
  candidateCaptureDevices: [],
  pairingConfidence: 'manual',
  pairingSource: 'manual',
  validationStatus: 'ready',
  compatibility: { commonVirtualSampleRates: [48_000], details: 'Compatible' },
  manual: true,
};

describe('product diagnostics', () => {
  it('keeps an unavailable saved microphone explicit and actionable', () => {
    const result = evaluateAudioConfiguration({
      inputs: [input],
      outputs: [playback],
      inputId: '',
      unavailableInputName: 'Travel microphone',
      monitorId: '',
      unavailableMonitorName: null,
      selectedRoute: route,
      routeValidation: {
        routeId: 'route',
        readiness: 'ready',
        message: 'Ready',
        negotiatedSampleRate: 48_000,
        captureEndpointAvailable: true,
      },
      status: stoppedStatus,
      inputSignalDetected: null,
      checkedAt: '2026-01-01T00:00:00.000Z',
    });
    expect(result.outcome).toBe('notReady');
    expect(result.items.find((item) => item.label === 'Input microphone')?.detail).toContain(
      'Travel microphone',
    );
  });

  it('classifies normal device failures with recovery actions', () => {
    expect(classifyRecoverableError('Device is already in use')?.category).toBe('exclusiveAccess');
    expect(classifyRecoverableError('Unsupported sample rate')?.category).toBe('unsupportedFormat');
  });

  it('creates a bounded path-free report and redacts path-like device values', () => {
    const report = createDiagnosticReport({
      product: {
        productName: 'Mam Voice Changer',
        applicationVersion: '0.1.0',
        prototype: true,
        operatingSystem: 'windows',
        architecture: 'x86_64',
        backendVersion: 'mam-voice-changer-rust 0.1.0',
      },
      status: stoppedStatus,
      parameters: defaultAudioParameters,
      inputDeviceName: 'C:\\Users\\person\\private microphone',
      processedOutputName: 'Virtual playback',
      monitorDeviceName: null,
      inputActivityDetected: null,
      lastError: null,
      lastSuccessfulConfiguration: null,
      timestamp: '2026-01-01T00:00:00.000Z',
    });
    expect(report).toContain('[redacted path-like value]');
    expect(report).not.toContain('C:\\Users\\person');
    expect(report).not.toContain('administrator');
    expect(report).not.toContain('ratings');
  });
});
