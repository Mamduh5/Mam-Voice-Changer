use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering},
    RwLock,
};

use serde::Serialize;

use crate::{
    audio::{reliability::ReliabilityProfile, stream_config::ActiveStreamFormat},
    state::engine_state::EngineState,
};

const UNOBSERVED_MINIMUM: u64 = u64::MAX;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: EngineState,
    pub input_level: f32,
    pub output_level: f32,
    pub monitor_level: f32,
    pub reliability_profile: ReliabilityProfile,
    pub input_callback_gaps: u64,
    pub input_ring_overflows: u64,
    pub expander_attenuated_frames: u64,
    pub dsp_input_underruns: u64,
    pub dsp_processing_deadline_misses: u64,
    pub destination_ring_overflows: u64,
    pub monitor_ring_overflows: u64,
    pub output_ring_underruns: u64,
    pub monitor_output_underruns: u64,
    pub output_callback_gaps: u64,
    pub monitor_callback_gaps: u64,
    pub concealed_frames: u64,
    pub monitor_concealed_frames: u64,
    pub stream_restart_count: u64,
    pub current_input_ring_fill_frames: u64,
    pub minimum_input_ring_fill_frames: u64,
    pub maximum_input_ring_fill_frames: u64,
    pub current_output_ring_fill_frames: u64,
    pub maximum_output_ring_fill_frames: u64,
    pub current_monitor_ring_fill_frames: u64,
    pub maximum_monitor_ring_fill_frames: u64,
    pub maximum_dsp_processing_time_ms: f32,
    pub startup_prefill_target_frames: u64,
    pub startup_prefill_achieved_frames: u64,
    pub startup_prefill_timed_out: bool,
    pub clock_drift_correction_ratio: f32,
    pub minimum_clock_drift_correction_ratio: f32,
    pub maximum_clock_drift_correction_ratio: f32,
    pub estimated_latency_ms: f32,
    pub dsp_processing_latency_ms: f32,
    pub total_estimated_latency_ms: f32,
    pub active_stream_format: Option<ActiveStreamFormat>,
    pub last_runtime_error: Option<String>,
    pub message: String,
}

pub struct SharedMetrics {
    state: AtomicU8,
    input_level: AtomicU32,
    output_level: AtomicU32,
    monitor_level: AtomicU32,
    reliability_profile: AtomicU8,
    input_callback_gaps: AtomicU64,
    input_ring_overflows: AtomicU64,
    expander_attenuated_frames: AtomicU64,
    dsp_input_underruns: AtomicU64,
    dsp_processing_deadline_misses: AtomicU64,
    destination_ring_overflows: AtomicU64,
    monitor_ring_overflows: AtomicU64,
    output_ring_underruns: AtomicU64,
    monitor_output_underruns: AtomicU64,
    output_callback_gaps: AtomicU64,
    monitor_callback_gaps: AtomicU64,
    concealed_frames: AtomicU64,
    monitor_concealed_frames: AtomicU64,
    stream_restart_count: AtomicU64,
    current_input_ring_fill_frames: AtomicU64,
    minimum_input_ring_fill_frames: AtomicU64,
    maximum_input_ring_fill_frames: AtomicU64,
    current_output_ring_fill_frames: AtomicU64,
    maximum_output_ring_fill_frames: AtomicU64,
    current_monitor_ring_fill_frames: AtomicU64,
    maximum_monitor_ring_fill_frames: AtomicU64,
    maximum_dsp_processing_microseconds: AtomicU64,
    startup_prefill_target_frames: AtomicU64,
    startup_prefill_achieved_frames: AtomicU64,
    startup_prefill_timed_out: AtomicBool,
    clock_drift_correction_ratio: AtomicU32,
    minimum_clock_drift_correction_ratio: AtomicU32,
    maximum_clock_drift_correction_ratio: AtomicU32,
    estimated_latency_ms: AtomicU32,
    dsp_processing_latency_ms: AtomicU32,
    total_estimated_latency_ms: AtomicU32,
    active_stream_format: RwLock<Option<ActiveStreamFormat>>,
    last_runtime_error: RwLock<Option<String>>,
}

