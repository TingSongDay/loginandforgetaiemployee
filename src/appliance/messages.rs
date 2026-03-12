use crate::appliance::platforms::{MessageDirection, MessagingPlatformDriver, PlatformMessageNode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub worker_id: String,
    pub platform_message_id: String,
    pub timestamp: DateTime<Utc>,
    pub direction: MessageDirection,
    pub author: Option<String>,
    pub text: String,
    pub raw_fingerprint: String,
}

pub fn normalize_message_records(
    worker_id: &str,
    driver: &dyn MessagingPlatformDriver,
    nodes: &[PlatformMessageNode],
) -> Vec<MessageRecord> {
    let mut seen = HashSet::new();
    let mut records = Vec::new();

    for node in nodes {
        let text = normalize_text(&driver.extract_message_text(node));
        if text.is_empty() {
            continue;
        }

        let platform_message_id = driver.extract_message_id(node);
        let direction = driver.extract_message_direction(node);
        let raw_fingerprint = fingerprint(&direction, &text);
        let dedupe_key = format!("{platform_message_id}:{raw_fingerprint}");
        if !seen.insert(dedupe_key) {
            continue;
        }

        records.push(MessageRecord {
            worker_id: worker_id.to_string(),
            platform_message_id,
            timestamp: Utc::now(),
            direction,
            author: None,
            text,
            raw_fingerprint,
        });
    }

    records
}

pub fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fingerprint(direction: &MessageDirection, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{direction:?}:{text}").as_bytes());
    hex::encode(&hasher.finalize()[..16])
}
