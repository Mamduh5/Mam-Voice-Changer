use std::{
    f32::consts::TAU,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    dsp::chain::DspParameters,
    voice_lab::{clip::AudioClip, wav},
};

use super::manifest::{
    AnalysisSegment, EvaluationCase, EvaluationManifest, FormantBand, MetricExpectations,
    SegmentKind, MANIFEST_SCHEMA_VERSION,
};

const SAMPLE_RATE: u32 = 48_000;
const ONE_SECOND: usize = SAMPLE_RATE as usize;
const NOISE_SEED: u32 = 0xA341_316C;

pub fn generate_example(directory: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("Cannot create example directory: {error}"))?;
    let fixtures = directory.join("fixtures");
    fs::create_dir_all(&fixtures)
        .map_err(|error| format!("Cannot create fixture directory: {error}"))?;

    write_fixture(&fixtures, "harmonic-90.wav", harmonic(90.0, ONE_SECOND, 1))?;
    write_fixture(
        &fixtures,
        "harmonic-220.wav",
        harmonic(220.0, ONE_SECOND * 2, 1),
    )?;
    write_fixture(
        &fixtures,
        "harmonic-400.wav",
        harmonic(400.0, ONE_SECOND, 1),
    )?;
    write_fixture(
        &fixtures,
        "formant-vowel.wav",
        formant_vowel(120.0, ONE_SECOND * 2),
    )?;
    write_fixture(
        &fixtures,
        "white-noise.wav",
        deterministic_noise(ONE_SECOND, 0.12),
    )?;
    write_fixture(&fixtures, "fricative.wav", fricative(ONE_SECOND, 0.12))?;
    write_fixture(&fixtures, "silence.wav", vec![0.0; ONE_SECOND])?;
    write_fixture(
        &fixtures,
        "vowel-fricative-vowel.wav",
        vowel_fricative_vowel(),
    )?;
    let mut impulse = vec![0.0; ONE_SECOND / 2];
    impulse[ONE_SECOND / 10] = 0.8;
    write_fixture(&fixtures, "impulse.wav", impulse)?;
    write_fixture(
        &fixtures,
        "stereo-linked-220.wav",
        harmonic(220.0, ONE_SECOND, 2),
    )?;
    write_fixture_at_rate(
        &fixtures,
        "harmonic-220-44100.wav",
        44_100,
        1,
        harmonic_at_rate(220.0, 44_100, 1, 44_100),
    )?;

    let manifest = example_manifest();
    let manifest_path = directory.join("evaluation-manifest.json");
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("Cannot serialize example manifest: {error}"))?;
    fs::write(&manifest_path, format!("{json}\n"))
        .map_err(|error| format!("Cannot write example manifest: {error}"))?;
    Ok(manifest_path)
}

fn write_fixture(directory: &Path, name: &str, samples: Vec<f32>) -> Result<(), String> {
    let channels = usize::from(name.starts_with("stereo-")) + 1;
    write_fixture_at_rate(directory, name, SAMPLE_RATE, channels, samples)
}

fn write_fixture_at_rate(
    directory: &Path,
    name: &str,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
) -> Result<(), String> {
    let clip = AudioClip::new(name, sample_rate, channels, samples)?;
    wav::export(&directory.join(name), &clip)
}

fn harmonic(frequency: f32, frames: usize, channels: usize) -> Vec<f32> {
    harmonic_at_rate(frequency, frames, channels, SAMPLE_RATE)
}

fn harmonic_at_rate(frequency: f32, frames: usize, channels: usize, sample_rate: u32) -> Vec<f32> {
    (0..frames)
        .flat_map(|frame| {
            let sample = (1..=8)
                .map(|harmonic| {
                    (TAU * frequency * harmonic as f32 * frame as f32 / sample_rate as f32).sin()
                        / harmonic as f32
                })
                .sum::<f32>()
                * 0.12;
            std::iter::repeat_n(sample, channels)
        })
        .collect()
}

fn formant_vowel(fundamental: f32, frames: usize) -> Vec<f32> {
    let harmonics = (1..=80)
        .filter_map(|harmonic| {
            let frequency = fundamental * harmonic as f32;
            (frequency < SAMPLE_RATE as f32 * 0.45).then(|| {
                let f1 = (-0.5 * ((frequency - 700.0) / 140.0).powi(2)).exp();
                let f2 = 0.7 * (-0.5 * ((frequency - 1_300.0) / 220.0).powi(2)).exp();
                let floor = 0.015 / harmonic as f32;
                (frequency, f1 + f2 + floor)
            })
        })
        .collect::<Vec<_>>();
    let normalization = harmonics
        .iter()
        .map(|(_, amplitude)| amplitude)
        .sum::<f32>();
    (0..frames)
        .map(|frame| {
            harmonics
                .iter()
                .map(|(frequency, amplitude)| {
                    amplitude * (TAU * frequency * frame as f32 / SAMPLE_RATE as f32).sin()
                })
                .sum::<f32>()
                * (0.3 / normalization)
        })
        .collect()
}