impl Default for SharedMetrics {
    fn default() -> Self {
        Self {
            state: AtomicU8::new(EngineState::Stopped as u8),
            input_level: AtomicU32::new(0.0_f32.to_bits()),
            output_level: AtomicU32::new(0.0_f32.to_bits()),
            monitor_level: AtomicU32::new(0.0_f32.to_bits()),
            reliability_profile: AtomicU8::new(profile_to_u8(ReliabilityProfile::Balanced)),
            input_callback_gaps: AtomicU64::new(0),
            input_ring_overflows: AtomicU64::new(0),
            expander_attenuated_frames: AtomicU64::new(0),
            dsp_input_underruns: AtomicU64::new(0),
            dsp_processing_deadline_misses: AtomicU64::new(0),
            destination_ring_overflows: AtomicU64::new(0),
            monitor_ring_overflows: AtomicU64::new(0),
            output_ring_underruns: AtomicU64::new(0),
            monitor_output_underruns: AtomicU64::new(0),
            output_callback_gaps: AtomicU64::new(0),
            monitor_callback_gaps: AtomicU64::new(0),
            concealed_frames: AtomicU64::new(0),
            monitor_concealed_frames: AtomicU64::new(0),
            stream_restart_count: AtomicU64::new(0),
            current_input_ring_fill_frames: AtomicU64::new(0),
            minimum_input_ring_fill_frames: AtomicU64::new(UNOBSERVED_MINIMUM),
            maximum_input_ring_fill_frames: AtomicU64::new(0),
            current_output_ring_fill_frames: AtomicU64::new(0),
            maximum_output_ring_fill_frames: AtomicU64::new(0),
            current_monitor_ring_fill_frames: AtomicU64::new(0),
            maximum_monitor_ring_fill_frames: AtomicU64::new(0),
            maximum_dsp_processing_microseconds: AtomicU64::new(0),
            startup_prefill_target_frames: AtomicU64::new(0),
            startup_prefill_achieved_frames: AtomicU64::new(0),
            startup_prefill_timed_out: AtomicBool::new(false),
            clock_drift_correction_ratio: AtomicU32::new(1.0_f32.to_bits()),
            minimum_clock_drift_correction_ratio: AtomicU32::new(1.0_f32.to_bits()),
            maximum_clock_drift_correction_ratio: AtomicU32::new(1.0_f32.to_bits()),
            estimated_latency_ms: AtomicU32::new(0.0_f32.to_bits()),
            dsp_processing_latency_ms: AtomicU32::new(0.0_f32.to_bits()),
            total_estimated_latency_ms: AtomicU32::new(0.0_f32.to_bits()),
            active_stream_format: RwLock::new(None),
            last_runtime_error: RwLock::new(None),
        }
    }
}

