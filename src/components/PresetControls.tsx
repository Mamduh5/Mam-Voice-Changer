import { useMemo, useState } from 'react';
import type { AudioParameters } from '../types/parameters';
import type { PresetCatalog } from '../types/presets';

type Props = {
  catalog: PresetCatalog | null;
  parameters: AudioParameters;
  disabled: boolean;
  busy: boolean;
  onApply: (id: string) => Promise<boolean>;
  onSave: (name: string, parameters: AudioParameters) => Promise<boolean>;
  onRename: (id: string, name: string) => Promise<boolean>;
  onDuplicate: (id: string) => Promise<boolean>;
  onDelete: (id: string) => Promise<boolean>;
  onReset: () => Promise<boolean>;
};

export function PresetControls({
  catalog,
  parameters,
  disabled,
  busy,
  onApply,
  onSave,
  onRename,
  onDuplicate,
  onDelete,
  onReset,
}: Props) {
  const [name, setName] = useState('');
  const selected = useMemo(
    () => catalog?.presets.find((preset) => preset.id === catalog.selectedPresetId) ?? null,
    [catalog],
  );
  const builtIns = catalog?.presets.filter((preset) => preset.builtIn) ?? [];
  const userPresets = catalog?.presets.filter((preset) => !preset.builtIn) ?? [];
  const controlsDisabled = disabled || busy || !catalog;
  const validName = name.trim().length > 0;

  const save = async () => {
    if (await onSave(name, parameters)) {
      setName('');
    }
  };

  const rename = async () => {
    if (selected && (await onRename(selected.id, name))) {
      setName('');
    }
  };

  const remove = async () => {
    if (
      selected &&
      window.confirm('Delete preset "' + selected.name + '"?') &&
      (await onDelete(selected.id))
    ) {
      setName('');
    }
  };

  return (
    <section className="card presets">
      <div className="section-heading">
        <h2>Presets</h2>
        <span className="filter-label">Stored on this device</span>
      </div>
      <div className="preset-row">
        <label>
          Active preset
          <select
            value={catalog?.selectedPresetId ?? ''}
            disabled={controlsDisabled}
            onChange={(event) => void onApply(event.target.value)}
          >
            {!catalog?.selectedPresetId && (
              <option value="" disabled>
                No preset selected
              </option>
            )}
            <optgroup label="Built in">
              {builtIns.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.name}
                </option>
              ))}
            </optgroup>
            {userPresets.length > 0 && (
              <optgroup label="My presets">
                {userPresets.map((preset) => (
                  <option key={preset.id} value={preset.id}>
                    {preset.name}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </label>
        <label>
          Preset name
          <input
            type="text"
            value={name}
            maxLength={64}
            placeholder={selected?.name ?? 'My preset'}
            disabled={controlsDisabled}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <div className="preset-actions">
          <button
            type="button"
            disabled={controlsDisabled || !validName}
            onClick={() => void save()}
          >
            Save
          </button>
          <button
            type="button"
            disabled={controlsDisabled || !validName || !selected || selected.builtIn}
            onClick={() => void rename()}
          >
            Rename
          </button>
          <button
            type="button"
            disabled={controlsDisabled || !selected}
            onClick={() => selected && void onDuplicate(selected.id)}
          >
            Duplicate
          </button>
          <button
            type="button"
            disabled={controlsDisabled || !selected || selected.builtIn}
            onClick={() => void remove()}
          >
            Delete
          </button>
          <button type="button" disabled={controlsDisabled} onClick={() => void onReset()}>
            Reset
          </button>
        </div>
      </div>
    </section>
  );
}
