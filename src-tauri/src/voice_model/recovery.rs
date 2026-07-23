use std::{fs, path::Path};

use super::{
    error::VoiceModelResult,
    indexes::{rebuild_indexes, RecoveryIndexesV1},
    qualification::{QualificationReportV1, QualificationState},
    state::{TrainingJob, TrainingJobState},
    storage::{atomic_write_json, read_json},
};

pub struct StartupRecovery {
    pub interrupted_jobs: Vec<String>,
    pub interrupted_qualifications: Vec<String>,
    pub indexes: RecoveryIndexesV1,
}

pub fn recover_startup(root: &Path) -> VoiceModelResult<StartupRecovery> {
    process_deletion_tombstones(root)?;
    let interrupted_jobs = recover_interrupted_jobs(root)?;
    let interrupted_qualifications = recover_interrupted_qualifications(root)?;
    let indexes = rebuild_indexes(root)?;
    Ok(StartupRecovery {
        interrupted_jobs,
        interrupted_qualifications,
        indexes,
    })
}

fn process_deletion_tombstones(root: &Path) -> VoiceModelResult<()> {
    for category in ["snapshots", "jobs", "temporary-inference", "imports"] {
        process_tombstone_directory(&root.join(category))?;
    }
    if let Ok(profiles) = fs::read_dir(root.join("profiles")) {
        for profile in profiles.filter_map(Result::ok) {
            process_tombstone_directory(&profile.path().join("artifacts"))?;
        }
    }
    Ok(())
}

fn process_tombstone_directory(root: &Path) -> VoiceModelResult<()> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(());
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(".delete-") || !name.ends_with(".tombstone") {
            continue;
        }
        let target_name = fs::read_to_string(entry.path()).map_err(|error| {
            super::error::VoiceModelError::storage("Cannot read deletion tombstone", error)
        })?;
        if target_name.is_empty()
            || target_name.contains('/')
            || target_name.contains('\\')
            || target_name == "."
            || target_name == ".."
        {
            continue;
        }
        let target = root.join(&target_name);
        if target.is_dir() {
            fs::remove_dir_all(&target).map_err(|error| {
                super::error::VoiceModelError::storage("Cannot finish tombstoned deletion", error)
            })?;
        }
        fs::remove_file(entry.path()).map_err(|error| {
            super::error::VoiceModelError::storage(
                "Cannot clear processed deletion tombstone",
                error,
            )
        })?;
    }
    Ok(())
}

pub fn recover_interrupted_jobs(root: &Path) -> VoiceModelResult<Vec<String>> {
    let mut recovered = Vec::new();
    let jobs_root = root.join("jobs");
    let Ok(entries) = fs::read_dir(&jobs_root) else {
        return Ok(recovered);
    };
    for entry in entries.filter_map(Result::ok) {
        let manifest = entry.path().join("job.json");
        if !manifest.is_file() {
            continue;
        }
        let mut job: TrainingJob = read_json(&manifest)?;
        if matches!(
            job.state,
            TrainingJobState::Validating
                | TrainingJobState::Snapshotting
                | TrainingJobState::Preparing
                | TrainingJobState::Preprocessing
                | TrainingJobState::Training
                | TrainingJobState::SavingCheckpoint
                | TrainingJobState::EvaluatingCheckpoint
                | TrainingJobState::Cancelling
        ) {
            job.state = TrainingJobState::Interrupted;
            job.worker_pid = None;
            job.error_summary = Some(
                "The application stopped before the worker sent a terminal event. Resume is never automatic."
                    .to_owned(),
            );
            atomic_write_json(&manifest, &job)?;
            recovered.push(job.job_id);
        }
    }
    Ok(recovered)
}

fn recover_interrupted_qualifications(root: &Path) -> VoiceModelResult<Vec<String>> {
    let mut recovered = Vec::new();
    let qualifications_root = root.join("qualifications");
    let Ok(entries) = fs::read_dir(&qualifications_root) else {
        return Ok(recovered);
    };
    for entry in entries.filter_map(Result::ok) {
        let manifest = entry.path().join("qualification.json");
        if !manifest.is_file() {
            continue;
        }
        let mut report: QualificationReportV1 = read_json(&manifest)?;
        if !matches!(
            report.run.state,
            QualificationState::Qualified
                | QualificationState::QualifiedWithWarnings
                | QualificationState::Failed
                | QualificationState::Cancelled
                | QualificationState::Interrupted
        ) {
            report.run.state = QualificationState::Interrupted;
            report.run.failures.push(
                "The application stopped before qualification reached a terminal state.".to_owned(),
            );
            atomic_write_json(&manifest, &report)?;
            recovered.push(report.run.qualification_id);
        }
    }
    Ok(recovered)
}
