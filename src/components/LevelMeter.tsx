type Props = {
  label: string;
  value: number;
};

export function LevelMeter({ label, value }: Props) {
  const percent = Math.min(100, Math.max(0, value * 100));
  const decibels = value > 0 ? Math.max(-96, 20 * Math.log10(value)) : -96;
  return (
    <div className="meter">
      <span>{label}</span>
      <div>
        <i style={{ width: `${percent}%` }} />
      </div>
      <b>
        {Math.round(percent)}% / {decibels.toFixed(1)} dBFS
      </b>
    </div>
  );
}
