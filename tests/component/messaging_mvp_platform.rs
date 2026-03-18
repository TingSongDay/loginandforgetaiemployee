use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use zeroclaw::appliance::{
    browser_runtime::{
        build_launch_plan, ManagedBrowserLaunchPlan, ManagedBrowserLaunchReport,
        ManagedBrowserPlacementCheck, ManagedBrowserPreflightCheck, ManagedBrowserRuntime,
        ManagedBrowserRuntimeKind, ManagedBrowserSession,
    },
    platforms::{
        MessageDirection, MessagingPlatformDriver, PlatformChallengeState, PlatformLoginState,
        PlatformMessageNode, PlatformWorkspaceState, WhatsAppWebDriver, WeChatWebDriver,
    },
    tile_manager::{TileManager, TilePlacement},
};
use zeroclaw::config::Config;
use zeroclaw::tools::ToolResult;

#[derive(Default)]
struct MockRuntime {
    visible: Mutex<HashMap<String, bool>>,
    text: Mutex<HashMap<String, String>>,
    actions: Mutex<Vec<String>>,
}

impl MockRuntime {
    fn set_visible(&self, selector: &str, visible: bool) {
        self.visible.lock().insert(selector.to_string(), visible);
    }

    fn set_text(&self, selector: &str, text: &str) {
        self.text
            .lock()
            .insert(selector.to_string(), text.to_string());
    }

    fn actions(&self) -> Vec<String> {
        self.actions.lock().clone()
    }
}

#[async_trait]
impl ManagedBrowserRuntime for MockRuntime {
    fn kind(&self) -> ManagedBrowserRuntimeKind {
        ManagedBrowserRuntimeKind::AgentBrowser
    }

    async fn launch(&self, plan: &ManagedBrowserLaunchPlan) -> Result<ManagedBrowserSession> {
        Ok(ManagedBrowserSession {
            worker_id: plan.worker_id.clone(),
            runtime_kind: self.kind(),
            placement: TilePlacement {
                worker_id: plan.worker_id.clone(),
                tile_position: plan.tile_position,
                window_origin_x: plan.window_origin_x,
                window_origin_y: plan.window_origin_y,
                viewport_width: plan.viewport_width,
                viewport_height: plan.viewport_height,
            },
            launch_plan: plan.clone(),
            launch_report: launch_report(plan),
        })
    }

    async fn connect(&self, _session: &ManagedBrowserSession) -> Result<()> {
        Ok(())
    }

    async fn open_url(&self, _session: &ManagedBrowserSession, url: &str) -> Result<ToolResult> {
        self.actions.lock().push(format!("open:{url}"));
        Ok(ok_tool("opened"))
    }

    async fn snapshot(&self, _session: &ManagedBrowserSession) -> Result<ToolResult> {
        Ok(ok_tool("{}"))
    }

    async fn click(&self, _session: &ManagedBrowserSession, selector: &str) -> Result<ToolResult> {
        self.actions.lock().push(format!("click:{selector}"));
        Ok(ok_tool("clicked"))
    }

    async fn fill(
        &self,
        _session: &ManagedBrowserSession,
        selector: &str,
        value: &str,
    ) -> Result<ToolResult> {
        self.actions.lock().push(format!("fill:{selector}:{value}"));
        Ok(ok_tool("filled"))
    }

    async fn type_text(
        &self,
        _session: &ManagedBrowserSession,
        selector: &str,
        text: &str,
    ) -> Result<ToolResult> {
        self.actions.lock().push(format!("type:{selector}:{text}"));
        Ok(ok_tool("typed"))
    }

