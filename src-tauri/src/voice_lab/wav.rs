use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::audio::sample_format::OutputSample;

use super::clip::{AudioClip, MAX_CLIP_SECONDS, SUPPORTED_SAMPLE_RATES};

pub fn import(path: &Path) -> Result<AudioClip, String> {
    require_wav_extension(path)?;
    let mut reader = WavReader::open(path).map_err(|error| format!("Cannot open WAV: {error}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels);
    if !SUPPORTED_SAMPLE_RATES.contains(&spec.sample_rate) {
        return Err("Voice Lab supports only 44.1 kHz and 48 kHz WAV files.".to_owned());
    }
    if !(1..=2).contains(&channels) {
        return Err("Voice Lab supports only mono or stereo WAV files.".to_owned());
    }
    if reader.duration() as usize > spec.sample_rate as usize * MAX_CLIP_SECONDS {
        return Err(format!(
            "Voice Lab clips cannot exceed {MAX_CLIP_SECONDS} seconds."
        ));
    }
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| normalize_integer(i64::from(value), 16))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, bits @ (24 | 32)) => reader
            .samples::<i32>()
            .map(|sample| {
                sample
                    .map(|value| normalize_integer(i64::from(value), bits))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("Supported WAV formats are PCM 16/24/32-bit or 32-bit float.".to_owned()),
    };
    let source_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Imported WAV")
        .to_owned();
    AudioClip::new(source_name, spec.sample_rate, channels, samples)
}

pub fn export(path: &Path, clip: &AudioClip) -> Result<(), String> {
    require_wav_extension(path)?;
    let spec = WavSpec {
        channels: clip.channels as u16,
        sample_rate: clip.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer =
        WavWriter::create(path, spec).map_err(|error| format!("Cannot create WAV: {error}"))?;
    for sample in &clip.samples {
        writer
            .write_sample(i16::from_normalized(*sample))
            .map_err(|error| format!("Cannot write WAV: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("Cannot finish WAV: {error}"))
}

fn normalize_integer(value: i64, bits: u16) -> f32 {
    let positive = ((1_i64 << (bits - 1)) - 1) as f32;
    let negative = (1_i64 << (bits - 1)) as f32;
    if value >= 0 {
        value as f32 / positive
    } else {
        value as f32 / negative
    }
}

fn require_wav_extension(path: &Path) -> Result<(), String> {
    let is_wav = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"));
    if is_wav {
        Ok(())
    } else {
        Err("Choose a file with a .wav extension.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use hound::{SampleFormat, WavSpec, WavWriter};

    use super::{export, import};
    use crate::voice_lab::clip::AudioClip;

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-voice-lab-{label}-{}-{}.wav",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn exports_and_imports_pcm16() {
        let target = path("round-trip");
        let clip = AudioClip::new("dry", 48_000, 2, vec![-1.0, 1.0, 0.25, -0.25]).unwrap();
        export(&target, &clip).unwrap();
        let loaded = import(&target).unwrap();
        assert_eq!(loaded.sample_rate, 48_000);
        assert_eq!(loaded.channels, 2);
        assert_eq!(loaded.frames(), 2);
        assert!((loaded.samples[2] - 0.25).abs() < 0.001);
        let _ = fs::remove_file(target);
    }

    #[test]
    fn imports_pcm24_and_rejects_unsupported_rates() {
        let valid = path("pcm24");
        let mut writer = WavWriter::create(
            &valid,
            WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 24,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample::<i32>(4_194_304).unwrap();
        writer.finalize().unwrap();
        assert!((import(&valid).unwrap().samples[0] - 0.5).abs() < 0.001);

        let invalid = path("rate");
        let mut writer = WavWriter::create(
            &invalid,
            WavSpec {
                channels: 1,
                sample_rate: 32_000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample::<i16>(0).unwrap();
        writer.finalize().unwrap();
        assert!(import(&invalid).is_err());
        let _ = fs::remove_file(valid);
        let _ = fs::remove_file(invalid);
    }

    #[test]
    fn rejects_oversized_wav_before_loading_its_samples() {
        let target = path("too-long");
        let mut writer = WavWriter::create(
            &target,
            WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..(44_100 * 15 + 1) {
            writer.write_sample::<i16>(0).unwrap();
        }
        writer.finalize().unwrap();
        let error = import(&target).unwrap_err();
        assert!(error.contains("cannot exceed 15 seconds"));
        let _ = fs::remove_file(target);
    }
}
