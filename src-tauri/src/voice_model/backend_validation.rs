use std::{fs, path::Path};

use serde_json::json;

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    state::{
        BackendReadiness, BackendValidationStatus, ModelBackendSettingsV1,
        SeedVcBackendConfiguration, MODEL_BACKEND_SETTINGS_SCHEMA_VERSION, WORKER_PROTOCOL_VERSION,
    },
    worker_process::validate_worker_handshake,
};

pub fn validate_settings(
    settings: &ModelBackendSettingsV1,
) -> VoiceModelResult<BackendValidationStatus> {
    if settings.schema_version != MODEL_BACKEND_SETTINGS_SCHEMA_VERSION {
        return Ok(status(
            BackendReadiness::ConfigurationInvalid,
            "The model-backend settings schema is unsupported.",
        ));
    }
    let Some(configuration) = settings.seed_vc.as_ref() else {
        return Ok(BackendValidationStatus::default());
    };
    if !Path::new(&configuration.python_executable).is_file() {
        return Ok(status(
            BackendReadiness::PythonMissing,
            "The configured Python executable is missing.",
        ));
    }
    let worker_root = Path::new(&configuration.worker_package_directory);
    if !worker_root.join("mam_voice_worker/__main__.py").is_file()
        || !worker_root.join("mam_voice_worker/protocol.py").is_file()
    {
        return Ok(status(
            BackendReadiness::WorkerMissing,
            "The configured Mam Voice worker package is missing.",
        ));
    }
    let backend_root = Path::new(&configuration.seed_vc_directory);
    if !backend_root.join("train.py").is_file() || !backend_root.join("inference.py").is_file() {
        return Ok(status(
            BackendReadiness::BackendMissing,
            "The configured Seed-VC checkout is missing train.py or inference.py.",
        ));
    }
    if !Path::new(&configuration.model_configuration_path).is_file() {
        return Ok(status(
            BackendReadiness::ConfigurationInvalid,
            "The configured Seed-VC model configuration is missing.",
        ));
    }
    if configuration.pretrained_checkpoint_paths.is_empty()
        || configuration
            .pretrained_checkpoint_paths
            .iter()
            .any(|path| !Path::new(path).is_file())
    {
        return Ok(status(
            BackendReadiness::CheckpointMissing,
            "Every required pretrained checkpoint must be configured and present.",
        ));
    }
    validate_output_directory(configuration)?;
    let report = validate_worker_handshake(
        configuration,
        json!({
            "backendId": "seed-vc-local",
            "seedVcDirectory": configuration.seed_vc_directory,
            "modelConfigurationPath": configuration.model_configuration_path,
            "pretrainedCheckpointPaths": configuration.pretrained_checkpoint_paths,
            "outputDirectory": configuration.output_directory,
            "requestedDevice": configuration.device,
            "requestedPrecision": configuration.precision,
            "requiredProtocolVersion": WORKER_PROTOCOL_VERSION,
        }),
    )?;
    if report.protocol_version != WORKER_PROTOCOL_VERSION {
        return Ok(status(
            BackendReadiness::ProtocolMismatch,
            "The configured worker uses an incompatible protocol version.",
        ));
    }
    if !report.devices.contains(&configuration.device)
        || !report.precisions.contains(&configuration.precision)
    {
        return Ok(BackendValidationStatus {
            readiness: BackendReadiness::UnsupportedHardware,
            message: "The configured device or precision is not supported by this environment."
                .to_owned(),
            capability_report: Some(report),
        });
    }
    Ok(BackendValidationStatus {
        readiness: BackendReadiness::Ready,
        message: "The optional local model backend is ready.".to_owned(),
        capability_report: Some(report),
    })
}

pub fn static_readiness(settings: &ModelBackendSettingsV1) -> BackendValidationStatus {
    match validate_static(settings) {
        Ok(()) => status(
            BackendReadiness::ConfigurationInvalid,
            "Validate the configured backend before training or conversion.",
        ),
        Err((readiness, message)) => status(readiness, message),
    }
}

fn validate_static(
    settings: &ModelBackendSettingsV1,
) -> Result<(), (BackendReadiness, &'static str)> {
    let Some(configuration) = settings.seed_vc.as_ref() else {
        return Err((
            BackendReadiness::NotConfigured,
            "Configure the optional local model backend.",
        ));
    };
    if !Path::new(&configuration.python_executable).is_file() {
        return Err((
            BackendReadiness::PythonMissing,
            "The configured Python executable is missing.",
        ));
    }
    if !Path::new(&configuration.worker_package_directory).is_dir() {
        return Err((
            BackendReadiness::WorkerMissing,
            "The configured worker package is missing.",
        ));
    }
    if !Path::new(&configuration.seed_vc_directory).is_dir() {
        return Err((
            BackendReadiness::BackendMissing,
            "The configured Seed-VC checkout is missing.",
        ));
    }
    if configuration.pretrained_checkpoint_paths.is_empty() {
        return Err((
            BackendReadiness::CheckpointMissing,
            "Configure the required pretrained checkpoints.",
        ));
    }
    Ok(())
}

fn validate_output_directory(configuration: &SeedVcBackendConfiguration) -> VoiceModelResult<()> {
    let output = Path::new(&configuration.output_directory);
    fs::create_dir_all(output)
        .map_err(|error| VoiceModelError::storage("Cannot create model output storage", error))?;
    let probe = output.join(format!(".mam-write-test-{}", std::process::id()));
    fs::write(&probe, b"write test")
        .and_then(|_| fs::remove_file(&probe))
        .map_err(|error| {
            VoiceModelError::new(
                VoiceModelErrorCode::StorageUnavailable,
                format!("The configured output directory is not writable: {error}"),
            )
        })
}

fn status(readiness: BackendReadiness, message: impl Into<String>) -> BackendValidationStatus {
    BackendValidationStatus {
        readiness,
        message: message.into(),
        capability_report: None,
    }
}
