use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    state::{BackendCapabilityReport, TrainingConfiguration, TrainingJobState},
};

pub fn validate_configuration(
    configuration: &TrainingConfiguration,
    capabilities: &BackendCapabilityReport,
) -> VoiceModelResult<Vec<String>> {
    if !(10..=100_000).contains(&configuration.maximum_steps) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidTrainingConfiguration,
            "Maximum steps must be between 10 and 100,000.",
        ));
    }
    if configuration.save_interval == 0 || configuration.save_interval > configuration.maximum_steps
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidTrainingConfiguration,
            "Save interval must be positive and no greater than maximum steps.",
        ));
    }
    if !(1..=64).contains(&configuration.batch_size) || configuration.worker_count > 16 {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidTrainingConfiguration,
            "Batch size must be 1–64 and worker count must be 0–16.",
        ));
    }
    if !capabilities.devices.contains(&configuration.device)
        || !capabilities.precisions.contains(&configuration.precision)
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::UnsupportedHardware,
            "The selected device or precision was not reported by the backend.",
        ));
    }
    let mut warnings = Vec::new();
    if configuration.device == super::state::ModelDevice::Cpu {
        warnings.push("CPU-only training may be extremely slow.".to_owned());
    }
    if configuration.batch_size > 8 {
        warnings.push("A large batch size can exhaust accelerator memory.".to_owned());
    }
    if configuration.worker_count > 4 {
        warnings.push("A high worker count can increase memory and disk pressure.".to_owned());
    }
    if configuration.maximum_steps > 10_000 {
        warnings.push("More training steps do not guarantee better quality.".to_owned());
    }
    Ok(warnings)
}

pub fn can_transition(from: TrainingJobState, to: TrainingJobState) -> bool {
    use TrainingJobState::{
        Cancelled, Cancelling, Completed, EvaluatingCheckpoint, Failed, Idle, Interrupted,
        NeedsRecovery, Preparing, Preprocessing, SavingCheckpoint, Snapshotting, Training,
        Validating,
    };
    matches!(
        (from, to),
        (Idle, Validating)
            | (Validating, Snapshotting | Preparing | Failed | Cancelling)
            | (Snapshotting, Preparing | Failed | Cancelling)
            | (Preparing, Preprocessing | Training | Failed | Cancelling)
            | (Preprocessing, Training | Failed | Cancelling)
            | (
                Training,
                SavingCheckpoint | EvaluatingCheckpoint | Completed | Failed | Cancelling
            )
            | (
                SavingCheckpoint,
                Training | EvaluatingCheckpoint | Completed | Failed | Cancelling
            )
            | (
                EvaluatingCheckpoint,
                Training | Completed | Failed | Cancelling
            )
            | (Cancelling, Cancelled | Failed | Interrupted)
            | (Interrupted, NeedsRecovery | Cancelled)
            | (NeedsRecovery, Preparing | Cancelled | Failed)
    )
}

pub fn require_transition(from: TrainingJobState, to: TrainingJobState) -> VoiceModelResult<()> {
    if can_transition(from, to) {
        Ok(())
    } else {
        Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidStateTransition,
            format!("Training cannot move from {from:?} to {to:?}."),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{can_transition, require_transition};
    use crate::voice_model::state::TrainingJobState;

    #[test]
    fn accepts_only_explicit_training_transitions() {
        assert!(can_transition(
            TrainingJobState::Preparing,
            TrainingJobState::Preprocessing
        ));
        assert!(can_transition(
            TrainingJobState::Training,
            TrainingJobState::Completed
        ));
        assert!(
            require_transition(TrainingJobState::Completed, TrainingJobState::Training).is_err()
        );
    }
}
