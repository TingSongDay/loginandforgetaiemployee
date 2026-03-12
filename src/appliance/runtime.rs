use crate::appliance::{
    browser_runtime::{
        build_launch_plan, runtime_for_config, runtime_kind_for_config, ManagedBrowserRuntime,
        ManagedBrowserRuntimeKind, ManagedBrowserSession,
    },
    dedupe_store::DedupeStore,
    platforms::{
        mvp_platform_driver, MessagingPlatformDriver, PlatformChallengeState, PlatformLoginState,
    },
    poller::run_station_poller,
    reply_engine::ReplyEngine,
    session_store::SessionStore,
    state::{
        StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
    },
    supervisor::StationSupervisor,
    tile_manager::TileManager,
};
use crate::config::Config;
use anyhow::{bail, Context, Result};
use std::sync::Arc;
use tokio::fs;

#[derive(Clone)]
pub struct StationRuntimeDependencies {
    pub runtime: Arc<dyn ManagedBrowserRuntime>,
    pub platform_driver: Arc<dyn MessagingPlatformDriver>,
}

#[derive(Debug, Clone)]
pub struct StationBootReport {
    pub supervisor: StationSupervisor,
    pub sessions: Vec<ManagedBrowserSession>,
    pub runtime_kind: ManagedBrowserRuntimeKind,
    pub platform_name: String,
}

impl StationRuntimeDependencies {
    pub fn from_config(config: &Config) -> Self {
        let platform_driver = mvp_platform_driver();
        let runtime = runtime_for_config(config, platform_driver.as_ref());
        Self {
            runtime,
            platform_driver,
        }
    }
}

pub async fn initialize_station(config: &Config) -> Result<StationBootReport> {
    initialize_station_with(config, StationRuntimeDependencies::from_config(config)).await
}

pub async fn initialize_station_with(
    config: &Config,
    deps: StationRuntimeDependencies,
) -> Result<StationBootReport> {
    if !config.station.enabled {
        bail!("station.enabled must be true before starting NeoHUman Station");
    }

    let tile_manager = TileManager::from_station_config(&config.station)?;
    deps.platform_driver.validate_selectors()?;
    let session_store = SessionStore::from_config(config);
    session_store.ensure_layout().await?;
    let dedupe_store = DedupeStore::from_config(config);

    let mut supervisor = StationSupervisor::from_config(config)?;
    supervisor.set_state(StationSupervisorState::StartingWorkers)?;

    let mut sessions = Vec::new();
    let mut station_is_degraded = false;

    for worker in &config.station.workers {
        if !worker.enabled {
            continue;
        }

        let launch_plan = build_launch_plan(config, &tile_manager, worker)?;
        let launch_outcome =
            launch_and_inspect_worker(worker, &launch_plan, &deps, &session_store, &mut supervisor)
                .await;

        match launch_outcome {
            Ok((session, worker_state)) => {
                if worker_state != WorkerRuntimeState::Ready {
                    station_is_degraded = true;
                }
                dedupe_store.ensure_layout_for_worker(worker).await?;
                sessions.push(session);
            }
            Err(error) => {
                station_is_degraded = true;
                let metadata = session_store
                    .mark_worker_error(
                        worker,
                        &launch_plan,
                        WorkerAttentionReason::BrowserLaunchFailed,
                        None,
                        error.to_string(),
                    )
                    .await?;
                supervisor.transition_worker_with_details(
                    &worker.id,
                    WorkerRuntimeState::Error,
                    Some(metadata.status),
                    Some(metadata.attention_reason),
                    Some(metadata.last_successful_login_at),
                )?;
            }
        }
    }

    supervisor.set_state(if station_is_degraded {
        StationSupervisorState::Degraded
    } else {
        StationSupervisorState::Ready
    })?;

    Ok(StationBootReport {
        supervisor,
        sessions,
        runtime_kind: runtime_kind_for_config(config),
        platform_name: deps.platform_driver.platform_name().to_string(),
    })
}

