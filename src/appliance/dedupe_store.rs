use crate::appliance::messages::MessageRecord;
use crate::config::{Config, StationWorkerConfig};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

const DEDUPE_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingReply {
    pub inbound_message_id: String,
    pub inbound_fingerprint: String,
    pub correlation_key: String,
    pub reply_text: String,
    pub staged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerDedupeState {
    pub metadata_version: u32,
    pub worker_id: String,
    pub bootstrapped: bool,
    pub last_seen_message_id: Option<String>,
    pub last_seen_inbound_message_id: Option<String>,
    pub last_processed_message_id: Option<String>,
    pub last_processed_message_fingerprint: Option<String>,
    pub last_reply_correlation_key: Option<String>,
    pub processed_message_ids: Vec<String>,
    pub processed_message_fingerprints: Vec<String>,
    pub pending_reply: Option<PendingReply>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupeStoreLoadResult {
    Missing,
    Loaded(WorkerDedupeState),
    Corrupted(String),
}

#[derive(Debug, Clone)]
pub struct DedupeStore {
    appliance_root: PathBuf,
}

impl DedupeStore {
    pub fn from_config(config: &Config) -> Self {
        Self {
            appliance_root: config.workspace_dir.join("station"),
        }
    }

    pub fn appliance_root(&self) -> &Path {
        &self.appliance_root
    }

    pub fn worker_state_path(&self, worker: &StationWorkerConfig) -> PathBuf {
        self.appliance_root.join(&worker.dedupe_store_path)
    }

    pub async fn ensure_layout_for_worker(&self, worker: &StationWorkerConfig) -> Result<()> {
        let path = self.worker_state_path(worker);
        let parent = path
            .parent()
            .context("worker dedupe path should have a parent directory")?;
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create dedupe store dir {}", parent.display()))
    }

    pub async fn load_worker_state(
        &self,
        worker: &StationWorkerConfig,
    ) -> Result<DedupeStoreLoadResult> {
        let path = self.worker_state_path(worker);
        let contents = match fs::read_to_string(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DedupeStoreLoadResult::Missing);
            }
            Err(error) => return Ok(DedupeStoreLoadResult::Corrupted(error.to_string())),
        };

        match serde_json::from_str::<WorkerDedupeState>(&contents) {
            Ok(state)
                if state.worker_id == worker.id
                    && state.metadata_version == DEDUPE_METADATA_VERSION =>
            {
                Ok(DedupeStoreLoadResult::Loaded(state))
            }
            Ok(_) => Ok(DedupeStoreLoadResult::Corrupted(
                "dedupe metadata failed validation".into(),
            )),
            Err(error) => Ok(DedupeStoreLoadResult::Corrupted(error.to_string())),
        }
    }

    pub async fn load_or_default(&self, worker: &StationWorkerConfig) -> Result<WorkerDedupeState> {
        match self.load_worker_state(worker).await? {
            DedupeStoreLoadResult::Loaded(state) => Ok(state),
            DedupeStoreLoadResult::Missing | DedupeStoreLoadResult::Corrupted(_) => {
                Ok(Self::default_state(&worker.id))
            }
        }
    }

    pub async fn save_worker_state(
        &self,
        worker: &StationWorkerConfig,
        state: &WorkerDedupeState,
    ) -> Result<()> {
        self.ensure_layout_for_worker(worker).await?;
        let path = self.worker_state_path(worker);
        let payload = serde_json::to_vec_pretty(state).context("serialize worker dedupe state")?;
        fs::write(&path, payload)
            .await
            .with_context(|| format!("write dedupe state {}", path.display()))
    }

    pub async fn checkpoint_messages(
        &self,
        worker: &StationWorkerConfig,
        state: &mut WorkerDedupeState,
        records: &[MessageRecord],
    ) -> Result<()> {
        if let Some(last) = records.last() {
            state.last_seen_message_id = Some(last.platform_message_id.clone());
        }
        if let Some(last_inbound) = records.iter().rev().find(|record| {
            matches!(
                record.direction,
                crate::appliance::platforms::MessageDirection::Inbound
            )
        }) {
            state.last_seen_inbound_message_id = Some(last_inbound.platform_message_id.clone());
        }
        state.bootstrapped = true;
        state.updated_at = Utc::now();
        self.save_worker_state(worker, state).await
    }

    pub async fn stage_pending_reply(
        &self,
        worker: &StationWorkerConfig,
        state: &mut WorkerDedupeState,
        message: &MessageRecord,
        correlation_key: &str,
        reply_text: &str,
    ) -> Result<()> {
        state.pending_reply = Some(PendingReply {
            inbound_message_id: message.platform_message_id.clone(),
            inbound_fingerprint: message.raw_fingerprint.clone(),
            correlation_key: correlation_key.to_string(),
            reply_text: reply_text.to_string(),
            staged_at: Utc::now(),
        });
        state.updated_at = Utc::now();
        self.save_worker_state(worker, state).await
    }

    pub async fn clear_pending_reply(
        &self,
        worker: &StationWorkerConfig,
        state: &mut WorkerDedupeState,
    ) -> Result<()> {
        state.pending_reply = None;
        state.updated_at = Utc::now();
        self.save_worker_state(worker, state).await
    }

    pub async fn commit_reply(
        &self,
        worker: &StationWorkerConfig,
        state: &mut WorkerDedupeState,
        message: &MessageRecord,
        correlation_key: &str,
    ) -> Result<()> {
        push_limited(
            &mut state.processed_message_ids,
            &message.platform_message_id,
            200,
        );
        push_limited(
            &mut state.processed_message_fingerprints,
            &message.raw_fingerprint,
            200,
        );
        state.last_processed_message_id = Some(message.platform_message_id.clone());
        state.last_processed_message_fingerprint = Some(message.raw_fingerprint.clone());
        state.last_reply_correlation_key = Some(correlation_key.to_string());
        state.pending_reply = None;
        state.updated_at = Utc::now();
        self.save_worker_state(worker, state).await
    }

    pub fn default_state(worker_id: &str) -> WorkerDedupeState {
        WorkerDedupeState {
            metadata_version: DEDUPE_METADATA_VERSION,
            worker_id: worker_id.to_string(),
            bootstrapped: false,
            last_seen_message_id: None,
            last_seen_inbound_message_id: None,
            last_processed_message_id: None,
            last_processed_message_fingerprint: None,
            last_reply_correlation_key: None,
            processed_message_ids: Vec::new(),
            processed_message_fingerprints: Vec::new(),
            pending_reply: None,
            updated_at: Utc::now(),
        }
    }
}

fn push_limited(entries: &mut Vec<String>, value: &str, limit: usize) {
    if entries.iter().any(|entry| entry == value) {
        return;
    }
    entries.push(value.to_string());
    if entries.len() > limit {
        let overflow = entries.len() - limit;
        entries.drain(0..overflow);
    }
}
