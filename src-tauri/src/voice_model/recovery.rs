use std::{fs, path::Path};

use super::{
    error::VoiceModelResult,
    state::{TrainingJob, TrainingJobState},
    storage::{atomic_write_json, read_json},
};

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
