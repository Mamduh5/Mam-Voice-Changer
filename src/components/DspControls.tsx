import type { AudioParameters } from '../types/parameters';

type Props = {
  parameters: AudioParameters;
  disabled: boolean;
  onChange: (changes: Partial<AudioParameters>) => void;
};

type GainControlProps = {
  label: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
};

function GainControl({ label, value, disabled, onChange }: GainControlProps) {
  return (
    <label className="gain-control">
      <span>
        {label} <strong>{value.toFixed(1)} dB</strong>
      </span>
      <input
        type="range"
        min="-24"
        max="24"
        step="0.5"
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

export function DspControls({ parameters, disabled, onChange }: Props) {
  return (
    <section className="card dsp-controls">
      <div className="section-heading">
        <h2>Processing</h2>
        <span className="filter-label">20 Hz high-pass</span>
      </div>
      <GainControl
        label="Input gain"
        value={parameters.inputGainDb}
        disabled={disabled}
        onChange={(inputGainDb) => onChange({ inputGainDb })}
      />
      <GainControl
        label="Output gain"
        value={parameters.outputGainDb}
        disabled={disabled}
        onChange={(outputGainDb) => onChange({ outputGainDb })}
      />
      <div className="dsp-switches">
        <label className="limiter-toggle">
          <input
            type="checkbox"
            checked={parameters.limiterEnabled}
            disabled={disabled}
            onChange={(event) => onChange({ limiterEnabled: event.target.checked })}
          />
          Soft limiter
        </label>
        <button
          type="button"
          className={parameters.bypass ? 'active' : ''}
          aria-pressed={parameters.bypass}
          disabled={disabled}
          onClick={() => onChange({ bypass: !parameters.bypass })}
        >
          Bypass
        </button>
        <button
          type="button"
          className={parameters.muted ? 'active danger' : ''}
          aria-pressed={parameters.muted}
          disabled={disabled}
          onClick={() => onChange({ muted: !parameters.muted })}
        >
          Mute
        </button>
      </div>
    </section>
  );
}
