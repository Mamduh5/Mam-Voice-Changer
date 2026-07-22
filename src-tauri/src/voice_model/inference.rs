use std::path::Path;

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    state::InferenceConfiguration,
};

pub const MAX_INFERENCE_SECONDS: u64 = 30;

pub fn validate_configuration(configuration: &InferenceConfiguration) -> VoiceModelResult<()> {
    if !(1..=100).contains(&configuration.diffusion_steps)
        || !(-24..=24).contains(&configuration.pitch_adjustment_semitones)
        || !(0.5..=2.0).contains(&configuration.length_adjustment)
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::GeneratedWavInvalid,
            "Inference controls are outside their supported bounds.",
        ));
    }
    Ok(())
}

pub fn validate_generated_wav(path: &Path) -> VoiceModelResult<crate::voice_lab::clip::AudioClip> {
    if !path.is_file() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::GeneratedWavInvalid,
            "The worker did not create the expected WAV output.",
        ));
    }
    let clip = crate::voice_lab::wav::import(path).map_err(|message| {
        let code = if message.contains("complete audio frames") {
            VoiceModelErrorCode::GeneratedWavEmpty
        } else if message.contains("invalid audio") {
            VoiceModelErrorCode::GeneratedAudioNonFinite
        } else {
            VoiceModelErrorCode::GeneratedWavInvalid
        };
        VoiceModelError::new(code, message)
    })?;
    if clip.samples.is_empty() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::GeneratedWavEmpty,
            "The generated WAV is empty.",
        ));
    }
    if clip.samples.iter().any(|sample| !sample.is_finite()) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::GeneratedAudioNonFinite,
            "The generated WAV contains non-finite samples.",
        ));
    }
    if clip.frames() as u64 > u64::from(clip.sample_rate) * MAX_INFERENCE_SECONDS {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::GeneratedWavInvalid,
            "The generated WAV exceeds the offline inference duration limit.",
        ));
    }
    Ok(clip)
}

#[cfg(test)]
mod tests {
    use super::validate_configuration;
    use crate::voice_model::state::{InferenceConfiguration, ModelDevice, ModelPrecision};

    fn configuration() -> InferenceConfiguration {
        InferenceConfiguration {
            diffusion_steps: 25,
            f0_conditioning: false,
            pitch_adjustment_semitones: 0,
            length_adjustment: 1.0,
            device: ModelDevice::Cpu,
            precision: ModelPrecision::Float32,
            reference_take_ids: Vec::new(),
        }
    }

    #[test]
    fn accepts_typed_defaults_and_rejects_each_unbounded_control() {
        assert!(validate_configuration(&configuration()).is_ok());
        let mut invalid = configuration();
        invalid.diffusion_steps = 0;
        assert!(validate_configuration(&invalid).is_err());
        invalid = configuration();
        invalid.pitch_adjustment_semitones = 25;
        assert!(validate_configuration(&invalid).is_err());
        invalid = configuration();
        invalid.length_adjustment = 2.1;
        assert!(validate_configuration(&invalid).is_err());
    }
}
