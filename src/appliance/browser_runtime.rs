use crate::appliance::{
    platforms::MessagingPlatformDriver,
    tile_manager::{TileManager, TilePlacement},
};
use crate::config::{Config, StationTilePosition, StationWorkerConfig};
use crate::security::SecurityPolicy;
use crate::tools::{
    browser::{BrowserAction, BrowserTool, ManagedBrowserLaunchOptions},
    ComputerUseConfig, ToolResult,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

const LAUNCH_METADATA_FILE: &str = "launch.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedBrowserRuntimeKind {
    AgentBrowser,
    RustNative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedBrowserLaunchPlan {
    pub worker_id: String,
    pub display_name: String,
    pub tile_position: StationTilePosition,
    pub window_origin_x: i32,
    pub window_origin_y: i32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub zoom_percent: u16,
    pub locale: String,
    pub timezone: String,
    pub user_agent: String,
    pub browser_binary_path: Option<String>,
    pub headless: bool,
    pub user_data_dir: PathBuf,
    pub session_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedBrowserLaunchReport {
    pub worker_id: String,
    pub runtime_kind: ManagedBrowserRuntimeKind,
    pub backend: String,
    pub session_name: String,
    pub browser_binary_path: Option<String>,
    pub user_data_dir: PathBuf,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub window_origin_x: i32,
    pub window_origin_y: i32,
    pub locale: String,
    pub timezone: String,
    pub user_agent: String,
    pub zoom_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedBrowserSession {
    pub worker_id: String,
    pub runtime_kind: ManagedBrowserRuntimeKind,
    pub placement: TilePlacement,
    pub launch_plan: ManagedBrowserLaunchPlan,
    pub launch_report: ManagedBrowserLaunchReport,
}

#[async_trait]
pub trait ManagedBrowserRuntime: Send + Sync {
    fn kind(&self) -> ManagedBrowserRuntimeKind;

    async fn launch(&self, plan: &ManagedBrowserLaunchPlan) -> Result<ManagedBrowserSession>;

    async fn connect(&self, session: &ManagedBrowserSession) -> Result<()>;

    async fn open_url(&self, session: &ManagedBrowserSession, url: &str) -> Result<ToolResult>;

    async fn snapshot(&self, session: &ManagedBrowserSession) -> Result<ToolResult>;

    async fn click(&self, session: &ManagedBrowserSession, selector: &str) -> Result<ToolResult>;

    async fn fill(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
        value: &str,
    ) -> Result<ToolResult>;

    async fn type_text(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
        text: &str,
    ) -> Result<ToolResult>;

    async fn get_text(&self, session: &ManagedBrowserSession, selector: &str)
        -> Result<ToolResult>;

    async fn is_visible(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
    ) -> Result<ToolResult>;

    async fn screenshot(&self, session: &ManagedBrowserSession, path: &str) -> Result<ToolResult>;

    async fn move_to_tile(
        &self,
        session: &ManagedBrowserSession,
        placement: &TilePlacement,
    ) -> Result<ManagedBrowserSession>;

    async fn close(&self, session: &ManagedBrowserSession) -> Result<ToolResult>;
}

#[derive(Clone)]
pub struct BrowserToolRuntime {
    kind: ManagedBrowserRuntimeKind,
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    native_webdriver_url: String,
    native_chrome_path: Option<String>,
    computer_use: ComputerUseConfig,
    sessions: Arc<RwLock<HashMap<String, Arc<BrowserTool>>>>,
}

impl BrowserToolRuntime {
    pub fn from_config(
        config: &Config,
        kind: ManagedBrowserRuntimeKind,
        allowed_domains: Vec<String>,
    ) -> Self {
        Self {
            kind,
            security: Arc::new(SecurityPolicy::from_config(
                &config.autonomy,
                &config.workspace_dir,
            )),
            allowed_domains,
            native_webdriver_url: config.browser.native_webdriver_url.clone(),
            native_chrome_path: config.browser.native_chrome_path.clone(),
            computer_use: ComputerUseConfig {
                endpoint: config.browser.computer_use.endpoint.clone(),
                api_key: config.browser.computer_use.api_key.clone(),
                timeout_ms: config.browser.computer_use.timeout_ms,
                allow_remote_endpoint: config.browser.computer_use.allow_remote_endpoint,
                window_allowlist: config.browser.computer_use.window_allowlist.clone(),
                max_coordinate_x: config.browser.computer_use.max_coordinate_x,
                max_coordinate_y: config.browser.computer_use.max_coordinate_y,
            },
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn tool_for_plan(&self, plan: &ManagedBrowserLaunchPlan) -> Arc<BrowserTool> {
        let backend = match self.kind {
            ManagedBrowserRuntimeKind::AgentBrowser => "agent_browser",
            ManagedBrowserRuntimeKind::RustNative => "rust_native",
        };

        Arc::new(BrowserTool::new_with_backend_and_launch_options(
            Arc::clone(&self.security),
            self.allowed_domains.clone(),
            Some(plan.session_name.clone()),
            Some(ManagedBrowserLaunchOptions {
                session_name: plan.session_name.clone(),
                headless: plan.headless,
                browser_binary_path: plan
                    .browser_binary_path
                    .clone()
                    .or_else(|| self.native_chrome_path.clone()),
                user_data_dir: plan.user_data_dir.clone(),
                locale: plan.locale.clone(),
                timezone: plan.timezone.clone(),
                user_agent: plan.user_agent.clone(),
                viewport_width: plan.viewport_width,
                viewport_height: plan.viewport_height,
                window_origin_x: plan.window_origin_x,
                window_origin_y: plan.window_origin_y,
                zoom_percent: plan.zoom_percent,
            }),
            backend.to_string(),
            plan.headless,
            self.native_webdriver_url.clone(),
            plan.browser_binary_path
                .clone()
                .or_else(|| self.native_chrome_path.clone()),
            self.computer_use.clone(),
        ))
    }

    fn session_tool(&self, session: &ManagedBrowserSession) -> Result<Arc<BrowserTool>> {
        self.sessions
            .read()
            .get(&session.worker_id)
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "managed browser session not found for {}",
                    session.worker_id
                )
            })
    }

    async fn persist_launch_report(&self, session: &ManagedBrowserSession) -> Result<()> {
        fs::create_dir_all(&session.launch_plan.user_data_dir)
            .await
            .with_context(|| {
                format!(
                    "create user data dir {}",
                    session.launch_plan.user_data_dir.display()
                )
            })?;

        let launch_metadata_path = session.launch_plan.user_data_dir.join(LAUNCH_METADATA_FILE);
        let payload =
            serde_json::to_vec_pretty(&session.launch_report).context("serialize launch report")?;
        fs::write(&launch_metadata_path, payload)
            .await
            .with_context(|| format!("write launch report to {}", launch_metadata_path.display()))
    }

    fn build_launch_report(&self, plan: &ManagedBrowserLaunchPlan) -> ManagedBrowserLaunchReport {
        ManagedBrowserLaunchReport {
            worker_id: plan.worker_id.clone(),
            runtime_kind: self.kind,
            backend: match self.kind {
                ManagedBrowserRuntimeKind::AgentBrowser => "agent_browser".to_string(),
                ManagedBrowserRuntimeKind::RustNative => "rust_native".to_string(),
            },
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
        }
    }
}

#[async_trait]
impl ManagedBrowserRuntime for BrowserToolRuntime {
    fn kind(&self) -> ManagedBrowserRuntimeKind {
        self.kind
    }

    async fn launch(&self, plan: &ManagedBrowserLaunchPlan) -> Result<ManagedBrowserSession> {
        fs::create_dir_all(&plan.user_data_dir)
            .await
            .with_context(|| {
                format!(
                    "create managed browser profile {}",
                    plan.user_data_dir.display()
                )
            })?;

        let tool = self.tool_for_plan(plan);
        tool.launch_managed_session().await?;
        self.sessions
            .write()
            .insert(plan.worker_id.clone(), Arc::clone(&tool));

        let session = ManagedBrowserSession {
            worker_id: plan.worker_id.clone(),
            runtime_kind: self.kind,
            placement: TilePlacement {
                worker_id: plan.worker_id.clone(),
                tile_position: plan.tile_position,
                window_origin_x: plan.window_origin_x,
                window_origin_y: plan.window_origin_y,
                viewport_width: plan.viewport_width,
                viewport_height: plan.viewport_height,
            },
            launch_plan: plan.clone(),
            launch_report: self.build_launch_report(plan),
        };
        self.persist_launch_report(&session).await?;
        Ok(session)
    }

    async fn connect(&self, session: &ManagedBrowserSession) -> Result<()> {
        self.session_tool(session)?.launch_managed_session().await
    }

    async fn open_url(&self, session: &ManagedBrowserSession, url: &str) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Open {
                url: url.to_string(),
            })
            .await
    }

    async fn snapshot(&self, session: &ManagedBrowserSession) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Snapshot {
                interactive_only: true,
                compact: true,
                depth: None,
            })
            .await
    }

    async fn click(&self, session: &ManagedBrowserSession, selector: &str) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Click {
                selector: selector.to_string(),
            })
            .await
    }

    async fn fill(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
        value: &str,
    ) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Fill {
                selector: selector.to_string(),
                value: value.to_string(),
            })
            .await
    }

    async fn type_text(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
        text: &str,
    ) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Type {
                selector: selector.to_string(),
                text: text.to_string(),
            })
            .await
    }

    async fn get_text(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
    ) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::GetText {
                selector: selector.to_string(),
            })
            .await
    }

    async fn is_visible(
        &self,
        session: &ManagedBrowserSession,
        selector: &str,
    ) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::IsVisible {
                selector: selector.to_string(),
            })
            .await
    }

    async fn screenshot(&self, session: &ManagedBrowserSession, path: &str) -> Result<ToolResult> {
        self.session_tool(session)?
            .execute_structured_action(BrowserAction::Screenshot {
                path: Some(path.to_string()),
                full_page: false,
            })
            .await
    }

    async fn move_to_tile(
        &self,
        session: &ManagedBrowserSession,
        placement: &TilePlacement,
    ) -> Result<ManagedBrowserSession> {
        let mut updated = session.clone();
        updated.placement = placement.clone();
        updated.launch_report.window_origin_x = placement.window_origin_x;
        updated.launch_report.window_origin_y = placement.window_origin_y;
        updated.launch_report.viewport_width = placement.viewport_width;
        updated.launch_report.viewport_height = placement.viewport_height;
        self.persist_launch_report(&updated).await?;
        Ok(updated)
    }

    async fn close(&self, session: &ManagedBrowserSession) -> Result<ToolResult> {
        let tool = self.session_tool(session)?;
        let result = tool.execute_structured_action(BrowserAction::Close).await?;
        self.sessions.write().remove(&session.worker_id);
        Ok(result)
    }
}

