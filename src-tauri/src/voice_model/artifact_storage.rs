use std::{fs, path::Path};

use serde::Deserialize;

use crate::voice_dataset::{
    hash::{sha256_bytes, sha256_file},
    storage::{new_id, timestamp},
};

use super::{
    artifact::{
        ModelApprovalStatus, ModelArtifactFile, TrainingSummary, VoiceModelArtifactV1,
        MODEL_ARTIFACT_SCHEMA_VERSION,
    },
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    snapshot::TrainingSnapshotV1,
    state::{TrainingJob, WORKER_PROTOCOL_VERSION},
    storage::{atomic_write_json, ensure_relative_path, managed_join},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletedTrainingPayload {
    backend_version: String,
    artifact_files: Vec<String>,
    training_summary: TrainingSummary,
}

pub fn create_artifact(
    models_root: &Path,
    job_directory: &Path,
    job: &TrainingJob,
    snapshot: &TrainingSnapshotV1,
    payload: serde_json::Value,
) -> VoiceModelResult<VoiceModelArtifactV1> {
    let payload: CompletedTrainingPayload = serde_json::from_value(payload).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The worker completion payload is malformed.",
        )
    })?;
    if payload.artifact_files.is_empty() || payload.artifact_files.len() > 32 {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::UnexpectedOutput,
            "The worker did not report a bounded set of artifact files.",
        ));
    }
    let now = timestamp().map_err(|error| {
        VoiceModelError::new(VoiceModelErrorCode::StorageUnavailable, error.message)
    })?;
    let artifact_id = new_id("artifact", &now);
    let artifact_directory = models_root
        .join("profiles")
        .join(&job.profile_id)
        .join("artifacts")
        .join(&artifact_id);
    let temporary = artifact_directory.with_file_name(format!(".{artifact_id}.tmp"));
    fs::create_dir_all(temporary.join("model"))
        .map_err(|error| VoiceModelError::storage("Cannot create artifact storage", error))?;

    let canonical_job = fs::canonicalize(job_directory)
        .map_err(|error| VoiceModelError::storage("Cannot resolve the model job", error))?;
    let mut model_files = Vec::new();
    for (index, relative) in payload.artifact_files.iter().enumerate() {
        ensure_relative_path(relative)?;
        let source = managed_join(job_directory, relative)?;
        let canonical_source = fs::canonicalize(&source).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::ArtifactMissing,
                "A worker-reported artifact file is missing.",
            )
        })?;
        if !canonical_source.starts_with(&canonical_job)
            || fs::symlink_metadata(&source)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(true)
            || !source.is_file()
        {
            let _ = fs::remove_dir_all(&temporary);
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::UnexpectedOutput,
                "The worker reported an unsafe artifact path.",
            ));
        }
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("bin")
            .to_ascii_lowercase();
        if !["pth", "pt", "safetensors", "yaml", "yml", "json"].contains(&extension.as_str()) {
            let _ = fs::remove_dir_all(&temporary);
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::UnexpectedOutput,
                "The worker reported an unsupported artifact file type.",
            ));
        }
        let destination_relative = format!("model/file-{index:03}.{extension}");
        let destination = managed_join(&temporary, &destination_relative)?;
        fs::copy(&source, &destination)
            .map_err(|error| VoiceModelError::storage("Cannot copy model artifact", error))?;
        let metadata = fs::metadata(&destination)
            .map_err(|error| VoiceModelError::storage("Cannot inspect model artifact", error))?;
        let content_hash = sha256_file(&destination)
            .map_err(|error| VoiceModelError::storage("Cannot hash model artifact", error))?;
        model_files.push(ModelArtifactFile {
            relative_path: destination_relative,
            content_hash,
            size_bytes: metadata.len(),
        });
    }
    let mut combined = String::new();
    for file in &model_files {
        combined.push_str(&file.relative_path);
        combined.push_str(&file.content_hash);
    }
    let suffix_start = artifact_id.len().saturating_sub(8);
    let artifact = VoiceModelArtifactV1 {
        schema_version: MODEL_ARTIFACT_SCHEMA_VERSION,
        artifact_id: artifact_id.clone(),
        profile_id: job.profile_id.clone(),
        display_name: format!("Synthetic model {}", &artifact_id[suffix_start..]),
        backend_id: job.backend_id.clone(),
        backend_version: payload.backend_version,
        worker_protocol_version: WORKER_PROTOCOL_VERSION,
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_hash: snapshot.content_hash.clone(),
        consent_version: snapshot.consent_version.clone(),
        consent_confirmed_at: snapshot.consent_confirmed_at.clone(),
        training_configuration: job.configuration.clone(),
        training_summary: payload.training_summary,
        model_files,
        model_content_hash: sha256_bytes(combined.as_bytes()),
        evaluation: None,
        approval_status: ModelApprovalStatus::Unevaluated,
        notes: None,
        created_at: now.clone(),
        updated_at: now,
    };
    atomic_write_json(&temporary.join("artifact.json"), &artifact)?;
    if let Some(parent) = artifact_directory.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            VoiceModelError::storage("Cannot create profile model storage", error)
        })?;
    }
    fs::rename(&temporary, &artifact_directory).map_err(|error| {
        let _ = fs::remove_dir_all(&temporary);
        VoiceModelError::new(
            VoiceModelErrorCode::AtomicWriteFailure,
            format!("Cannot commit the versioned model artifact: {error}"),
        )
    })?;
    Ok(artifact)
}
