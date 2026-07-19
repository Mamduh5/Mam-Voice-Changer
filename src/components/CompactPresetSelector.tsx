import type { PresetCatalog } from '../types/presets';

export function CompactPresetSelector({
  catalog,
  disabled,
  onApply,
}: {
  catalog: PresetCatalog | null;
  disabled: boolean;
  onApply: (id: string) => Promise<boolean>;
}) {
  return (
    <section className="card compact-preset">
      <h2>Voice preset</h2>
      <label>
        Active preset
        <select
          value={catalog?.selectedPresetId ?? ''}
          disabled={disabled || !catalog}
          onChange={(event) => void onApply(event.target.value)}
        >
          {(catalog?.presets ?? []).map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.name}
              {preset.builtIn ? '' : ' · My preset'}
            </option>
          ))}
        </select>
      </label>
    </section>
  );
}
