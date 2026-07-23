export function SyntheticAudioNotice() {
  return (
    <div className="model-synthetic-notice" role="status">
      <strong>Synthetic voice output.</strong> Training and conversion remain local. Do not
      represent generated speech as an authentic recording of the target speaker. Consent revocation
      disables managed dependent models; exported copies must be managed separately.
    </div>
  );
}
