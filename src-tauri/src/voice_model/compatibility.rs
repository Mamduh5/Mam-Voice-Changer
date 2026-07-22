use serde::{Deserialize, Serialize};

use super::state::{ModelDevice, ModelPrecision, WORKER_PROTOCOL_VERSION};

pub const COMPATIBILITY_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const SEED_VC_EXPERIMENTAL_PROFILE_ID: &str = "seed-vc-experimental-v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualificationSupportStatus {
    Unknown,
    Experimental,
    Candidate,
    Qualified,
    Deprecated,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryIdentity {
    pub provider: String,
    pub owner: String,
    pub name: String,
    pub canonical_remote: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionRequirement {
    pub minimum_inclusive: String,
    pub maximum_exclusive: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageRequirement {
    pub package: String,
    pub requirement: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedBackendFile {
    pub role: String,
    pub relative_path: String,
    pub required: bool,
    pub expected_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckpointRole {
    pub role: String,
    pub display_name: String,
    pub required: bool,
    pub redistributable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredBackendCapabilities {
    pub training: bool,
    pub resume: bool,
    pub offline_inference: bool,
    pub multiple_references: bool,
    pub checkpoint_inspection: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackendCompatibilityProfileV1 {
    pub schema_version: u32,
    pub profile_id: String,
    pub backend_id: String,
    pub display_name: String,
    pub support_status: QualificationSupportStatus,
    pub repository_identity: RepositoryIdentity,
    pub supported_commit_shas: Vec<String>,
    pub worker_adapter_version: String,
    pub protocol_version: u32,
    pub python_requirement: VersionRequirement,
    pub package_requirements: Vec<PackageRequirement>,
    pub expected_files: Vec<ExpectedBackendFile>,
    pub configuration_files: Vec<ExpectedBackendFile>,
    pub checkpoint_roles: Vec<CheckpointRole>,
    pub supported_devices: Vec<ModelDevice>,
    pub supported_precisions: Vec<ModelPrecision>,
    pub capabilities: DeclaredBackendCapabilities,
    pub notices: Vec<String>,
}

impl BackendCompatibilityProfileV1 {
    pub fn supports_commit(&self, commit: &str) -> bool {
        !commit.is_empty()
            && self
                .supported_commit_shas
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(commit))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMPATIBILITY_PROFILE_SCHEMA_VERSION {
            return Err("Unsupported compatibility-profile schema.".to_owned());
        }
        if self.profile_id.is_empty() || self.backend_id.is_empty() {
            return Err("Compatibility-profile IDs are required.".to_owned());
        }
        if self.protocol_version != WORKER_PROTOCOL_VERSION {
            return Err("Compatibility profile uses an unsupported protocol.".to_owned());
        }
        for commit in &self.supported_commit_shas {
            if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err("Supported Git revisions must be exact 40-character SHAs.".to_owned());
            }
        }
        Ok(())
    }
}

pub fn built_in_profiles() -> Vec<BackendCompatibilityProfileV1> {
    vec![experimental_seed_vc_profile()]
}

pub fn profile(profile_id: &str) -> Option<BackendCompatibilityProfileV1> {
    built_in_profiles()
        .into_iter()
        .find(|profile| profile.profile_id == profile_id)
}

#[cfg(test)]
fn profiles_have_unique_ids(profiles: &[BackendCompatibilityProfileV1]) -> bool {
    let mut ids = std::collections::HashSet::new();
    profiles
        .iter()
        .all(|profile| ids.insert(&profile.profile_id))
}

fn experimental_seed_vc_profile() -> BackendCompatibilityProfileV1 {
    BackendCompatibilityProfileV1 {
        schema_version: COMPATIBILITY_PROFILE_SCHEMA_VERSION,
        profile_id: SEED_VC_EXPERIMENTAL_PROFILE_ID.to_owned(),
        backend_id: "seed-vc-local".to_owned(),
        display_name: "Seed-VC local (experimental, revision unpinned)".to_owned(),
        support_status: QualificationSupportStatus::Experimental,
        repository_identity: RepositoryIdentity {
            provider: "git".to_owned(),
            owner: "Plachtaa".to_owned(),
            name: "seed-vc".to_owned(),
            canonical_remote: "https://github.com/Plachtaa/seed-vc".to_owned(),
        },
        // Deliberately empty until the adapter is inspected against an exact checkout.
        supported_commit_shas: Vec::new(),
        worker_adapter_version: "mam-seed-vc-adapter-v2-experimental".to_owned(),
        protocol_version: WORKER_PROTOCOL_VERSION,
        python_requirement: VersionRequirement {
            minimum_inclusive: "3.10.0".to_owned(),
            maximum_exclusive: "3.12.0".to_owned(),
        },
        package_requirements: vec![
            package("torch", ">=2.0,<3", true),
            package("torchaudio", ">=2.0,<3", true),
            package("numpy", ">=1.23,<3", true),
            package("scipy", ">=1.9,<2", true),
            package("librosa", ">=0.10,<1", true),
            package("soundfile", ">=0.12,<1", true),
        ],
        expected_files: vec![
            expected("trainingEntryPoint", "train.py", true),
            expected("inferenceEntryPoint", "inference.py", true),
            expected("adapterImport", "modules/commons.py", true),
        ],
        configuration_files: vec![expected("modelConfiguration", "", true)],
        checkpoint_roles: vec![CheckpointRole {
            role: "baseModel".to_owned(),
            display_name: "Seed-VC base checkpoint".to_owned(),
            required: true,
            redistributable: false,
        }],
        supported_devices: vec![ModelDevice::Cpu, ModelDevice::Cuda],
        supported_precisions: vec![
            ModelPrecision::Float32,
            ModelPrecision::Float16,
            ModelPrecision::Bfloat16,
        ],
        capabilities: DeclaredBackendCapabilities {
            training: true,
            resume: true,
            offline_inference: true,
            multiple_references: false,
            checkpoint_inspection: true,
        },
        notices: vec![
            "No automatic downloads are permitted. The configured third-party Python code may still be capable of network access outside Mam Voice Changer's control.".to_owned(),
            "Redistribution permission has not been verified for configured checkpoint files.".to_owned(),
        ],
    }
}

fn package(package: &str, requirement: &str, required: bool) -> PackageRequirement {
    PackageRequirement {
        package: package.to_owned(),
        requirement: requirement.to_owned(),
        required,
    }
}

fn expected(role: &str, relative_path: &str, required: bool) -> ExpectedBackendFile {
    ExpectedBackendFile {
        role: role.to_owned(),
        relative_path: relative_path.to_owned(),
        required,
        expected_sha256: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trip_is_strict_and_experimental_is_not_qualified() {
        let profile = experimental_seed_vc_profile();
        profile.validate().expect("valid profile");
        let json = serde_json::to_string(&profile).expect("serialize");
        let decoded: BackendCompatibilityProfileV1 =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, profile);
        assert_eq!(
            decoded.support_status,
            QualificationSupportStatus::Experimental
        );
        assert!(!decoded.supports_commit(&"a".repeat(40)));
        let mut future = serde_json::to_value(profile).expect("value");
        future["schemaVersion"] = 99.into();
        let decoded: BackendCompatibilityProfileV1 =
            serde_json::from_value(future).expect("shape remains parseable");
        assert!(decoded.validate().is_err());
    }

    #[test]
    fn built_in_profile_ids_are_unique_and_capabilities_are_factual() {
        let profiles = built_in_profiles();
        assert!(profiles_have_unique_ids(&profiles));
        assert!(profiles[0].capabilities.offline_inference);
        assert!(!profiles[0].capabilities.multiple_references);
    }

    #[test]
    fn exact_commit_matching_rejects_abbreviations_and_unknown_revisions() {
        let mut profile = experimental_seed_vc_profile();
        profile.supported_commit_shas = vec!["a".repeat(40)];
        assert!(profile.supports_commit(&"a".repeat(40)));
        assert!(!profile.supports_commit("aaaaaaaa"));
        assert!(!profile.supports_commit(&"b".repeat(40)));
    }
}
