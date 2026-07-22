import { open, save } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';
import type { VoiceModelArtifact } from '../../types/voiceModel';

export function ModelPortabilityPanel({
  artifact,
  profileId,
  consentVersion,
  busy,
  onExport,
  onImport,
}: {
  artifact: VoiceModelArtifact | null;
  profileId: string | null;
  consentVersion: string | null;
  busy: boolean;
  onExport: (
    artifactId: string,
    destination: string,
    licensingAcknowledged: boolean,
  ) => Promise<unknown>;
  onImport: (request: {
    packagePath: string;
    profileId: string;
    activeConsentVersion: string;
    associationConfirmed: boolean;
  }) => Promise<unknown>;
}) {
  const [licensingAcknowledged, setLicensingAcknowledged] = useState(false);
  const [associationConfirmed, setAssociationConfirmed] = useState(false);
  const exportPackage = async () => {
    if (!artifact) return;
    const destination = await save({
      defaultPath: `${artifact.displayName.replace(/[^a-z0-9_-]+/gi, '-')}.mam-model.zip`,
      filters: [{ name: 'Mam Voice model package', extensions: ['zip'] }],
    });
    if (destination) await onExport(artifact.artifactId, destination, licensingAcknowledged);
  };
  const importPackage = async () => {
    if (!profileId || !consentVersion || !associationConfirmed) return;
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Mam Voice model package', extensions: ['zip'] }],
    });
    if (typeof selected === 'string')
      await onImport({
        packagePath: selected,
        profileId,
        activeConsentVersion: consentVersion,
        associationConfirmed,
      });
  };
  return (
    <section className="card model-portability-panel">
      <div className="section-heading">
        <h2>Model portability</h2>
        <span>{artifact?.portabilityStatus ?? 'No model selected'}</span>
      </div>
      {artifact && (
        <>
          <div className="model-metrics">
            <span>Profile: {artifact.compatibilityProfileId || 'unknown'}</span>
            <span>Backend revision: {artifact.backendRevision ?? 'unknown'}</span>
            <span>Adapter: {artifact.adapterVersion || 'unknown'}</span>
            <span>Environment: {artifact.health}</span>
            <span>
              External checkpoints: {artifact.checkpointIdentities.length || 'unspecified'}
            </span>
          </div>
          {artifact.licenseNotices.map((notice) => (
            <div className="dataset-safety" key={`${notice.role}-${notice.label}`}>
              {notice.label}: {notice.notice}
            </div>
          ))}
          <label className="dataset-consent-check">
            <input
              type="checkbox"
              checked={licensingAcknowledged}
              onChange={(event) => setLicensingAcknowledged(event.target.checked)}
            />
            I understand that backend, checkpoint, configuration, adapter, and user-trained artifact
            licensing are separate. Redistribution permission may be unknown.
          </label>
          <button
            type="button"
            disabled={busy || !licensingAcknowledged}
            onClick={() => void exportPackage()}
          >
            Export model package
          </button>
        </>
      )}
      <h3>Import an untrusted package</h3>
      <p>
        Import validates bounded paths, file counts, sizes, schema, and hashes. It never executes
        package content. Imported models remain unevaluated and unapproved and must be associated
        with this consent-active profile by opaque ID.
      </p>
      <label className="dataset-consent-check">
        <input
          type="checkbox"
          checked={associationConfirmed}
          onChange={(event) => setAssociationConfirmed(event.target.checked)}
        />
        Associate the imported provenance with the currently selected consent-active profile; do not
        match by display name.
      </label>
      <button
        type="button"
        disabled={busy || !profileId || !consentVersion || !associationConfirmed}
        onClick={() => void importPackage()}
      >
        Import model package
      </button>
    </section>
  );
}
