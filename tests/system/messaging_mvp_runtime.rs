use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use zeroclaw::{
    appliance::{
        browser_runtime::{
            build_launch_plan, ManagedBrowserLaunchPlan, ManagedBrowserLaunchReport,
            ManagedBrowserRuntime, ManagedBrowserRuntimeKind, ManagedBrowserSession,
        },
        initialize_station_with, mark_challenge_complete, mark_login_complete, pause_worker,
        platforms::{
            MessageDirection, MessagingPlatformDriver, PlatformChallengeState, PlatformLoginState,
            PlatformMessageNode, PlatformSelectorMap,
        },
        resume_worker,
        session_store::SessionStore,
        state::{
            StationSupervisorState, WorkerAttentionReason, WorkerRuntimeState, WorkerSessionStatus,
        },
        tile_manager::{TileManager, TilePlacement},
        StationRuntimeDependencies,
    },
    config::Config,
    tools::ToolResult,
};

fn temp_station_config() -> (TempDir, Config) {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut config = Config::default();
    config.station.enabled = true;
    config.workspace_dir = temp_dir.path().join("workspace");
    config.config_path = temp_dir.path().join("config.toml");
    (temp_dir, config)
}

#[derive(Default)]
struct MockRuntime;

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
            launch_report: ManagedBrowserLaunchReport {
                worker_id: plan.worker_id.clone(),
                runtime_kind: self.kind(),
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
            },
        })
    }

    async fn connect(&self, _session: &ManagedBrowserSession) -> Result<()> {
        Ok(())
    }

    async fn open_url(&self, _session: &ManagedBrowserSession, _url: &str) -> Result<ToolResult> {
        Ok(ok_tool("opened"))
    }

    async fn snapshot(&self, _session: &ManagedBrowserSession) -> Result<ToolResult> {
        Ok(ok_tool("{}"))
    }

    async fn click(&self, _session: &ManagedBrowserSession, _selector: &str) -> Result<ToolResult> {
        Ok(ok_tool("clicked"))
    }

    async fn fill(
        &self,
        _session: &ManagedBrowserSession,
        _selector: &str,
        _value: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool("filled"))
    }

    async fn type_text(
        &self,
        _session: &ManagedBrowserSession,
        _selector: &str,
        _text: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool("typed"))
    }

    async fn get_text(
        &self,
        _session: &ManagedBrowserSession,
        _selector: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool(""))
    }

    async fn is_visible(
        &self,
        _session: &ManagedBrowserSession,
        _selector: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool("false"))
    }

    async fn screenshot(
        &self,
        _session: &ManagedBrowserSession,
        _path: &str,
    ) -> Result<ToolResult> {
        Ok(ok_tool("screenshot"))
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

#[derive(Default)]
struct MockPlatformDriver {
    login_states: Mutex<HashMap<String, PlatformLoginState>>,
    challenge_states: Mutex<HashMap<String, PlatformChallengeState>>,
}

impl MockPlatformDriver {
    fn set_login_state(&self, worker_id: &str, state: PlatformLoginState) {
        self.login_states
            .lock()
            .insert(worker_id.to_string(), state);
    }

    fn set_challenge_state(&self, worker_id: &str, state: PlatformChallengeState) {
        self.challenge_states
            .lock()
            .insert(worker_id.to_string(), state);
    }
}

#[async_trait]
impl MessagingPlatformDriver for MockPlatformDriver {
    fn platform_id(&self) -> &'static str {
        "wechat_web"
    }

    fn platform_name(&self) -> &'static str {
        "WeChat Web"
    }

    fn workspace_url(&self) -> &'static str {
        "https://web.wechat.com"
    }

    fn selector_map(&self) -> &PlatformSelectorMap {
        static SELECTORS: std::sync::OnceLock<PlatformSelectorMap> = std::sync::OnceLock::new();
        SELECTORS.get_or_init(|| PlatformSelectorMap {
            conversation_list: ".chat_list".into(),
            conversation_item: ".chat_item".into(),
            message_list: ".box_chat".into(),
            incoming_message: ".message".into(),
            outgoing_message: ".message.me".into(),
            reply_input: "#editArea".into(),
            send_button: ".btn_send".into(),
            login_markers: vec!["img.qrcode".into()],
            challenge_markers: vec![".dialog_ft".into()],
        })
    }

    async fn open_workspace(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()> {
        runtime.open_url(session, self.workspace_url()).await?;
        Ok(())
    }

    async fn detect_login_state(
        &self,
        _runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformLoginState> {
        Ok(self
            .login_states
            .lock()
            .get(&session.worker_id)
            .copied()
            .unwrap_or(PlatformLoginState::LoginRequired))
    }

    async fn detect_challenge_state(
        &self,
        _runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformChallengeState> {
        Ok(self
            .challenge_states
            .lock()
            .get(&session.worker_id)
            .copied()
            .unwrap_or(PlatformChallengeState::Clear))
    }

    async fn list_visible_messages(
        &self,
        _runtime: &dyn ManagedBrowserRuntime,
        _session: &ManagedBrowserSession,
    ) -> Result<Vec<PlatformMessageNode>> {
        Ok(vec![PlatformMessageNode {
            dom_id: Some("bootstrapped".into()),
            text: "hello".into(),
            direction: MessageDirection::Inbound,
        }])
    }

    async fn focus_reply_box(
        &self,
        _runtime: &dyn ManagedBrowserRuntime,
        _session: &ManagedBrowserSession,
    ) -> Result<()> {
        Ok(())
    }

    async fn send_reply(
        &self,
        _runtime: &dyn ManagedBrowserRuntime,
        _session: &ManagedBrowserSession,
        _reply: &str,
    ) -> Result<()> {
        Ok(())
    }
}

fn ok_tool(output: &str) -> ToolResult {
    ToolResult {
        success: true,
        output: output.to_string(),
        error: None,
    }
}

fn deps(driver: Arc<MockPlatformDriver>) -> StationRuntimeDependencies {
    StationRuntimeDependencies {
        runtime: Arc::new(MockRuntime),
        platform_driver: driver,
    }
}

#[tokio::test]
async fn station_runtime_marks_fresh_workers_login_required() {
    let (_temp_dir, config) = temp_station_config();
    let driver = Arc::new(MockPlatformDriver::default());

    let report = initialize_station_with(&config, deps(driver))
        .await
        .expect("station runtime should initialize");

    assert_eq!(report.sessions.len(), 2);
    assert_eq!(report.platform_name, "WeChat Web");
    assert_eq!(report.supervisor.state(), StationSupervisorState::Degraded);
    assert_eq!(report.sessions[0].launch_plan.window_origin_x, 0);
    assert_eq!(
        report.sessions[1].launch_plan.window_origin_x,
        report.sessions[0].launch_plan.viewport_width as i32
    );

    for worker in report.supervisor.workers() {
        assert_eq!(worker.state, WorkerRuntimeState::LoginRequired);
        assert_eq!(
            worker.attention_reason,
            Some(WorkerAttentionReason::SessionInvalid)
        );
    }

    let store = SessionStore::from_config(&config);
    assert!(
        tokio::fs::try_exists(store.worker_session_metadata_path("worker_a"))
            .await
            .expect("worker a metadata exists")
    );
    assert!(
        tokio::fs::try_exists(store.worker_session_metadata_path("worker_b"))
            .await
            .expect("worker b metadata exists")
    );
}

#[tokio::test]
async fn station_runtime_restores_ready_workers_from_live_logged_in_sessions() {
    let (_temp_dir, config) = temp_station_config();
    let driver = Arc::new(MockPlatformDriver::default());
    driver.set_login_state("worker_a", PlatformLoginState::LoggedIn);
    driver.set_login_state("worker_b", PlatformLoginState::LoggedIn);

    let report = initialize_station_with(&config, deps(driver))
        .await
        .expect("station runtime should initialize");

    assert_eq!(report.platform_name, "WeChat Web");
    assert_eq!(report.supervisor.state(), StationSupervisorState::Ready);
    assert!(report
        .supervisor
        .workers()
        .iter()
        .all(|worker| worker.state == WorkerRuntimeState::Ready));
}

#[tokio::test]
async fn station_runtime_surfaces_challenge_required_per_worker() {
    let (_temp_dir, config) = temp_station_config();
    let driver = Arc::new(MockPlatformDriver::default());
    driver.set_login_state("worker_a", PlatformLoginState::LoggedIn);
    driver.set_login_state("worker_b", PlatformLoginState::LoggedIn);
    driver.set_challenge_state("worker_b", PlatformChallengeState::ChallengeRequired);

    let report = initialize_station_with(&config, deps(driver))
        .await
        .expect("station runtime should initialize");

    assert_eq!(report.platform_name, "WeChat Web");
    assert_eq!(report.supervisor.state(), StationSupervisorState::Degraded);
    assert_eq!(
        report
            .supervisor
            .worker("worker_a")
            .expect("worker a")
            .state,
        WorkerRuntimeState::Ready
    );
    let worker_b_runtime = report.supervisor.worker("worker_b").expect("worker b");
    assert_eq!(
        worker_b_runtime.state,
        WorkerRuntimeState::ChallengeRequired
    );
    assert_eq!(
        worker_b_runtime.attention_reason,
        Some(WorkerAttentionReason::ChallengeDetected)
    );
}

#[tokio::test]
async fn station_runtime_writes_launch_metadata_for_each_worker() {
    let (_temp_dir, config) = temp_station_config();
    let driver = Arc::new(MockPlatformDriver::default());
    driver.set_login_state("worker_a", PlatformLoginState::LoggedIn);
    driver.set_login_state("worker_b", PlatformLoginState::LoggedIn);

    let report = initialize_station_with(&config, deps(driver))
        .await
        .expect("station runtime should initialize");

    let tile_manager = TileManager::from_station_config(&config.station).expect("tile manager");
    for worker in &config.station.workers {
        let plan = build_launch_plan(&config, &tile_manager, worker).expect("launch plan");
        let launch_metadata = plan.user_data_dir.join("launch.json");
        assert!(
            tokio::fs::try_exists(&launch_metadata)
                .await
                .expect("launch metadata exists"),
            "missing launch metadata for {}",
            worker.id
        );
    }
    assert_eq!(report.sessions.len(), 2);
}

#[tokio::test]
async fn station_worker_pause_and_resume_are_persisted() {
    let (_temp_dir, config) = temp_station_config();
    pause_worker(&config, "worker_a")
        .await
        .expect("pause worker");

    let store = SessionStore::from_config(&config);
    let paused = store
        .load_worker_session("worker_a")
        .await
        .expect("load worker")
        .into_loaded()
        .expect("paused metadata");
    assert!(paused.paused);

    resume_worker(&config, "worker_a")
        .await
        .expect("resume worker");
    let resumed = store
        .load_worker_session("worker_a")
        .await
        .expect("load worker")
        .into_loaded()
        .expect("resumed metadata");
    assert!(!resumed.paused);
}

#[tokio::test]
async fn station_worker_manual_completion_marks_session_active() {
    let (_temp_dir, config) = temp_station_config();
    pause_worker(&config, "worker_b")
        .await
        .expect("seed metadata");
    mark_login_complete(&config, "worker_b")
        .await
        .expect("mark login complete");

    let store = SessionStore::from_config(&config);
    let active = store
        .load_worker_session("worker_b")
        .await
        .expect("load worker")
        .into_loaded()
        .expect("active metadata");
    assert_eq!(active.status, WorkerSessionStatus::Active);
    assert!(!active.paused);

    mark_challenge_complete(&config, "worker_b")
        .await
        .expect("mark challenge complete");
    let active_again = store
        .load_worker_session("worker_b")
        .await
        .expect("reload worker")
        .into_loaded()
        .expect("active metadata");
    assert_eq!(active_again.status, WorkerSessionStatus::Active);
}

trait SessionLoadExt {
    fn into_loaded(self) -> Option<zeroclaw::appliance::session_store::PersistedWorkerSession>;
}

impl SessionLoadExt for zeroclaw::appliance::session_store::SessionStoreLoadResult {
    fn into_loaded(self) -> Option<zeroclaw::appliance::session_store::PersistedWorkerSession> {
        match self {
            zeroclaw::appliance::session_store::SessionStoreLoadResult::Loaded(metadata) => {
                Some(metadata)
            }
            _ => None,
        }
    }
}
