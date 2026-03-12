use crate::{
    appliance::messages::MessageRecord,
    config::{Config, StationWorkerConfig},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyDecision {
    pub correlation_key: String,
    pub reply_text: String,
}

#[derive(Debug, Clone)]
pub struct ReplyEngine {
    station_name: String,
    operator_display_name: Option<String>,
    reply_mode: String,
}

impl ReplyEngine {
    pub fn from_config(config: &Config) -> Self {
        Self {
            station_name: config.station.station_name.clone(),
            operator_display_name: config.station.operator_display_name.clone(),
            reply_mode: config.station.reply_mode.clone(),
        }
    }

    pub fn decide_reply(
        &self,
        worker: &StationWorkerConfig,
        message: &MessageRecord,
    ) -> Option<ReplyDecision> {
        if self.reply_mode.trim().eq_ignore_ascii_case("off") || message.text.trim().is_empty() {
            return None;
        }

        let operator = self
            .operator_display_name
            .as_deref()
            .unwrap_or(&self.station_name);
        let reply_text = format!(
            "Received by {}. {} will follow up shortly.",
            worker.display_name, operator
        );

        Some(ReplyDecision {
            correlation_key: correlation_key(&worker.id, message),
            reply_text,
        })
    }
}

fn correlation_key(worker_id: &str, message: &MessageRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{worker_id}:{}:{}",
            message.platform_message_id, message.raw_fingerprint
        )
        .as_bytes(),
    );
    hex::encode(&hasher.finalize()[..16])
}
