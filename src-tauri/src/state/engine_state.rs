use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum EngineState {
    #[default]
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Stopping = 3,
    Error = 4,
}

impl EngineState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Starting,
            2 => Self::Running,
            3 => Self::Stopping,
            4 => Self::Error,
            _ => Self::Stopped,
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Stopped, Self::Starting)
                | (Self::Starting, Self::Running | Self::Error | Self::Stopped)
                | (Self::Running, Self::Stopping | Self::Error)
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
}
