import type { AudioParameters } from '../types/parameters';

type Props = {
  parameters: AudioParameters;
  disabled: boolean;
  onChange: (changes: Partial<AudioParameters>) => void;
};

type SliderProps = {
  label: string;
  value: number;
  displayValue?: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  disabled: boolean;
  onChange: (value: number) => void;
};

function SliderControl({
  label,
  value,
  displayValue = value,
  min,
  max,
  step,
  unit,
  disabled,
  onChange,
}: SliderProps) {
  return (
    <label className="gain-control">
      <span>
        {label}{' '}
        <strong>
          {displayValue.toFixed(unit === '%' ? 0 : 1)} {unit}
        </strong>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
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
      <SliderControl
        label="Pitch"
        value={parameters.pitchSemitones}
        min={-12}
        max={12}
        step={0.5}
        unit="st"
        disabled={disabled}
        onChange={(pitchSemitones) => onChange({ pitchSemitones })}
      />
      <SliderControl
        label="Dry / wet"
        value={parameters.dryWet}
        displayValue={parameters.dryWet * 100}
        min={0}
        max={1}
        step={0.01}
        unit="%"
        disabled={disabled}
        onChange={(dryWet) => onChange({ dryWet })}
      />
      <SliderControl
        label="Input gain"
        value={parameters.inputGainDb}
        min={-24}
        max={24}
        step={0.5}
        unit="dB"
        disabled={disabled}
        onChange={(inputGainDb) => onChange({ inputGainDb })}
      />
      <SliderControl
        label="Output gain"
        value={parameters.outputGainDb}
        min={-24}
        max={24}
        step={0.5}
        unit="dB"
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
