use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::audio::offline_resampler::resample_linear_offline;

use super::{
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    profile::VoiceProfileMetadata,
    take::OriginalFormatMetadata,
};

pub const CANONICAL_SAMPLE_RATE: u32 = 48_000;
pub const CANONICAL_CHANNELS: u16 = 1;
pub const CANONICAL_BITS_PER_SAMPLE: u16 = 24;
pub const MAX_IMPORTED_SECONDS: u32 = 120;

pub struct ImportedAudio {
    pub samples: Vec<f32>,
    pub original_format: OriginalFormatMetadata,
    pub non_finite_count: u64,
}

pub fn import_wav(path: &Path) -> DatasetResult<ImportedAudio> {
    require_wav(path)?;
    let mut reader = WavReader::open(path).map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            "The selected file is not a readable uncompressed WAV.",
        )
    })?;
    let spec = reader.spec();
    if ![44_100, 48_000].contains(&spec.sample_rate) || !(1..=2).contains(&spec.channels) {
        return Err(DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            "Dataset import supports mono/stereo WAV at 44.1 or 48 kHz.",
        ));
    }
    if reader.duration() == 0 {
        return Err(DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            "The selected WAV is empty.",
        ));
    }
    if reader.duration() > spec.sample_rate.saturating_mul(MAX_IMPORTED_SECONDS) {
        return Err(DatasetError::new(
            DatasetErrorCode::ImportTooLong,
            format!("Imported WAV files cannot exceed {MAX_IMPORTED_SECONDS} seconds."),
        ));
    }
    let mut non_finite_count = 0_u64;
    let interleaved = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|sample| {
                sample
                    .map(|value| {
                        if value.is_finite() {
                            value.clamp(-1.0, 1.0)
                        } else {
                            non_finite_count += 1;
                            0.0
                        }
                    })
                    .map_err(|_| {
                        DatasetError::new(
                            DatasetErrorCode::UnsupportedWav,
                            "The float WAV contains invalid sample data.",
                        )
                    })
            })
            .collect::<DatasetResult<Vec<_>>>()?,
        (SampleFormat::Int, 16) => read_integer::<i16>(&mut reader, 16)?,
        (SampleFormat::Int, bits @ (24 | 32)) => read_integer::<i32>(&mut reader, bits)?,
        _ => {
            return Err(DatasetError::new(
                DatasetErrorCode::UnsupportedWav,
                "Supported WAV formats are PCM 16/24/32-bit or IEEE float32.",
            ))
        }
    };
    let mono = to_mono(&interleaved, usize::from(spec.channels));
    let samples = if spec.sample_rate == CANONICAL_SAMPLE_RATE {
        mono
    } else {
        resample_to_canonical(&mono, spec.sample_rate, MAX_IMPORTED_SECONDS as usize)?
    };
    if samples.is_empty() {
        return Err(DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            "The selected WAV contains no complete frames.",
        ));
    }
    Ok(ImportedAudio {
        samples,
        original_format: OriginalFormatMetadata {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            bits_per_sample: spec.bits_per_sample,
            sample_format: match spec.sample_format {
                SampleFormat::Float => "float",
                SampleFormat::Int => "pcm",
            }
            .to_owned(),
        },
        non_finite_count,
    })
}

fn read_integer<T>(
    reader: &mut WavReader<std::io::BufReader<std::fs::File>>,
    bits: u16,
) -> DatasetResult<Vec<f32>>
where
    T: hound::Sample + Into<i64>,
{
    let positive = ((1_i64 << (bits - 1)) - 1) as f32;
    let negative = (1_i64 << (bits - 1)) as f32;
    reader
        .samples::<T>()
        .map(|sample| {
            sample
                .map(|value| {
                    let value: i64 = value.into();
                    if value >= 0 {
                        value as f32 / positive
                    } else {
                        value as f32 / negative
                    }
                })
                .map_err(|_| {
                    DatasetError::new(
                        DatasetErrorCode::UnsupportedWav,
                        "The PCM WAV contains invalid sample data.",
                    )
                })
        })
        .collect()
}

pub fn read_canonical_wav(path: &Path) -> DatasetResult<Vec<f32>> {
    let imported = import_wav(path)?;
    if imported.original_format.sample_rate != CANONICAL_SAMPLE_RATE
        || imported.original_format.channels != 1
        || imported.original_format.bits_per_sample != 24
    {
        return Err(DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "A managed take is not canonical PCM24 mono 48 kHz WAV.",
        ));
    }
    Ok(imported.samples)
}