pub fn runtime_kind_for_config(config: &Config) -> ManagedBrowserRuntimeKind {
    if config.browser.backend.eq_ignore_ascii_case("rust_native") {
        ManagedBrowserRuntimeKind::RustNative
    } else {
        ManagedBrowserRuntimeKind::AgentBrowser
    }
}

pub fn allowed_domains_for_station(
    config: &Config,
    platform_driver: &dyn MessagingPlatformDriver,
) -> Vec<String> {
    if !config.browser.allowed_domains.is_empty() {
        return config.browser.allowed_domains.clone();
    }

    platform_driver.allowed_domains()
}

pub fn runtime_for_config(
    config: &Config,
    platform_driver: &dyn MessagingPlatformDriver,
) -> Arc<dyn ManagedBrowserRuntime> {
    Arc::new(BrowserToolRuntime::from_config(
        config,
        runtime_kind_for_config(config),
        allowed_domains_for_station(config, platform_driver),
    ))
}

pub fn build_launch_plan(
    config: &Config,
    tile_manager: &TileManager,
    worker: &StationWorkerConfig,
) -> Result<ManagedBrowserLaunchPlan> {
    let placement = tile_manager.build_expected_placement(worker)?;
    let managed = &worker.managed_browser;

    Ok(ManagedBrowserLaunchPlan {
        worker_id: worker.id.clone(),
        display_name: worker.display_name.clone(),
        tile_position: worker.tile_position,
        window_origin_x: placement.window_origin_x,
        window_origin_y: placement.window_origin_y,
        viewport_width: placement.viewport_width,
        viewport_height: placement.viewport_height,
        zoom_percent: managed.zoom_percent,
        locale: managed.locale.clone(),
        timezone: managed.timezone.clone(),
        user_agent: managed.user_agent.clone(),
        browser_binary_path: managed.browser_binary_path.clone(),
        headless: managed.headless,
        user_data_dir: config
            .workspace_dir
            .join("station")
            .join("profiles")
            .join(&worker.profile_name),
        session_name: worker.profile_name.clone(),
    })
}
