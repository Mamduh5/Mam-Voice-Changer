import type {
  AudioDevice,
  ExternalAudioRoute,
  LastSuccessfulConfiguration,
  RouteCompatibilityResult,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { AudioParameters } from '../types/parameters';
import type { ProductInformation } from '../types/product';

export type DiagnosticOutcome = 'ready' | 'readyWithWarnings' | 'notReady';
export type DiagnosticSeverity = 'pass' | 'warning' | 'failure';

export type DiagnosticItem = {
  severity: DiagnosticSeverity;
  label: string;
  detail: string;
  action: string | null;
};

export type ConfigurationDiagnostic = {
  outcome: DiagnosticOutcome;
  checkedAt: string;
  items: DiagnosticItem[];
};

export type RecoverableError = {
  category:
    | 'deviceUnavailable'
    | 'permission'
    | 'exclusiveAccess'
    | 'unsupportedFormat'
    | 'streamStart'
    | 'backendUnavailable'
    | 'unknown';
  action: string;
};

export function classifyRecoverableError(message: string | null): RecoverableError | null {
  if (!message) return null;
  const normalized = message.toLowerCase();
  if (
    normalized.includes('no audio device') ||
    normalized.includes('unavailable') ||
    normalized.includes('disconnected') ||
    normalized.includes('removed')
  ) {
    return {
      category: 'deviceUnavailable',
      action: 'Refresh devices, then deliberately reselect the missing endpoint.',
    };
  }
  if (normalized.includes('permission') || normalized.includes('access denied')) {
    return {
      category: 'permission',
      action: 'Enable desktop microphone access in Windows Privacy settings, then retry.',
    };
  }
  if (normalized.includes('exclusive') || normalized.includes('in use')) {
    return {
      category: 'exclusiveAccess',
      action: 'Close the application holding exclusive access, then retry.',
    };
  }
  if (
    normalized.includes('sample rate') ||
    normalized.includes('channel') ||
    normalized.includes('format')
  ) {
    return {
      category: 'unsupportedFormat',
      action: 'Set the devices to a common Windows format, preferably 48 kHz, then refresh.',
    };
  }
  if (normalized.includes('start') || normalized.includes('stream')) {
    return {
      category: 'streamStart',
      action: 'Stop processing, refresh devices, and retry after checking Windows Sound settings.',
    };
  }
  if (normalized.includes('worker') || normalized.includes('backend')) {
    return {
      category: 'backendUnavailable',
      action: 'Stop processing and retry. Restart the application only if the retry still fails.',
    };
  }
  return {
    category: 'unknown',
    action: 'Review the technical detail, refresh devices, and retry the operation.',
  };
}

function item(
  severity: DiagnosticSeverity,
  label: string,
  detail: string,
  action: string | null = null,
): DiagnosticItem {
  return { severity, label, detail, action };
}

function selectedDevice(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id) ?? null;
}

function normalizedDeviceName(name: string) {
  return name.trim().toLowerCase();
}

export function evaluateAudioConfiguration({
  inputs,
  outputs,
  inputId,
  unavailableInputName,
  monitorId,
  unavailableMonitorName,
  selectedRoute,
  routeValidation,
  status,
  inputSignalDetected,
  checkedAt = new Date().toISOString(),
}: {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  inputId: string;
  unavailableInputName: string | null;
  monitorId: string;
  unavailableMonitorName: string | null;
  selectedRoute: ExternalAudioRoute | null;
  routeValidation: RouteCompatibilityResult;
  status: EngineStatus;
  inputSignalDetected: boolean | null;
  checkedAt?: string;
}): ConfigurationDiagnostic {
  const items: DiagnosticItem[] = [];
  const input = selectedDevice(inputs, inputId);
  if (input) {
    items.push(
      item(
        'pass',
        'Input microphone',
        `${input.name} is available (${input.channelCounts.join('/')} channel support).`,
      ),
    );
  } else {
    items.push(
      item(
        'failure',
        'Input microphone',
        unavailableInputName
          ? `Saved device "${unavailableInputName}" is unavailable.`
          : 'No physical input microphone is selected.',
        'Refresh devices and select an available physical microphone.',
      ),
    );
  }

  if (!selectedRoute) {
    items.push(
      item(
        'failure',
        'Processed output',
        'No saved playback/capture route is selected.',
        'Install or select a virtual audio cable pair, save it on Use, then run diagnostics again.',
      ),
    );
  } else if (routeValidation.readiness !== 'ready') {
    items.push(
      item(
        'failure',
        'Processed output',
        routeValidation.message,
        routeValidation.readiness === 'engineActive'
          ? 'Stop processing before changing or validating the route.'
          : 'Refresh devices, review the playback/capture pair, and validate it again.',
      ),
    );
  } else {
    items.push(
      item(
        'pass',
        'Processed output',
        `${selectedRoute.playbackDevice.name} is ready at ${routeValidation.negotiatedSampleRate ?? 'an available'} Hz.`,
      ),
    );
  }

  const monitor = selectedDevice(outputs, monitorId);
  if (monitor) {
    items.push(item('pass', 'Optional monitor', `${monitor.name} is available for Test.`));
    if (input && normalizedDeviceName(input.name) === normalizedDeviceName(monitor.name)) {
      items.push(
        item(
          'warning',
          'Feedback risk',
          'The input and monitor have the same display name and may be the same physical device.',
          'Use headphones and keep monitoring volume low, or select another monitor.',
        ),
      );
    } else if (!monitor.isLikelyVirtual) {
      items.push(
        item(
          'warning',
          'Monitoring safety',
          'A physical playback device is selected for Test monitoring.',
          'Prefer headphones. Disable Test monitoring if speakers could feed the active microphone.',
        ),
      );
    }
  } else if (unavailableMonitorName) {
    items.push(
      item(
        'warning',
        'Optional monitor',
        `Saved monitor "${unavailableMonitorName}" is unavailable; monitoring remains off.`,
        'Refresh devices and select another monitor only if local Test monitoring is needed.',
      ),
    );
  } else {
    items.push(
      item(
        'warning',
        'Optional monitor',
        'No local monitor is selected. External Use can still work.',
        'Select headphones on Test only if local monitoring is needed.',
      ),
    );
  }

  items.push(
    item(
      status.state === 'error' ? 'failure' : 'pass',
      'Processing state',
      `${status.state}: ${status.message}`,
      status.state === 'error'
        ? (classifyRecoverableError(status.lastRuntimeError)?.action ?? null)
        : null,
    ),
  );

  if (status.activeStreamFormat) {
    items.push(
      item(
        'pass',
        'Active stream format',
        `${status.activeStreamFormat.inputSampleRate} Hz, ${status.activeStreamFormat.inputChannels} input channel(s).`,
      ),
    );
  } else {
    items.push(
      item(
        'warning',
        'Active stream format',
        'No live stream is open, so callback channel and final stream-format checks are unavailable.',
        'Start processing after the configuration is ready, then run diagnostics again.',
      ),
    );
  }

  if (inputSignalDetected === false) {
    items.push(
      item(
        'warning',
        'Input activity',
        'No input signal was detected during the recent running interval.',
        'Unmute the microphone, check Windows input level and permission, then speak and retry.',
      ),
    );
  } else if (inputSignalDetected === true) {
    items.push(item('pass', 'Input activity', 'Input activity is currently detected.'));
  } else {
    items.push(
      item(
        'warning',
        'Input activity',
        'Input activity is not available while processing is stopped.',
        'Start processing and speak into the microphone to verify signal flow.',
      ),
    );
  }

  if (status.inputClipping || status.outputClipping || status.monitorClipping) {
    items.push(
      item(
        'warning',
        'Clipping',
        'A clipping indicator is latched for this session.',
        'Lower input or output gain, clear the clipping warning, and test again.',
      ),
    );
  } else {
    items.push(item('pass', 'Clipping', 'No clipping has been latched.'));
  }

  const outcome: DiagnosticOutcome = items.some((entry) => entry.severity === 'failure')
    ? 'notReady'
    : items.some((entry) => entry.severity === 'warning')
      ? 'readyWithWarnings'
      : 'ready';
  return { outcome, checkedAt, items };
}

