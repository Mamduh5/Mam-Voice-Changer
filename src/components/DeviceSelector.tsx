import type { AudioDevice } from '../types/audio';

type Props = {
  label: string;
  value: string;
  devices: AudioDevice[];
  disabled: boolean;
  allowEmpty?: boolean;
  emptyLabel?: string;
  showOutputClassification?: boolean;
  onChange: (id: string) => void;
};

export function DeviceSelector({
  label,
  value,
  devices,
  disabled,
  allowEmpty = false,
  emptyLabel = 'Select a device',
  showOutputClassification = false,
  onChange,
}: Props) {
  return (
    <label>
      {label}
      <select
        value={value}
        disabled={disabled || (devices.length === 0 && !allowEmpty)}
        onChange={(event) => onChange(event.target.value)}
      >
        {(allowEmpty || devices.length === 0) && (
          <option value="">{devices.length === 0 ? 'No devices found' : emptyLabel}</option>
        )}
        {devices.map((device) => (
          <option key={device.id} value={device.id}>
            {device.name}
            {device.isDefault ? ' (Default)' : ''}
            {showOutputClassification &&
              (device.isLikelyVirtual ? ' - likely virtual' : ' - physical playback')}
          </option>
        ))}
      </select>
    </label>
  );
}
