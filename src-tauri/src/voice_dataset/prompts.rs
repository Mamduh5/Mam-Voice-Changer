use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptCategory {
    NeutralStatement,
    Question,
    NumbersAndDates,
    NamesAndProperNouns,
    Plosives,
    Sibilants,
    SustainedVowels,
    ShortPhrase,
    LongPhrase,
    ExpressiveVariation,
    Custom,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoicePrompt {
    pub id: String,
    pub text: String,
    pub category: PromptCategory,
    pub recommended_take_duration_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptPack {
    pub id: String,
    pub version: u32,
    pub display_name: String,
    pub language: String,
    pub prompts: Vec<VoicePrompt>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptPackReference {
    pub id: String,
    pub version: u32,
}

pub fn built_in_english_pack() -> PromptPack {
    let entries = [
        (
            "en-neutral-01",
            "The window is open to the morning air.",
            PromptCategory::NeutralStatement,
        ),
        (
            "en-question-01",
            "Would you like some tea before we leave?",
            PromptCategory::Question,
        ),
        (
            "en-numbers-01",
            "Our appointment is at nine fifteen on July twenty-third.",
            PromptCategory::NumbersAndDates,
        ),
        (
            "en-names-01",
            "Maya and Daniel visited Bangkok together.",
            PromptCategory::NamesAndProperNouns,
        ),
        (
            "en-plosives-01",
            "Bright paper boats bob beside the pier.",
            PromptCategory::Plosives,
        ),
        (
            "en-sibilants-01",
            "Soft summer breezes crossed the silent street.",
            PromptCategory::Sibilants,
        ),
        (
            "en-vowel-01",
            "Hold the sound: ah, ee, oh.",
            PromptCategory::SustainedVowels,
        ),
        (
            "en-short-01",
            "I am ready now.",
            PromptCategory::ShortPhrase,
        ),
        (
            "en-long-01",
            "After the rain stopped, we walked slowly home and talked about the day.",
            PromptCategory::LongPhrase,
        ),
        (
            "en-expression-01",
            "That is wonderful news!",
            PromptCategory::ExpressiveVariation,
        ),
        (
            "en-neutral-02",
            "A small lamp glows beside the book.",
            PromptCategory::NeutralStatement,
        ),
        (
            "en-question-02",
            "Did you remember to lock the garden gate?",
            PromptCategory::Question,
        ),
    ];
    let pack = PromptPack {
        id: "mam-english-core".to_owned(),
        version: 1,
        display_name: "Mam English Core".to_owned(),
        language: "English".to_owned(),
        prompts: entries
            .into_iter()
            .map(|(id, text, category)| VoicePrompt {
                id: id.to_owned(),
                text: text.to_owned(),
                category,
                recommended_take_duration_ms: Some(6_000),
            })
            .collect(),
    };
    debug_assert!(validate_prompt_pack(&pack));
    pack
}

pub fn validate_prompt_pack(pack: &PromptPack) -> bool {
    let mut ids = HashSet::new();
    !pack.prompts.is_empty()
        && pack
            .prompts
            .iter()
            .all(|prompt| !prompt.text.trim().is_empty() && ids.insert(&prompt.id))
}

#[cfg(test)]
mod tests {
    use super::{built_in_english_pack, validate_prompt_pack, PromptCategory};

    #[test]
    fn built_in_pack_has_unique_utf8_prompts_and_categories() {
        let pack = built_in_english_pack();
        assert!(validate_prompt_pack(&pack));
        assert!(pack.prompts.iter().any(|prompt| prompt.text.is_ascii()));
        assert!(pack
            .prompts
            .iter()
            .any(|prompt| prompt.category == PromptCategory::Plosives));
        assert!(pack.prompts.len() >= 10);
    }
}
