use tempfile::TempDir;
use zeroclaw::{
    appliance::{
        dedupe_store::DedupeStore,
        detector::detect_new_inbound_message,
        messages::{normalize_message_records, MessageRecord},
        platforms::{MessageDirection, WeChatWebDriver},
        reply_engine::ReplyEngine,
    },
    config::Config,
};

fn temp_station_config() -> (TempDir, Config) {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut config = Config::default();
    config.station.enabled = true;
    config.workspace_dir = temp_dir.path().join("workspace");
    config.config_path = temp_dir.path().join("config.toml");
    (temp_dir, config)
}

fn inbound(worker_id: &str, message_id: &str, text: &str) -> MessageRecord {
    MessageRecord {
        worker_id: worker_id.to_string(),
        platform_message_id: message_id.to_string(),
        timestamp: chrono::Utc::now(),
        direction: MessageDirection::Inbound,
        author: None,
        text: text.to_string(),
        raw_fingerprint: format!("fp-{message_id}"),
    }
}

#[test]
fn message_normalization_collapses_whitespace_and_deduplicates() {
    let driver = WeChatWebDriver::default();
    let nodes = vec![
        zeroclaw::appliance::platforms::PlatformMessageNode {
            dom_id: Some("msg-1".into()),
            text: " hello   world ".into(),
            direction: MessageDirection::Inbound,
        },
        zeroclaw::appliance::platforms::PlatformMessageNode {
            dom_id: Some("msg-1".into()),
            text: "hello world".into(),
            direction: MessageDirection::Inbound,
        },
    ];

    let records = normalize_message_records("worker_a", &driver, &nodes);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].text, "hello world");
}

#[tokio::test]
async fn dedupe_store_bootstrap_prevents_replaying_existing_history() {
    let (_temp_dir, config) = temp_station_config();
    let worker = &config.station.workers[0];
    let store = DedupeStore::from_config(&config);

    let initial_records = vec![inbound(&worker.id, "msg-1", "hello")];
    let mut state = store.load_or_default(worker).await.expect("default state");
    assert!(detect_new_inbound_message(&state, &initial_records).is_none());

    store
        .checkpoint_messages(worker, &mut state, &initial_records)
        .await
        .expect("checkpoint history");
    assert!(state.bootstrapped);
    assert!(detect_new_inbound_message(&state, &initial_records).is_none());

    let next_records = vec![
        initial_records[0].clone(),
        inbound(&worker.id, "msg-2", "follow up"),
    ];
    let detected = detect_new_inbound_message(&state, &next_records).expect("new inbound");
    assert_eq!(detected.platform_message_id, "msg-2");
}

#[tokio::test]
async fn dedupe_store_pending_and_commit_block_duplicates() {
    let (_temp_dir, config) = temp_station_config();
    let worker = &config.station.workers[0];
    let store = DedupeStore::from_config(&config);
    let mut state = store.load_or_default(worker).await.expect("default state");
    let records = vec![inbound(&worker.id, "msg-1", "hello")];

    store
        .checkpoint_messages(worker, &mut state, &records)
        .await
        .expect("checkpoint");

    let next_records = vec![
        records[0].clone(),
        inbound(&worker.id, "msg-2", "new inbound"),
    ];
    let detected = detect_new_inbound_message(&state, &next_records).expect("detect inbound");
    store
        .stage_pending_reply(worker, &mut state, &detected, "corr-1", "reply")
        .await
        .expect("stage reply");
    assert!(detect_new_inbound_message(&state, &next_records).is_none());

    store
        .commit_reply(worker, &mut state, &detected, "corr-1")
        .await
        .expect("commit reply");
    assert!(detect_new_inbound_message(&state, &next_records).is_none());
}

#[test]
fn reply_engine_returns_deterministic_text() {
    let config = Config::default();
    let engine = ReplyEngine::from_config(&config);
    let worker = &config.station.workers[0];
    let decision = engine
        .decide_reply(worker, &inbound(&worker.id, "msg-1", "Can you help?"))
        .expect("reply decision");

    assert!(decision.reply_text.contains(&worker.display_name));
    assert!(!decision.correlation_key.is_empty());
}