fn deterministic_noise(frames: usize, amplitude: f32) -> Vec<f32> {
    let mut state = NOISE_SEED;
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as f32 * (2.0 / u32::MAX as f32) - 1.0) * amplitude
        })
        .collect()
}

fn fricative(frames: usize, amplitude: f32) -> Vec<f32> {
    let noise = deterministic_noise(frames, amplitude);
    let mut previous = 0.0;
    noise
        .into_iter()
        .map(|sample| {
            let high_passed = sample - previous;
            previous = sample;
            high_passed * 0.5
        })
        .collect()
}

fn vowel_fricative_vowel() -> Vec<f32> {
    let segment = ONE_SECOND / 2;
    let mut samples = harmonic(220.0, segment, 1);
    samples.extend(fricative(segment, 0.1));
    samples.extend(harmonic(180.0, segment, 1));
    samples
}

fn evaluation_parameters() -> DspParameters {
    DspParameters {
        dry_wet: 1.0,
        output_gain_db: 0.0,
        limiter_enabled: false,
        ..DspParameters::default()
    }
}

fn common_expectations() -> MetricExpectations {
    MetricExpectations {
        maximum_duration_delta_frames: Some(0),
        maximum_non_finite_samples: Some(0),
        maximum_output_clipping_ratio: Some(0.001),
        ..MetricExpectations::default()
    }
}

fn voiced_segment(end_ms: u64) -> Vec<AnalysisSegment> {
    vec![AnalysisSegment {
        label: "voiced".to_owned(),
        start_ms: 250,
        end_ms,
        kind: SegmentKind::Voiced,
    }]
}

fn pitch_case(id: &str, description: &str, semitones: f32) -> EvaluationCase {
    let mut parameters = evaluation_parameters();
    parameters.pitch_semitones = semitones;
    let expected = 2.0_f64.powf(f64::from(semitones) / 12.0);
    let mut expectations = common_expectations();
    expectations.maximum_pitch_error_cents = Some(45.0);
    expectations.maximum_voiced_unvoiced_disagreement = Some(0.2);
    expectations.minimum_voiced_frame_coverage = Some(0.7);
    EvaluationCase {
        id: id.to_owned(),
        description: description.to_owned(),
        input: "fixtures/harmonic-220.wav".to_owned(),
        parameters,
        expected_pitch_ratio: Some(expected),
        segments: voiced_segment(1_750),
        formant_bands: Vec::new(),
        expectations,
        tags: vec!["synthetic".to_owned(), "pitch".to_owned()],
    }
}

fn preservation_case(amount: f32) -> EvaluationCase {
    let mut parameters = evaluation_parameters();
    parameters.pitch_semitones = 7.0;
    parameters.consonant_preservation = amount;
    EvaluationCase {
        id: format!("preservation-{}", (amount * 10.0) as u32),
        description: format!("Vowel-fricative-vowel with consonant preservation {amount:.1}"),
        input: "fixtures/vowel-fricative-vowel.wav".to_owned(),
        parameters,
        expected_pitch_ratio: Some(2.0_f64.powf(7.0 / 12.0)),
        segments: vec![
            AnalysisSegment {
                label: "first-vowel".to_owned(),
                start_ms: 150,
                end_ms: 450,
                kind: SegmentKind::Voiced,
            },
            AnalysisSegment {
                label: "fricative".to_owned(),
                start_ms: 600,
                end_ms: 900,
                kind: SegmentKind::Unvoiced,
            },
            AnalysisSegment {
                label: "second-vowel".to_owned(),
                start_ms: 1_100,
                end_ms: 1_400,
                kind: SegmentKind::Voiced,
            },
        ],
        formant_bands: Vec::new(),
        expectations: common_expectations(),
        tags: vec![
            "synthetic".to_owned(),
            "fricative".to_owned(),
            "preservation".to_owned(),
        ],
    }
}

fn formant_case(id: &str, semitones: f32) -> EvaluationCase {
    let mut parameters = evaluation_parameters();
    parameters.formant_shift_semitones = semitones;
    EvaluationCase {
        id: id.to_owned(),
        description: format!("Synthetic vowel formant shift {semitones:+.0} semitones"),
        input: "fixtures/formant-vowel.wav".to_owned(),
        parameters,
        expected_pitch_ratio: Some(1.0),
        segments: voiced_segment(1_750),
        formant_bands: vec![
            FormantBand {
                label: "F1-like".to_owned(),
                minimum_hz: 400.0,
                maximum_hz: 1_050.0,
            },
            FormantBand {
                label: "F2-like".to_owned(),
                minimum_hz: 900.0,
                maximum_hz: 1_900.0,
            },
        ],
        expectations: common_expectations(),
        tags: vec!["synthetic".to_owned(), "formant".to_owned()],
    }
}

