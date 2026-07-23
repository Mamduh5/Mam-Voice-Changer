import { useState } from 'react';
import type { ReturnTypeOfUseVoiceProfiles } from './profileTypes';
import { VoiceProfileConsent } from './VoiceProfileConsent';
import { VoiceProfileEditor } from './VoiceProfileEditor';
import { VoiceProfileList } from './VoiceProfileList';

export function VoiceProfilesPage({ profiles }: { profiles: ReturnTypeOfUseVoiceProfiles }) {
  const [query, setQuery] = useState('');
  return (
    <div className="page-stack voice-profiles-page">
      <section className="card voice-lab-intro sticky-workspace-header">
        <div>
          <p className="eyebrow">Central local identity and consent workspace</p>
          <h2>Voice Profiles</h2>
          <p>
            Create and manage consent, profile metadata, Dataset health, storage, and model
            dependencies in one place.
          </p>
        </div>
        <span className="bounded-label">One shared selection</span>
      </section>
      {profiles.error && (
        <div className="error" role="alert">
          <strong>Voice Profiles:</strong> {profiles.error}
        </div>
      )}
      <div className="profiles-master-detail">
        <aside className="workspace-sidebar">
          <VoiceProfileList
            profiles={profiles.profiles}
            currentId={profiles.selectedProfileId}
            query={query}
            busy={profiles.busy}
            onQuery={setQuery}
            onSelect={profiles.selectProfile}
            onRepair={profiles.repairProfile}
          />
          <VoiceProfileConsent busy={profiles.busy} onCreate={profiles.createProfile} />
        </aside>
        <div className="workspace-main">
          {profiles.manifest && profiles.selectedSummary ? (
            <VoiceProfileEditor key={profiles.manifest.profile.id} profiles={profiles} />
          ) : (
            <section className="dataset-empty">
              <h3>Select or create a voice profile</h3>
              <p>
                Profile management, consent, storage health, Dataset summaries, and model
                dependencies appear here.
              </p>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}
