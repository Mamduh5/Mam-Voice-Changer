import type { ApplicationPage } from '../types/audio';

const pages: Array<{ id: ApplicationPage; label: string }> = [
  { id: 'use', label: 'Use' },
  { id: 'test', label: 'Test' },
  { id: 'diagnostics', label: 'Settings & Diagnostics' },
];

export function PageNavigation({
  page,
  onNavigate,
}: {
  page: ApplicationPage;
  onNavigate: (page: ApplicationPage) => void;
}) {
  return (
    <nav className="page-navigation" aria-label="Application sections">
      {pages.map((item) => (
        <button
          type="button"
          key={item.id}
          className={page === item.id ? 'active' : ''}
          aria-current={page === item.id ? 'page' : undefined}
          onClick={() => onNavigate(item.id)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}
