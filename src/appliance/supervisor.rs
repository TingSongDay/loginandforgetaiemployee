use crate::appliance::state::{
    StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
};
use crate::config::{Config, StationConfig, StationTilePosition};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationWorkerRuntime {
    pub id: String,
    pub display_name: String,
    pub tile_position: StationTilePosition,
    pub state: WorkerRuntimeState,
    pub profile_name: String,
    pub workspace_name: String,
    pub dedupe_store_path: String,
    pub session_status: WorkerSessionStatus,
    pub attention_reason: Option<WorkerAttentionReason>,
    pub last_successful_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct StationSupervisor {
    state: StationSupervisorState,
    workers: Vec<StationWorkerRuntime>,
}

impl StationSupervisor {
    pub fn from_config(config: &Config) -> Result<Self> {
        Self::from_station_config(&config.station)
    }

    pub fn from_station_config(config: &StationConfig) -> Result<Self> {
        let mut workers = Vec::with_capacity(config.workers.len());
        for worker in &config.workers {
            workers.push(StationWorkerRuntime {
                id: worker.id.clone(),
                display_name: worker.display_name.clone(),
                tile_position: worker.tile_position.clone(),
                state: WorkerRuntimeState::Booting,
                profile_name: worker.profile_name.clone(),
                workspace_name: worker.workspace_name.clone(),
                dedupe_store_path: worker.dedupe_store_path.clone(),
                session_status: WorkerSessionStatus::Unknown,
                attention_reason: None,
                last_successful_login_at: None,
            });
        }

        workers.sort_by_key(|worker| match worker.tile_position {
            StationTilePosition::Left => 0,
            StationTilePosition::Right => 1,
        });

        Ok(Self {
            state: StationSupervisorState::Booting,
            workers,
        })
    }

    pub fn state(&self) -> StationSupervisorState {
        self.state
    }

    pub fn workers(&self) -> &[StationWorkerRuntime] {
        &self.workers
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn worker(&self, id: &str) -> Option<&StationWorkerRuntime> {
        self.workers.iter().find(|worker| worker.id == id)
    }

    pub fn set_state(&mut self, next: StationSupervisorState) -> Result<()> {
        if !self.state.can_transition_to(next) {
            return Err(anyhow!(
                "invalid station supervisor transition: {:?} -> {:?}",
                self.state,
                next
            ));
        }
        self.state = next;
        Ok(())
    }

    pub fn transition_worker(&mut self, id: &str, next: WorkerRuntimeState) -> Result<()> {
        self.transition_worker_with_details(id, next, None, None, None)
    }

    pub fn transition_worker_with_details(
        &mut self,
        id: &str,
        next: WorkerRuntimeState,
        session_status: Option<WorkerSessionStatus>,
        attention_reason: Option<Option<WorkerAttentionReason>>,
        last_successful_login_at: Option<Option<DateTime<Utc>>>,
    ) -> Result<()> {
        let worker = self
            .workers
            .iter_mut()
            .find(|worker| worker.id == id)
            .ok_or_else(|| anyhow!("unknown station worker: {id}"))?;

        if !worker.state.can_transition_to(next) {
            return Err(anyhow!(
                "invalid worker transition for {id}: {:?} -> {:?}",
                worker.state,
                next
            ));
        }

        worker.state = next;
        if let Some(session_status) = session_status {
            worker.session_status = session_status;
        }
        if let Some(attention_reason) = attention_reason {
            worker.attention_reason = attention_reason;
        }
        if let Some(last_successful_login_at) = last_successful_login_at {
            worker.last_successful_login_at = last_successful_login_at;
        }
        Ok(())
    }

    pub fn left_worker(&self) -> Option<&StationWorkerRuntime> {
        self.workers
            .iter()
            .find(|worker| worker.tile_position == StationTilePosition::Left)
    }

    pub fn right_worker(&self) -> Option<&StationWorkerRuntime> {
        self.workers
            .iter()
            .find(|worker| worker.tile_position == StationTilePosition::Right)
    }
}
