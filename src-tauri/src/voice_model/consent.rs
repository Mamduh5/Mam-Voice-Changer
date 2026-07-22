use crate::voice_dataset::source::ManifestDatasetSource;

use super::error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult};

pub fn require_active_consent(
    source: &ManifestDatasetSource,
    expected_version: Option<&str>,
) -> VoiceModelResult<()> {
    let consent = source.consent();
    if !consent.consent_confirmed || !consent.confirmed_by_user || consent.revoked_at.is_some() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ConsentInactive,
            "Active target-speaker consent is required for model training and conversion.",
        ));
    }
    if expected_version.is_some_and(|version| version != consent.consent_version) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ConsentInactive,
            "The model consent provenance does not match the active Dataset consent.",
        ));
    }
    Ok(())
}
