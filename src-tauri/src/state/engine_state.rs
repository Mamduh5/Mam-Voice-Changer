use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum EngineState {
    #[default]
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Degraded = 3,
    Recovering = 4,
    Stopping = 5,
    Error = 6,
}

impl EngineState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Starting,
            2 => Self::Running,
            3 => Self::Degraded,
            4 => Self::Recovering,
            5 => Self::Stopping,
            6 => Self::Error,
            _ => Self::Stopped,
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Stopped, Self::Starting)
                | (Self::Starting, Self::Running | Self::Error | Self::Stopped)
                | (
                    Self::Running,
                    Self::Degraded | Self::Recovering | Self::Stopping | Self::Error
                )
                | (
                    Self::Degraded,
                    Self::Running | Self::Recovering | Self::Stopping | Self::Error
                )
                | (
                    Self::Recovering,
                    Self::Running | Self::Stopping | Self::Error
                )
                | (Self::Stopping, Self::Stopped | Self::Error)
                | (Self::Error, Self::Starting | Self::Stopped)
        ) || self == next
    }
}

#[cfg(test)]
mod tests {
    use super::EngineState;

    #[test]
    fn accepts_normal_engine_lifecycle() {
        assert!(EngineState::Stopped.can_transition_to(EngineState::Starting));
        assert!(EngineState::Starting.can_transition_to(EngineState::Running));
        assert!(EngineState::Running.can_transition_to(EngineState::Stopping));
        assert!(EngineState::Stopping.can_transition_to(EngineState::Stopped));
    }

    #[test]
    fn error_state_can_recover_by_starting_again() {
        assert!(EngineState::Running.can_transition_to(EngineState::Error));
        assert!(EngineState::Error.can_transition_to(EngineState::Starting));
        assert!(!EngineState::Stopped.can_transition_to(EngineState::Running));
    }

    #[test]
    fn recovery_and_degraded_states_have_explicit_safe_transitions() {
        assert!(EngineState::Running.can_transition_to(EngineState::Recovering));
        assert!(EngineState::Recovering.can_transition_to(EngineState::Running));
        assert!(EngineState::Recovering.can_transition_to(EngineState::Stopping));
        assert!(EngineState::Running.can_transition_to(EngineState::Degraded));
        assert!(EngineState::Degraded.can_transition_to(EngineState::Stopping));
        assert!(!EngineState::Stopped.can_transition_to(EngineState::Recovering));
    }
}
