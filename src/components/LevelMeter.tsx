type Props = {
  label: string;
  value: number;
};

export function LevelMeter({ label, value }: Props) {
  const percent = Math.min(100, Math.max(0, value * 100));
  return (
    <div className="meter">
      <span>{label}</span>
      <div>
        <i style={{ width: `${percent}%` }} />
      </div>
      <b>{Math.round(percent)}%</b>
    </div>
  );
}
