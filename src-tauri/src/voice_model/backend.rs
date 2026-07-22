use serde_json::json;

use super::{
    artifact::VoiceModelArtifactV1,
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    snapshot::TrainingSnapshotV1,
    state::{
        InferenceConfiguration, SeedVcBackendConfiguration, TrainingConfiguration,
        WORKER_PROTOCOL_VERSION,
    },
    worker_protocol::{WorkerCommand, WorkerRequest},
};

pub trait VoiceModelBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn build_training_request(
        &self,
        context: TrainingRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest>;
    fn build_inference_request(
        &self,
        context: InferenceRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest>;
}

pub struct TrainingRequestContext<'a> {
    pub request_id: &'a str,
    pub snapshot: &'a TrainingSnapshotV1,
    pub snapshot_directory: &'a str,
    pub configuration: &'a TrainingConfiguration,
    pub backend: &'a SeedVcBackendConfiguration,
    pub job_directory: &'a str,
    pub resume: bool,
}

pub struct InferenceRequestContext<'a> {
    pub request_id: &'a str,
    pub artifact: &'a VoiceModelArtifactV1,
    pub artifact_directory: &'a str,
    pub source_path: &'a str,
    pub reference_paths: &'a [String],
    pub configuration: &'a InferenceConfiguration,
    pub output_path: &'a str,
    pub backend: &'a SeedVcBackendConfiguration,
}

#[derive(Default)]
pub struct SeedVcLocalBackend;

impl VoiceModelBackend for SeedVcLocalBackend {
    fn backend_id(&self) -> &'static str {
        "seed-vc-local"
    }

    fn display_name(&self) -> &'static str {
        "Seed-VC local backend"
    }

    fn build_training_request(
        &self,
        context: TrainingRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest> {
        if context.snapshot.takes.is_empty() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::NoAcceptedTakes,
                "The snapshot contains no accepted training takes.",
            ));
        }
        Ok(WorkerRequest::new(
            context.request_id,
            if context.resume {
                WorkerCommand::ResumeTraining
            } else {
                WorkerCommand::StartTraining
            },
            json!({
                "backendId": self.backend_id(),
                "seedVcDirectory": context.backend.seed_vc_directory,
                "modelConfigurationPath": context.backend.model_configuration_path,
                "pretrainedCheckpointPaths": context.backend.pretrained_checkpoint_paths,
                "snapshotDirectory": context.snapshot_directory,
                "jobDirectory": context.job_directory,
                "trainingConfiguration": context.configuration,
            }),
        ))
    }

    fn build_inference_request(
        &self,
        context: InferenceRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest> {
        if context.reference_paths.is_empty() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ReferenceAudioMissing,
                "Select at least one accepted reference take.",
            ));
        }
        Ok(WorkerRequest::new(
            context.request_id,
            WorkerCommand::RunInference,
            json!({
                "backendId": self.backend_id(),
                "seedVcDirectory": context.backend.seed_vc_directory,
                "modelConfigurationPath": context.backend.model_configuration_path,
                "artifactDirectory": context.artifact_directory,
                "modelFiles": context.artifact.model_files,
                "sourcePath": context.source_path,
                "referencePaths": context.reference_paths,
                "outputPath": context.output_path,
                "inferenceConfiguration": context.configuration,
            }),
        ))
    }
}

#[cfg(test)]
pub struct MockVoiceModelBackend;

#[cfg(test)]
impl VoiceModelBackend for MockVoiceModelBackend {
    fn backend_id(&self) -> &'static str {
        "mock"
    }
    fn display_name(&self) -> &'static str {
        "Mock voice model backend"
    }
    fn build_training_request(
        &self,
        context: TrainingRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest> {
        Ok(WorkerRequest::new(
            context.request_id,
            if context.resume {
                WorkerCommand::ResumeTraining
            } else {
                WorkerCommand::StartTraining
            },
            json!({"mock": true}),
        ))
    }
    fn build_inference_request(
        &self,
        context: InferenceRequestContext<'_>,
    ) -> VoiceModelResult<WorkerRequest> {
        Ok(WorkerRequest::new(
            context.request_id,
            WorkerCommand::RunInference,
            json!({"mock": true}),
        ))
    }
}

pub fn hello_request(request_id: &str) -> WorkerRequest {
    WorkerRequest {
        protocol_version: WORKER_PROTOCOL_VERSION,
        request_id: request_id.to_owned(),
        command: WorkerCommand::Hello,
        payload: json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::{MockVoiceModelBackend, VoiceModelBackend};

    #[test]
    fn mock_backend_is_explicit_and_identifiable() {
        let backend = MockVoiceModelBackend;
        assert_eq!(backend.backend_id(), "mock");
        assert_eq!(backend.display_name(), "Mock voice model backend");
    }
}
