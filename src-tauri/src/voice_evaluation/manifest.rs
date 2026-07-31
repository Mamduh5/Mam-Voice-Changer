use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::dsp::chain::DspParameters;

pub const MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const PREVIOUS_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_CASES: usize = 128;
const MAX_ID_CHARS: usize = 64;
const MAX_DESCRIPTION_CHARS: usize = 512;
const MAX_LABEL_CHARS: usize = 64;
const MAX_TAGS: usize = 32;
const MAX_SEGMENTS: usize = 64;
const MAX_FORMANT_BANDS: usize = 8;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationManifest {
    pub schema_version: u32,
    pub corpus_root: String,
    pub cases: Vec<EvaluationCase>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationCase {
    pub id: String,
    pub description: String,
    pub input: String,
    #[serde(default)]
    pub renderer: EvaluationRenderer,
    #[serde(default)]
    pub comparison_group: Option<String>,
    pub parameters: DspParameters,
    #[serde(default)]
    pub expected_pitch_ratio: Option<f64>,
    #[serde(default)]
    pub segments: Vec<AnalysisSegment>,
    #[serde(default)]
    pub formant_bands: Vec<FormantBand>,
    #[serde(default)]
    pub expectations: MetricExpectations,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "camelCase")]
pub enum EvaluationRenderer {
    #[default]
    ExistingDsp,
    WorldReference,
}

