use serde::{Deserialize, Serialize};

use super::error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualModelRatings {
    pub intelligibility: u8,
    pub target_similarity: u8,
    pub naturalness: u8,
    pub stability: u8,
    pub noise_and_artifacts: u8,
    pub notes: Option<String>,
    pub listening_confirmed: bool,
}

impl ManualModelRatings {
    pub fn validate(&self) -> VoiceModelResult<()> {
        for value in [
            self.intelligibility,
            self.target_similarity,
            self.naturalness,
            self.stability,
            self.noise_and_artifacts,
        ] {
            if !(1..=5).contains(&value) {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::EvaluationIncomplete,
                    "Every manual model rating must be between 1 and 5.",
                ));
            }
        }
        if !self.listening_confirmed {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::EvaluationIncomplete,
                "Confirm manual listening before approving a model.",
            ));
        }
        if self.notes.as_ref().is_some_and(|notes| {
            notes.chars().count() > 2_000 || notes.chars().any(char::is_control)
        }) {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::EvaluationIncomplete,
                "Evaluation notes must be at most 2,000 visible characters.",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEvaluationClip {
    pub phrase_id: String,
    pub phrase_label: String,
    pub result_id: String,
    pub successful: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEvaluationSummary {
    pub schema_version: u32,
    pub clips: Vec<ModelEvaluationClip>,
    pub ratings: ManualModelRatings,
    pub completed_at: String,
}

impl ModelEvaluationSummary {
    pub fn validate_for_approval(&self) -> VoiceModelResult<()> {
        self.ratings.validate()?;
        if self.clips.is_empty() || !self.clips.iter().any(|clip| clip.successful) {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::EvaluationIncomplete,
                "At least one successful synthetic conversion is required for approval.",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationPhrase {
    pub phrase_id: &'static str,
    pub category: &'static str,
    pub text: &'static str,
}

pub fn built_in_evaluation_phrases() -> Vec<EvaluationPhrase> {
    vec![
        EvaluationPhrase { phrase_id: "neutral", category: "Neutral", text: "The quiet room makes every word easy to hear." },
        EvaluationPhrase { phrase_id: "long", category: "Long sentence", text: "Before the meeting begins, please check the notes and read the final paragraph slowly." },
        EvaluationPhrase { phrase_id: "question", category: "Question", text: "Could you bring the blue folder tomorrow?" },
        EvaluationPhrase { phrase_id: "numbers", category: "Numbers", text: "The reference number is forty-two, seven, nineteen." },
        EvaluationPhrase { phrase_id: "plosives", category: "Plosives", text: "Peter packed a bright paper bag." },
        EvaluationPhrase { phrase_id: "sibilants", category: "Sibilants", text: "Six soft silver sails crossed the sea." },
        EvaluationPhrase { phrase_id: "vowel", category: "Sustained vowel", text: "Hold the sound ah at a comfortable pitch." },
        EvaluationPhrase { phrase_id: "pitch", category: "Pitch variation", text: "Read this once gently, then once with a slightly higher pitch." },
    ]
}

#[cfg(test)]
mod tests {
    use super::{ManualModelRatings, ModelEvaluationClip, ModelEvaluationSummary};

    fn ratings(confirmed: bool) -> ManualModelRatings {
        ManualModelRatings {
            intelligibility: 4,
            target_similarity: 3,
            naturalness: 4,
            stability: 3,
            noise_and_artifacts: 4,
            notes: None,
            listening_confirmed: confirmed,
        }
    }

    #[test]
    fn approval_requires_listening_and_a_successful_conversion() {
        let mut summary = ModelEvaluationSummary {
            schema_version: 1,
            clips: Vec::new(),
            ratings: ratings(false),
            completed_at: "1".to_owned(),
        };
        assert!(summary.validate_for_approval().is_err());
        summary.ratings = ratings(true);
        summary.clips.push(ModelEvaluationClip {
            phrase_id: "neutral".to_owned(),
            phrase_label: "Neutral".to_owned(),
            result_id: "result-1".to_owned(),
            successful: true,
        });
        assert!(summary.validate_for_approval().is_ok());
    }
}
