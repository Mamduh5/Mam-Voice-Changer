import { useState } from 'react';
import type {
  AudioDevice,
  ExternalAudioRoute,
  LastSuccessfulConfiguration,
  RouteCompatibilityResult,
} from '../types/audio';
import type { EngineStatus } from '../types/engine';
import type { AudioParameters } from '../types/parameters';
import type { ProductInformation } from '../types/product';
import {
  createDiagnosticReport,
  evaluateAudioConfiguration,
  type ConfigurationDiagnostic,
} from '../utils/productDiagnostics';

const outcomeLabel = {
  ready: 'Ready',
  readyWithWarnings: 'Ready with warnings',
  notReady: 'Not ready',
} as const;

function selectedName(devices: AudioDevice[], id: string) {
  return devices.find((device) => device.id === id)?.name ?? null;
}

export function ProductDiagnosticsPanel({
  inputs,
  outputs,
  inputId,
  unavailableInputName,
  monitorId,
  unavailableMonitorName,
  selectedRoute,
  routeValidation,
  status,
  parameters,
  product,
  lastSuccessfulConfiguration,
  inputActivityDetected,
  disabled,
  changesDisabled,
  onRefresh,
  onClearClipping,
  onResetSafeDefaults,
  onOpenSetup,
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
  parameters: AudioParameters;
  product: ProductInformation | null;
  lastSuccessfulConfiguration: LastSuccessfulConfiguration | null;
  inputActivityDetected: boolean | null;
  disabled: boolean;
  changesDisabled: boolean;
  onRefresh: () => Promise<void>;
  onClearClipping: () => Promise<void>;
  onResetSafeDefaults: () => Promise<boolean>;
  onOpenSetup: () => void;
}) {
  const [diagnostic, setDiagnostic] = useState<ConfigurationDiagnostic | null>(null);
  const [report, setReport] = useState('');
  const [copyStatus, setCopyStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState<'refresh' | 'reset' | null>(null);

  const runDiagnostic = () => {
    setDiagnostic(
      evaluateAudioConfiguration({
        inputs,
        outputs,
        inputId,
        unavailableInputName,
        monitorId,
        unavailableMonitorName,
        selectedRoute,
        routeValidation,
        status,
        inputSignalDetected: inputActivityDetected,
      }),
    );
  };

  const copyReport = async () => {
    const next = createDiagnosticReport({
      product,
      status,
      parameters,
      inputDeviceName: selectedName(inputs, inputId) ?? unavailableInputName,
      processedOutputName: selectedRoute?.playbackDevice.name ?? null,
      monitorDeviceName: selectedName(outputs, monitorId) ?? unavailableMonitorName,
      inputActivityDetected,
      lastError: status.lastRuntimeError,
      lastSuccessfulConfiguration,
    });
    setReport(next);
    try {
      if (!navigator.clipboard?.writeText) throw new Error('Clipboard API unavailable');
      await navigator.clipboard.writeText(next);
      setCopyStatus('Diagnostic report copied. Review device names before sharing.');
    } catch {
      setCopyStatus('Automatic copy is unavailable. Select and copy the report below.');
    }
  };

  const refresh = async () => {
    setBusy('refresh');
    try {
      await onRefresh();
      setDiagnostic(null);
    } finally {
      setBusy(null);
    }
  };

  const reset = async () => {
    if (
      !window.confirm(
        'Reset DSP values to safe defaults? Device and route selections will be kept, and processing will not start.',
      )
    ) {
      return;
    }
    setBusy('reset');
    try {
      await onResetSafeDefaults();
    } finally {
      setBusy(null);
    }
  };

  return (
    <section className="card product-diagnostics">
      <div className="section-heading">
        <div>
          <h2>Audio readiness check</h2>
          <p>Read-only checks; running diagnostics never changes DSP parameters.</p>
        </div>
        {diagnostic && (
          <span className={`diagnostic-outcome ${diagnostic.outcome}`}>
            {outcomeLabel[diagnostic.outcome]}
          </span>
        )}
      </div>
      <div className="diagnostic-actions">
        <button type="button" disabled={disabled} onClick={runDiagnostic}>
          Run audio check
        </button>
        <button
          type="button"
          disabled={changesDisabled || busy !== null}
          onClick={() => void refresh()}
        >
          {busy === 'refresh' ? 'Refreshing...' : 'Refresh devices'}
        </button>
        <button type="button" onClick={() => void copyReport()}>
          Copy diagnostic report
        </button>
        <button
          type="button"
          disabled={changesDisabled || busy !== null}
          onClick={() => void reset()}
        >
          {busy === 'reset' ? 'Resetting...' : 'Reset to safe defaults'}
        </button>
        <button type="button" onClick={onOpenSetup}>
          Reopen quick setup
        </button>
        {(status.inputClipping || status.outputClipping || status.monitorClipping) && (
          <button type="button" onClick={() => void onClearClipping()}>
            Clear clipping warning
          </button>
        )}
      </div>
      {diagnostic && (
        <div className="diagnostic-results">
          {diagnostic.items.map((entry) => (
            <article className={entry.severity} key={`${entry.label}-${entry.detail}`}>
              <strong>{entry.label}</strong>
              <p>{entry.detail}</p>
              {entry.action && <small>Next action: {entry.action}</small>}
            </article>
          ))}
          <small>Checked {new Date(diagnostic.checkedAt).toLocaleString()}</small>
        </div>
      )}
      {copyStatus && <p role="status">{copyStatus}</p>}
      {report && (
        <details>
          <summary>Review diagnostic report</summary>
          <textarea
            className="diagnostic-report"
            readOnly
            value={report}
            aria-label="Diagnostic report"
          />
        </details>
      )}
    </section>
  );
}
