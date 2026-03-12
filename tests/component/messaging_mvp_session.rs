use tempfile::TempDir;
use zeroclaw::appliance::{
    browser_runtime::build_launch_plan,
    session_store::{SessionStore, SessionStoreLoadResult},
    state::{WorkerAttentionReason, WorkerSessionStatus},
    tile_manager::TileManager,
};
use zeroclaw::config::Config;

fn temp_station_config() -> (TempDir, Config) {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut config = Config::default();
    config.station.enabled = true;
    config.workspace_dir = temp_dir.path().join("workspace");
    config.config_path = temp_dir.path().join("config.toml");
    (temp_dir, config)
}

#[tokio::test]
async fn session_store_round_trips_worker_metadata() {
    let (_temp_dir, config) = temp_station_config();
    let store = SessionStore::from_config(&config);
    store.ensure_layout().await.expect("session layout");

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let worker = &config.station.workers[0];
    let plan = build_launch_plan(&config, &tile_manager, worker).expect("launch plan");

    let saved = store
        .mark_worker_active(worker, &plan)
        .await
        .expect("save active session");

    match store
        .load_worker_session(&worker.id)
        .await
        .expect("load session")
    {
        SessionStoreLoadResult::Loaded(loaded) => {
            assert_eq!(loaded.worker_id, worker.id);
            assert_eq!(loaded.status, WorkerSessionStatus::Active);
            assert_eq!(loaded.browser_user_data_dir, saved.browser_user_data_dir);
            assert_eq!(
                loaded.last_successful_login_at,
                saved.last_successful_login_at
            );
        }
        other => panic!("expected loaded session metadata, got {other:?}"),
    }
}

#[tokio::test]
async fn corrupted_session_metadata_fails_safe() {
    let (_temp_dir, config) = temp_station_config();
    let store = SessionStore::from_config(&config);
    store.ensure_layout().await.expect("session layout");

    let metadata_path = store.worker_session_metadata_path("worker_a");
    tokio::fs::create_dir_all(metadata_path.parent().expect("parent"))
        .await
        .expect("session dir");
    tokio::fs::write(&metadata_path, "{ definitely not valid json")
        .await
        .expect("write corrupt metadata");

    let result = store
        .load_worker_session("worker_a")
        .await
        .expect("load result");

    assert!(matches!(result, SessionStoreLoadResult::Corrupted(_)));
}

#[tokio::test]
async fn reset_only_deletes_appliance_owned_state() {
    let (temp_dir, config) = temp_station_config();
    let store = SessionStore::from_config(&config);
    store.ensure_layout().await.expect("session layout");

    let external_profile = temp_dir.path().join("external-profile");
    tokio::fs::create_dir_all(&external_profile)
        .await
        .expect("external profile");

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let worker = &config.station.workers[0];
    let plan = build_launch_plan(&config, &tile_manager, worker).expect("launch plan");

    let mut saved = store
        .mark_worker_active(worker, &plan)
        .await
        .expect("save active session");
    saved.browser_user_data_dir = external_profile.display().to_string();
    store
        .save_worker_session(&saved)
        .await
        .expect("overwrite session metadata");

    store
        .reset_worker_state(&saved)
        .await
        .expect("reset worker state");

    assert!(!tokio::fs::try_exists(store.worker_session_dir(&worker.id))
        .await
        .expect("check session dir"));
    assert!(tokio::fs::try_exists(&external_profile)
        .await
        .expect("check external profile"));
}

#[tokio::test]
async fn worker_session_stores_remain_isolated() {
    let (_temp_dir, config) = temp_station_config();
    let store = SessionStore::from_config(&config);
    store.ensure_layout().await.expect("session layout");

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let worker_a = &config.station.workers[0];
    let worker_b = &config.station.workers[1];
    let plan_a = build_launch_plan(&config, &tile_manager, worker_a).expect("left plan");
    let plan_b = build_launch_plan(&config, &tile_manager, worker_b).expect("right plan");

    store
        .mark_worker_active(worker_a, &plan_a)
        .await
        .expect("save worker a");
    store
        .mark_worker_login_required(
            worker_b,
            &plan_b,
            WorkerAttentionReason::SessionMissing,
            None,
        )
        .await
        .expect("save worker b");

    let loaded_a = store
        .load_worker_session(&worker_a.id)
        .await
        .expect("load worker a");
    let loaded_b = store
        .load_worker_session(&worker_b.id)
        .await
        .expect("load worker b");

    match (loaded_a, loaded_b) {
        (
            SessionStoreLoadResult::Loaded(worker_a_metadata),
            SessionStoreLoadResult::Loaded(worker_b_metadata),
        ) => {
            assert_eq!(worker_a_metadata.status, WorkerSessionStatus::Active);
            assert_eq!(worker_b_metadata.status, WorkerSessionStatus::LoginRequired);
            assert_ne!(
                store.worker_session_metadata_path(&worker_a.id),
                store.worker_session_metadata_path(&worker_b.id)
            );
        }
        other => panic!("expected both worker session metadata files, got {other:?}"),
    }
}
