use crate::appliance::{
    browser_runtime::{ManagedBrowserRuntime, ManagedBrowserSession},
    platforms::{
        selector_is_visible, MessageDirection, MessagingPlatformDriver, PlatformChallengeState,
        PlatformLoginState, PlatformMessageNode, PlatformSelectorMap, PlatformWorkspaceState,
    },
};
use anyhow::{bail, Result};
use async_trait::async_trait;

const WHATSAPP_WEB_URL: &str = "https://web.whatsapp.com";

#[derive(Debug, Clone)]
pub struct WhatsAppWebDriver {
    selectors: PlatformSelectorMap,
}

impl Default for WhatsAppWebDriver {
    fn default() -> Self {
        Self {
            selectors: PlatformSelectorMap {
                conversation_list: "#pane-side, [data-testid='chat-list']".into(),
                conversation_item:
                    "#pane-side [role='listitem'], #pane-side [data-testid='cell-frame-container']"
                        .into(),
                search_input:
                    "[data-testid='chat-list-search'] div[contenteditable='true'], [data-testid='chat-list-search'] input, #side div[contenteditable='true'][data-tab='3']".into(),
                active_chat_header:
                    "#main header, header [data-testid='conversation-info-header']".into(),
                message_list:
                    "#main [data-testid='conversation-panel-messages'], #main .copyable-area"
                        .into(),
                incoming_message: "#main .message-in".into(),
                outgoing_message: "#main .message-out".into(),
                reply_input:
                    "footer [data-testid='conversation-compose-box-input'], footer div[contenteditable='true'][data-tab], footer div[role='textbox']".into(),
                send_button:
                    "footer button[aria-label='Send'], footer span[data-icon='send']".into(),
                login_markers: vec![
                    "[data-testid='qrcode']".into(),
                    "canvas[aria-label*='QR']".into(),
                    "div[data-ref] canvas".into(),
                ],
                challenge_markers: vec![
                    "[data-testid='popup-controls-ok']".into(),
                    "[data-testid='reconnect-button']".into(),
                    "[data-testid='alert-phone']".into(),
                ],
                modal_markers: vec![
                    "[role='dialog']".into(),
                    "[data-testid='popup-panel']".into(),
                    "[data-animate-modal-popup='true']".into(),
                ],
                overlay_markers: vec![
                    "[data-testid='popup-panel']".into(),
                    "[data-animate-modal-popup='true']".into(),
                    "[data-testid='drawer-backdrop']".into(),
                ],
            },
        }
    }
}

#[async_trait]
impl MessagingPlatformDriver for WhatsAppWebDriver {
    fn platform_id(&self) -> &'static str {
        "whatsapp_web"
    }

    fn platform_name(&self) -> &'static str {
        "WhatsApp Web"
    }

    fn workspace_url(&self) -> &'static str {
        WHATSAPP_WEB_URL
    }

    fn selector_map(&self) -> &PlatformSelectorMap {
        &self.selectors
    }

    async fn open_workspace(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<()> {
        let result = runtime.open_url(session, WHATSAPP_WEB_URL).await?;
        ensure_tool_success("open_workspace", result)
    }

    async fn detect_login_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformLoginState> {
        if selector_is_visible(runtime, session, &self.selectors.conversation_list).await?
            || selector_is_visible(runtime, session, &self.selectors.message_list).await?
        {
            return Ok(PlatformLoginState::LoggedIn);
        }

        for selector in &self.selectors.login_markers {
            if selector_is_visible(runtime, session, selector).await? {
                return Ok(PlatformLoginState::LoginRequired);
            }
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

    async fn detect_workspace_state(
        &self,
        runtime: &dyn ManagedBrowserRuntime,
        session: &ManagedBrowserSession,
    ) -> Result<PlatformWorkspaceState> {
        if self.detect_login_state(runtime, session).await? != PlatformLoginState::LoggedIn {
            return Ok(PlatformWorkspaceState::LoginRequired);
        }

        if self.detect_challenge_state(runtime, session).await?
            == PlatformChallengeState::ChallengeRequired
        {
            return Ok(PlatformWorkspaceState::ModalOpen);
        }

        for selector in &self.selectors.modal_markers {
            if selector_is_visible(runtime, session, selector).await? {
                return Ok(PlatformWorkspaceState::ModalOpen);
            }
        }

        for selector in &self.selectors.overlay_markers {
            if selector_is_visible(runtime, session, selector).await? {
                return Ok(PlatformWorkspaceState::UnexpectedOverlay);
            }
        }

        if selector_is_visible(runtime, session, &self.selectors.reply_input).await?
            && selector_is_visible(runtime, session, &self.selectors.message_list).await?
        {
            return Ok(PlatformWorkspaceState::ChatOpen);
        }

        if selector_is_visible(runtime, session, &self.selectors.search_input).await? {
            return Ok(PlatformWorkspaceState::SearchOpen);
        }

        if selector_is_visible(runtime, session, &self.selectors.conversation_list).await? {
            return Ok(PlatformWorkspaceState::ChatListVisible);
        }

        Ok(PlatformWorkspaceState::ErrorOrUnknown)
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
        let preflight = runtime
            .preflight_check(
                session,
                &[
                    self.selectors.reply_input.clone(),
                    self.selectors.message_list.clone(),
                ],
            )
            .await?;
        if !preflight.passed {
            bail!(
                "reply box preflight failed; missing selectors: {:?}",
                preflight.missing_selectors
            );
        }

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

        let send_preflight = runtime
            .preflight_check(session, &[self.selectors.send_button.clone()])
            .await?;
        if !send_preflight.passed {
            bail!(
                "send button preflight failed; missing selectors: {:?}",
                send_preflight.missing_selectors
            );
        }

        let send_result = runtime.click(session, &self.selectors.send_button).await?;
        ensure_tool_success("send_reply", send_result)?;

        let outgoing_after = runtime
            .get_text(session, &self.selectors.outgoing_message)
            .await?;
        if !outgoing_after.output.contains(reply) {
            bail!("send_reply verification failed: outbound transcript did not include reply");
        }

        Ok(())
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

fn ensure_tool_success(action: &str, result: crate::tools::ToolResult) -> Result<()> {
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