impl SharedMetrics {
    pub fn set_state(&self, state: EngineState) {
        self.state.store(state as u8, Ordering::Release);
        if !matches!(state, EngineState::Running | EngineState::Degraded) {
            self.input_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
            self.output_level
                .store(0.0_f32.to_bits(), Ordering::Relaxed);
            self.monitor_level
                .store(0.0_f32.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn set_input_level(&self, level: f32) {
        self.input_level
            .store(finite_level(level).to_bits(), Ordering::Relaxed);
    }

    pub fn set_output_level(&self, level: f32, monitor: bool) {
        let target = if monitor {
            &self.monitor_level
        } else {
            &self.output_level
        };
        target.store(finite_level(level).to_bits(), Ordering::Relaxed);
    }

    pub fn record_input_callback_gap(&self) {
        self.input_callback_gaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_input_overrun(&self) {
        self.input_ring_overflows.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_expander_attenuated_frames(&self, frames: usize) {
        self.expander_attenuated_frames
            .fetch_add(frames as u64, Ordering::Relaxed);
    }

    pub fn record_dsp_input_underrun(&self) {
        self.dsp_input_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dsp_deadline(&self, elapsed_microseconds: u64, missed: bool) {
        update_maximum(
            &self.maximum_dsp_processing_microseconds,
            elapsed_microseconds,
        );
        if missed {
            self.dsp_processing_deadline_misses
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_output_ring_overflow(&self, monitor: bool) {
        let target = if monitor {
            &self.monitor_ring_overflows
        } else {
            &self.destination_ring_overflows
        };
        target.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_output_underrun(&self, monitor: bool) {
        let target = if monitor {
            &self.monitor_output_underruns
        } else {
            &self.output_ring_underruns
        };
        target.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_output_callback_gap(&self, monitor: bool) {
        let target = if monitor {
            &self.monitor_callback_gaps
        } else {
            &self.output_callback_gaps
        };
        target.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_concealed_frames(&self, frames: usize, monitor: bool) {
        let target = if monitor {
            &self.monitor_concealed_frames
        } else {
            &self.concealed_frames
        };
        target.fetch_add(frames as u64, Ordering::Relaxed);
    }

    pub fn record_stream_restart(&self) {
        self.stream_restart_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_input_ring_fill(&self, frames: usize) {
        let value = frames as u64;
        self.current_input_ring_fill_frames
            .store(value, Ordering::Relaxed);
        update_minimum(&self.minimum_input_ring_fill_frames, value);
        update_maximum(&self.maximum_input_ring_fill_frames, value);
    }

    pub fn update_output_ring_fill(&self, frames: usize, monitor: bool) {
        let value = frames as u64;
        let (current, maximum) = if monitor {
            (
                &self.current_monitor_ring_fill_frames,
                &self.maximum_monitor_ring_fill_frames,
            )
        } else {
            (
                &self.current_output_ring_fill_frames,
                &self.maximum_output_ring_fill_frames,
            )
        };
        current.store(value, Ordering::Relaxed);
        update_maximum(maximum, value);
    }

    pub fn set_startup_prefill(&self, target: usize, achieved: usize, timed_out: bool) {
        self.startup_prefill_target_frames
            .store(target as u64, Ordering::Relaxed);
        self.startup_prefill_achieved_frames
            .store(achieved as u64, Ordering::Relaxed);
        self.startup_prefill_timed_out
            .store(timed_out, Ordering::Relaxed);
    }

    pub fn reset_session(&self, profile: ReliabilityProfile) {
        self.reliability_profile
            .store(profile_to_u8(profile), Ordering::Relaxed);
        for counter in [
            &self.input_callback_gaps,
            &self.input_ring_overflows,
            &self.expander_attenuated_frames,
            &self.dsp_input_underruns,
            &self.dsp_processing_deadline_misses,
            &self.destination_ring_overflows,
            &self.monitor_ring_overflows,
            &self.output_ring_underruns,
            &self.monitor_output_underruns,
            &self.output_callback_gaps,
            &self.monitor_callback_gaps,
            &self.concealed_frames,
            &self.monitor_concealed_frames,
            &self.stream_restart_count,
            &self.current_input_ring_fill_frames,
            &self.maximum_input_ring_fill_frames,
            &self.current_output_ring_fill_frames,
            &self.maximum_output_ring_fill_frames,
            &self.current_monitor_ring_fill_frames,
            &self.maximum_monitor_ring_fill_frames,
            &self.maximum_dsp_processing_microseconds,
            &self.startup_prefill_target_frames,
            &self.startup_prefill_achieved_frames,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
        self.minimum_input_ring_fill_frames
            .store(UNOBSERVED_MINIMUM, Ordering::Relaxed);
        self.startup_prefill_timed_out
            .store(false, Ordering::Relaxed);
        for ratio in [
            &self.clock_drift_correction_ratio,
            &self.minimum_clock_drift_correction_ratio,
            &self.maximum_clock_drift_correction_ratio,
        ] {
            ratio.store(1.0_f32.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn set_stream_details(
        &self,
        format: ActiveStreamFormat,
        device_latency_ms: f32,
        dsp_latency_ms: f32,
    ) {
        if let Ok(mut current) = self.active_stream_format.write() {
            *current = Some(format);
        }
        let total = device_latency_ms + dsp_latency_ms;
        self.estimated_latency_ms
            .store(total.to_bits(), Ordering::Relaxed);
        self.dsp_processing_latency_ms
            .store(dsp_latency_ms.to_bits(), Ordering::Relaxed);
        self.total_estimated_latency_ms
            .store(total.to_bits(), Ordering::Relaxed);
    }

    pub fn clear_stream_details(&self) {
        if let Ok(mut current) = self.active_stream_format.write() {
            *current = None;
        }
        for value in [
            &self.estimated_latency_ms,
            &self.dsp_processing_latency_ms,
            &self.total_estimated_latency_ms,
        ] {
            value.store(0.0_f32.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn set_last_error(&self, error: Option<String>) {
        if let Ok(mut current) = self.last_runtime_error.write() {
            *current = error;
        }
    }

    pub fn snapshot(&self) -> EngineStatus {
        let state = EngineState::from_u8(self.state.load(Ordering::Acquire));
        let minimum_fill = self.minimum_input_ring_fill_frames.load(Ordering::Relaxed);
        EngineStatus {
            state,
            input_level: load_f32(&self.input_level),
            output_level: load_f32(&self.output_level),
            monitor_level: load_f32(&self.monitor_level),
            reliability_profile: profile_from_u8(self.reliability_profile.load(Ordering::Relaxed)),
            input_callback_gaps: self.input_callback_gaps.load(Ordering::Relaxed),
            input_ring_overflows: self.input_ring_overflows.load(Ordering::Relaxed),
            expander_attenuated_frames: self.expander_attenuated_frames.load(Ordering::Relaxed),
            dsp_input_underruns: self.dsp_input_underruns.load(Ordering::Relaxed),
            dsp_processing_deadline_misses: self
                .dsp_processing_deadline_misses
                .load(Ordering::Relaxed),
            destination_ring_overflows: self.destination_ring_overflows.load(Ordering::Relaxed),
            monitor_ring_overflows: self.monitor_ring_overflows.load(Ordering::Relaxed),
            output_ring_underruns: self.output_ring_underruns.load(Ordering::Relaxed),
            monitor_output_underruns: self.monitor_output_underruns.load(Ordering::Relaxed),
            output_callback_gaps: self.output_callback_gaps.load(Ordering::Relaxed),
            monitor_callback_gaps: self.monitor_callback_gaps.load(Ordering::Relaxed),
            concealed_frames: self.concealed_frames.load(Ordering::Relaxed),
            monitor_concealed_frames: self.monitor_concealed_frames.load(Ordering::Relaxed),
            stream_restart_count: self.stream_restart_count.load(Ordering::Relaxed),
            current_input_ring_fill_frames: self
                .current_input_ring_fill_frames
                .load(Ordering::Relaxed),
            minimum_input_ring_fill_frames: if minimum_fill == UNOBSERVED_MINIMUM {
                0
            } else {
                minimum_fill
            },
            maximum_input_ring_fill_frames: self
                .maximum_input_ring_fill_frames
                .load(Ordering::Relaxed),
            current_output_ring_fill_frames: self
                .current_output_ring_fill_frames
                .load(Ordering::Relaxed),
            maximum_output_ring_fill_frames: self
                .maximum_output_ring_fill_frames
                .load(Ordering::Relaxed),
            current_monitor_ring_fill_frames: self
                .current_monitor_ring_fill_frames
                .load(Ordering::Relaxed),
            maximum_monitor_ring_fill_frames: self
                .maximum_monitor_ring_fill_frames
                .load(Ordering::Relaxed),
            maximum_dsp_processing_time_ms: self
                .maximum_dsp_processing_microseconds
                .load(Ordering::Relaxed) as f32
                / 1_000.0,
            startup_prefill_target_frames: self
                .startup_prefill_target_frames
                .load(Ordering::Relaxed),
            startup_prefill_achieved_frames: self
                .startup_prefill_achieved_frames
                .load(Ordering::Relaxed),
            startup_prefill_timed_out: self.startup_prefill_timed_out.load(Ordering::Relaxed),
            clock_drift_correction_ratio: load_f32(&self.clock_drift_correction_ratio),
            minimum_clock_drift_correction_ratio: load_f32(
                &self.minimum_clock_drift_correction_ratio,
            ),
            maximum_clock_drift_correction_ratio: load_f32(
                &self.maximum_clock_drift_correction_ratio,
            ),
            estimated_latency_ms: load_f32(&self.estimated_latency_ms),
            dsp_processing_latency_ms: load_f32(&self.dsp_processing_latency_ms),
            total_estimated_latency_ms: load_f32(&self.total_estimated_latency_ms),
            active_stream_format: self
                .active_stream_format
                .read()
                .map(|format| format.clone())
                .unwrap_or_default(),
            last_runtime_error: self
                .last_runtime_error
                .read()
                .map(|error| error.clone())
                .unwrap_or_else(|_| {
                    Some("Runtime diagnostics are temporarily unavailable".to_owned())
                }),
            message: match state {
                EngineState::Stopped => "Ready to start".to_owned(),
                EngineState::Starting => "Starting and prefilling audio streams".to_owned(),
                EngineState::Running => "Audio processing is active".to_owned(),
                EngineState::Degraded => {
                    "Destination is active; local monitoring is degraded".to_owned()
                }
                EngineState::Recovering => "Recovering audio streams".to_owned(),
                EngineState::Stopping => "Stopping audio streams".to_owned(),
                EngineState::Error => "Audio engine needs attention".to_owned(),
            },
        }
    }
}

fn finite_level(level: f32) -> f32 {
    if level.is_finite() {
        level.max(0.0)
    } else {
        0.0
    }
}

fn load_f32(value: &AtomicU32) -> f32 {
    f32::from_bits(value.load(Ordering::Relaxed))
}

fn update_maximum(target: &AtomicU64, value: u64) {
    let _ = target.fetch_max(value, Ordering::Relaxed);
}

fn update_minimum(target: &AtomicU64, value: u64) {
    let _ = target.fetch_min(value, Ordering::Relaxed);
}

const fn profile_to_u8(profile: ReliabilityProfile) -> u8 {
    match profile {
        ReliabilityProfile::LowLatency => 0,
        ReliabilityProfile::Balanced => 1,
        ReliabilityProfile::Reliable => 2,
    }
}

const fn profile_from_u8(value: u8) -> ReliabilityProfile {
    match value {
        0 => ReliabilityProfile::LowLatency,
        2 => ReliabilityProfile::Reliable,
        _ => ReliabilityProfile::Balanced,
    }
}

#[cfg(test)]
mod tests {
    use super::SharedMetrics;
    use crate::audio::reliability::ReliabilityProfile;

    #[test]
    fn counters_are_specific_and_reset_per_session() {
        let metrics = SharedMetrics::default();
        metrics.record_input_callback_gap();
        metrics.record_output_underrun(false);
        metrics.record_concealed_frames(12, false);
        let before = metrics.snapshot();
        assert_eq!(before.input_callback_gaps, 1);
        assert_eq!(before.output_ring_underruns, 1);
        assert_eq!(before.monitor_output_underruns, 0);
        assert_eq!(before.concealed_frames, 12);

        metrics.reset_session(ReliabilityProfile::Reliable);
        let after = metrics.snapshot();
        assert_eq!(after.input_callback_gaps, 0);
        assert_eq!(after.output_ring_underruns, 0);
        assert_eq!(after.reliability_profile, ReliabilityProfile::Reliable);
    }

    #[test]
    fn ring_fill_tracks_current_minimum_and_maximum() {
        let metrics = SharedMetrics::default();
        for fill in [10, 4, 18, 7] {
            metrics.update_input_ring_fill(fill);
        }
        let status = metrics.snapshot();
        assert_eq!(status.current_input_ring_fill_frames, 7);
        assert_eq!(status.minimum_input_ring_fill_frames, 4);
        assert_eq!(status.maximum_input_ring_fill_frames, 18);
    }

    #[test]
    fn deadline_tracking_does_not_increment_unrelated_counters() {
        let metrics = SharedMetrics::default();
        metrics.record_dsp_deadline(2_500, true);
        let status = metrics.snapshot();
        assert_eq!(status.dsp_processing_deadline_misses, 1);
        assert_eq!(status.maximum_dsp_processing_time_ms, 2.5);
        assert_eq!(status.input_callback_gaps, 0);
        assert_eq!(status.output_ring_underruns, 0);
    }
}