function safeText(value: string | null | undefined) {
  if (!value) return 'Not available';
  if (/[a-z]:\\/i.test(value) || value.includes('/Users/') || value.includes('\\Users\\')) {
    return '[redacted path-like value]';
  }
  return value.replace(/[\r\n\t]/g, ' ').slice(0, 512);
}

export function createDiagnosticReport({
  product,
  status,
  parameters,
  inputDeviceName,
  processedOutputName,
  monitorDeviceName,
  inputActivityDetected,
  lastError,
  lastSuccessfulConfiguration,
  timestamp = new Date().toISOString(),
}: {
  product: ProductInformation | null;
  status: EngineStatus;
  parameters: AudioParameters;
  inputDeviceName: string | null;
  processedOutputName: string | null;
  monitorDeviceName: string | null;
  inputActivityDetected: boolean | null;
  lastError: string | null;
  lastSuccessfulConfiguration: LastSuccessfulConfiguration | null;
  timestamp?: string;
}) {
  const format = status.activeStreamFormat;
  const error = classifyRecoverableError(lastError);
  const report = {
    notice:
      'Safe to review before sharing. Device display names may still identify personal hardware.',
    product: {
      name: safeText(product?.productName),
      applicationVersion: safeText(product?.applicationVersion),
      prototype: product?.prototype ?? true,
      operatingSystem: safeText(product?.operatingSystem),
      architecture: safeText(product?.architecture),
      backendVersion: safeText(product?.backendVersion),
    },
    timestamp,
    processing: {
      state: status.state,
      purpose: status.routePurpose,
      message: safeText(status.message),
      inputActivityDetected,
      clipping: {
        input: status.inputClipping,
        processedOutput: status.outputClipping,
        monitor: status.monitorClipping,
      },
      lastRecoverableErrorCategory: error?.category ?? null,
      lastRecoverableErrorDetail: safeText(lastError),
    },
    selectedDevices: {
      input: safeText(inputDeviceName),
      processedOutput: safeText(processedOutputName),
      localMonitor: safeText(monitorDeviceName),
      monitoringEnabled:
        status.routePurpose === 'test' &&
        ['running', 'degraded', 'recovering'].includes(status.state),
    },
    activeFormat: format
      ? {
          inputSampleRate: format.inputSampleRate,
          processedDestinationSampleRate: format.processedDestinationSampleRate,
          localMonitorSampleRate: format.localMonitorSampleRate,
          inputChannels: format.inputChannels,
          processedDestinationChannels: format.processedDestinationChannels,
          localMonitorChannels: format.localMonitorChannels,
        }
      : null,
    dspParameters: parameters,
    lastSuccessfulConfiguration: lastSuccessfulConfiguration
      ? {
          ...lastSuccessfulConfiguration,
          inputDeviceName: safeText(lastSuccessfulConfiguration.inputDeviceName),
          outputDeviceName: safeText(lastSuccessfulConfiguration.outputDeviceName),
        }
      : null,
  };
  return `Mam Voice Changer diagnostic report\nReview before sharing: device names may be identifying.\n\n${JSON.stringify(report, null, 2)}`;
}
