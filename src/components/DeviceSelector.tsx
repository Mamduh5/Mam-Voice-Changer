import type { AudioDevice } from '../types/audio';

type Props = {
  label: string;
  value: string;
  devices: AudioDevice[];
  disabled: boolean;
  onChange: (id: string) => void;
};

export function DeviceSelector({ label, value, devices, disabled, onChange }: Props) {
  return (
    <label>
      {label}
      <select
        value={value}
        disabled={disabled || devices.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        {devices.length === 0 && <option value="">No devices found</option>}
        {devices.map((device) => (
          <option key={device.id} value={device.id}>
            {device.name}
            {device.isDefault ? ' (Default)' : ''}
          </option>
        ))}
      </select>
    </label>
  );
}