pub fn write_canonical_wav(path: &Path, samples: &[f32]) -> DatasetResult<()> {
    let spec = WavSpec {
        channels: CANONICAL_CHANNELS,
        sample_rate: CANONICAL_SAMPLE_RATE,
        bits_per_sample: CANONICAL_BITS_PER_SAMPLE,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|error| DatasetError::storage("Cannot create managed WAV", error))?;
    for sample in samples {
        if !sample.is_finite() {
            return Err(DatasetError::new(
                DatasetErrorCode::UnsupportedWav,
                "Non-finite audio cannot be persisted.",
            ));
        }
        let value = (sample.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
        writer
            .write_sample(value)
            .map_err(|error| DatasetError::storage("Cannot write managed WAV", error))?;
    }
    writer
        .finalize()
        .map_err(|error| DatasetError::storage("Cannot finalize managed WAV", error))
}

fn require_wav(path: &Path) -> DatasetResult<()> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("wav"))
    {
        Ok(())
    } else {
        Err(DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            "Choose a .wav file.",
        ))
    }
}

pub fn to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub fn resample_to_canonical(
    samples: &[f32],
    source_rate: u32,
    maximum_seconds: usize,
) -> DatasetResult<Vec<f32>> {
    resample_linear_offline(
        samples,
        usize::from(CANONICAL_CHANNELS),
        source_rate,
        CANONICAL_SAMPLE_RATE,
        CANONICAL_SAMPLE_RATE as usize * maximum_seconds,
    )
    .map_err(|error| {
        DatasetError::new(
            DatasetErrorCode::UnsupportedWav,
            format!("Offline Dataset sample-rate conversion failed: {error}."),
        )
    })
}

#[allow(dead_code)]
fn _profile_type_anchor(_: &VoiceProfileMetadata) {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use hound::{SampleFormat, WavSpec, WavWriter};

    use super::{import_wav, resample_to_canonical, to_mono, CANONICAL_SAMPLE_RATE};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-dataset-import-{label}-{}-{}.wav",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn stereo_to_mono_and_resampling_are_offline_and_deterministic() {
        assert_eq!(to_mono(&[1.0, -1.0, 0.5, 0.5], 2), vec![0.0, 0.5]);
        assert_eq!(
            resample_to_canonical(&[0.0, 1.0], 24_000, 1).unwrap().len(),
            4
        );
    }

    #[test]
    fn imports_pcm16_pcm24_pcm32_and_float32_without_changing_sources() {
        for (label, bits, format) in [
            ("pcm16", 16, SampleFormat::Int),
            ("pcm24", 24, SampleFormat::Int),
            ("pcm32", 32, SampleFormat::Int),
            ("float32", 32, SampleFormat::Float),
        ] {
            let target = path(label);
            let mut writer = WavWriter::create(
                &target,
                WavSpec {
                    channels: 1,
                    sample_rate: 48_000,
                    bits_per_sample: bits,
                    sample_format: format,
                },
            )
            .unwrap();
            match (format, bits) {
                (SampleFormat::Float, _) => {
                    writer.write_sample::<f32>(0.25).unwrap();
                }
                (SampleFormat::Int, 16) => {
                    writer.write_sample::<i16>(8_192).unwrap();
                }
                (SampleFormat::Int, _) => {
                    writer.write_sample::<i32>(1 << (bits - 3)).unwrap();
                }
            }
            writer.finalize().unwrap();
            let before = fs::read(&target).unwrap();
            let imported = import_wav(&target).unwrap();
            assert_eq!(imported.samples.len(), 1);
            assert_eq!(fs::read(&target).unwrap(), before);
            fs::remove_file(target).unwrap();
        }
    }

    #[test]
    fn imports_stereo_44_1_as_canonical_rate_mono_and_rejects_bad_inputs() {
        let target = path("stereo-441");
        let mut writer = WavWriter::create(
            &target,
            WavSpec {
                channels: 2,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..44_100 {
            writer.write_sample::<i16>(16_384).unwrap();
            writer.write_sample::<i16>(-16_384).unwrap();
        }
        writer.finalize().unwrap();
        let imported = import_wav(&target).unwrap();
        assert_eq!(imported.samples.len(), CANONICAL_SAMPLE_RATE as usize);
        assert!(imported.samples.iter().all(|sample| sample.abs() < 0.001));
        fs::remove_file(target).unwrap();
        let invalid = path("empty");
        WavWriter::create(
            &invalid,
            WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 8,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap()
        .finalize()
        .unwrap();
        assert!(import_wav(&invalid).is_err());
        fs::remove_file(invalid).unwrap();
    }
}
