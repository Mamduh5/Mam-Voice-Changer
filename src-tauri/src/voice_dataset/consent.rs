use serde::{Deserialize, Serialize};

pub const CONSENT_VERSION: &str = "voice-dataset-consent-v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsentMetadata {
    pub consent_confirmed: bool,
    pub consent_version: String,
    pub confirmed_at: String,
    pub confirmed_by_user: bool,
    pub recorded_consent_take_id: Option<String>,
    pub revoked_at: Option<String>,
    pub notes: Option<String>,
}
