use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    dsp::chain::DspParameters,
    voice_evaluation::{
        manifest::{resolve_case_input, validate_relative_path},
        world::validate_supported_parameters,
    },
};

pub const LISTENING_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_LISTENING_CLIPS: usize = 128;
const MAX_ID_CHARS: usize = 64;
const MAX_TITLE_CHARS: usize = 160;
const MAX_DESCRIPTION_CHARS: usize = 512;
const MAX_TAG_CHARS: usize = 64;
const MAX_TAGS: usize = 32;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListeningManifest {
    pub schema_version: u32,
    pub corpus_root: String,
    pub study: ListeningStudy,
    pub clips: Vec<ListeningClip>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListeningStudy {
    pub id: String,
    pub title: String,
    pub seed: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListeningClip {
    pub id: String,
    pub input: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub transform: ListeningTransform,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListeningTransform {
    pub pitch_semitones: f32,
    pub formant_shift_semitones: f32,
    pub consonant_preservation: f32,
    pub dry_wet: f32,
    #[serde(default)]
    pub age_character: f32,
    #[serde(default)]
    pub breathiness: f32,
    #[serde(default)]
    pub tremor: f32,
    #[serde(default)]
    pub gate_enabled: bool,
    #[serde(default = "default_gate_threshold_db")]
    pub gate_threshold_db: f32,
    #[serde(default)]
    pub input_gain_db: f32,
    #[serde(default)]
    pub output_gain_db: f32,
    #[serde(default = "default_master_ceiling_db")]
    pub master_ceiling_db: f32,
    #[serde(default)]
    pub warmth_db: f32,
    #[serde(default)]
    pub brightness_db: f32,
    #[serde(default)]
    pub limiter_enabled: bool,
    #[serde(default)]
    pub bypass: bool,
    #[serde(default)]
    pub muted: bool,
}

impl ListeningManifest {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("Cannot read listening manifest: {error}"))?;
        Self::from_json(&contents)
    }

    pub fn from_json(contents: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(contents)
            .map_err(|error| format!("Listening manifest is not valid JSON: {error}"))?;
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                "Listening manifest schemaVersion must be an unsigned integer.".to_owned()
            })?;
        if version != u64::from(LISTENING_MANIFEST_SCHEMA_VERSION) {
            return Err(format!(
                "Unsupported listening manifest schema version {version}; expected {LISTENING_MANIFEST_SCHEMA_VERSION}."
            ));
        }
        let manifest: Self = serde_json::from_value(value)
            .map_err(|error| format!("Listening manifest shape is invalid: {error}"))?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    pub fn validate_structure(&self) -> Result<(), String> {
        if self.schema_version != LISTENING_MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported listening manifest schema version {}; expected {LISTENING_MANIFEST_SCHEMA_VERSION}.",
                self.schema_version
            ));
        }
        validate_relative_path(&self.corpus_root, "corpusRoot")?;
        validate_id(&self.study.id, "study id")?;
        validate_visible(&self.study.title, MAX_TITLE_CHARS, "study title")?;
        if self.clips.is_empty() {
            return Err("Listening manifest must contain at least one clip.".to_owned());
        }
        if self.clips.len() > MAX_LISTENING_CLIPS {
            return Err(format!(
                "Listening manifest contains {} clips; the limit is {MAX_LISTENING_CLIPS}.",
                self.clips.len()
            ));
        }
        let mut ids = HashSet::with_capacity(self.clips.len());
        for clip in &self.clips {
            clip.validate()?;
            if !ids.insert(clip.id.as_str()) {
                return Err(format!("Listening clip id '{}' is duplicated.", clip.id));
            }
        }
        Ok(())
    }

    pub fn resolve_corpus_root(&self, manifest_path: &Path) -> Result<PathBuf, String> {
        let parent = manifest_path
            .parent()
            .ok_or_else(|| "Listening manifest path has no parent directory.".to_owned())?;
        let root = parent
            .join(&self.corpus_root)
            .canonicalize()
            .map_err(|error| {
                format!(
                    "Cannot resolve listening corpusRoot '{}': {error}",
                    self.corpus_root
                )
            })?;
        if !root.is_dir() {
            return Err("Listening corpusRoot is not a directory.".to_owned());
        }
        Ok(root)
    }

    pub(crate) fn resolve_inputs(
        &self,
        manifest_path: &Path,
    ) -> Result<Vec<(ListeningClip, PathBuf)>, String> {
        let root = self.resolve_corpus_root(manifest_path)?;
        let mut resolved = Vec::with_capacity(self.clips.len());
        for clip in &self.clips {
            let path = resolve_case_input(&root, &clip.input)
                .map_err(|error| format!("Clip '{}': {error}", clip.id))?;
            resolved.push((clip.clone(), path));
        }
        Ok(resolved)
    }
}

impl ListeningClip {
    fn validate(&self) -> Result<(), String> {
        validate_id(&self.id, "clip id")?;
        validate_relative_path(&self.input, "clip input")?;
        validate_visible(&self.description, MAX_DESCRIPTION_CHARS, "clip description")?;
        if self.tags.len() > MAX_TAGS {
            return Err(format!(
                "Listening clip '{}' has too many tags; the limit is {MAX_TAGS}.",
                self.id
            ));
        }
        for tag in &self.tags {
            validate_visible(tag, MAX_TAG_CHARS, "clip tag")?;
        }
        self.transform.parameters().map(|_| ())
    }
}

impl ListeningTransform {
    pub(crate) fn parameters(self) -> Result<DspParameters, String> {
        let parameters = DspParameters {
            pitch_semitones: self.pitch_semitones,
            formant_shift_semitones: self.formant_shift_semitones,
            consonant_preservation: self.consonant_preservation,
            dry_wet: self.dry_wet,
            age_character: self.age_character,
            breathiness: self.breathiness,
            tremor: self.tremor,
            gate_enabled: self.gate_enabled,
            gate_threshold_db: self.gate_threshold_db,
            input_gain_db: self.input_gain_db,
            output_gain_db: self.output_gain_db,
            master_ceiling_db: self.master_ceiling_db,
            warmth_db: self.warmth_db,
            brightness_db: self.brightness_db,
            limiter_enabled: self.limiter_enabled,
            bypass: self.bypass,
            muted: self.muted,
        }
        .validate()?;
        validate_supported_parameters(parameters)
            .map_err(|error| format!("Listening transforms must be supported by WORLD: {error}"))?;
        Ok(parameters)
    }

    pub(crate) fn grouping_key(self) -> String {
        format!(
            "pitch{:+.2}_formant{:+.2}_preservation{:.2}_dryWet{:.2}",
            self.pitch_semitones,
            self.formant_shift_semitones,
            self.consonant_preservation,
            self.dry_wet
        )
    }
}

fn default_gate_threshold_db() -> f32 {
    DspParameters::default().gate_threshold_db
}

fn default_master_ceiling_db() -> f32 {
    DspParameters::default().master_ceiling_db
}

fn validate_id(value: &str, label: &str) -> Result<(), String> {
    if !value.is_empty()
        && value.len() <= MAX_ID_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(())
    } else {
        Err(format!(
            "Listening {label} must be 1-{MAX_ID_CHARS} ASCII letters, digits, '-' or '_'."
        ))
    }
}

fn validate_visible(value: &str, limit: usize, label: &str) -> Result<(), String> {
    if !value.trim().is_empty()
        && value.chars().count() <= limit
        && !value.chars().any(char::is_control)
    {
        Ok(())
    } else {
        Err(format!(
            "Listening {label} must contain 1-{limit} visible characters."
        ))
    }
}
