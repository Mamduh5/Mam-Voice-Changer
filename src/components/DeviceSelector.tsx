import type { AudioDevice } from '../types/audio';

type Props = {
  label: string;
  value: string;
  devices: AudioDevice[];
  disabled: boolean;
  allowEmpty?: boolean;
  emptyLabel?: string;
  showOutputClassification?: boolean;
  unavailableName?: string | null;
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
  unavailableName = null,
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
        {(allowEmpty || devices.length === 0 || Boolean(unavailableName)) && (
          <option value="">
            {unavailableName
              ? `Select another device (${unavailableName} is unavailable)`
              : devices.length === 0
                ? 'No devices found'
                : emptyLabel}
          </option>
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
