use crate::audio::sample_format::InputSample;

pub fn mapped_sample<T: InputSample>(
    input_frame: &[T],
    output_channel: usize,
    output_channels: usize,
) -> f32 {
    match (input_frame.len(), output_channels) {
        (0, _) | (_, 0) => 0.0,
        (_, 1) => average(input_frame),
        (1, _) => input_frame[0].normalized(),
        (input_channels, _) if output_channel < input_channels => {
            input_frame[output_channel].normalized()
        }
        _ => average(input_frame),
    }
}

fn average<T: InputSample>(frame: &[T]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    frame.iter().map(|sample| sample.normalized()).sum::<f32>() / frame.len() as f32
}

#[cfg(test)]
mod tests {
    use super::mapped_sample;

    #[test]
    fn duplicates_mono_into_stereo() {
        let frame = [0.25_f32];
        assert_eq!(mapped_sample(&frame, 0, 2), 0.25);
        assert_eq!(mapped_sample(&frame, 1, 2), 0.25);
    }

    #[test]
    fn averages_stereo_into_mono() {
        assert_eq!(mapped_sample(&[0.5_f32, -0.25], 0, 1), 0.125);
    }

    #[test]
    fn preserves_matching_stereo_channels() {
        let frame = [0.2_f32, 0.7];
        assert_eq!(mapped_sample(&frame, 0, 2), 0.2);
        assert_eq!(mapped_sample(&frame, 1, 2), 0.7);
    }
}
