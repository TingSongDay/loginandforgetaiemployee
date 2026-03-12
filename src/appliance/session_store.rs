use crate::appliance::{
    browser_runtime::ManagedBrowserLaunchPlan,
    state::{WorkerAttentionReason, WorkerSessionStatus},
};
use crate::config::{Config, StationWorkerConfig};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

const SESSION_METADATA_FILE: &str = "session.json";
const SESSION_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedWorkerSession {
    pub metadata_version: u32,
    pub worker_id: String,
    pub profile_name: String,
    pub workspace_name: String,
    pub session_name: String,
    pub browser_user_data_dir: String,
    pub status: WorkerSessionStatus,
    #[serde(default)]
    pub paused: bool,
    pub attention_reason: Option<WorkerAttentionReason>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub last_successful_login_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStoreLoadResult {
    Missing,
    Loaded(PersistedWorkerSession),
    Corrupted(String),
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    appliance_root: PathBuf,
    sessions_root: PathBuf,
}

impl SessionStore {
    pub fn from_config(config: &Config) -> Self {
        let appliance_root = config.workspace_dir.join("station");
        Self::new(appliance_root)
    }

    pub fn new(appliance_root: PathBuf) -> Self {
        Self {
            sessions_root: appliance_root.join("sessions"),
            appliance_root,
        }
    }

    pub fn appliance_root(&self) -> &Path {
        &self.appliance_root
    }

    pub fn sessions_root(&self) -> &Path {
        &self.sessions_root
    }

    pub fn worker_session_dir(&self, worker_id: &str) -> PathBuf {
        self.sessions_root.join(worker_id)
    }

    pub fn worker_session_metadata_path(&self, worker_id: &str) -> PathBuf {
        self.worker_session_dir(worker_id)
            .join(SESSION_METADATA_FILE)
    }

    pub async fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.sessions_root)
            .await
            .with_context(|| {
                format!(
                    "create session store root at {}",
                    self.sessions_root.display()
                )
            })
    }

    pub async fn load_worker_session(&self, worker_id: &str) -> Result<SessionStoreLoadResult> {
        let metadata_path = self.worker_session_metadata_path(worker_id);
        let contents = match fs::read_to_string(&metadata_path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SessionStoreLoadResult::Missing);
            }
            Err(error) => {
                return Ok(SessionStoreLoadResult::Corrupted(error.to_string()));
            }
        };

        match serde_json::from_str::<PersistedWorkerSession>(&contents) {
            Ok(metadata)
                if metadata.worker_id == worker_id
                    && metadata.metadata_version == SESSION_METADATA_VERSION =>
            {
                Ok(SessionStoreLoadResult::Loaded(metadata))
            }
            Ok(_) => Ok(SessionStoreLoadResult::Corrupted(
                "session metadata failed validation".into(),
            )),
            Err(error) => Ok(SessionStoreLoadResult::Corrupted(error.to_string())),
        }
    }

    pub async fn save_worker_session(&self, metadata: &PersistedWorkerSession) -> Result<()> {
        let metadata_path = self.worker_session_metadata_path(&metadata.worker_id);
        let session_dir = metadata_path
            .parent()
            .context("session metadata path should have a parent directory")?;

        fs::create_dir_all(session_dir)
            .await
            .with_context(|| format!("create session dir at {}", session_dir.display()))?;

        let serialized =
            serde_json::to_vec_pretty(metadata).context("serialize session metadata")?;
        let temp_path = metadata_path.with_extension("json.tmp");
        fs::write(&temp_path, serialized)
            .await
            .with_context(|| format!("write temp session metadata to {}", temp_path.display()))?;
        fs::rename(&temp_path, &metadata_path)
            .await
            .with_context(|| format!("rename session metadata into {}", metadata_path.display()))?;
        Ok(())
    }

    pub async fn mark_worker_active(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
    ) -> Result<PersistedWorkerSession> {
        let existing = self.load_or_initialize_worker(worker, plan).await?;
        let metadata = self.build_metadata(
            worker,
            plan,
            WorkerSessionStatus::Active,
            None,
            existing.paused,
            None,
            Some(Utc::now()),
        );
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn mark_worker_login_required(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
        reason: WorkerAttentionReason,
        last_successful_login_at: Option<DateTime<Utc>>,
    ) -> Result<PersistedWorkerSession> {
        let existing = self.load_or_initialize_worker(worker, plan).await?;
        let metadata = self.build_metadata(
            worker,
            plan,
            WorkerSessionStatus::LoginRequired,
            Some(reason),
            existing.paused,
            None,
            last_successful_login_at,
        );
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn mark_worker_challenge_required(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
        last_successful_login_at: Option<DateTime<Utc>>,
    ) -> Result<PersistedWorkerSession> {
        let existing = self.load_or_initialize_worker(worker, plan).await?;
        let metadata = self.build_metadata(
            worker,
            plan,
            WorkerSessionStatus::ChallengeRequired,
            Some(WorkerAttentionReason::ChallengeDetected),
            existing.paused,
            None,
            last_successful_login_at,
        );
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn mark_worker_error(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
        reason: WorkerAttentionReason,
        last_successful_login_at: Option<DateTime<Utc>>,
        last_error: String,
    ) -> Result<PersistedWorkerSession> {
        let existing = self.load_or_initialize_worker(worker, plan).await?;
        let metadata = self.build_metadata(
            worker,
            plan,
            WorkerSessionStatus::Unknown,
            Some(reason),
            existing.paused,
            Some(last_error),
            last_successful_login_at,
        );
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn set_worker_paused(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
        paused: bool,
    ) -> Result<PersistedWorkerSession> {
        let mut metadata = self.load_or_initialize_worker(worker, plan).await?;
        metadata.paused = paused;
        metadata.attention_reason = if paused {
            Some(WorkerAttentionReason::ManualPause)
        } else {
            None
        };
        metadata.last_error = None;
        metadata.updated_at = Utc::now();
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn mark_worker_login_complete(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
    ) -> Result<PersistedWorkerSession> {
        let mut metadata = self.load_or_initialize_worker(worker, plan).await?;
        metadata.status = WorkerSessionStatus::Active;
        metadata.attention_reason = None;
        metadata.last_error = None;
        metadata.paused = false;
        metadata.last_successful_login_at = Some(Utc::now());
        metadata.updated_at = Utc::now();
        self.save_worker_session(&metadata).await?;
        Ok(metadata)
    }

    pub async fn mark_worker_challenge_complete(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
    ) -> Result<PersistedWorkerSession> {
        self.mark_worker_login_complete(worker, plan).await
    }

    pub async fn load_or_initialize_worker(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
    ) -> Result<PersistedWorkerSession> {
        match self.load_worker_session(&worker.id).await? {
            SessionStoreLoadResult::Loaded(metadata) => Ok(metadata),
            SessionStoreLoadResult::Missing | SessionStoreLoadResult::Corrupted(_) => {
                Ok(self.default_worker_session(worker, plan))
            }
        }
    }

    pub async fn reset_worker_state(&self, metadata: &PersistedWorkerSession) -> Result<()> {
        let session_dir = self.worker_session_dir(&metadata.worker_id);
        if fs::try_exists(&session_dir).await.unwrap_or(false) {
            fs::remove_dir_all(&session_dir)
                .await
                .with_context(|| format!("remove session dir {}", session_dir.display()))?;
        }

        let user_data_dir = PathBuf::from(&metadata.browser_user_data_dir);
        if user_data_dir.starts_with(&self.appliance_root)
            && fs::try_exists(&user_data_dir).await.unwrap_or(false)
        {
            fs::remove_dir_all(&user_data_dir)
                .await
                .with_context(|| format!("remove browser profile {}", user_data_dir.display()))?;
        }

        Ok(())
    }

    fn build_metadata(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
        status: WorkerSessionStatus,
        attention_reason: Option<WorkerAttentionReason>,
        paused: bool,
        last_error: Option<String>,
        last_successful_login_at: Option<DateTime<Utc>>,
    ) -> PersistedWorkerSession {
        PersistedWorkerSession {
            metadata_version: SESSION_METADATA_VERSION,
            worker_id: worker.id.clone(),
            profile_name: worker.profile_name.clone(),
            workspace_name: worker.workspace_name.clone(),
            session_name: plan.session_name.clone(),
            browser_user_data_dir: plan.user_data_dir.display().to_string(),
            status,
            paused,
            attention_reason,
            last_error,
            last_successful_login_at,
            updated_at: Utc::now(),
        }
    }

    pub fn default_worker_session(
        &self,
        worker: &StationWorkerConfig,
        plan: &ManagedBrowserLaunchPlan,
    ) -> PersistedWorkerSession {
        self.build_metadata(
            worker,
            plan,
            WorkerSessionStatus::Unknown,
            None,
            false,
            None,
            None,
        )
    }
}
