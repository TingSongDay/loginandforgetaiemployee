use zeroclaw::appliance::{
    state::{
        StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
    },
    supervisor::StationSupervisor,
};
use zeroclaw::config::Config;

#[test]
fn station_supervisor_defaults_to_booting_with_left_and_right_workers() {
    let mut config = Config::default();
    config.station.enabled = true;

    let supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    assert_eq!(supervisor.state(), StationSupervisorState::Booting);
    assert_eq!(supervisor.worker_count(), 2);
    assert_eq!(
        supervisor.left_worker().expect("left worker").id,
        "worker_a"
    );
    assert_eq!(
        supervisor.right_worker().expect("right worker").id,
        "worker_b"
    );
}

#[test]
fn worker_transitions_are_isolated() {
    let mut config = Config::default();
    config.station.enabled = true;

    let mut supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    supervisor
        .transition_worker("worker_a", WorkerRuntimeState::Ready)
        .expect("worker a should transition");

    assert_eq!(
        supervisor.worker("worker_a").expect("worker a").state,
        WorkerRuntimeState::Ready
    );
    assert_eq!(
        supervisor.worker("worker_b").expect("worker b").state,
        WorkerRuntimeState::Booting
    );
}

#[test]
fn worker_session_intervention_details_are_isolated() {
    let mut config = Config::default();
    config.station.enabled = true;

    let mut supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    supervisor
        .transition_worker_with_details(
            "worker_a",
            WorkerRuntimeState::LoginRequired,
            Some(WorkerSessionStatus::LoginRequired),
            Some(Some(WorkerAttentionReason::SessionMissing)),
            Some(None),
        )
        .expect("worker a should require login");

    let worker_a = supervisor.worker("worker_a").expect("worker a");
    let worker_b = supervisor.worker("worker_b").expect("worker b");

    assert_eq!(worker_a.state, WorkerRuntimeState::LoginRequired);
    assert_eq!(worker_a.session_status, WorkerSessionStatus::LoginRequired);
    assert_eq!(
        worker_a.attention_reason,
        Some(WorkerAttentionReason::SessionMissing)
    );
    assert_eq!(worker_b.state, WorkerRuntimeState::Booting);
    assert_eq!(worker_b.session_status, WorkerSessionStatus::Unknown);
    assert_eq!(worker_b.attention_reason, None);
}

#[test]
fn invalid_supervisor_transition_is_rejected() {
    let mut config = Config::default();
    config.station.enabled = true;

    let mut supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    let error = supervisor
        .set_state(StationSupervisorState::Ready)
        .expect_err("booting should not jump directly to ready");
    assert!(error
        .to_string()
        .contains("invalid station supervisor transition"));
}
