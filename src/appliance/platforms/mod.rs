use crate::appliance::browser_runtime::{ManagedBrowserRuntime, ManagedBrowserSession};
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub mod wechat;

pub use wechat::WeChatWebDriver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformLoginState {
    LoggedIn,
    LoginRequired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformChallengeState {
    Clear,
    ChallengeRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformMessageNode {
    pub dom_id: Option<String>,
    pub text: String,
    pub direction: MessageDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformSelectorMap {
    pub conversation_list: String,
    pub conversation_item: String,
    pub message_list: String,
    pub incoming_message: String,
    pub outgoing_message: String,
    pub reply_input: String,
    pub send_button: String,
    pub login_markers: Vec<String>,
    pub challenge_markers: Vec<String>,
}

impl PlatformSelectorMap {
    pub fn validate(&self) -> Result<()> {
        for (name, selector) in [
            ("conversation_list", &self.conversation_list),
            ("conversation_item", &self.conversation_item),
            ("message_list", &self.message_list),
            ("incoming_message", &self.incoming_message),
            ("outgoing_message", &self.outgoing_message),
            ("reply_input", &self.reply_input),
            ("send_button", &self.send_button),
        ] {
            if selector.trim().is_empty() {
                bail!("platform selector `{name}` must not be empty");
            }
        }

        if self.login_markers.is_empty() {
            bail!("platform selector map must include at least one login marker");
        }
        if self
            .login_markers
            .iter()
            .any(|selector| selector.trim().is_empty())
        {
            bail!("platform login markers must not contain empty selectors");
        }
        if self.challenge_markers.is_empty() {
            bail!("platform selector map must include at least one challenge marker");
        }
        if self
            .challenge_markers
            .iter()
            .any(|selector| selector.trim().is_empty())
        {
            bail!("platform challenge markers must not contain empty selectors");
        }

        Ok(())
    }
}

#[async_trait]
pub trait MessagingPlatformDriver: Send + Sync {
    fn platform_id(&self) -> &'static str;

    fn platform_name(&self) -> &'static str;

    fn workspace_url(&self) -> &'static str;

    fn allowed_domains(&self) -> Vec<String> {
        reqwest::Url::parse(self.workspace_url())
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .into_iter()
            .collect()
    }

    fn selector_map(&self) -> &PlatformSelectorMap;

    fn validate_selectors(&self) -> Result<()> {
        self.selector_map().validate()
    }

    async fn open_workspace(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()>;

    async fn detect_login_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformLoginState>;

    async fn detect_challenge_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformChallengeState>;

    async fn list_visible_messages(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<Vec<PlatformMessageNode>>;

    fn extract_message_id(&self, node: &PlatformMessageNode) -> String {
        if let Some(dom_id) = &node.dom_id {
            return dom_id.clone();
        }

        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}:{}", node.direction, node.text).as_bytes());
        let digest = hasher.finalize();
        hex::encode(&digest[..8])
    }

    fn extract_message_text(&self, node: &PlatformMessageNode) -> String {
        node.text.trim().to_string()
    }

    fn extract_message_direction(&self, node: &PlatformMessageNode) -> MessageDirection {
        node.direction
    }

    async fn focus_reply_box(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()>;

    async fn send_reply(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
        reply: &str,
    ) -> Result<()>;
}

pub fn mvp_platform_driver() -> Arc<dyn MessagingPlatformDriver> {
    Arc::new(WeChatWebDriver::default())
}