pub async fn run_station(config: Config) -> Result<()> {
    let deps = StationRuntimeDependencies::from_config(&config);
    let mut report = initialize_station_with(&config, deps.clone()).await?;

    tracing::info!(
        station = %config.station.station_name,
        workers = report.supervisor.worker_count(),
        runtime_kind = ?report.runtime_kind,
        "NeoHUman Station initialized"
    );

    println!("{}", station_status_summary(&config, &report));
    let session_store = SessionStore::from_config(&config);
    let dedupe_store = DedupeStore::from_config(&config);
    let reply_engine = ReplyEngine::from_config(&config);
    run_station_poller(
        &config,
        &mut report.supervisor,
        &report.sessions,
        deps.runtime,
        deps.platform_driver,
        &session_store,
        &dedupe_store,
        &reply_engine,
    )
    .await
}

pub async fn pause_worker(config: &Config, worker_id: &str) -> Result<()> {
    let (worker, plan) = worker_and_plan(config, worker_id)?;
    let store = SessionStore::from_config(config);
    store.ensure_layout().await?;
    store.set_worker_paused(worker, &plan, true).await?;
    Ok(())
}

pub async fn resume_worker(config: &Config, worker_id: &str) -> Result<()> {
    let (worker, plan) = worker_and_plan(config, worker_id)?;
    let store = SessionStore::from_config(config);
    store.ensure_layout().await?;
    store.set_worker_paused(worker, &plan, false).await?;
    Ok(())
}

pub async fn mark_login_complete(config: &Config, worker_id: &str) -> Result<()> {
    let (worker, plan) = worker_and_plan(config, worker_id)?;
    let store = SessionStore::from_config(config);
    store.ensure_layout().await?;
    store.mark_worker_login_complete(worker, &plan).await?;
    Ok(())
}

pub async fn mark_challenge_complete(config: &Config, worker_id: &str) -> Result<()> {
    let (worker, plan) = worker_and_plan(config, worker_id)?;
    let store = SessionStore::from_config(config);
    store.ensure_layout().await?;
    store.mark_worker_challenge_complete(worker, &plan).await?;
    Ok(())
}

pub fn station_status_summary(config: &Config, report: &StationBootReport) -> String {
    let mut lines = vec![
        format!("🧑‍💼 {}", config.station.station_name),
        format!("Platform: {:?}", config.station.platform),
        format!("Messaging platform: {}", report.platform_name),
        format!("Supervisor: {:?}", report.supervisor.state()),
        format!("Browser runtime: {:?}", report.runtime_kind),
    ];

    for session in &report.sessions {
        let worker = report.supervisor.worker(&session.worker_id);
        let state = worker.map(|worker| worker.state);
        let session_status = worker.map(|worker| worker.session_status);
        let attention_reason = worker.and_then(|worker| worker.attention_reason);
        lines.push(format!(
            "- {} [{}] state={:?} session={:?} attention={:?} tile={:?} window=({}, {}) viewport={}x{} zoom={} profile={} backend={}",
            session.launch_plan.display_name,
            session.worker_id,
            state,
            session_status,
            attention_reason,
            session.placement.tile_position,
            session.placement.window_origin_x,
            session.placement.window_origin_y,
            session.placement.viewport_width,
            session.placement.viewport_height,
            session.launch_plan.zoom_percent,
            session.launch_plan.session_name,
            session.launch_report.backend,
        ));
    }

    lines.join("\n")
}

