use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::dsp::chain::DspParameters;

pub struct ParameterState {
    pitch_semitones: AtomicU32,
    formant_shift_semitones: AtomicU32,
    dry_wet: AtomicU32,
    age_character: AtomicU32,
    breathiness: AtomicU32,
    tremor: AtomicU32,
    gate_enabled: AtomicBool,
    gate_threshold_db: AtomicU32,
    input_gain_db: AtomicU32,
    output_gain_db: AtomicU32,
    master_ceiling_db: AtomicU32,
    warmth_db: AtomicU32,
    brightness_db: AtomicU32,
    limiter_enabled: AtomicBool,
    bypass: AtomicBool,
    muted: AtomicBool,
}

impl Default for ParameterState {
    fn default() -> Self {
        let parameters = DspParameters::default();
        Self {
            pitch_semitones: AtomicU32::new(parameters.pitch_semitones.to_bits()),
            formant_shift_semitones: AtomicU32::new(parameters.formant_shift_semitones.to_bits()),
            dry_wet: AtomicU32::new(parameters.dry_wet.to_bits()),
            age_character: AtomicU32::new(parameters.age_character.to_bits()),
            breathiness: AtomicU32::new(parameters.breathiness.to_bits()),
            tremor: AtomicU32::new(parameters.tremor.to_bits()),
            gate_enabled: AtomicBool::new(parameters.gate_enabled),
            gate_threshold_db: AtomicU32::new(parameters.gate_threshold_db.to_bits()),
            input_gain_db: AtomicU32::new(parameters.input_gain_db.to_bits()),
            output_gain_db: AtomicU32::new(parameters.output_gain_db.to_bits()),
            master_ceiling_db: AtomicU32::new(parameters.master_ceiling_db.to_bits()),
            warmth_db: AtomicU32::new(parameters.warmth_db.to_bits()),
            brightness_db: AtomicU32::new(parameters.brightness_db.to_bits()),
            limiter_enabled: AtomicBool::new(parameters.limiter_enabled),
            bypass: AtomicBool::new(parameters.bypass),
            muted: AtomicBool::new(parameters.muted),
        }
    }
}

impl ParameterState {
    pub fn update(&self, parameters: DspParameters) -> Result<(), String> {
        let parameters = parameters.validate()?;
        self.pitch_semitones
            .store(parameters.pitch_semitones.to_bits(), Ordering::Release);
        self.formant_shift_semitones.store(
            parameters.formant_shift_semitones.to_bits(),
            Ordering::Release,
        );
        self.dry_wet
            .store(parameters.dry_wet.to_bits(), Ordering::Release);
        self.age_character
            .store(parameters.age_character.to_bits(), Ordering::Release);
        self.breathiness
            .store(parameters.breathiness.to_bits(), Ordering::Release);
        self.tremor
            .store(parameters.tremor.to_bits(), Ordering::Release);
        self.gate_enabled
            .store(parameters.gate_enabled, Ordering::Release);
        self.gate_threshold_db
            .store(parameters.gate_threshold_db.to_bits(), Ordering::Release);
        self.input_gain_db
            .store(parameters.input_gain_db.to_bits(), Ordering::Release);
        self.output_gain_db
            .store(parameters.output_gain_db.to_bits(), Ordering::Release);
        self.master_ceiling_db
            .store(parameters.master_ceiling_db.to_bits(), Ordering::Release);
        self.warmth_db
            .store(parameters.warmth_db.to_bits(), Ordering::Release);
        self.brightness_db
            .store(parameters.brightness_db.to_bits(), Ordering::Release);
        self.limiter_enabled
            .store(parameters.limiter_enabled, Ordering::Release);
        self.bypass.store(parameters.bypass, Ordering::Release);
        self.muted.store(parameters.muted, Ordering::Release);
        Ok(())
    }

    pub fn snapshot(&self) -> DspParameters {
        DspParameters {
            pitch_semitones: f32::from_bits(self.pitch_semitones.load(Ordering::Acquire)),
            formant_shift_semitones: f32::from_bits(
                self.formant_shift_semitones.load(Ordering::Acquire),
            ),
            dry_wet: f32::from_bits(self.dry_wet.load(Ordering::Acquire)),
            age_character: f32::from_bits(self.age_character.load(Ordering::Acquire)),
            breathiness: f32::from_bits(self.breathiness.load(Ordering::Acquire)),
            tremor: f32::from_bits(self.tremor.load(Ordering::Acquire)),
            gate_enabled: self.gate_enabled.load(Ordering::Acquire),
            gate_threshold_db: f32::from_bits(self.gate_threshold_db.load(Ordering::Acquire)),
            input_gain_db: f32::from_bits(self.input_gain_db.load(Ordering::Acquire)),
            output_gain_db: f32::from_bits(self.output_gain_db.load(Ordering::Acquire)),
            master_ceiling_db: f32::from_bits(self.master_ceiling_db.load(Ordering::Acquire)),
            warmth_db: f32::from_bits(self.warmth_db.load(Ordering::Acquire)),
            brightness_db: f32::from_bits(self.brightness_db.load(Ordering::Acquire)),
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
            pitch_semitones: 5.0,
            formant_shift_semitones: -2.0,
            dry_wet: 0.75,
            age_character: 0.8,
            breathiness: 0.45,
            tremor: 0.3,
            gate_enabled: true,
            gate_threshold_db: -45.0,
            input_gain_db: 3.0,
            output_gain_db: -6.0,
            master_ceiling_db: -3.0,
            warmth_db: 1.5,
            brightness_db: -1.0,
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