impl EvaluationRenderer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExistingDsp => "existingDsp",
            Self::WorldReference => "worldReference",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisSegment {
    pub label: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub kind: SegmentKind,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SegmentKind {
    Voiced,
    Unvoiced,
    Silence,
    All,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormantBand {
    pub label: String,
    pub minimum_hz: f64,
    pub maximum_hz: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricExpectations {
    pub maximum_duration_delta_frames: Option<u64>,
    pub maximum_non_finite_samples: Option<u64>,
    pub maximum_output_clipping_ratio: Option<f64>,
    pub maximum_pitch_error_cents: Option<f64>,
    pub maximum_voiced_unvoiced_disagreement: Option<f64>,
    pub minimum_voiced_frame_coverage: Option<f64>,
    pub maximum_neutral_f0_drift_cents: Option<f64>,
    pub maximum_neutral_rms_change_db: Option<f64>,
    pub minimum_formant_ratio: Option<f64>,
    pub maximum_formant_ratio: Option<f64>,
    pub minimum_unvoiced_high_frequency_energy_ratio: Option<f64>,
    pub maximum_unvoiced_log_spectral_distance_db: Option<f64>,
    pub maximum_real_time_factor: Option<f64>,
}

impl EvaluationManifest {
    pub fn from_json(contents: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(contents)
            .map_err(|error| format!("Evaluation manifest is not valid JSON: {error}"))?;
        let schema_version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                "Evaluation manifest schemaVersion must be an unsigned integer.".to_owned()
            })?;
        if schema_version == u64::from(PREVIOUS_MANIFEST_SCHEMA_VERSION)
            && value
                .get("cases")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|cases| {
                    cases.iter().any(|case| {
                        case.get("renderer").is_some() || case.get("comparisonGroup").is_some()
                    })
                })
        {
            return Err(
                "Evaluation manifest schema version 1 cannot select a renderer or comparison group; use schema version 2."
                    .to_owned(),
            );
        }
        let manifest: Self = serde_json::from_str(contents)
            .map_err(|error| format!("Evaluation manifest is not valid JSON: {error}"))?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    pub fn validate_structure(&self) -> Result<(), String> {
        if !matches!(
            self.schema_version,
            PREVIOUS_MANIFEST_SCHEMA_VERSION | MANIFEST_SCHEMA_VERSION
        ) {
            return Err(format!(
                "Unsupported evaluation manifest schema version {}. Supported versions are {} and {}.",
                self.schema_version, PREVIOUS_MANIFEST_SCHEMA_VERSION, MANIFEST_SCHEMA_VERSION
            ));
        }
        validate_relative_path(&self.corpus_root, "corpusRoot")?;
        if self.cases.is_empty() {
            return Err("Evaluation manifest must contain at least one case.".to_owned());
        }
        if self.cases.len() > MAX_CASES {
            return Err(format!(
                "Evaluation manifest contains {} cases; the limit is {MAX_CASES}.",
                self.cases.len()
            ));
        }
        let mut ids = HashSet::with_capacity(self.cases.len());
        for case in &self.cases {
            if self.schema_version == PREVIOUS_MANIFEST_SCHEMA_VERSION
                && (case.renderer != EvaluationRenderer::ExistingDsp
                    || case.comparison_group.is_some())
            {
                return Err(
                    "Evaluation manifest schema version 1 supports only the implicit existingDsp renderer and no comparison groups."
                        .to_owned(),
                );
            }
            case.validate()?;
            if !ids.insert(case.id.as_str()) {
                return Err(format!("Evaluation case id '{}' is duplicated.", case.id));
            }
        }
        Ok(())
    }

    pub fn resolve_corpus_root(&self, manifest_path: &Path) -> Result<PathBuf, String> {
        let parent = manifest_path
            .parent()
            .ok_or_else(|| "Manifest path has no parent directory.".to_owned())?;
        let candidate = parent.join(Path::new(&self.corpus_root));
        candidate
            .canonicalize()
            .map_err(|error| format!("Cannot resolve corpusRoot '{}': {error}", self.corpus_root))
    }
}

impl EvaluationCase {
    fn validate(&self) -> Result<(), String> {
        if !is_safe_id(&self.id) {
            return Err(format!(
                "Evaluation case id '{}' must be 1-{MAX_ID_CHARS} ASCII letters, digits, '-' or '_'.",
                self.id
            ));
        }
        validate_visible_string(&self.description, MAX_DESCRIPTION_CHARS, "description")?;
        validate_relative_path(&self.input, "input")?;
        if let Some(group) = &self.comparison_group {
            if !is_safe_id(group) {
                return Err(format!(
                    "Evaluation comparison group '{}' must be 1-{MAX_ID_CHARS} ASCII letters, digits, '-' or '_'.",
                    group
                ));
            }
        }
        self.parameters.validate()?;
        if self
            .expected_pitch_ratio
            .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
        {
            return Err(format!(
                "Case '{}' expectedPitchRatio must be finite and positive.",
                self.id
            ));
        }
        if self.tags.len() > MAX_TAGS {
            return Err(format!("Case '{}' has too many tags.", self.id));
        }
        for tag in &self.tags {
            validate_visible_string(tag, MAX_LABEL_CHARS, "tag")?;
        }
        if self.segments.len() > MAX_SEGMENTS {
            return Err(format!("Case '{}' has too many segments.", self.id));
        }
        for segment in &self.segments {
            validate_visible_string(&segment.label, MAX_LABEL_CHARS, "segment label")?;
            if segment.start_ms >= segment.end_ms {
                return Err(format!(
                    "Case '{}' segment '{}' must have startMs < endMs.",
                    self.id, segment.label
                ));
            }
        }
        if self.formant_bands.len() > MAX_FORMANT_BANDS {
            return Err(format!("Case '{}' has too many formant bands.", self.id));
        }
        for band in &self.formant_bands {
            validate_visible_string(&band.label, MAX_LABEL_CHARS, "formant band label")?;
            if !band.minimum_hz.is_finite()
                || !band.maximum_hz.is_finite()
                || band.minimum_hz < 20.0
                || band.minimum_hz >= band.maximum_hz
            {
                return Err(format!(
                    "Case '{}' formant band '{}' has invalid frequency bounds.",
                    self.id, band.label
                ));
            }
        }
        self.expectations.validate(&self.id)
    }
}

impl MetricExpectations {
    fn validate(&self, case_id: &str) -> Result<(), String> {
        for (name, value) in [
            (
                "maximumOutputClippingRatio",
                self.maximum_output_clipping_ratio,
            ),
            ("maximumPitchErrorCents", self.maximum_pitch_error_cents),
            (
                "maximumVoicedUnvoicedDisagreement",
                self.maximum_voiced_unvoiced_disagreement,
            ),
            (
                "minimumVoicedFrameCoverage",
                self.minimum_voiced_frame_coverage,
            ),
            (
                "maximumNeutralF0DriftCents",
                self.maximum_neutral_f0_drift_cents,
            ),
            (
                "maximumNeutralRmsChangeDb",
                self.maximum_neutral_rms_change_db,
            ),
            ("minimumFormantRatio", self.minimum_formant_ratio),
            ("maximumFormantRatio", self.maximum_formant_ratio),
            (
                "minimumUnvoicedHighFrequencyEnergyRatio",
                self.minimum_unvoiced_high_frequency_energy_ratio,
            ),
            (
                "maximumUnvoicedLogSpectralDistanceDb",
                self.maximum_unvoiced_log_spectral_distance_db,
            ),
            ("maximumRealTimeFactor", self.maximum_real_time_factor),
        ] {
            if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
                return Err(format!(
                    "Case '{case_id}' expectation {name} must be finite and non-negative."
                ));
            }
        }
        for (name, value) in [
            (
                "maximumOutputClippingRatio",
                self.maximum_output_clipping_ratio,
            ),
            (
                "maximumVoicedUnvoicedDisagreement",
                self.maximum_voiced_unvoiced_disagreement,
            ),
            (
                "minimumVoicedFrameCoverage",
                self.minimum_voiced_frame_coverage,
            ),
        ] {
            if value.is_some_and(|value| value > 1.0) {
                return Err(format!(
                    "Case '{case_id}' expectation {name} cannot exceed 1.0."
                ));
            }
        }
        if let (Some(minimum), Some(maximum)) =
            (self.minimum_formant_ratio, self.maximum_formant_ratio)
        {
            if minimum > maximum {
                return Err(format!(
                    "Case '{case_id}' minimumFormantRatio cannot exceed maximumFormantRatio."
                ));
            }
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

pub fn resolve_case_input(corpus_root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative_path(relative, "input")?;
    let candidate = corpus_root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("Cannot read input WAV '{relative}': {error}"))?;
    if !canonical.starts_with(corpus_root) {
        return Err(format!(
            "Input WAV '{relative}' resolves outside corpusRoot."
        ));
    }
    if !canonical.is_file() {
        return Err(format!("Input WAV '{relative}' is not a file."));
    }
    Ok(canonical)
}

fn validate_relative_path(value: &str, label: &str) -> Result<(), String> {
    let path = Path::new(value);
    let valid = !value.is_empty()
        && !path.is_absolute()
        && !value.contains('\\')
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{label} must be a normalized relative path without traversal."
        ))
    }
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_visible_string(value: &str, maximum: usize, label: &str) -> Result<(), String> {
    let length = value.chars().count();
    if length == 0 || length > maximum || value.chars().any(char::is_control) {
        Err(format!(
            "Evaluation {label} must contain 1-{maximum} visible characters."
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str) -> EvaluationCase {
        EvaluationCase {
            id: id.to_owned(),
            description: "test case".to_owned(),
            input: "fixture.wav".to_owned(),
            renderer: EvaluationRenderer::ExistingDsp,
            comparison_group: None,
            parameters: DspParameters::default(),
            expected_pitch_ratio: None,
            segments: Vec::new(),
            formant_bands: Vec::new(),
            expectations: MetricExpectations::default(),
            tags: Vec::new(),
        }
    }

    fn manifest() -> EvaluationManifest {
        EvaluationManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            corpus_root: ".".to_owned(),
            cases: vec![case("valid-case")],
        }
    }

    #[test]
    fn validates_schema_ids_parameters_segments_and_expectations() {
        assert!(manifest().validate_structure().is_ok());

        let mut invalid = manifest();
        invalid.schema_version = 99;
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.cases.clear();
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.cases.push(case("valid-case"));
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.cases[0].parameters.dry_wet = 2.0;
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.cases[0].segments.push(AnalysisSegment {
            label: "bad".to_owned(),
            start_ms: 10,
            end_ms: 10,
            kind: SegmentKind::All,
        });
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.cases[0].expectations.maximum_pitch_error_cents = Some(f64::NAN);
        assert!(invalid.validate_structure().is_err());
    }

    #[test]
    fn rejects_unknown_fields_and_path_traversal() {
        let json = serde_json::to_string(&manifest()).unwrap();
        let unknown = json.replacen(
            "\"schemaVersion\":2",
            "\"schemaVersion\":2,\"unknown\":true",
            1,
        );
        assert!(EvaluationManifest::from_json(&unknown).is_err());

        let mut invalid = manifest();
        invalid.cases[0].input = "../escape.wav".to_owned();
        assert!(invalid.validate_structure().is_err());
        invalid = manifest();
        invalid.corpus_root = "C:/private".to_owned();
        assert!(invalid.validate_structure().is_err());
    }

    #[test]
    fn schema_one_defaults_to_existing_dsp_and_schema_two_selects_world() {
        let mut legacy = manifest();
        legacy.schema_version = PREVIOUS_MANIFEST_SCHEMA_VERSION;
        let mut value = serde_json::to_value(&legacy).unwrap();
        let case = value["cases"][0].as_object_mut().unwrap();
        case.remove("renderer");
        case.remove("comparisonGroup");
        let json = serde_json::to_string(&value).unwrap();
        let parsed = EvaluationManifest::from_json(&json).unwrap();
        assert_eq!(parsed.cases[0].renderer, EvaluationRenderer::ExistingDsp);

        let mut current = manifest();
        current.cases[0].renderer = EvaluationRenderer::WorldReference;
        current.cases[0].comparison_group = Some("neutral".to_owned());
        let json = serde_json::to_string(&current).unwrap();
        assert_eq!(
            EvaluationManifest::from_json(&json).unwrap().cases[0].renderer,
            EvaluationRenderer::WorldReference
        );

        let legacy_with_renderer = json.replacen("\"schemaVersion\":2", "\"schemaVersion\":1", 1);
        assert!(EvaluationManifest::from_json(&legacy_with_renderer).is_err());
        assert!(EvaluationManifest::from_json(
            &json.replace("\"worldReference\"", "\"unknownRenderer\"")
        )
        .is_err());
    }
}
