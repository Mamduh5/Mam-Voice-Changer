use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    storage::atomic_write_json,
};

pub const RECOVERY_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ManagedItemKind {
    QualificationRun,
    TrainingSnapshot,
    TrainingJob,
    ModelArtifact,
    TemporaryInferenceResult,
    ImportedArtifactPackage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ManagedItemHealth {
    Healthy,
    Interrupted,
    Incomplete,
    NeedsRepair,
    MissingFiles,
    UnexpectedFiles,
    HashMismatch,
    UnsupportedSchema,
    DisabledByConsent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedIndexEntry {
    pub item_id: String,
    pub kind: ManagedItemKind,
    pub relative_path: String,
    pub health: ManagedItemHealth,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryIndexesV1 {
    pub schema_version: u32,
    pub entries: Vec<ManagedIndexEntry>,
    pub orphaned_paths: Vec<String>,
    pub incomplete_paths: Vec<String>,
}

pub fn rebuild_indexes(root: &Path) -> VoiceModelResult<RecoveryIndexesV1> {
    let mut entries = Vec::new();
    let mut incomplete_paths = Vec::new();
    scan_flat(
        root,
        "qualifications",
        "qualification.json",
        ManagedItemKind::QualificationRun,
        &mut entries,
        &mut incomplete_paths,
    )?;
    scan_flat(
        root,
        "snapshots",
        "snapshot.json",
        ManagedItemKind::TrainingSnapshot,
        &mut entries,
        &mut incomplete_paths,
    )?;
    scan_flat(
        root,
        "jobs",
        "job.json",
        ManagedItemKind::TrainingJob,
        &mut entries,
        &mut incomplete_paths,
    )?;
    scan_flat(
        root,
        "temporary-inference",
        "provenance.json",
        ManagedItemKind::TemporaryInferenceResult,
        &mut entries,
        &mut incomplete_paths,
    )?;
    scan_flat(
        root,
        "imports",
        "",
        ManagedItemKind::ImportedArtifactPackage,
        &mut entries,
        &mut incomplete_paths,
    )?;
    let profiles_root = root.join("profiles");
    if let Ok(profiles) = fs::read_dir(&profiles_root) {
        for profile in profiles.filter_map(Result::ok) {
            let artifacts = profile.path().join("artifacts");
            scan_directory(
                root,
                &artifacts,
                "artifact.json",
                ManagedItemKind::ModelArtifact,
                &mut entries,
                &mut incomplete_paths,
            )?;
        }
    }
    let mut ids = HashSet::new();
    if !entries
        .iter()
        .all(|entry| ids.insert(entry.item_id.clone()))
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::StorageUnavailable,
            "Recovery indexes contain duplicate opaque IDs.",
        ));
    }
    entries.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    incomplete_paths.sort();
    incomplete_paths.dedup();
    let indexes = RecoveryIndexesV1 {
        schema_version: RECOVERY_INDEX_SCHEMA_VERSION,
        entries,
        orphaned_paths: Vec::new(),
        incomplete_paths,
    };
    atomic_write_json(&root.join("recovery-indexes.json"), &indexes)?;
    Ok(indexes)
}

fn scan_flat(
    root: &Path,
    relative: &str,
    manifest_name: &str,
    kind: ManagedItemKind,
    entries: &mut Vec<ManagedIndexEntry>,
    incomplete: &mut Vec<String>,
) -> VoiceModelResult<()> {
    scan_directory(
        root,
        &root.join(relative),
        manifest_name,
        kind,
        entries,
        incomplete,
    )
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    manifest_name: &str,
    kind: ManagedItemKind,
    entries: &mut Vec<ManagedIndexEntry>,
    incomplete: &mut Vec<String>,
) -> VoiceModelResult<()> {
    let Ok(items) = fs::read_dir(directory) else {
        return Ok(());
    };
    for item in items.filter_map(Result::ok) {
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        let relative = path
            .strip_prefix(root)
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.clone());
        if name.starts_with('.') || name.ends_with(".partial") || name.ends_with(".tmp") {
            incomplete.push(relative);
            continue;
        }
        let manifest = if manifest_name.is_empty() {
            path.is_file()
        } else {
            path.join(manifest_name).is_file()
        };
        if manifest {
            entries.push(ManagedIndexEntry {
                item_id: path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&name)
                    .to_owned(),
                kind,
                relative_path: relative,
                health: ManagedItemHealth::Healthy,
            });
        } else if path.is_dir() {
            incomplete.push(relative);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuilds_from_manifests_and_classifies_incomplete_directories() {
        let root = std::env::temp_dir().join(format!("mam-index-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("jobs/job-valid")).expect("valid");
        fs::write(root.join("jobs/job-valid/job.json"), b"{}").expect("manifest");
        fs::create_dir_all(root.join("jobs/.job-partial.tmp")).expect("partial");
        let index = rebuild_indexes(&root).expect("index");
        assert!(index
            .entries
            .iter()
            .any(|entry| entry.item_id == "job-valid"));
        assert!(!index.incomplete_paths.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