    async fn get_text(
        &self,
        _session: &ManagedBrowserSession,
        selector: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool(
            self.text
                .lock()
                .get(selector)
                .cloned()
                .unwrap_or_default()
                .as_str(),
        ))
    }

    async fn is_visible(
        &self,
        _session: &ManagedBrowserSession,
        selector: &str,
    ) -> Result<ToolResult> {
        let visible = self.visible.lock().get(selector).copied().unwrap_or(false);
        Ok(ok_tool(if visible { "true" } else { "false" }))
    }

    async fn screenshot(
        &self,
        _session: &ManagedBrowserSession,
        _path: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool("screenshot"))
    }

    async fn ensure_canonical_placement(
        &self,
        session: &ManagedBrowserSession,
    ) -> Result<ManagedBrowserSession> {
        Ok(session.clone())
    }

    async fn verify_canonical_placement(
        &self,
        session: &ManagedBrowserSession,
    ) -> Result<ManagedBrowserPlacementCheck> {
        Ok(ManagedBrowserPlacementCheck {
            matches_canonical: true,
            window_origin_x: session.launch_plan.window_origin_x,
            window_origin_y: session.launch_plan.window_origin_y,
            viewport_width: session.launch_plan.viewport_width,
            viewport_height: session.launch_plan.viewport_height,
        })
    }

    async fn preflight_check(
        &self,
        session: &ManagedBrowserSession,
        required_selectors: &[String],
    ) -> Result<ManagedBrowserPreflightCheck> {
        Ok(ManagedBrowserPreflightCheck {
            passed: true,
            placement: ManagedBrowserPlacementCheck {
                matches_canonical: true,
                window_origin_x: session.launch_plan.window_origin_x,
                window_origin_y: session.launch_plan.window_origin_y,
                viewport_width: session.launch_plan.viewport_width,
                viewport_height: session.launch_plan.viewport_height,
            },
            visible_selectors: required_selectors.to_vec(),
            missing_selectors: Vec::new(),
        })
    }

    async fn capture_recovery_artifacts(
        &self,
        _session: &ManagedBrowserSession,
        artifact_prefix: &str,
    ) -> Result<Vec<String>> {
        Ok(vec![format!("{artifact_prefix}-recovery.png")])
    }

    async fn move_to_tile(
        &self,
        session: &ManagedBrowserSession,
        placement: &TilePlacement,
    ) -> Result<ManagedBrowserSession> {
        let mut updated = session.clone();
        updated.placement = placement.clone();
        Ok(updated)
    }

    async fn close(&self, _session: &ManagedBrowserSession) -> Result<ToolResult> {
        Ok(ok_tool("closed"))
    }
}

fn ok_tool(output: &str) -> ToolResult {
    ToolResult {
        success: true,
        output: output.to_string(),
        error: None,
    }
}

fn test_session() -> ManagedBrowserSession {
    let mut config = Config::default();
    config.station.enabled = true;
    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    let plan =
        build_launch_plan(&config, &tile_manager, &config.station.workers[0]).expect("launch plan");

    let report = launch_report(&plan);

    ManagedBrowserSession {
        worker_id: plan.worker_id.clone(),
        runtime_kind: ManagedBrowserRuntimeKind::AgentBrowser,
        placement: TilePlacement {
            worker_id: plan.worker_id.clone(),
            tile_position: plan.tile_position,
            window_origin_x: plan.window_origin_x,
            window_origin_y: plan.window_origin_y,
            viewport_width: plan.viewport_width,
            viewport_height: plan.viewport_height,
        },
        launch_plan: plan,
        launch_report: report,
    }
}

fn launch_report(plan: &ManagedBrowserLaunchPlan) -> ManagedBrowserLaunchReport {
    ManagedBrowserLaunchReport {
        worker_id: plan.worker_id.clone(),
        runtime_kind: ManagedBrowserRuntimeKind::AgentBrowser,
        backend: "agent_browser".into(),
        session_name: plan.session_name.clone(),
        browser_binary_path: plan.browser_binary_path.clone(),
        user_data_dir: plan.user_data_dir.clone(),
        viewport_width: plan.viewport_width,
        viewport_height: plan.viewport_height,
        window_origin_x: plan.window_origin_x,
        window_origin_y: plan.window_origin_y,
        locale: plan.locale.clone(),
        timezone: plan.timezone.clone(),
        user_agent: plan.user_agent.clone(),
        zoom_percent: plan.zoom_percent,
        actual_window_origin_x: plan.window_origin_x,
        actual_window_origin_y: plan.window_origin_y,
        actual_viewport_width: plan.viewport_width,
        actual_viewport_height: plan.viewport_height,
        display_scale_mode: plan.display_scale_mode.clone(),
    }
}

