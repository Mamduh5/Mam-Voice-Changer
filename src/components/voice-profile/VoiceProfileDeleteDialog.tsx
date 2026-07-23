import { useEffect, useRef } from 'react';

const focusableSelector =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function VoiceProfileDeleteDialog({
  open,
  profileName,
  busy,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  profileName: string;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
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
        return;
      }
      if (event.key !== 'Tab' || !dialog.current) return;
      const focusable = Array.from(dialog.current.querySelectorAll<HTMLElement>(focusableSelector));
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
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
    <div className="dialog-backdrop">
      <div
        ref={dialog}
        className="profile-delete-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="delete-profile-title"
        aria-describedby="delete-profile-description"
      >
        <h2 id="delete-profile-title">Delete {profileName}?</h2>
        <p id="delete-profile-description">
          This removes managed raw and derived recordings, manifest, and consent metadata. Dependent
          model artifacts are disabled and active profile work is cancelled safely. Exported copies
          outside the application cannot be deleted automatically.
        </p>
        <div className="voice-lab-actions">
          <button ref={cancel} type="button" disabled={busy} onClick={onCancel}>
            Keep profile
          </button>
          <button type="button" className="danger-outline" disabled={busy} onClick={onConfirm}>
            Delete profile and managed data
          </button>
        </div>
      </div>
    </div>
  );
}
