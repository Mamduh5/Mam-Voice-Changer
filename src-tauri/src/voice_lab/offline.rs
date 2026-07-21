use serde::Serialize;

use crate::dsp::{
    chain::{DspChain, DspParameters},
    processor::AudioProcessor,
};

use super::clip::AudioClip;

pub const OFFLINE_BLOCK_FRAMES: usize = 512;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMetadata {
    pub latency_frames: usize,
    pub block_frames: usize,
}

pub struct RenderedClip {
    pub clip: AudioClip,
    pub metadata: RenderMetadata,
}

/// Stable insertion point for a future offline conversion implementation.
/// Phase 1 provides only the existing Mam DSP implementation below.
pub trait OfflineVoiceProcessor: Send {
    fn render(
        &mut self,
        input: &AudioClip,
        parameters: DspParameters,
    ) -> Result<RenderedClip, String>;
}

#[derive(Default)]
pub struct ExistingDspOfflineProcessor;

impl OfflineVoiceProcessor for ExistingDspOfflineProcessor {
    fn render(
        &mut self,
        input: &AudioClip,
        parameters: DspParameters,
    ) -> Result<RenderedClip, String> {
        let parameters = parameters.validate()?;
        let mut chain = DspChain::default();
        chain.prepare(input.sample_rate, input.channels, OFFLINE_BLOCK_FRAMES);
        chain.reset();
        chain.set_parameters(parameters);
        let latency_frames = chain.latency_frames();

        let input_frames = input.frames();
        let render_frames =
            (input_frames + latency_frames).div_ceil(OFFLINE_BLOCK_FRAMES) * OFFLINE_BLOCK_FRAMES;
        let mut rendered = vec![0.0_f32; render_frames * input.channels];
        rendered[..input.samples.len()].copy_from_slice(&input.samples);
        for block in rendered.chunks_mut(OFFLINE_BLOCK_FRAMES * input.channels) {
            chain.process(block);
        }

        let aligned_start = latency_frames * input.channels;
        let aligned_end = aligned_start + input.samples.len();
        if aligned_end > rendered.len() {
            return Err("The DSP renderer did not produce enough aligned audio.".to_owned());
        }
        let samples = rendered[aligned_start..aligned_end].to_vec();
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err("The DSP renderer produced invalid audio.".to_owned());
        }
        let clip = AudioClip::new(
            format!("{} (processed)", input.source_name),
            input.sample_rate,
            input.channels,
            samples,
        )?;
        Ok(RenderedClip {
            clip,
            metadata: RenderMetadata {
                latency_frames,
                block_frames: OFFLINE_BLOCK_FRAMES,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ExistingDspOfflineProcessor, OfflineVoiceProcessor, OFFLINE_BLOCK_FRAMES};
    use crate::{dsp::chain::DspParameters, voice_lab::clip::AudioClip};

    #[test]
    fn renders_aligned_finite_audio_with_original_length() {
        let input = AudioClip::new("dry", 48_000, 1, vec![0.1; 1_001]).unwrap();
        let mut backend = ExistingDspOfflineProcessor;
        let rendered = backend.render(&input, DspParameters::default()).unwrap();
        assert_eq!(rendered.clip.samples.len(), input.samples.len());
        assert!(rendered
            .clip
            .samples
            .iter()
            .all(|sample| sample.is_finite()));
        assert_eq!(rendered.metadata.block_frames, OFFLINE_BLOCK_FRAMES);
    }

    #[test]
    fn a_fresh_chain_makes_repeated_renders_deterministic() {
        let samples = (0..2_000)
            .map(|index| (index as f32 * 0.013).sin() * 0.2)
            .collect();
        let input = AudioClip::new("dry", 44_100, 1, samples).unwrap();
        let mut backend = ExistingDspOfflineProcessor;
        let first = backend.render(&input, DspParameters::default()).unwrap();
        let second = backend.render(&input, DspParameters::default()).unwrap();
        assert_eq!(first.clip.samples, second.clip.samples);
    }
}
