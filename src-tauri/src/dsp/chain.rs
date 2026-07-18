use serde::{Deserialize, Serialize};

use super::{gain::Gain, high_pass::HighPass, limiter::SoftLimiter, processor::AudioProcessor};

pub const MIN_GAIN_DB: f32 = -24.0;
pub const MAX_GAIN_DB: f32 = 24.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DspParameters {
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub limiter_enabled: bool,
    pub bypass: bool,
    pub muted: bool,
}

impl Default for DspParameters {
    fn default() -> Self {
        Self {
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            limiter_enabled: true,
            bypass: false,
            muted: false,
        }
    }
}

impl DspParameters {
    pub fn validate(self) -> Result<Self, String> {
        validate_gain("Input gain", self.input_gain_db)?;
        validate_gain("Output gain", self.output_gain_db)?;
        Ok(self)
    }
}

fn validate_gain(label: &str, gain_db: f32) -> Result<(), String> {
    if gain_db.is_finite() && (MIN_GAIN_DB..=MAX_GAIN_DB).contains(&gain_db) {
        Ok(())
    } else {
        Err(format!(
            "{label} must be a finite value between {MIN_GAIN_DB} dB and {MAX_GAIN_DB} dB."
        ))
    }
}

pub struct DspChain {
    parameters: DspParameters,
    input_gain: Gain,
    high_pass: HighPass,
    output_gain: Gain,
    limiter: SoftLimiter,
}

impl Default for DspChain {
    fn default() -> Self {
        let parameters = DspParameters::default();
        Self {
            parameters,
            input_gain: Gain::new(parameters.input_gain_db),
            high_pass: HighPass::default(),
            output_gain: Gain::new(parameters.output_gain_db),
            limiter: SoftLimiter,
        }
    }
}

impl DspChain {
    pub fn set_parameters(&mut self, parameters: DspParameters) {
        self.parameters = parameters;
        self.input_gain.set_gain_db(parameters.input_gain_db);
        self.output_gain.set_gain_db(parameters.output_gain_db);
    }
}

impl AudioProcessor for DspChain {
    fn prepare(&mut self, sample_rate: u32, channels: usize, block_size: usize) {
        self.input_gain.prepare(sample_rate, channels, block_size);
        self.high_pass.prepare(sample_rate, channels, block_size);
        self.output_gain.prepare(sample_rate, channels, block_size);
        self.limiter.prepare(sample_rate, channels, block_size);
    }

    fn process(&mut self, samples: &mut [f32]) {
        if self.parameters.muted {
            samples.fill(0.0);
            return;
        }
        if self.parameters.bypass {
            return;
        }

        self.input_gain.process(samples);
        self.high_pass.process(samples);
        self.output_gain.process(samples);
        if self.parameters.limiter_enabled {
            self.limiter.process(samples);
        }
    }

    fn reset(&mut self) {
        self.input_gain.reset();
        self.high_pass.reset();
        self.output_gain.reset();
        self.limiter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, DspChain, DspParameters};

    fn prepared_chain(parameters: DspParameters) -> DspChain {
        let mut chain = DspChain::default();
        chain.prepare(48_000, 1, 256);
        chain.set_parameters(parameters);
        chain
    }

    #[test]
    fn bypass_returns_the_input_unchanged() {
        let mut chain = prepared_chain(DspParameters {
            input_gain_db: 24.0,
            output_gain_db: 24.0,
            bypass: true,
            ..DspParameters::default()
        });
        let mut samples = [0.25, -0.5];

        chain.process(&mut samples);

        assert_eq!(samples, [0.25, -0.5]);
    }

    #[test]
    fn mute_wins_over_bypass() {
        let mut chain = prepared_chain(DspParameters {
            bypass: true,
            muted: true,
            ..DspParameters::default()
        });
        let mut samples = [0.25, -0.5];

        chain.process(&mut samples);

        assert_eq!(samples, [0.0, 0.0]);
    }

    #[test]
    fn validates_gain_ranges() {
        assert!(DspParameters {
            input_gain_db: f32::NAN,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
        assert!(DspParameters {
            output_gain_db: 24.1,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
    }
}