fn example_manifest() -> EvaluationManifest {
    let mut neutral_expectations = common_expectations();
    neutral_expectations.maximum_neutral_f0_drift_cents = Some(35.0);
    neutral_expectations.maximum_neutral_rms_change_db = Some(3.0);
    neutral_expectations.minimum_voiced_frame_coverage = Some(0.7);
    let neutral = EvaluationCase {
        id: "neutral".to_owned(),
        description: "Neutral deterministic DSP baseline".to_owned(),
        input: "fixtures/harmonic-220.wav".to_owned(),
        parameters: evaluation_parameters(),
        expected_pitch_ratio: Some(1.0),
        segments: voiced_segment(1_750),
        formant_bands: Vec::new(),
        expectations: neutral_expectations,
        tags: vec![
            "synthetic".to_owned(),
            "neutral".to_owned(),
            "vocal-aging-disabled".to_owned(),
        ],
    };
    let silence = EvaluationCase {
        id: "silence-safety".to_owned(),
        description: "Silence numerical-safety baseline".to_owned(),
        input: "fixtures/silence.wav".to_owned(),
        parameters: evaluation_parameters(),
        expected_pitch_ratio: None,
        segments: vec![AnalysisSegment {
            label: "silence".to_owned(),
            start_ms: 0,
            end_ms: 1_000,
            kind: SegmentKind::Silence,
        }],
        formant_bands: Vec::new(),
        expectations: common_expectations(),
        tags: vec!["synthetic".to_owned(), "silence".to_owned()],
    };
    let sample_rate_case = EvaluationCase {
        id: "neutral-44100-mono".to_owned(),
        description: "Neutral 44.1 kHz mono structural baseline".to_owned(),
        input: "fixtures/harmonic-220-44100.wav".to_owned(),
        parameters: evaluation_parameters(),
        expected_pitch_ratio: Some(1.0),
        segments: voiced_segment(900),
        formant_bands: Vec::new(),
        expectations: common_expectations(),
        tags: vec!["synthetic".to_owned(), "44.1khz".to_owned()],
    };
    let stereo_case = EvaluationCase {
        id: "neutral-48000-stereo".to_owned(),
        description: "Neutral 48 kHz linked-stereo structural baseline".to_owned(),
        input: "fixtures/stereo-linked-220.wav".to_owned(),
        parameters: evaluation_parameters(),
        expected_pitch_ratio: Some(1.0),
        segments: voiced_segment(900),
        formant_bands: Vec::new(),
        expectations: common_expectations(),
        tags: vec!["synthetic".to_owned(), "stereo".to_owned()],
    };
    EvaluationManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        corpus_root: ".".to_owned(),
        cases: vec![
            neutral,
            pitch_case("pitch-up-twelve", "One-octave upward pitch shift", 12.0),
            pitch_case(
                "pitch-down-twelve",
                "One-octave downward pitch shift",
                -12.0,
            ),
            pitch_case(
                "pitch-up-seven",
                "Seven-semitone equal-tempered pitch shift",
                7.0,
            ),
            formant_case("formant-up-four", 4.0),
            formant_case("formant-down-four", -4.0),
            preservation_case(0.0),
            preservation_case(0.5),
            preservation_case(1.0),
            sample_rate_case,
            stereo_case,
            silence,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_fixtures_repeat_and_stereo_is_linked() {
        assert_eq!(
            deterministic_noise(4_800, 0.1),
            deterministic_noise(4_800, 0.1)
        );
        assert_eq!(harmonic(90.0, ONE_SECOND, 1).len(), ONE_SECOND);
        assert_eq!(harmonic(220.0, ONE_SECOND * 2, 1).len(), ONE_SECOND * 2);
        assert_eq!(formant_vowel(120.0, ONE_SECOND * 2).len(), ONE_SECOND * 2);
        assert_eq!(deterministic_noise(ONE_SECOND, 0.12).len(), ONE_SECOND);
        assert_eq!(fricative(ONE_SECOND, 0.12).len(), ONE_SECOND);
        let sequence = vowel_fricative_vowel();
        assert_eq!(sequence.len(), ONE_SECOND * 3 / 2);
        let stereo = harmonic(220.0, ONE_SECOND, 2);
        assert_eq!(stereo.len(), ONE_SECOND * 2);
        assert!(stereo
            .chunks_exact(2)
            .all(|frame| frame[0].to_bits() == frame[1].to_bits()));
        let manifest = example_manifest();
        assert_eq!(manifest.cases.len(), 12);
        let preservation = manifest
            .cases
            .iter()
            .find(|case| case.id == "preservation-5")
            .unwrap();
        assert_eq!(
            preservation
                .segments
                .iter()
                .map(|segment| (segment.label.as_str(), segment.kind))
                .collect::<Vec<_>>(),
            [
                ("first-vowel", SegmentKind::Voiced),
                ("fricative", SegmentKind::Unvoiced),
                ("second-vowel", SegmentKind::Voiced),
            ]
        );
        assert!(manifest
            .cases
            .iter()
            .any(|case| case.tags.iter().any(|tag| tag == "vocal-aging-disabled")));
    }
}
