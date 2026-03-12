use crate::appliance::{
    browser_runtime::{ManagedBrowserRuntime, ManagedBrowserSession},
    platforms::{
        MessageDirection, MessagingPlatformDriver, PlatformChallengeState, PlatformLoginState,
        PlatformMessageNode, PlatformSelectorMap,
    },
};
use crate::tools::ToolResult;
use anyhow::{bail, Result};
use async_trait::async_trait;

const WECHAT_URL: &str = "https://web.wechat.com";

#[derive(Debug, Clone)]
pub struct WeChatWebDriver {
    selectors: PlatformSelectorMap,
}

impl Default for WeChatWebDriver {
    fn default() -> Self {
        Self {
            selectors: PlatformSelectorMap {
                conversation_list: ".chat_list, .ng-chat-list, [class*='chat_list']".into(),
                conversation_item: ".chat_item, .ng-chat-item".into(),
                message_list: ".box_chat, .message-container, [class*='message']".into(),
                incoming_message:
                    ".message:not(.me):not(.self), .msg:not(.me):not(.self), .bubble:not(.me):not(.self)".into(),
                outgoing_message:
                    ".message.me, .message.self, .msg.me, .msg.self, .bubble.me, .bubble.self"
                        .into(),
                reply_input:
                    "#editArea, .input-wrapper textarea, [contenteditable='true']".into(),
                send_button: ".btn_send, .send-btn, [title='send']".into(),
                login_markers: vec![
                    "img.qrcode, .qrcode img, [class*='qrcode']".into(),
                    ".chat_list, .ng-chat-list, [class*='chat_list']".into(),
                ],
                challenge_markers: vec![
                    ".association".into(),
                    ".qrcode-msg".into(),
                    ".login__desc".into(),
                    ".dialog_ft".into(),
                ],
            },
        }
    }
}

#[async_trait]
impl MessagingPlatformDriver for WeChatWebDriver {
    fn platform_id(&self) -> &'static str {
        "wechat_web"
    }

    fn platform_name(&self) -> &'static str {
        "WeChat Web"
    }

    fn workspace_url(&self) -> &'static str {
        WECHAT_URL
    }

    fn selector_map(&self) -> &PlatformSelectorMap {
        &self.selectors
    }

    async fn open_workspace(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()> {
        let result = runtime.open_url(session, WECHAT_URL).await?;
        ensure_tool_success("open_workspace", result)
    }

    async fn detect_login_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformLoginState> {
        if selector_is_visible(runtime, session, &self.selectors.conversation_list).await? {
            return Ok(PlatformLoginState::LoggedIn);
        }

        if selector_is_visible(runtime, session, &self.selectors.login_markers[0]).await? {
            return Ok(PlatformLoginState::LoginRequired);
        }

        Ok(PlatformLoginState::Unknown)
    }

    async fn detect_challenge_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformChallengeState> {
        for selector in &self.selectors.challenge_markers {
            if selector_is_visible(runtime, session, selector).await? {
                return Ok(PlatformChallengeState::ChallengeRequired);
            }
        }

        Ok(PlatformChallengeState::Clear)
    }

    async fn list_visible_messages(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<Vec<PlatformMessageNode>> {
        let mut messages = Vec::new();
        let inbound = runtime
            .get_text(session, &self.selectors.incoming_message)
            .await?;
        let outbound = runtime
            .get_text(session, &self.selectors.outgoing_message)
            .await?;

        messages.extend(parse_text_block(&inbound.output, MessageDirection::Inbound));
        messages.extend(parse_text_block(
            &outbound.output,
            MessageDirection::Outbound,
        ));

        Ok(messages)
    }

    async fn focus_reply_box(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()> {
        let result = runtime.click(session, &self.selectors.reply_input).await?;
        ensure_tool_success("focus_reply_box", result)
    }

    async fn send_reply(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
        reply: &str,
    ) -> Result<()> {
        self.focus_reply_box(runtime, session).await?;

        let type_result = runtime
            .type_text(session, &self.selectors.reply_input, reply)
            .await?;
        ensure_tool_success("type_reply", type_result)?;

        let send_result = runtime.click(session, &self.selectors.send_button).await?;
        ensure_tool_success("send_reply", send_result)
    }
}

fn parse_text_block(text: &str, direction: MessageDirection) -> Vec<PlatformMessageNode> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| PlatformMessageNode {
            dom_id: None,
            text: line.to_string(),
            direction,
        })
        .collect()
}

fn ensure_tool_success(action: &str, result: ToolResult) -> Result<()> {
    if result.success {
        return Ok(());
    }

    bail!(
        "{action} failed: {}",
        result
            .error
            .unwrap_or_else(|| "browser runtime returned an unsuccessful result".into())
    )
}

async fn selector_is_visible(
    runtime: &dyn ManagedBrowserRuntime,
    session: &ManagedBrowserSession,
    selector: &str,
) -> Result<bool> {
    let result = runtime.is_visible(session, selector).await?;
    parse_visibility_result(result)
}

fn parse_visibility_result(result: ToolResult) -> Result<bool> {
    if !result.success {
        return Ok(false);
    }

    let output = result.output.trim();
    if output.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if output.eq_ignore_ascii_case("false") || output.is_empty() {
        return Ok(false);
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(value) = parsed.as_bool() {
            return Ok(value);
        }
        if let Some(value) = parsed.get("visible").and_then(serde_json::Value::as_bool) {
            return Ok(value);
        }
        if let Some(value) = parsed.get("data").and_then(serde_json::Value::as_bool) {
            return Ok(value);
        }
    }

    Ok(false)
}
