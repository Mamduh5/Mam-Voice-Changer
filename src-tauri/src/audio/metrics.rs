use std::sync::{
    atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering},
    RwLock,
};

use serde::Serialize;

use crate::{audio::stream_config::ActiveStreamFormat, state::engine_state::EngineState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: EngineState,
    pub input_level: f32,
    pub output_level: f32,
    pub input_overruns: u64,
    pub output_underruns: u64,
    pub dsp_input_underruns: u64,
    pub dsp_output_overruns: u64,
    pub estimated_latency_ms: f32,
    pub active_stream_format: Option<ActiveStreamFormat>,
    pub last_runtime_error: Option<String>,
    pub message: String,
}

pub struct SharedMetrics {
    state: AtomicU8,
    input_level: AtomicU32,
    output_level: AtomicU32,
    input_overruns: AtomicU64,
    output_underruns: AtomicU64,
    dsp_input_underruns: AtomicU64,
    dsp_output_overruns: AtomicU64,
    estimated_latency_ms: AtomicU32,
    active_stream_format: RwLock<Option<ActiveStreamFormat>>,
    last_runtime_error: RwLock<Option<String>>,
}

impl Default for SharedMetrics {
    fn default() -> Self {
        Self {
            state: AtomicU8::new(EngineState::Stopped as u8),
            input_level: AtomicU32::new(0.0_f32.to_bits()),
            output_level: AtomicU32::new(0.0_f32.to_bits()),
            input_overruns: AtomicU64::new(0),
            output_underruns: AtomicU64::new(0),
            dsp_input_underruns: AtomicU64::new(0),
            dsp_output_overruns: AtomicU64::new(0),
            estimated_latency_ms: AtomicU32::new(0.0_f32.to_bits()),
            active_stream_format: RwLock::new(None),
            last_runtime_error: RwLock::new(None),
        }
    }
}

impl SharedMetrics {
    pub fn set_state(&self, state: EngineState) {
        self.state.store(state as u8, Ordering::Release);
        if state != EngineState::Running {
            self.input_level.store(0.0_f32.to_bits(), Ordering::Relaxed);
            self.output_level
                .store(0.0_f32.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn set_input_level(&self, level: f32) {
        self.input_level.store(level.to_bits(), Ordering::Relaxed);
    }

    pub fn set_output_level(&self, level: f32) {
        self.output_level.store(level.to_bits(), Ordering::Relaxed);
    }

    pub fn record_input_overrun(&self) {
        self.input_overruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_output_underrun(&self) {
        self.output_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dsp_input_underrun(&self) {
        self.dsp_input_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dsp_output_overrun(&self) {
        self.dsp_output_overruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset_counters(&self) {
        self.input_overruns.store(0, Ordering::Relaxed);
        self.output_underruns.store(0, Ordering::Relaxed);
        self.dsp_input_underruns.store(0, Ordering::Relaxed);
        self.dsp_output_overruns.store(0, Ordering::Relaxed);
    }

    pub fn set_stream_details(&self, format: ActiveStreamFormat, latency_ms: f32) {
        if let Ok(mut current) = self.active_stream_format.write() {
            *current = Some(format);
        }
        self.estimated_latency_ms
            .store(latency_ms.to_bits(), Ordering::Relaxed);
    }

    pub fn clear_stream_details(&self) {
        if let Ok(mut current) = self.active_stream_format.write() {
            *current = None;
        }
        self.estimated_latency_ms
            .store(0.0_f32.to_bits(), Ordering::Relaxed);
    }

    pub fn set_last_error(&self, error: Option<String>) {
        if let Ok(mut current) = self.last_runtime_error.write() {
            *current = error;
        }
    }

    pub fn snapshot(&self) -> EngineStatus {
        let state = EngineState::from_u8(self.state.load(Ordering::Acquire));
        let active_stream_format = self
            .active_stream_format
            .read()
            .map(|format| format.clone())
            .unwrap_or_default();
        let last_runtime_error = self
            .last_runtime_error
            .read()
            .map(|error| error.clone())
            .unwrap_or_else(|_| Some("Runtime diagnostics are temporarily unavailable".to_owned()));
        EngineStatus {
            state,
            input_level: f32::from_bits(self.input_level.load(Ordering::Relaxed)),
            output_level: f32::from_bits(self.output_level.load(Ordering::Relaxed)),
            input_overruns: self.input_overruns.load(Ordering::Relaxed),
            output_underruns: self.output_underruns.load(Ordering::Relaxed),
            dsp_input_underruns: self.dsp_input_underruns.load(Ordering::Relaxed),
            dsp_output_overruns: self.dsp_output_overruns.load(Ordering::Relaxed),
            estimated_latency_ms: f32::from_bits(self.estimated_latency_ms.load(Ordering::Relaxed)),
            active_stream_format,
            last_runtime_error,
            message: match state {
                EngineState::Stopped => "Ready to start".to_owned(),
                EngineState::Starting => "Starting audio streams".to_owned(),
                EngineState::Running => "Audio processing is active".to_owned(),
                EngineState::Stopping => "Stopping audio streams".to_owned(),
                EngineState::Error => "Audio engine needs attention".to_owned(),
            },
        }
    }
}
