import type { DatasetTake } from '../../types/voiceDataset';
import { filterDatasetTakes, type DatasetTakeFilter } from '../../utils/datasetNavigation';
import { qualityLabels } from '../../utils/datasetQualityLabels';

const filters: DatasetTakeFilter[] = [
  'all',
  'pending',
  'accepted',
  'rejected',
  'warning',
  'failed',
  'excluded',
  'imported',
  'recorded',
];

export function DatasetTakeList({
  takes,
  filter,
  selectedId,
  onFilter,
  onSelect,
}: {
  takes: DatasetTake[];
  filter: DatasetTakeFilter;
  selectedId: string | null;
  onFilter: (filter: DatasetTakeFilter) => void;
  onSelect: (id: string) => void;
}) {
  const visible = filterDatasetTakes(takes, filter);
  return (
    <section className="card dataset-takes">
      <div className="section-heading">
        <h2>Take review queue</h2>
        <span>{visible.length} shown</span>
      </div>
      <div className="dataset-filters">
        {filters.map((item) => (
          <button
            type="button"
            key={item}
            className={filter === item ? 'active' : ''}
            onClick={() => onFilter(item)}
          >
            {item}
          </button>
        ))}
      </div>
      <div className="dataset-take-buttons">
        {visible.map((take) => (
          <button
            type="button"
            key={take.id}
            className={selectedId === take.id ? 'active' : ''}
            onClick={() => onSelect(take.id)}
          >
            <strong>{take.promptText ?? 'Unprompted recording'}</strong>
            <small>
              {take.source} · {(take.durationMs / 1_000).toFixed(2)} s · {take.reviewStatus} ·{' '}
              {qualityLabels[take.quality.classification]}
            </small>
          </button>
        ))}
        {!visible.length && <p>No takes match this filter.</p>}
      </div>
    </section>
  );
}