#[test]
fn wechat_selector_map_is_complete() {
    let driver = WeChatWebDriver::default();

    driver
        .validate_selectors()
        .expect("selectors should validate");
    assert_eq!(driver.platform_id(), "wechat_web");
    assert_eq!(driver.platform_name(), "WeChat Web");
    assert!(driver.selector_map().reply_input.contains("editArea"));
}

#[test]
fn whatsapp_selector_map_is_complete() {
    let driver = WhatsAppWebDriver::default();

    driver
        .validate_selectors()
        .expect("selectors should validate");
    assert_eq!(driver.platform_id(), "whatsapp_web");
    assert_eq!(driver.platform_name(), "WhatsApp Web");
    assert!(driver.selector_map().reply_input.contains("footer"));
}

#[tokio::test]
async fn wechat_driver_detects_login_required_from_qr_code() {
    let runtime = MockRuntime::default();
    let session = test_session();
    let driver = WeChatWebDriver::default();

    runtime.set_visible(&driver.selector_map().login_markers[0], true);

    let state = driver
        .detect_login_state(&runtime, &session)
        .await
        .expect("login state");

    assert_eq!(state, PlatformLoginState::LoginRequired);
}

#[tokio::test]
async fn wechat_driver_detects_challenge_markers() {
    let runtime = MockRuntime::default();
    let session = test_session();
    let driver = WeChatWebDriver::default();

    runtime.set_visible(&driver.selector_map().challenge_markers[1], true);

    let state = driver
        .detect_challenge_state(&runtime, &session)
        .await
        .expect("challenge state");

    assert_eq!(state, PlatformChallengeState::ChallengeRequired);
}

#[tokio::test]
async fn whatsapp_driver_detects_chat_open_workspace_state() {
    let runtime = MockRuntime::default();
    let session = test_session();
    let driver = WhatsAppWebDriver::default();

    runtime.set_visible(&driver.selector_map().conversation_list, true);
    runtime.set_visible(&driver.selector_map().message_list, true);
    runtime.set_visible(&driver.selector_map().reply_input, true);

    let state = driver
        .detect_workspace_state(&runtime, &session)
        .await
        .expect("workspace state");

    assert_eq!(state, PlatformWorkspaceState::ChatOpen);
}

#[tokio::test]
async fn wechat_driver_sends_reply_using_frozen_selectors() {
    let runtime = MockRuntime::default();
    let session = test_session();
    let driver = WeChatWebDriver::default();

    driver
        .send_reply(&runtime, &session, "hello from NeoHUman")
        .await
        .expect("reply should send");

    assert_eq!(
        runtime.actions(),
        vec![
            format!("click:{}", driver.selector_map().reply_input),
            format!(
                "type:{}:{}",
                driver.selector_map().reply_input,
                "hello from NeoHUman"
            ),
            format!("click:{}", driver.selector_map().send_button),
        ]
    );
}

#[tokio::test]
async fn wechat_driver_lists_visible_messages_with_direction() {
    let runtime = MockRuntime::default();
    let session = test_session();
    let driver = WeChatWebDriver::default();

    runtime.set_text(&driver.selector_map().incoming_message, "hello\nneed help");
    runtime.set_text(&driver.selector_map().outgoing_message, "on it");

    let messages = driver
        .list_visible_messages(&runtime, &session)
        .await
        .expect("messages");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].direction, MessageDirection::Inbound);
    assert_eq!(messages[2].direction, MessageDirection::Outbound);
}

#[test]
fn wechat_driver_generates_stable_ids_when_dom_id_is_missing() {
    let driver = WeChatWebDriver::default();
    let node = PlatformMessageNode {
        dom_id: None,
        text: "same message".into(),
        direction: MessageDirection::Inbound,
    };

    let first = driver.extract_message_id(&node);
    let second = driver.extract_message_id(&node);

    assert_eq!(first, second);
    assert!(!first.is_empty());
}
