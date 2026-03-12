use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRuntimeState {
    Booting,
    LoginRequired,
    ChallengeRequired,
    Ready,
    ProcessingMessage,
    Paused,
    Error,
}

impl WorkerRuntimeState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use WorkerRuntimeState::{
            Booting, ChallengeRequired, Error, LoginRequired, Paused, ProcessingMessage, Ready,
        };

        match (self, next) {
            (current, target) if current == target => true,
            (Booting, LoginRequired | ChallengeRequired | Ready | Error) => true,
            (LoginRequired, ChallengeRequired | Ready | Paused | Error) => true,
            (ChallengeRequired, Ready | Paused | Error) => true,
            (Ready, ProcessingMessage | LoginRequired | ChallengeRequired | Paused | Error) => true,
            (ProcessingMessage, Ready | Paused | Error) => true,
            (Paused, Ready | LoginRequired | ChallengeRequired | Error) => true,
            (Error, Booting | LoginRequired | ChallengeRequired | Ready | Paused) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerSessionStatus {
    Unknown,
    Active,
    LoginRequired,
    ChallengeRequired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerAttentionReason {
    SessionMissing,
    SessionInvalid,
    SessionMetadataCorrupted,
    ChallengeDetected,
    BrowserLaunchFailed,
    PlatformCheckFailed,
    ManualPause,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StationSupervisorState {
    Booting,
    StartingWorkers,
    Ready,
    Degraded,
    Paused,
    Error,
}

impl StationSupervisorState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use StationSupervisorState::{Booting, Degraded, Error, Paused, Ready, StartingWorkers};

        match (self, next) {
            (current, target) if current == target => true,
            (Booting, StartingWorkers | Error) => true,
            (StartingWorkers, Ready | Degraded | Error) => true,
            (Ready, Degraded | Paused | Error) => true,
            (Degraded, Ready | Paused | Error) => true,
            (Paused, Ready | Degraded | Error) => true,
            (Error, Booting | StartingWorkers | Paused) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
    };

    #[test]
    fn worker_runtime_allows_expected_transitions() {
        assert!(WorkerRuntimeState::Booting.can_transition_to(WorkerRuntimeState::Ready));
        assert!(WorkerRuntimeState::Ready.can_transition_to(WorkerRuntimeState::ProcessingMessage));
        assert!(WorkerRuntimeState::Paused.can_transition_to(WorkerRuntimeState::Ready));
        assert!(!WorkerRuntimeState::ProcessingMessage
            .can_transition_to(WorkerRuntimeState::LoginRequired));
    }

    #[test]
    fn supervisor_runtime_allows_expected_transitions() {
        assert!(StationSupervisorState::Booting
            .can_transition_to(StationSupervisorState::StartingWorkers));
        assert!(StationSupervisorState::StartingWorkers
            .can_transition_to(StationSupervisorState::Ready));
        assert!(StationSupervisorState::Ready.can_transition_to(StationSupervisorState::Degraded));
        assert!(!StationSupervisorState::Ready
            .can_transition_to(StationSupervisorState::StartingWorkers));
    }

    #[test]
    fn worker_session_status_values_are_stable() {
        assert_eq!(
            serde_json::to_string(&WorkerSessionStatus::ChallengeRequired).unwrap(),
            "\"challenge_required\""
        );
    }

    #[test]
    fn worker_attention_reason_values_are_stable() {
        assert_eq!(
            serde_json::to_string(&WorkerAttentionReason::SessionMissing).unwrap(),
            "\"session_missing\""
        );
    }
}
