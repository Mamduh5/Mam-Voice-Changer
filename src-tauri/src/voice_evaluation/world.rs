use std::{
    ffi::{c_char, c_int, CStr},
    ptr::{self, NonNull},
    slice,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    dsp::chain::DspParameters,
    voice_lab::clip::{AudioClip, SUPPORTED_SAMPLE_RATES},
};

pub const WORLD_UPSTREAM: &str = "https://github.com/mmorise/World";
pub const WORLD_RELEASE: &str = "v1.0.1";
pub const WORLD_REVISION: &str = "d625e7608ca23a870018f01e7c562ac683d9847f";
const ERROR_CAPACITY: usize = 256;
const MASK_ATTACK_MS: f64 = 5.0;
const MASK_RELEASE_MS: f64 = 20.0;
const BOUNDARY_FADE_MS: f64 = 5.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldConfiguration {
    pub frame_period_ms: f64,
    pub f0_floor_hz: f64,
    pub f0_ceiling_hz: f64,
}

impl Default for WorldConfiguration {
    fn default() -> Self {
        Self {
            frame_period_ms: 5.0,
            f0_floor_hz: 50.0,
            f0_ceiling_hz: 1_000.0,
        }
    }
}

impl WorldConfiguration {
    fn validate(self, sample_rate: u32) -> Result<Self, WorldError> {
        if !SUPPORTED_SAMPLE_RATES.contains(&sample_rate) {
            return Err(WorldError::InvalidInput(format!(
                "WORLD reference rendering supports only {} Hz and {} Hz.",
                SUPPORTED_SAMPLE_RATES[0], SUPPORTED_SAMPLE_RATES[1]
            )));
        }
        if !self.frame_period_ms.is_finite()
            || !(0.1..=20.0).contains(&self.frame_period_ms)
            || !self.f0_floor_hz.is_finite()
            || !self.f0_ceiling_hz.is_finite()
            || self.f0_floor_hz < 20.0
            || self.f0_floor_hz >= self.f0_ceiling_hz
            || self.f0_ceiling_hz >= f64::from(sample_rate) * 0.5
        {
            return Err(WorldError::InvalidInput(
                "WORLD configuration contains invalid frame-period or F0 limits.".to_owned(),
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorldChannelPolicy {
    #[default]
    MonoDirect,
    StereoAverageThenDuplicate,
}

impl WorldChannelPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MonoDirect => "monoDirect",
            Self::StereoAverageThenDuplicate => "stereoAverageThenDuplicate",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DurationAdjustment {
    #[default]
    None,
    TrimmedWithBoundaryFade,
    ZeroPaddedWithBoundaryFade,
}

impl DurationAdjustment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TrimmedWithBoundaryFade => "trimmedWithBoundaryFade",
            Self::ZeroPaddedWithBoundaryFade => "zeroPaddedWithBoundaryFade",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldRenderMetadata {
    pub upstream: String,
    pub release: String,
    pub revision: String,
    pub configuration: WorldConfiguration,
    pub fft_size: usize,
    pub frame_count: usize,
    pub bins_per_frame: usize,
    pub raw_synthesis_frames: usize,
    pub final_frames: usize,
    pub duration_adjustment: DurationAdjustment,
    pub channel_policy: WorldChannelPolicy,
    pub f0_voiced_frame_count: usize,
    pub transformed_f0_clamp_count: usize,
    pub bulk_alignment_offset_frames: i64,
    #[serde(default)]
    pub render_wall_time_ms: f64,
    #[serde(default)]
    pub real_time_factor: f64,
    pub aperiodicity_frequency_warped: bool,
    pub deterministic_excitation_seed: String,
    pub consonant_mask_attack_ms: f64,
    pub consonant_mask_release_ms: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct WorldRenderedClip {
    pub clip: AudioClip,
    pub metadata: WorldRenderMetadata,
}

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("WORLD native status {status}: {message}")]
    Native { status: i32, message: String },
    #[error("WORLD returned malformed metadata: {0}")]
    MalformedMetadata(String),
}

#[repr(C)]
struct NativeConfiguration {
    frame_period_ms: f64,
    f0_floor_hz: f64,
    f0_ceiling_hz: f64,
}

impl From<WorldConfiguration> for NativeConfiguration {
    fn from(value: WorldConfiguration) -> Self {
        Self {
            frame_period_ms: value.frame_period_ms,
            f0_floor_hz: value.f0_floor_hz,
            f0_ceiling_hz: value.f0_ceiling_hz,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativeMetadata {
    sample_rate: u32,
    frame_count: usize,
    fft_size: usize,
    bins_per_frame: usize,
    raw_synthesis_frames: usize,
    frame_period_ms: f64,
    f0_floor_hz: f64,
    f0_ceiling_hz: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativeTransformStats {
    voiced_frame_count: usize,
    clamped_f0_frame_count: usize,
}

enum NativeWorldResult {}

unsafe extern "C" {
    #[cfg(test)]
    fn mam_world_checked_matrix_length(rows: usize, columns: usize, length: *mut usize) -> c_int;
    #[cfg(test)]
    fn mam_world_warp_spectral_envelope(
        source: *const f64,
        bins: usize,
        formant_semitones: f64,
        destination: *mut f64,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn mam_world_analyze(
        samples: *const f32,
        sample_count: usize,
        sample_rate: u32,
        configuration: *const NativeConfiguration,
        output: *mut *mut NativeWorldResult,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn mam_world_destroy(result: *mut NativeWorldResult);
    #[cfg(test)]
    fn mam_world_live_result_count() -> usize;
    fn mam_world_get_metadata(
        result: *const NativeWorldResult,
        metadata: *mut NativeMetadata,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn mam_world_time_axis(result: *const NativeWorldResult) -> *const f64;
    fn mam_world_f0(result: *const NativeWorldResult) -> *const f64;
    fn mam_world_spectral_envelope(result: *const NativeWorldResult) -> *const f64;
    fn mam_world_aperiodicity(result: *const NativeWorldResult) -> *const f64;
    fn mam_world_transform(
        result: *mut NativeWorldResult,
        pitch_semitones: f64,
        formant_semitones: f64,
        stats: *mut NativeTransformStats,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
    fn mam_world_synthesize(
        result: *const NativeWorldResult,
        output: *mut f32,
        output_capacity: usize,
        written: *mut usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> c_int;
}

pub struct WorldAnalysis {
    handle: NonNull<NativeWorldResult>,
    metadata: NativeMetadata,
}

unsafe impl Send for WorldAnalysis {}

impl WorldAnalysis {
    pub fn analyze(
        input: &[f32],
        sample_rate: u32,
        configuration: WorldConfiguration,
    ) -> Result<Self, WorldError> {
        if input.is_empty() {
            return Err(WorldError::InvalidInput(
                "WORLD analysis requires non-empty mono audio.".to_owned(),
            ));
        }
        if input.iter().any(|sample| !sample.is_finite()) {
            return Err(WorldError::InvalidInput(
                "WORLD analysis input must contain only finite samples.".to_owned(),
            ));
        }
        let configuration = configuration.validate(sample_rate)?;
        let native_configuration = NativeConfiguration::from(configuration);
        let mut native = ptr::null_mut();
        native_call(|error, capacity| unsafe {
            mam_world_analyze(
                input.as_ptr(),
                input.len(),
                sample_rate,
                &native_configuration,
                &mut native,
                error,
                capacity,
            )
        })?;
        let handle = NonNull::new(native).ok_or_else(|| {
            WorldError::MalformedMetadata("successful analysis returned a null handle".to_owned())
        })?;
        let mut analysis = Self {
            handle,
            metadata: NativeMetadata::default(),
        };
        analysis.metadata = analysis.read_metadata()?;
        analysis.validate_slices()?;
        Ok(analysis)
    }

    fn read_metadata(&self) -> Result<NativeMetadata, WorldError> {
        let mut metadata = NativeMetadata::default();
        native_call(|error, capacity| unsafe {
            mam_world_get_metadata(self.handle.as_ptr(), &mut metadata, error, capacity)
        })?;
        let matrix_length = metadata
            .frame_count
            .checked_mul(metadata.bins_per_frame)
            .ok_or_else(|| {
                WorldError::MalformedMetadata("feature matrix dimensions overflow".to_owned())
            })?;
        if metadata.sample_rate == 0
            || metadata.frame_count == 0
            || metadata.fft_size < 2
            || metadata.bins_per_frame != metadata.fft_size / 2 + 1
            || metadata.raw_synthesis_frames == 0
            || matrix_length == 0
            || !metadata.frame_period_ms.is_finite()
            || !metadata.f0_floor_hz.is_finite()
            || !metadata.f0_ceiling_hz.is_finite()
        {
            return Err(WorldError::MalformedMetadata(
                "feature dimensions or configuration are invalid".to_owned(),
            ));
        }
        Ok(metadata)
    }

    fn validate_slices(&self) -> Result<(), WorldError> {
        let features = self.features()?;
        if features.time_axis.iter().any(|value| !value.is_finite())
            || features
                .f0
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            || features
                .spectral_envelope
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || features
                .aperiodicity
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(WorldError::MalformedMetadata(
                "feature arrays contain invalid values".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn features(&self) -> Result<WorldFeatures<'_>, WorldError> {
        let matrix_length = self
            .metadata
            .frame_count
            .checked_mul(self.metadata.bins_per_frame)
            .ok_or_else(|| {
                WorldError::MalformedMetadata("feature matrix dimensions overflow".to_owned())
            })?;
        let time_axis =
            NonNull::new(unsafe { mam_world_time_axis(self.handle.as_ptr()) }.cast_mut())
                .ok_or_else(|| {
                    WorldError::MalformedMetadata("null time-axis pointer".to_owned())
                })?;
        let f0 = NonNull::new(unsafe { mam_world_f0(self.handle.as_ptr()) }.cast_mut())
            .ok_or_else(|| WorldError::MalformedMetadata("null F0 pointer".to_owned()))?;
        let spectral =
            NonNull::new(unsafe { mam_world_spectral_envelope(self.handle.as_ptr()) }.cast_mut())
                .ok_or_else(|| WorldError::MalformedMetadata("null spectral pointer".to_owned()))?;
        let aperiodicity =
            NonNull::new(unsafe { mam_world_aperiodicity(self.handle.as_ptr()) }.cast_mut())
                .ok_or_else(|| {
                    WorldError::MalformedMetadata("null aperiodicity pointer".to_owned())
                })?;
        Ok(WorldFeatures {
            time_axis: unsafe {
                slice::from_raw_parts(time_axis.as_ptr(), self.metadata.frame_count)
            },
            f0: unsafe { slice::from_raw_parts(f0.as_ptr(), self.metadata.frame_count) },
            spectral_envelope: unsafe { slice::from_raw_parts(spectral.as_ptr(), matrix_length) },
            aperiodicity: unsafe { slice::from_raw_parts(aperiodicity.as_ptr(), matrix_length) },
            frame_count: self.metadata.frame_count,
            fft_size: self.metadata.fft_size,
            bins_per_frame: self.metadata.bins_per_frame,
            sample_rate: self.metadata.sample_rate,
            frame_period_ms: self.metadata.frame_period_ms,
        })
    }

    pub fn transform(
        &mut self,
        pitch_semitones: f32,
        formant_semitones: f32,
    ) -> Result<WorldTransformStats, WorldError> {
        let mut stats = NativeTransformStats::default();
        native_call(|error, capacity| unsafe {
            mam_world_transform(
                self.handle.as_ptr(),
                f64::from(pitch_semitones),
                f64::from(formant_semitones),
                &mut stats,
                error,
                capacity,
            )
        })?;
        self.validate_slices()?;
        Ok(WorldTransformStats {
            voiced_frame_count: stats.voiced_frame_count,
            clamped_f0_frame_count: stats.clamped_f0_frame_count,
        })
    }

    pub fn synthesize(&self) -> Result<Vec<f32>, WorldError> {
        let mut output = vec![0.0; self.metadata.raw_synthesis_frames];
        let mut written = 0;
        native_call(|error, capacity| unsafe {
            mam_world_synthesize(
                self.handle.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
                &mut written,
                error,
                capacity,
            )
        })?;
        if written != output.len() || output.iter().any(|sample| !sample.is_finite()) {
            return Err(WorldError::MalformedMetadata(
                "synthesis returned an invalid output length or sample".to_owned(),
            ));
        }
        Ok(output)
    }
}

impl Drop for WorldAnalysis {
    fn drop(&mut self) {
        unsafe { mam_world_destroy(self.handle.as_ptr()) };
    }
}

pub struct WorldFeatures<'a> {
    pub time_axis: &'a [f64],
    pub f0: &'a [f64],
    pub spectral_envelope: &'a [f64],
    pub aperiodicity: &'a [f64],
    pub frame_count: usize,
    pub fft_size: usize,
    pub bins_per_frame: usize,
    pub sample_rate: u32,
    pub frame_period_ms: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldTransformStats {
    pub voiced_frame_count: usize,
    pub clamped_f0_frame_count: usize,
}

#[derive(Default)]
pub struct WorldReferenceProcessor {
    pub configuration: WorldConfiguration,
}

impl WorldReferenceProcessor {
    pub fn render(
        &mut self,
        input: &AudioClip,
        parameters: DspParameters,
    ) -> Result<WorldRenderedClip, WorldError> {
        let parameters = parameters.validate().map_err(WorldError::InvalidInput)?;
        validate_supported_parameters(parameters)?;
        let (mono, channel_policy) = linked_mono(input)?;
        let mut analysis = WorldAnalysis::analyze(&mono, input.sample_rate, self.configuration)?;
        let original_f0 = analysis.features()?.f0.to_vec();
        let dimensions = {
            let features = analysis.features()?;
            if features.sample_rate != input.sample_rate
                || (features.frame_period_ms - self.configuration.frame_period_ms).abs()
                    > f64::EPSILON
            {
                return Err(WorldError::MalformedMetadata(
                    "native analysis changed the requested sample rate or frame period".to_owned(),
                ));
            }
            (
                features.fft_size,
                features.frame_count,
                features.bins_per_frame,
            )
        };
        let transform = analysis.transform(
            parameters.pitch_semitones,
            parameters.formant_shift_semitones,
        )?;
        let raw = analysis.synthesize()?;
        let raw_frames = raw.len();
        let (mut world, duration_adjustment) =
            adjust_duration(raw, input.frames(), input.sample_rate);
        let mask = smoothed_unvoiced_mask(
            &original_f0,
            world.len(),
            input.sample_rate,
            self.configuration.frame_period_ms,
        );
        for index in 0..world.len() {
            let preservation =
                f64::from(parameters.consonant_preservation) * f64::from(mask[index]);
            let preserved = f64::from(world[index]) * (1.0 - preservation)
                + f64::from(mono[index]) * preservation;
            let wet = f64::from(parameters.dry_wet);
            world[index] = (f64::from(mono[index]) * (1.0 - wet) + preserved * wet) as f32;
        }
        if world.iter().any(|sample| !sample.is_finite()) {
            return Err(WorldError::MalformedMetadata(
                "offline preservation or dry/wet blending produced invalid audio".to_owned(),
            ));
        }
        let samples = world
            .iter()
            .flat_map(|sample| std::iter::repeat_n(*sample, input.channels))
            .collect::<Vec<_>>();
        let clip = AudioClip::new(
            format!("WORLD reference: {}", input.source_name),
            input.sample_rate,
            input.channels,
            samples,
        )
        .map_err(WorldError::InvalidInput)?;
        Ok(WorldRenderedClip {
            clip,
            metadata: WorldRenderMetadata {
                upstream: WORLD_UPSTREAM.to_owned(),
                release: WORLD_RELEASE.to_owned(),
                revision: WORLD_REVISION.to_owned(),
                configuration: self.configuration,
                fft_size: dimensions.0,
                frame_count: dimensions.1,
                bins_per_frame: dimensions.2,
                raw_synthesis_frames: raw_frames,
                final_frames: input.frames(),
                duration_adjustment,
                channel_policy,
                f0_voiced_frame_count: transform.voiced_frame_count,
                transformed_f0_clamp_count: transform.clamped_f0_frame_count,
                bulk_alignment_offset_frames: 0,
                render_wall_time_ms: 0.0,
                real_time_factor: 0.0,
                aperiodicity_frequency_warped: false,
                deterministic_excitation_seed:
                    "official WORLD randn_reseed fixed state per synthesis call".to_owned(),
                consonant_mask_attack_ms: MASK_ATTACK_MS,
                consonant_mask_release_ms: MASK_RELEASE_MS,
                warnings: vec![
                    "WORLD is an experimental offline evaluator backend.".to_owned(),
                    "Aperiodicity is preserved without frequency-axis warping.".to_owned(),
                    "Bulk alignment correction was not applied; source and WORLD use the common time origin."
                        .to_owned(),
                ],
            },
        })
    }
}

fn native_call(call: impl FnOnce(*mut c_char, usize) -> c_int) -> Result<(), WorldError> {
    let mut error = [0 as c_char; ERROR_CAPACITY];
    let status = call(error.as_mut_ptr(), error.len());
    if status == 0 {
        return Ok(());
    }
    let message = unsafe { CStr::from_ptr(error.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Err(WorldError::Native {
        status,
        message: if message.is_empty() {
            "native WORLD call failed without details".to_owned()
        } else {
            message
        },
    })
}

fn validate_supported_parameters(parameters: DspParameters) -> Result<(), WorldError> {
    let defaults = DspParameters::default();
    let unsupported = [
        (parameters.age_character != 0.0, "ageCharacter"),
        (parameters.breathiness != 0.0, "breathiness"),
        (parameters.tremor != 0.0, "tremor"),
        (parameters.gate_enabled, "gateEnabled"),
        (
            parameters.gate_threshold_db != defaults.gate_threshold_db,
            "gateThresholdDb",
        ),
        (parameters.input_gain_db != 0.0, "inputGainDb"),
        (parameters.output_gain_db != 0.0, "outputGainDb"),
        (
            parameters.master_ceiling_db != defaults.master_ceiling_db,
            "masterCeilingDb",
        ),
        (parameters.warmth_db != 0.0, "warmthDb"),
        (parameters.brightness_db != 0.0, "brightnessDb"),
        (parameters.limiter_enabled, "limiterEnabled"),
        (parameters.bypass, "bypass"),
        (parameters.muted, "muted"),
    ]
    .into_iter()
    .filter_map(|(non_neutral, name)| non_neutral.then_some(name))
    .collect::<Vec<_>>();
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(WorldError::InvalidInput(format!(
            "WORLD reference renderer does not support non-neutral parameter(s): {}.",
            unsupported.join(", ")
        )))
    }
}

fn linked_mono(input: &AudioClip) -> Result<(Vec<f32>, WorldChannelPolicy), WorldError> {
    match input.channels {
        1 => Ok((input.samples.clone(), WorldChannelPolicy::MonoDirect)),
        2 => Ok((
            input
                .samples
                .chunks_exact(2)
                .map(|frame| (frame[0] + frame[1]) * 0.5)
                .collect(),
            WorldChannelPolicy::StereoAverageThenDuplicate,
        )),
        _ => Err(WorldError::InvalidInput(
            "WORLD reference renderer requires mono or stereo input.".to_owned(),
        )),
    }
}

fn adjust_duration(
    mut raw: Vec<f32>,
    requested_frames: usize,
    sample_rate: u32,
) -> (Vec<f32>, DurationAdjustment) {
    if raw.len() == requested_frames {
        return (raw, DurationAdjustment::None);
    }
    let fade_frames =
        ((f64::from(sample_rate) * BOUNDARY_FADE_MS / 1_000.0).round() as usize).max(1);
    let adjustment = if raw.len() > requested_frames {
        raw.truncate(requested_frames);
        DurationAdjustment::TrimmedWithBoundaryFade
    } else {
        raw.resize(requested_frames, 0.0);
        DurationAdjustment::ZeroPaddedWithBoundaryFade
    };
    let start = raw.len().saturating_sub(fade_frames);
    let denominator = (raw.len() - start).max(1) as f32;
    for (offset, sample) in raw[start..].iter_mut().enumerate() {
        *sample *= 1.0 - (offset + 1) as f32 / denominator;
    }
    (raw, adjustment)
}

fn smoothed_unvoiced_mask(
    f0: &[f64],
    sample_count: usize,
    sample_rate: u32,
    frame_period_ms: f64,
) -> Vec<f32> {
    let frame_step = f64::from(sample_rate) * frame_period_ms / 1_000.0;
    let attack = smoothing_coefficient(sample_rate, MASK_ATTACK_MS);
    let release = smoothing_coefficient(sample_rate, MASK_RELEASE_MS);
    let mut state = 0.0_f64;
    (0..sample_count)
        .map(|sample| {
            let frame_position = sample as f64 / frame_step;
            let lower = (frame_position.floor() as usize).min(f0.len() - 1);
            let upper = (lower + 1).min(f0.len() - 1);
            let fraction = frame_position - frame_position.floor();
            let lower_value = if f0[lower] == 0.0 { 1.0 } else { 0.0 };
            let upper_value = if f0[upper] == 0.0 { 1.0 } else { 0.0 };
            let target = lower_value + (upper_value - lower_value) * fraction;
            let coefficient = if target > state { attack } else { release };
            state = target + coefficient * (state - target);
            state.clamp(0.0, 1.0) as f32
        })
        .collect()
}

fn smoothing_coefficient(sample_rate: u32, milliseconds: f64) -> f64 {
    (-1.0 / (f64::from(sample_rate) * milliseconds / 1_000.0)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harmonic(sample_rate: u32, frequency: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                (std::f32::consts::TAU * frequency * frame as f32 / sample_rate as f32).sin() * 0.1
            })
            .collect()
    }

    fn formant_vowel(sample_rate: u32, frames: usize) -> Vec<f32> {
        let fundamental = 120.0_f32;
        let harmonics = (1..=80)
            .filter_map(|harmonic| {
                let frequency = fundamental * harmonic as f32;
                (frequency < sample_rate as f32 * 0.45).then(|| {
                    let f1 = (-0.5 * ((frequency - 700.0) / 140.0).powi(2)).exp();
                    let f2 = 0.7 * (-0.5 * ((frequency - 1_300.0) / 220.0).powi(2)).exp();
                    (frequency, f1 + f2 + 0.015 / harmonic as f32)
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
                        amplitude
                            * (std::f32::consts::TAU * frequency * frame as f32
                                / sample_rate as f32)
                                .sin()
                    })
                    .sum::<f32>()
                    * (0.3 / normalization)
            })
            .collect()
    }

    fn mean_peak_bin(features: &WorldFeatures<'_>, minimum_hz: f64, maximum_hz: f64) -> usize {
        let bin_hz = f64::from(features.sample_rate) / features.fft_size as f64;
        let minimum = (minimum_hz / bin_hz).ceil() as usize;
        let maximum = ((maximum_hz / bin_hz).floor() as usize).min(features.bins_per_frame - 1);
        (minimum..=maximum)
            .max_by(|left, right| {
                let energy = |bin: usize| {
                    (0..features.frame_count)
                        .map(|frame| {
                            features.spectral_envelope[frame * features.bins_per_frame + bin]
                        })
                        .sum::<f64>()
                };
                energy(*left).total_cmp(&energy(*right))
            })
            .unwrap()
    }

    const TEST_FFT_SIZE: usize = 4_096;
    const TEST_BINS: usize = TEST_FFT_SIZE / 2 + 1;
    const TEST_SAMPLE_RATE: f64 = 48_000.0;

    fn synthetic_envelope(peaks: &[(f64, f64, f64)]) -> Vec<f64> {
        let bin_hz = TEST_SAMPLE_RATE / TEST_FFT_SIZE as f64;
        (0..TEST_BINS)
            .map(|bin| {
                let frequency = bin as f64 * bin_hz;
                peaks.iter().fold(1.0e-9, |value, (center, width, height)| {
                    value + height * (-0.5 * ((frequency - center) / width).powi(2)).exp()
                })
            })
            .collect()
    }

    fn warp_envelope(source: &[f64], semitones: f64) -> Vec<f64> {
        let mut destination = vec![0.0; source.len()];
        let mut error = [0 as c_char; ERROR_CAPACITY];
        let status = unsafe {
            mam_world_warp_spectral_envelope(
                source.as_ptr(),
                source.len(),
                semitones,
                destination.as_mut_ptr(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_eq!(
            status,
            0,
            "{}",
            unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
        );
        destination
    }

    fn envelope_peak_hz(envelope: &[f64], minimum_hz: f64, maximum_hz: f64) -> f64 {
        let bin_hz = TEST_SAMPLE_RATE / TEST_FFT_SIZE as f64;
        let minimum = (minimum_hz / bin_hz).ceil() as usize;
        let maximum = ((maximum_hz / bin_hz).floor() as usize).min(envelope.len() - 1);
        let peak = (minimum..=maximum)
            .max_by(|left, right| envelope[*left].total_cmp(&envelope[*right]))
            .unwrap();
        peak as f64 * bin_hz
    }

    fn assert_peak_near(measured: f64, expected: f64) {
        let bin_hz = TEST_SAMPLE_RATE / TEST_FFT_SIZE as f64;
        assert!(
            (measured - expected).abs() <= bin_hz * 1.5,
            "measured {measured:.3} Hz, expected {expected:.3} Hz"
        );
    }

    fn assert_envelopes_close(left: &[f64], right: &[f64]) {
        assert_eq!(left.len(), right.len());
        for (left, right) in left.iter().zip(right) {
            let tolerance = 1.0e-15_f64.max(right.abs() * 1.0e-12);
            assert!((left - right).abs() <= tolerance);
        }
    }

    fn parameters() -> DspParameters {
        DspParameters {
            dry_wet: 1.0,
            output_gain_db: 0.0,
            limiter_enabled: false,
            ..DspParameters::default()
        }
    }

    #[test]
    fn native_validation_rejects_null_zero_and_invalid_rate() {
        let configuration = NativeConfiguration::from(WorldConfiguration::default());
        let mut output = ptr::null_mut();
        let mut error = [0 as c_char; ERROR_CAPACITY];
        let null = unsafe {
            mam_world_analyze(
                ptr::null(),
                1,
                48_000,
                &configuration,
                &mut output,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_ne!(null, 0);
        let sample = 0.0_f32;
        let zero = unsafe {
            mam_world_analyze(
                &sample,
                0,
                48_000,
                &configuration,
                &mut output,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_ne!(zero, 0);
        assert!(WorldAnalysis::analyze(&[sample], 0, WorldConfiguration::default()).is_err());
        let mut length = usize::MAX;
        let overflow = unsafe { mam_world_checked_matrix_length(usize::MAX, 2, &mut length) };
        assert_ne!(overflow, 0);
        assert_eq!(length, 0);

        let analysis = WorldAnalysis::analyze(
            &harmonic(48_000, 220.0, 4_800),
            48_000,
            WorldConfiguration::default(),
        )
        .unwrap();
        let null_metadata = unsafe {
            mam_world_get_metadata(
                analysis.handle.as_ptr(),
                ptr::null_mut(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_ne!(null_metadata, 0);
        let mut tiny_output = [0.0_f32; 1];
        let mut written = usize::MAX;
        let too_small = unsafe {
            mam_world_synthesize(
                analysis.handle.as_ptr(),
                tiny_output.as_mut_ptr(),
                tiny_output.len(),
                &mut written,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_ne!(too_small, 0);
        assert_eq!(written, 0);
    }

    #[test]
    fn analysis_dimensions_values_silence_and_drop_are_safe() {
        for sample_rate in [44_100, 48_000] {
            let samples = harmonic(sample_rate, 180.0, sample_rate as usize / 4);
            let analysis =
                WorldAnalysis::analyze(&samples, sample_rate, WorldConfiguration::default())
                    .unwrap();
            let features = analysis.features().unwrap();
            assert_eq!(features.sample_rate, sample_rate);
            assert_eq!(
                features.spectral_envelope.len(),
                features.frame_count * features.bins_per_frame
            );
            assert_eq!(
                features.aperiodicity.len(),
                features.frame_count * features.bins_per_frame
            );
            assert!(features.f0.iter().all(|value| value.is_finite()));
        }
        let silence =
            WorldAnalysis::analyze(&[0.0; 12_000], 48_000, WorldConfiguration::default()).unwrap();
        assert!(silence
            .features()
            .unwrap()
            .f0
            .iter()
            .all(|value| *value == 0.0));
        let live_with_silence = unsafe { mam_world_live_result_count() };
        drop(silence);
        assert!(unsafe { mam_world_live_result_count() } < live_with_silence);
    }

    #[test]
    fn pitch_formant_and_unvoiced_transforms_preserve_valid_structure() {
        let samples = harmonic(48_000, 220.0, 24_000);
        let mut analysis =
            WorldAnalysis::analyze(&samples, 48_000, WorldConfiguration::default()).unwrap();
        let before = analysis.features().unwrap().f0.to_vec();
        let stats = analysis.transform(7.0, 4.0).unwrap();
        let after = analysis.features().unwrap();
        let ratio = 2.0_f64.powf(7.0 / 12.0);
        for (source, transformed) in before.iter().zip(after.f0) {
            if *source == 0.0 {
                assert_eq!(*transformed, 0.0);
            } else {
                assert!((*transformed / source - ratio).abs() < 1.0e-10);
            }
        }
        assert!(stats.voiced_frame_count > 0);
        assert!(after
            .spectral_envelope
            .iter()
            .all(|value| value.is_finite() && *value > 0.0));
        assert!(after
            .aperiodicity
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value)));
    }

    #[test]
    fn native_envelope_warp_is_symmetric_immutable_and_identity_safe() {
        let source = synthetic_envelope(&[(700.0, 35.0, 1.0)]);
        let original = source.clone();
        for semitones in [0.0_f64, 4.0, -4.0, 6.0, -6.0] {
            let output = warp_envelope(&source, semitones);
            let ratio = 2.0_f64.powf(semitones / 12.0);
            let measured = envelope_peak_hz(&output, 200.0, 1_400.0);
            assert_peak_near(measured, 700.0 * ratio);
            assert!(output.iter().all(|value| value.is_finite() && *value > 0.0));
        }
        assert_eq!(source, original);
        let identity = warp_envelope(&source, 0.0);
        assert_envelopes_close(&identity, &source);
        assert_envelopes_close(&warp_envelope(&identity, 0.0), &identity);
    }

    #[test]
    fn native_two_peak_warp_preserves_direction_order_and_source() {
        let source = synthetic_envelope(&[(700.0, 35.0, 1.0), (1_300.0, 45.0, 0.8)]);
        let original = source.clone();
        for semitones in [4.0_f64, -4.0] {
            let output = warp_envelope(&source, semitones);
            let ratio = 2.0_f64.powf(semitones / 12.0);
            let first = envelope_peak_hz(&output, 500.0 * ratio, 900.0 * ratio);
            let second = envelope_peak_hz(&output, 1_100.0 * ratio, 1_500.0 * ratio);
            assert_peak_near(first, 700.0 * ratio);
            assert_peak_near(second, 1_300.0 * ratio);
            assert_eq!(first < second, 700.0 * ratio < 1_300.0 * ratio);
            if semitones > 0.0 {
                assert!(first > 700.0 && second > 1_300.0);
            } else {
                assert!(first < 700.0 && second < 1_300.0);
            }
        }
        assert_eq!(source, original);
    }

    #[test]
    fn native_envelope_warp_boundaries_remain_positive_and_clamped() {
        let source = synthetic_envelope(&[
            (120.0, 18.0, 1.0),
            (1_100.0, 40.0, 0.8),
            (18_000.0, 100.0, 0.6),
        ]);
        for semitones in [6.0_f64, -6.0] {
            let output = warp_envelope(&source, semitones);
            assert!(output.iter().all(|value| value.is_finite() && *value > 0.0));
            assert_eq!(output.len(), source.len());
        }
    }

    #[test]
    fn envelope_warp_moves_feature_peaks_without_changing_f0_voicing() {
        let samples = formant_vowel(48_000, 24_000);
        let mut upward =
            WorldAnalysis::analyze(&samples, 48_000, WorldConfiguration::default()).unwrap();
        let source_f0 = upward.features().unwrap().f0.to_vec();
        let source_aperiodicity = upward.features().unwrap().aperiodicity.to_vec();
        let source_peak = mean_peak_bin(&upward.features().unwrap(), 400.0, 1_050.0);
        upward.transform(0.0, 4.0).unwrap();
        let upward_features = upward.features().unwrap();
        assert!(mean_peak_bin(&upward_features, 400.0, 1_050.0) > source_peak);
        assert_eq!(upward_features.f0, source_f0);
        assert_eq!(upward_features.aperiodicity, source_aperiodicity);

        let mut downward =
            WorldAnalysis::analyze(&samples, 48_000, WorldConfiguration::default()).unwrap();
        downward.transform(0.0, -4.0).unwrap();
        assert!(mean_peak_bin(&downward.features().unwrap(), 400.0, 1_050.0) < source_peak);
    }

    #[test]
    fn positive_and_negative_formant_renders_are_bit_identical() {
        let input =
            AudioClip::new("formant-vowel", 48_000, 1, formant_vowel(48_000, 48_000)).unwrap();
        for semitones in [4.0_f32, -4.0] {
            let mut render_parameters = parameters();
            render_parameters.formant_shift_semitones = semitones;
            let first = WorldReferenceProcessor::default()
                .render(&input, render_parameters)
                .unwrap();
            let second = WorldReferenceProcessor::default()
                .render(&input, render_parameters)
                .unwrap();
            assert_eq!(first.clip.samples, second.clip.samples);
            assert_eq!(first.clip.frames(), input.frames());
            assert_eq!(first.clip.sample_rate, input.sample_rate);
        }
    }

    #[test]
    fn fricative_like_noise_remains_predominantly_unvoiced() {
        let mut state = 0xA341_316C_u32;
        let mut previous = 0.0_f32;
        let samples = (0..24_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let noise = (state as f32 * (2.0 / u32::MAX as f32) - 1.0) * 0.1;
                let high_passed = (noise - previous) * 0.5;
                previous = noise;
                high_passed
            })
            .collect::<Vec<_>>();
        let analysis =
            WorldAnalysis::analyze(&samples, 48_000, WorldConfiguration::default()).unwrap();
        let features = analysis.features().unwrap();
        let unvoiced = features.f0.iter().filter(|f0| **f0 == 0.0).count();
        assert!(unvoiced as f64 / features.frame_count as f64 >= 0.9);
    }

    #[test]
    fn renderer_has_exact_duration_linked_channels_and_repeatability() {
        let input = AudioClip::new(
            "stereo",
            48_000,
            2,
            harmonic(48_000, 220.0, 24_000)
                .into_iter()
                .flat_map(|sample| [sample, sample])
                .collect(),
        )
        .unwrap();
        let first = WorldReferenceProcessor::default()
            .render(&input, parameters())
            .unwrap();
        let second = WorldReferenceProcessor::default()
            .render(&input, parameters())
            .unwrap();
        assert_eq!(first.clip.frames(), input.frames());
        assert_eq!(first.clip.channels, input.channels);
        assert!(first
            .clip
            .samples
            .chunks_exact(2)
            .all(|frame| frame[0].to_bits() == frame[1].to_bits()));
        assert_eq!(first.clip.samples, second.clip.samples);
        assert_eq!(
            first.metadata.channel_policy,
            WorldChannelPolicy::StereoAverageThenDuplicate
        );
    }

    #[test]
    fn unsupported_non_neutral_parameters_fail_clearly() {
        let input = AudioClip::new("mono", 48_000, 1, vec![0.0; 4_800]).unwrap();
        let error = WorldReferenceProcessor::default()
            .render(
                &input,
                DspParameters {
                    breathiness: 0.1,
                    output_gain_db: 0.0,
                    limiter_enabled: false,
                    ..DspParameters::default()
                },
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("breathiness"));
    }

    #[test]
    fn pinned_source_provenance_and_license_are_present() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let provenance = std::fs::read_to_string(root.join("vendor/world/PROVENANCE.md")).unwrap();
        let license =
            std::fs::read_to_string(root.join("vendor/world/licenses/WORLD-modified-BSD.txt"))
                .unwrap();
        assert!(provenance.contains(WORLD_UPSTREAM));
        assert!(provenance.contains(WORLD_RELEASE));
        assert!(provenance.contains(WORLD_REVISION));
        assert!(license.contains("Redistribution and use in source and binary forms"));
        assert!(root.join("vendor/world/src/harvest.cpp").is_file());
        assert!(root.join("vendor/world/src/cheaptrick.cpp").is_file());
        assert!(root.join("vendor/world/src/d4c.cpp").is_file());
        assert!(root.join("vendor/world/src/synthesis.cpp").is_file());
    }
}
