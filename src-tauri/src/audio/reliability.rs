use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReliabilityProfile {
    LowLatency,
    #[default]
    Balanced,
    Reliable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReliabilityConfig {
    pub requested_buffer_frames: u32,
    pub input_ring_milliseconds: u32,
    pub output_ring_milliseconds: u32,
    pub startup_prefill_frames: u32,
    pub startup_timeout_milliseconds: u64,
    pub worker_wake_timeout_milliseconds: u64,
    pub underrun_tolerance_blocks: u32,
    pub concealment_milliseconds: u32,
}

impl ReliabilityProfile {
    pub const fn config(self) -> ReliabilityConfig {
        match self {
            Self::LowLatency => ReliabilityConfig {
                requested_buffer_frames: 128,
                input_ring_milliseconds: 80,
                output_ring_milliseconds: 80,
                startup_prefill_frames: 256,
                startup_timeout_milliseconds: 500,
                worker_wake_timeout_milliseconds: 5,
                underrun_tolerance_blocks: 0,
                concealment_milliseconds: 3,
            },
            Self::Balanced => ReliabilityConfig {
                requested_buffer_frames: 256,
                input_ring_milliseconds: 250,
                output_ring_milliseconds: 250,
                startup_prefill_frames: 1_024,
                startup_timeout_milliseconds: 1_000,
                worker_wake_timeout_milliseconds: 10,
                underrun_tolerance_blocks: 1,
                concealment_milliseconds: 6,
            },
            Self::Reliable => ReliabilityConfig {
                requested_buffer_frames: 512,
                input_ring_milliseconds: 500,
                output_ring_milliseconds: 500,
                startup_prefill_frames: 2_048,
                startup_timeout_milliseconds: 1_500,
                worker_wake_timeout_milliseconds: 20,
                underrun_tolerance_blocks: 2,
                concealment_milliseconds: 10,
            },
        }
    }

    pub fn worker_wake_timeout(self) -> Duration {
        Duration::from_millis(self.config().worker_wake_timeout_milliseconds)
    }

    pub fn startup_timeout(self) -> Duration {
        Duration::from_millis(self.config().startup_timeout_milliseconds)
    }
}

#[cfg(test)]
mod tests {
    use super::ReliabilityProfile;

    #[test]
    fn profiles_trade_latency_for_bounded_reliability_values() {
        let low = ReliabilityProfile::LowLatency.config();
        let balanced = ReliabilityProfile::Balanced.config();
        let reliable = ReliabilityProfile::Reliable.config();

        assert_eq!(
            (
                low.requested_buffer_frames,
                low.input_ring_milliseconds,
                low.output_ring_milliseconds,
                low.startup_prefill_frames,
                low.startup_timeout_milliseconds,
                low.worker_wake_timeout_milliseconds,
                low.underrun_tolerance_blocks,
                low.concealment_milliseconds,
            ),
            (128, 80, 80, 256, 500, 5, 0, 3)
        );
        assert_eq!(
            (
                balanced.requested_buffer_frames,
                balanced.input_ring_milliseconds,
                balanced.output_ring_milliseconds,
                balanced.startup_prefill_frames,
                balanced.startup_timeout_milliseconds,
                balanced.worker_wake_timeout_milliseconds,
                balanced.underrun_tolerance_blocks,
                balanced.concealment_milliseconds,
            ),
            (256, 250, 250, 1_024, 1_000, 10, 1, 6)
        );
        assert_eq!(
            (
                reliable.requested_buffer_frames,
                reliable.input_ring_milliseconds,
                reliable.output_ring_milliseconds,
                reliable.startup_prefill_frames,
                reliable.startup_timeout_milliseconds,
                reliable.worker_wake_timeout_milliseconds,
                reliable.underrun_tolerance_blocks,
                reliable.concealment_milliseconds,
            ),
            (512, 500, 500, 2_048, 1_500, 20, 2, 10)
        );

        assert!(low.requested_buffer_frames < balanced.requested_buffer_frames);
        assert!(balanced.requested_buffer_frames < reliable.requested_buffer_frames);
        assert!(low.startup_prefill_frames < balanced.startup_prefill_frames);
        assert!(balanced.startup_prefill_frames < reliable.startup_prefill_frames);
        assert!(low.output_ring_milliseconds < balanced.output_ring_milliseconds);
        assert!(balanced.output_ring_milliseconds < reliable.output_ring_milliseconds);
        assert_eq!(ReliabilityProfile::default(), ReliabilityProfile::Balanced);
    }
}
