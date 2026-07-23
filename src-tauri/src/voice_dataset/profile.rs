use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceProfileMetadata {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub primary_language: String,
    pub locale_tag: Option<String>,
    pub collection_goal_minutes: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVoiceProfileRequest {
    pub display_name: String,
    pub description: Option<String>,
    pub primary_language: String,
    pub locale_tag: Option<String>,
    pub collection_goal_minutes: Option<u32>,
    pub consent_confirmed: bool,
    pub confirmed_by_user: bool,
    pub consent_version: String,
    pub consent_notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateVoiceProfileRequest {
    pub display_name: String,
    pub description: Option<String>,
    pub primary_language: String,
    pub locale_tag: Option<String>,
    pub collection_goal_minutes: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileHealth {
    Healthy,
    NeedsRepair,
    MissingFiles,
    OrphanedFiles,
    UnsupportedSchema,
    CorruptManifest,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProfileSummary {
    pub profile: VoiceProfileMetadata,
    pub health: ProfileHealth,
    pub managed_storage_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceProfileIndexV1 {
    pub schema_version: u32,
    pub profiles: Vec<VoiceProfileMetadata>,
    pub updated_at: String,
}
