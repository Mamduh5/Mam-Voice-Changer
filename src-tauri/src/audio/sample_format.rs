use cpal::SizedSample;

pub trait InputSample: SizedSample + Copy + Send + 'static {
    fn normalized(self) -> f32;
}

impl InputSample for f32 {
    fn normalized(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

impl InputSample for i16 {
    fn normalized(self) -> f32 {
        if self >= 0 {
            f32::from(self) / f32::from(i16::MAX)
        } else {
            f32::from(self) / -f32::from(i16::MIN)
        }
    }
}

impl InputSample for u16 {
    fn normalized(self) -> f32 {
        (f32::from(self) / f32::from(u16::MAX)) * 2.0 - 1.0
    }
}

pub trait OutputSample: SizedSample + Copy + Send + 'static {
    fn from_normalized(sample: f32) -> Self;
}

impl OutputSample for f32 {
    fn from_normalized(sample: f32) -> Self {
        sample.clamp(-1.0, 1.0)
    }
}

impl OutputSample for i16 {
    fn from_normalized(sample: f32) -> Self {
        let sample = sample.clamp(-1.0, 1.0);
        if sample >= 0.0 {
            (sample * f32::from(i16::MAX)).round() as i16
        } else {
            (sample * -f32::from(i16::MIN)).round() as i16
        }
    }
}

impl OutputSample for u16 {
    fn from_normalized(sample: f32) -> Self {
        (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5) * f32::from(u16::MAX)).round() as u16
    }
}

pub fn supported(format: cpal::SampleFormat) -> bool {
    matches!(
        format,
        cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
    )
}

#[cfg(test)]
mod tests {
    use super::{InputSample, OutputSample};

    #[test]
    fn converts_i16_endpoints_to_normalized_f32() {
        assert_eq!(i16::MIN.normalized(), -1.0);
        assert_eq!(i16::MAX.normalized(), 1.0);
        assert_eq!(0_i16.normalized(), 0.0);
    }

    #[test]
    fn converts_u16_midpoint_and_endpoints() {
        assert_eq!(u16::MIN.normalized(), -1.0);
        assert_eq!(u16::MAX.normalized(), 1.0);
        assert!((32_768_u16.normalized()).abs() < 0.000_1);
    }

    #[test]
    fn clamps_output_conversion() {
        assert_eq!(i16::from_normalized(-2.0), i16::MIN);
        assert_eq!(i16::from_normalized(2.0), i16::MAX);
        assert_eq!(u16::from_normalized(-1.0), u16::MIN);
        assert_eq!(u16::from_normalized(1.0), u16::MAX);
    }
}