async fn launch_and_inspect_worker(
    worker: &crate::config::StationWorkerConfig,
    launch_plan: &crate::appliance::browser_runtime::ManagedBrowserLaunchPlan,
    deps: &StationRuntimeDependencies,
    session_store: &SessionStore,
    supervisor: &mut StationSupervisor,
) -> Result<(ManagedBrowserSession, WorkerRuntimeState)> {
    let session = deps.runtime.launch(launch_plan).await?;
    deps.runtime.connect(&session).await?;
    let session = deps
        .runtime
        .move_to_tile(&session, &session.placement)
        .await?;
    persist_launch_report(&session).await?;
    deps.platform_driver
        .open_workspace(deps.runtime.as_ref(), &session)
        .await?;

    let existing = session_store
        .load_or_initialize_worker(worker, launch_plan)
        .await?;
    if existing.paused {
        supervisor.transition_worker_with_details(
            &worker.id,
            WorkerRuntimeState::Paused,
            Some(existing.status),
            Some(Some(WorkerAttentionReason::ManualPause)),
            Some(existing.last_successful_login_at),
        )?;
        return Ok((session, WorkerRuntimeState::Paused));
    }

    let login_state = deps
        .platform_driver
        .detect_login_state(deps.runtime.as_ref(), &session)
        .await
        .with_context(|| format!("detect login state for {}", worker.id))?;
    let challenge_state = deps
        .platform_driver
        .detect_challenge_state(deps.runtime.as_ref(), &session)
        .await
        .with_context(|| format!("detect challenge state for {}", worker.id))?;

    let metadata = match (login_state, challenge_state) {
        (PlatformLoginState::LoggedIn, PlatformChallengeState::Clear) => {
            session_store
                .mark_worker_active(worker, launch_plan)
                .await?
        }
        (_, PlatformChallengeState::ChallengeRequired) => {
            session_store
                .mark_worker_challenge_required(
                    worker,
                    launch_plan,
                    existing.last_successful_login_at,
                )
                .await?
        }
        (PlatformLoginState::LoginRequired | PlatformLoginState::Unknown, _) => {
            session_store
                .mark_worker_login_required(
                    worker,
                    launch_plan,
                    if login_state == PlatformLoginState::Unknown {
                        WorkerAttentionReason::PlatformCheckFailed
                    } else {
                        WorkerAttentionReason::SessionInvalid
                    },
                    existing.last_successful_login_at,
                )
                .await?
        }
    };

    let (worker_state, session_status, attention_reason) = runtime_state_from_metadata(&metadata);
    supervisor.transition_worker_with_details(
        &worker.id,
        worker_state,
        Some(session_status),
        Some(attention_reason),
        Some(metadata.last_successful_login_at),
    )?;

    Ok((session, worker_state))
}

fn worker_and_plan<'a>(
    config: &'a Config,
    worker_id: &str,
) -> Result<(
    &'a crate::config::StationWorkerConfig,
    crate::appliance::browser_runtime::ManagedBrowserLaunchPlan,
)> {
    let tile_manager = TileManager::from_station_config(&config.station)?;
    let worker = config
        .station
        .workers
        .iter()
        .find(|worker| worker.id == worker_id)
        .ok_or_else(|| anyhow::anyhow!("unknown station worker: {worker_id}"))?;
    let plan = build_launch_plan(config, &tile_manager, worker)?;
    Ok((worker, plan))
}

fn runtime_state_from_metadata(
    metadata: &crate::appliance::session_store::PersistedWorkerSession,
) -> (
    WorkerRuntimeState,
    WorkerSessionStatus,
    Option<WorkerAttentionReason>,
) {
    if metadata.paused {
        return (
            WorkerRuntimeState::Paused,
            metadata.status,
            Some(WorkerAttentionReason::ManualPause),
        );
    }

    let runtime_state = match metadata.status {
        WorkerSessionStatus::Active => WorkerRuntimeState::Ready,
        WorkerSessionStatus::LoginRequired | WorkerSessionStatus::Unknown => {
            WorkerRuntimeState::LoginRequired
        }
        WorkerSessionStatus::ChallengeRequired => WorkerRuntimeState::ChallengeRequired,
    };

    (runtime_state, metadata.status, metadata.attention_reason)
}

async fn persist_launch_report(session: &ManagedBrowserSession) -> Result<()> {
    fs::create_dir_all(&session.launch_plan.user_data_dir)
        .await
        .with_context(|| {
            format!(
                "create launch report dir {}",
                session.launch_plan.user_data_dir.display()
            )
        })?;
    let path = session.launch_plan.user_data_dir.join("launch.json");
    let payload =
        serde_json::to_vec_pretty(&session.launch_report).context("serialize launch report")?;
    fs::write(&path, payload)
        .await
        .with_context(|| format!("write launch report {}", path.display()))
}
