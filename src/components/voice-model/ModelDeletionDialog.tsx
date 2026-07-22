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
  if (!open) return null;
  return (
    <div role="dialog" aria-modal="true" className="model-deletion-dialog">
      <h3>Delete managed {label}?</h3>
      <p>Source Dataset takes and exported copies outside managed storage are not deleted.</p>
      <button type="button" className="danger-outline" onClick={onConfirm}>
        Delete {label}
      </button>
      <button type="button" onClick={onCancel}>
        Keep it
      </button>
    </div>
  );
}
