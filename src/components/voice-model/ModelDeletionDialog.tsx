import { useEffect, useRef } from 'react';

export function ModelDeletionDialog({
  open,
  label,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  label: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const dialog = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return undefined;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    cancel.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onCancel();
      }
      if (event.key !== 'Tab' || !dialog.current) return;
      const focusable = Array.from(
        dialog.current.querySelectorAll<HTMLButtonElement>('button:not([disabled])'),
      );
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      previous?.focus();
    };
  }, [onCancel, open]);
  if (!open) return null;
  return (
    <div
      ref={dialog}
      role="dialog"
      aria-modal="true"
      aria-labelledby="model-deletion-title"
      className="model-deletion-dialog"
    >
      <h3 id="model-deletion-title">Delete managed {label}?</h3>
      <p>Source Dataset takes and exported copies outside managed storage are not deleted.</p>
      <button type="button" className="danger-outline" onClick={onConfirm}>
        Delete {label}
      </button>
      <button ref={cancel} type="button" onClick={onCancel}>
        Keep it
      </button>
    </div>
  );
}
