use crate::appliance::{
    browser_runtime::ManagedBrowserSession,
    dedupe_store::DedupeStore,
    detector::detect_new_inbound_message,
    messages::normalize_message_records,
    platforms::{
        MessagingPlatformDriver, PlatformChallengeState, PlatformLoginState,
        PlatformWorkspaceState,
    },
    reply_engine::ReplyEngine,
    session_store::SessionStore,
    state::{
        StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
    },
    supervisor::StationSupervisor,
};
use crate::{appliance::browser_runtime::ManagedBrowserRuntime, config::Config};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WorkerPollState {
    pub consecutive_failures: u32,
    pub next_poll_at: Instant,
}

impl Default for WorkerPollState {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            next_poll_at: Instant::now(),
        }
    }
}

pub async fn run_station_poller(
    config: &Config,
    supervisor: &mut StationSupervisor,
    sessions: &[ManagedBrowserSession],
    runtime: Arc<dyn ManagedBrowserRuntime>,
    platform_driver: Arc<dyn MessagingPlatformDriver>,
    session_store: &SessionStore,
    dedupe_store: &DedupeStore,
    reply_engine: &ReplyEngine,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_millis(
        config.station.message_poll_interval_ms,
    ));
    let mut worker_states: HashMap<String, WorkerPollState> = sessions
        .iter()
        .map(|session| (session.worker_id.clone(), WorkerPollState::default()))
        .collect();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            _ = interval.tick() => {
                poll_once(
                    config,
                    supervisor,
                    sessions,
                    runtime.clone(),
                    platform_driver.clone(),
                    session_store,
                    dedupe_store,
                    reply_engine,
                    &mut worker_states,
                ).await?;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn poll_once(
    config: &Config,
    supervisor: &mut StationSupervisor,
    sessions: &[ManagedBrowserSession],
    runtime: Arc<dyn ManagedBrowserRuntime>,
    platform_driver: Arc<dyn MessagingPlatformDriver>,
    session_store: &SessionStore,
    dedupe_store: &DedupeStore,
    reply_engine: &ReplyEngine,
    worker_states: &mut HashMap<String, WorkerPollState>,
) -> Result<()> {
    let mut degraded = false;

    for session in sessions {
        let Some(worker) = config
            .station
            .workers
            .iter()
            .find(|worker| worker.id == session.worker_id)
        else {
            continue;
        };

        let worker_poll_state = worker_states
            .entry(worker.id.clone())
            .or_insert_with(WorkerPollState::default);
        if Instant::now() < worker_poll_state.next_poll_at {
            continue;
        }

        let outcome = poll_worker_once(
            worker,
            session,
            runtime.as_ref(),
            platform_driver.as_ref(),
            session_store,
            dedupe_store,
            reply_engine,
            supervisor,
        )
        .await;

        match outcome {
            Ok(is_ready) => {
                worker_poll_state.consecutive_failures = 0;
                worker_poll_state.next_poll_at = Instant::now();
                if !is_ready {
                    degraded = true;
                }
            }
            Err(error) => {
                degraded = true;
                worker_poll_state.consecutive_failures += 1;
                let backoff_secs = 2_u64.pow(worker_poll_state.consecutive_failures.min(5));
                worker_poll_state.next_poll_at = Instant::now() + Duration::from_secs(backoff_secs);
                let _ = supervisor.transition_worker_with_details(
                    &worker.id,
                    WorkerRuntimeState::Error,
                    Some(WorkerSessionStatus::Unknown),
                    Some(Some(WorkerAttentionReason::PlatformCheckFailed)),
                    Some(None),
                );
                tracing::warn!(worker_id = %worker.id, error = %error, "station poll failed");
            }
        }
    }

    supervisor.set_state(if degraded {
        StationSupervisorState::Degraded
    } else {
        StationSupervisorState::Ready
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn poll_worker_once(
    worker: &crate::config::StationWorkerConfig,
    session: &ManagedBrowserSession,
    runtime: &dyn ManagedBrowserRuntime,
    platform_driver: &dyn MessagingPlatformDriver,
    session_store: &SessionStore,
    dedupe_store: &DedupeStore,
    reply_engine: &ReplyEngine,
    supervisor: &mut StationSupervisor,
) -> Result<bool> {
    let session = if worker.managed_browser.snap_back_before_interaction {
        runtime.ensure_canonical_placement(session).await?
    } else {
        session.clone()
    };

    let metadata = session_store
        .load_or_initialize_worker(worker, &session.launch_plan)
        .await?;
    if metadata.paused {
        supervisor.transition_worker_with_details(
            &worker.id,
            WorkerRuntimeState::Paused,
            Some(metadata.status),
            Some(Some(WorkerAttentionReason::ManualPause)),
            Some(metadata.last_successful_login_at),
        )?;
        return Ok(false);
    }

    let login_state = platform_driver.detect_login_state(runtime, &session).await?;
    if login_state != PlatformLoginState::LoggedIn {
        let metadata = session_store
            .mark_worker_login_required(
                worker,
                &session.launch_plan,
                WorkerAttentionReason::SessionInvalid,
                metadata.last_successful_login_at,
            )
            .await?;
        supervisor.transition_worker_with_details(
            &worker.id,
            WorkerRuntimeState::LoginRequired,
            Some(metadata.status),
            Some(metadata.attention_reason),
            Some(metadata.last_successful_login_at),
        )?;
        return Ok(false);
    }

    let challenge_state = platform_driver
        .detect_challenge_state(runtime, &session)
        .await?;
    if challenge_state == PlatformChallengeState::ChallengeRequired {
        let metadata = session_store
            .mark_worker_challenge_required(
                worker,
                &session.launch_plan,
                metadata.last_successful_login_at,
            )
            .await?;
        supervisor.transition_worker_with_details(
            &worker.id,
            WorkerRuntimeState::ChallengeRequired,
            Some(metadata.status),
            Some(metadata.attention_reason),
            Some(metadata.last_successful_login_at),
        )?;
        return Ok(false);
    }

    let metadata = session_store
        .mark_worker_active(worker, &session.launch_plan)
        .await?;
    supervisor.transition_worker_with_details(
        &worker.id,
        WorkerRuntimeState::Ready,
        Some(metadata.status),
        Some(None),
        Some(metadata.last_successful_login_at),
    )?;

    let workspace_state = platform_driver
        .detect_workspace_state(runtime, &session)
        .await?;
    match workspace_state {
        PlatformWorkspaceState::ChatListVisible
        | PlatformWorkspaceState::ChatOpen
        | PlatformWorkspaceState::SearchOpen => {}
        PlatformWorkspaceState::LoginRequired => {
            let metadata = session_store
                .mark_worker_login_required(
                    worker,
                    &session.launch_plan,
                    WorkerAttentionReason::SessionInvalid,
                    metadata.last_successful_login_at,
                )
                .await?;
            supervisor.transition_worker_with_details(
                &worker.id,
                WorkerRuntimeState::LoginRequired,
                Some(metadata.status),
                Some(metadata.attention_reason),
                Some(metadata.last_successful_login_at),
            )?;
            return Ok(false);
        }
        PlatformWorkspaceState::ModalOpen | PlatformWorkspaceState::UnexpectedOverlay => {
            supervisor.transition_worker_with_details(
                &worker.id,
                WorkerRuntimeState::Error,
                Some(WorkerSessionStatus::Active),
                Some(Some(WorkerAttentionReason::PlatformCheckFailed)),
                Some(metadata.last_successful_login_at),
            )?;
            let _ = runtime
                .capture_recovery_artifacts(&session, &format!("{}-workspace", worker.id))
                .await;
            return Ok(false);
        }
        PlatformWorkspaceState::ErrorOrUnknown => {
            supervisor.transition_worker_with_details(
                &worker.id,
                WorkerRuntimeState::Error,
                Some(WorkerSessionStatus::Unknown),
                Some(Some(WorkerAttentionReason::PlatformCheckFailed)),
                Some(metadata.last_successful_login_at),
            )?;
            let _ = runtime
                .capture_recovery_artifacts(&session, &format!("{}-unknown", worker.id))
                .await;
            return Ok(false);
        }
    }

    let preflight = runtime
        .preflight_check(&session, &[platform_driver.selector_map().message_list.clone()])
        .await?;
    if !preflight.passed {
        supervisor.transition_worker_with_details(
            &worker.id,
            WorkerRuntimeState::Error,
            Some(WorkerSessionStatus::Unknown),
            Some(Some(WorkerAttentionReason::PlatformCheckFailed)),
            Some(metadata.last_successful_login_at),
        )?;
        let _ = runtime
            .capture_recovery_artifacts(&session, &format!("{}-preflight", worker.id))
            .await;
        return Ok(false);
    }

    let message_nodes = platform_driver
        .list_visible_messages(runtime, &session)
        .await
        .with_context(|| format!("list visible messages for {}", worker.id))?;
    let records = normalize_message_records(&worker.id, platform_driver, &message_nodes);

    let mut dedupe_state = dedupe_store.load_or_default(worker).await?;
    if !dedupe_state.bootstrapped {
        dedupe_store
            .checkpoint_messages(worker, &mut dedupe_state, &records)
            .await?;
        return Ok(true);
    }

    let inbound_message = detect_new_inbound_message(&dedupe_state, &records);
    dedupe_store
        .checkpoint_messages(worker, &mut dedupe_state, &records)
        .await?;
    let Some(inbound_message) = inbound_message else {
        return Ok(true);
    };

    let Some(decision) = reply_engine.decide_reply(worker, &inbound_message) else {
        return Ok(true);
    };

    supervisor.transition_worker(&worker.id, WorkerRuntimeState::ProcessingMessage)?;
    dedupe_store
        .stage_pending_reply(
            worker,
            &mut dedupe_state,
            &inbound_message,
            &decision.correlation_key,
            &decision.reply_text,
        )
        .await?;

    let send_result = platform_driver
        .send_reply(runtime, &session, &decision.reply_text)
        .await;

    match send_result {
        Ok(()) => {
            dedupe_store
                .commit_reply(
                    worker,
                    &mut dedupe_state,
                    &inbound_message,
                    &decision.correlation_key,
                )
                .await?;
            let metadata = session_store
                .mark_worker_active(worker, &session.launch_plan)
                .await?;
            supervisor.transition_worker_with_details(
                &worker.id,
                WorkerRuntimeState::Ready,
                Some(metadata.status),
                Some(None),
                Some(metadata.last_successful_login_at),
            )?;
            Ok(true)
        }
        Err(error) => {
            dedupe_store
                .clear_pending_reply(worker, &mut dedupe_state)
                .await?;
            supervisor.transition_worker_with_details(
                &worker.id,
                WorkerRuntimeState::Error,
                Some(WorkerSessionStatus::Active),
                Some(Some(WorkerAttentionReason::PlatformCheckFailed)),
                Some(metadata.last_successful_login_at),
            )?;
            Err(error)
        }
    }
}
