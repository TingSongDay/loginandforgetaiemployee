use zeroclaw::config::{Config, StationTilePosition};

#[test]
fn station_config_defaults_to_kiosk_left_surface_and_operator_panel() {
    let config = Config::default();
    let workers = &config.station.workers;

    assert!(!config.station.enabled);
    assert_eq!(config.station.station_name, "NeoHUman Station");
    assert_eq!(workers.len(), 2);
    assert_eq!(workers[0].id, "worker_a");
    assert_eq!(workers[0].managed_browser.viewport_width, 960);
    assert_eq!(workers[0].managed_browser.viewport_height, 900);
    assert!(workers[0].managed_browser.snap_back_before_interaction);
    assert!(workers[0].managed_browser.preflight_verification_enabled);
    assert_eq!(workers[0].tile_position, StationTilePosition::Left);
    assert_eq!(workers[1].tile_position, StationTilePosition::Right);
    assert!(config.station.right_panel.enabled);
    assert_eq!(config.station.right_panel.runtime_mode, "web_dashboard");
    assert_eq!(config.station.right_panel.local_url_or_path, "/_app");
}

#[test]
fn station_config_parses_explicit_workers() {
    let toml_str = r#"
[station]
enabled = true
message_poll_interval_ms = 2500
manual_intervention_timeout_secs = 600
reply_mode = "deterministic"
operator_display_name = "Operator Neo"

[[station.workers]]
id = "worker_a"
display_name = "Worker A"
tile_position = "left"
profile_name = "worker-a"
workspace_name = "worker-a"
dedupe_store_path = "state/worker-a.json"

[[station.workers]]
id = "worker_b"
display_name = "Worker B"
tile_position = "right"
profile_name = "worker-b"
workspace_name = "worker-b"
dedupe_store_path = "state/worker-b.json"
"#;

    let parsed: Config = toml::from_str(toml_str).expect("station config should parse");
    parsed
        .validate()
        .expect("parsed station config should validate");

    assert!(parsed.station.enabled);
    assert_eq!(parsed.station.message_poll_interval_ms, 2500);
    assert_eq!(
        parsed.station.operator_display_name.as_deref(),
        Some("Operator Neo")
    );
}

#[test]
fn station_config_rejects_zero_poll_interval() {
    let toml_str = r#"
[station]
enabled = true
message_poll_interval_ms = 0

[[station.workers]]
id = "worker_a"
display_name = "Worker A"
tile_position = "left"
profile_name = "worker-a"
workspace_name = "worker-a"
dedupe_store_path = "state/worker-a.json"

[[station.workers]]
id = "worker_b"
display_name = "Worker B"
tile_position = "right"
profile_name = "worker-b"
workspace_name = "worker-b"
dedupe_store_path = "state/worker-b.json"
"#;

    let parsed: Config = toml::from_str(toml_str).expect("station config should parse");
    let error = parsed.validate().expect_err("zero poll interval must fail");
    assert!(error
        .to_string()
        .contains("station.message_poll_interval_ms must be greater than 0"));
}

#[test]
fn station_config_rejects_duplicate_tiles() {
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

[[station.workers]]
id = "worker_b"
display_name = "Worker B"
tile_position = "left"
profile_name = "worker-b"
workspace_name = "worker-b"
dedupe_store_path = "state/worker-b.json"
"#;

    let parsed: Config = toml::from_str(toml_str).expect("station config should parse");
    let error = parsed
        .validate()
        .expect_err("duplicate left tile must fail");
    assert!(error
        .to_string()
        .contains("station.workers[1].tile_position must be unique"));
}
