use zeroclaw::appliance::{
    browser_runtime::build_launch_plan,
    platforms::mvp_platform_driver,
    state::{
        StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
    },
    supervisor::StationSupervisor,
    tile_manager::TileManager,
};
use zeroclaw::config::Config;

#[test]
fn station_supervisor_can_prepare_two_worker_flow() {
    let mut config = Config::default();
    config.station.enabled = true;

    let mut supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    supervisor
        .set_state(StationSupervisorState::StartingWorkers)
        .expect("enter starting workers");
    supervisor
        .transition_worker("worker_a", WorkerRuntimeState::Ready)
        .expect("worker a ready");
    supervisor
        .transition_worker("worker_b", WorkerRuntimeState::Ready)
        .expect("worker b ready");
    supervisor
        .set_state(StationSupervisorState::Ready)
        .expect("station ready");

    assert_eq!(supervisor.state(), StationSupervisorState::Ready);
    assert!(supervisor
        .workers()
        .iter()
        .all(|worker| worker.state == WorkerRuntimeState::Ready));

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let left_plan = build_launch_plan(&config, &tile_manager, &config.station.workers[0])
        .expect("left launch plan");
    let right_plan = build_launch_plan(&config, &tile_manager, &config.station.workers[1])
        .expect("right launch plan");

    assert_eq!(left_plan.window_origin_x, 0);
    assert_eq!(right_plan.window_origin_x, left_plan.viewport_width as i32);
}

#[test]
fn station_supervisor_can_surface_worker_specific_intervention_state() {
    let mut config = Config::default();
    config.station.enabled = true;

    let mut supervisor = StationSupervisor::from_config(&config).expect("station supervisor");
    supervisor
        .set_state(StationSupervisorState::StartingWorkers)
        .expect("enter starting workers");
    supervisor
        .transition_worker_with_details(
            "worker_a",
            WorkerRuntimeState::Ready,
            Some(WorkerSessionStatus::Active),
            Some(None),
            Some(None),
        )
        .expect("worker a ready");
    supervisor
        .transition_worker_with_details(
            "worker_b",
            WorkerRuntimeState::ChallengeRequired,
            Some(WorkerSessionStatus::ChallengeRequired),
            Some(Some(WorkerAttentionReason::ChallengeDetected)),
            Some(None),
        )
        .expect("worker b challenged");
    supervisor
        .set_state(StationSupervisorState::Degraded)
        .expect("station degraded");

    assert_eq!(supervisor.state(), StationSupervisorState::Degraded);
    assert_eq!(
        supervisor.worker("worker_a").expect("worker a").state,
        WorkerRuntimeState::Ready
    );
    assert_eq!(
        supervisor
            .worker("worker_b")
            .expect("worker b")
            .attention_reason,
        Some(WorkerAttentionReason::ChallengeDetected)
    );
}

#[test]
fn mvp_platform_driver_defaults_to_whatsapp() {
    let driver = mvp_platform_driver();

    driver
        .validate_selectors()
        .expect("selectors should validate");
    assert_eq!(driver.platform_id(), "whatsapp_web");
    assert_eq!(driver.platform_name(), "WhatsApp Web");
}
