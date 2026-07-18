use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::dsp::chain::DspParameters;

pub struct ParameterState {
    input_gain_db: AtomicU32,
    output_gain_db: AtomicU32,
    limiter_enabled: AtomicBool,
    bypass: AtomicBool,
    muted: AtomicBool,
}

impl Default for ParameterState {
    fn default() -> Self {
        let parameters = DspParameters::default();
        Self {
            input_gain_db: AtomicU32::new(parameters.input_gain_db.to_bits()),
            output_gain_db: AtomicU32::new(parameters.output_gain_db.to_bits()),
            limiter_enabled: AtomicBool::new(parameters.limiter_enabled),
            bypass: AtomicBool::new(parameters.bypass),
            muted: AtomicBool::new(parameters.muted),
        }
    }
}

impl ParameterState {
    pub fn update(&self, parameters: DspParameters) -> Result<(), String> {
        let parameters = parameters.validate()?;
        self.input_gain_db
            .store(parameters.input_gain_db.to_bits(), Ordering::Release);
        self.output_gain_db
            .store(parameters.output_gain_db.to_bits(), Ordering::Release);
        self.limiter_enabled
            .store(parameters.limiter_enabled, Ordering::Release);
        self.bypass.store(parameters.bypass, Ordering::Release);
        self.muted.store(parameters.muted, Ordering::Release);
        Ok(())
    }

    pub fn snapshot(&self) -> DspParameters {
        DspParameters {
            input_gain_db: f32::from_bits(self.input_gain_db.load(Ordering::Acquire)),
            output_gain_db: f32::from_bits(self.output_gain_db.load(Ordering::Acquire)),
            limiter_enabled: self.limiter_enabled.load(Ordering::Acquire),
            bypass: self.bypass.load(Ordering::Acquire),
            muted: self.muted.load(Ordering::Acquire),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParameterState;
    use crate::dsp::chain::DspParameters;

    #[test]
    fn stores_a_valid_snapshot_and_rejects_invalid_gain() {
        let state = ParameterState::default();
        let parameters = DspParameters {
            input_gain_db: 3.0,
            output_gain_db: -6.0,
            limiter_enabled: false,
            bypass: true,
            muted: true,
        };

        state.update(parameters).unwrap();
        assert_eq!(state.snapshot(), parameters);
        assert!(state
            .update(DspParameters {
                input_gain_db: 25.0,
                ..parameters
            })
            .is_err());
        assert_eq!(state.snapshot(), parameters);
    }
}
