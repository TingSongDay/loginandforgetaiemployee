use zeroclaw::appliance::{
    browser_runtime::{build_launch_plan, runtime_kind_for_config, ManagedBrowserRuntimeKind},
    tile_manager::TileManager,
};
use zeroclaw::config::Config;

#[test]
fn managed_browser_defaults_are_tiled_for_two_workers() {
    let config = Config::default();
    let worker_a = &config.station.workers[0];
    let worker_b = &config.station.workers[1];

    assert_eq!(worker_a.managed_browser.viewport_width, 960);
    assert_eq!(worker_a.managed_browser.window_origin_x, 0);
    assert_eq!(worker_b.managed_browser.window_origin_x, 960);
    assert_eq!(worker_b.managed_browser.viewport_height, 900);
}

#[test]
fn station_config_rejects_empty_browser_binary_path() {
    let toml_str = r#"
[station]
enabled = true

[[station.workers]]
id = "worker_a"
display_name = "Worker A"
tile_position = "left"
profile_name = "worker-a"
workspace_name = "worker-a"
dedupe_store_path = "state/worker-a.json"
[station.workers.managed_browser]
browser_binary_path = "   "

[[station.workers]]
id = "worker_b"
display_name = "Worker B"
tile_position = "right"
profile_name = "worker-b"
workspace_name = "worker-b"
dedupe_store_path = "state/worker-b.json"
"#;

    let parsed: Config = toml::from_str(toml_str).expect("station config should parse");
    let error = parsed
        .validate()
        .expect_err("empty browser binary path must fail");
    assert!(error
        .to_string()
        .contains("station.workers[0].managed_browser.browser_binary_path must not be empty"));
}

#[test]
fn tile_manager_builds_expected_left_and_right_layout() {
    let config = Config::default();
    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");

    let left = tile_manager
        .placement_for_worker("worker_a")
        .expect("left placement");
    let right = tile_manager
        .placement_for_worker("worker_b")
        .expect("right placement");

    assert_eq!(left.window_origin_x, 0);
    assert_eq!(right.window_origin_x, left.viewport_width as i32);
    assert_eq!(left.viewport_height, right.viewport_height);
}

#[test]
fn launch_plan_is_deterministic_for_right_worker() {
    let mut config = Config::default();
    config.station.enabled = true;

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let plan =
        build_launch_plan(&config, &tile_manager, &config.station.workers[1]).expect("launch plan");

    assert_eq!(plan.worker_id, "worker_b");
    assert_eq!(plan.window_origin_x, 960);
    assert!(plan.user_data_dir.ends_with("station/profiles/worker-b"));
}

#[test]
fn runtime_kind_uses_rust_native_when_requested() {
    let mut config = Config::default();
    config.browser.backend = "rust_native".into();

    assert_eq!(
        runtime_kind_for_config(&config),
        ManagedBrowserRuntimeKind::RustNative
    );
}
