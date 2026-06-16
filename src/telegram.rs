use crate::adapters::AdapterHub;
use crate::config::{Config, ExternalChannel};
use crate::{channels, logutil, shutdown};
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use tokio::sync::watch;
use tokio::time::Duration;

#[derive(Debug, Deserialize)]
struct UpdatesResponse {
    ok: bool,
    result: Vec<Update>,
}

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
}

pub async fn run_loop(cfg: Config, adapters: AdapterHub) -> Result<()> {
    run_loop_with_shutdown(cfg, adapters, None).await
}

pub async fn run_loop_with_shutdown(
    cfg: Config,
    adapters: AdapterHub,
    mut shutdown_rx: Option<watch::Receiver<bool>>,
) -> Result<()> {
    if !cfg.telegram.enabled {
        return Ok(());
    }
    let bot_token = cfg
        .resolve_telegram_bot_token()
        .ok_or_else(|| anyhow!("missing telegram bot token"))?;

    logutil::append_log(&cfg.logging, "telegram loop starting")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("failed building telegram client")?;

    let mut offset = 0_i64;
    loop {
        let updates = poll_updates(&client, &bot_token, offset, 10).await?;
        for update in updates {
            offset = offset.max(update.update_id + 1);
            if let Some(message) = update.message {
                let _ = handle_message(&cfg, &client, &bot_token, message).await;
                let _ = adapters.send_simple("telegram", "message processed").await;
            }
        }

        if should_shutdown(&mut shutdown_rx, cfg.telegram.poll_interval_seconds).await {
            logutil::append_log(&cfg.logging, "telegram loop stopping on shutdown signal")?;
            break;
        }
    }

    Ok(())
}

async fn poll_updates(
    client: &reqwest::Client,
    bot_token: &str,
    offset: i64,
    timeout_secs: u64,
) -> Result<Vec<Update>> {
    let url = format!("https://api.telegram.org/bot{bot_token}/getUpdates");
    let response = client
        .get(&url)
        .query(&[
            ("offset", offset.to_string()),
            ("timeout", timeout_secs.to_string()),
            ("allowed_updates", "[\"message\"]".to_string()),
        ])
        .send()
        .await
        .with_context(|| format!("telegram getUpdates request failed {url}"))?
        .error_for_status()
        .with_context(|| format!("telegram getUpdates non-success status {url}"))?
        .json::<UpdatesResponse>()
        .await
        .context("failed to decode telegram getUpdates response")?;

    if !response.ok {
        return Err(anyhow!("telegram getUpdates returned ok=false"));
    }
    Ok(response.result)
}

async fn handle_message(
    cfg: &Config,
    client: &reqwest::Client,
    bot_token: &str,
    message: Message,
) -> Result<()> {
    if let Some(allow_chat) = cfg.telegram.allowed_chat_id
        && message.chat.id != allow_chat
    {
        return Ok(());
    }

    let Some(text) = message
        .text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(());
    };

    let reply = match parse_command(text) {
        TelegramCommand::Help => help_text().to_string(),
        TelegramCommand::Status => format!(
            "quant-m online\nnode: {}\nllm_enabled: {}",
            cfg.node_id, cfg.llm.enabled
        ),
        TelegramCommand::MessageIntent(prompt) => {
            let intent = channels::classify_channel_message(ExternalChannel::Telegram, &prompt);
            intent.operator_reply()
        }
    };

    send_message(client, bot_token, message.chat.id, &reply).await
}

async fn send_message(
    client: &reqwest::Client,
    bot_token: &str,
    chat_id: i64,
    text: &str,
) -> Result<()> {
    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let safe_text = truncate_telegram_message(text);
    client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": safe_text
        }))
        .send()
        .await
        .with_context(|| format!("telegram sendMessage request failed {url}"))?
        .error_for_status()
        .with_context(|| format!("telegram sendMessage non-success status {url}"))?;
    Ok(())
}

async fn should_shutdown(
    shutdown_rx: &mut Option<watch::Receiver<bool>>,
    poll_interval_seconds: u64,
) -> bool {
    if let Some(rx) = shutdown_rx.as_mut() {
        tokio::select! {
            _ = shutdown::wait_for_shutdown_signal() => true,
            changed = rx.changed() => changed.is_ok() && *rx.borrow(),
            _ = tokio::time::sleep(Duration::from_secs(poll_interval_seconds.max(1))) => false,
        }
    } else {
        tokio::select! {
            _ = shutdown::wait_for_shutdown_signal() => true,
            _ = tokio::time::sleep(Duration::from_secs(poll_interval_seconds.max(1))) => false,
        }
    }
}

enum TelegramCommand {
    Help,
    Status,
    MessageIntent(String),
}

fn parse_command(text: &str) -> TelegramCommand {
    if text.eq_ignore_ascii_case("/help") || text.eq_ignore_ascii_case("help") {
        return TelegramCommand::Help;
    }
    if text.eq_ignore_ascii_case("/status") {
        return TelegramCommand::Status;
    }

    if let Some(prompt) = text.strip_prefix("/ask ") {
        let prompt = prompt.trim();
        if !prompt.is_empty() {
            return TelegramCommand::MessageIntent(prompt.to_string());
        }
    }

    TelegramCommand::MessageIntent(text.to_string())
}

fn help_text() -> &'static str {
    "Quant-M Telegram commands:\n/status\n/ask <evidence or question>\n/help\n\nChannel text is evidence only; it cannot execute workflows, call providers, or bypass approval."
}

fn truncate_telegram_message(text: &str) -> String {
    const MAX: usize = 3900;
    if text.len() <= MAX {
        text.to_string()
    } else {
        format!("{}...[truncated]", &text[..MAX])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help() {
        assert!(matches!(parse_command("/help"), TelegramCommand::Help));
    }

    #[test]
    fn parse_status() {
        assert!(matches!(parse_command("/status"), TelegramCommand::Status));
    }

    #[test]
    fn parse_ask_with_prefix() {
        match parse_command("/ask hello") {
            TelegramCommand::MessageIntent(prompt) => assert_eq!(prompt, "hello"),
            _ => panic!("expected message intent"),
        }
    }

    #[test]
    fn parse_consensus_text_as_non_executing_message_intent() {
        match parse_command("quant-m consensus --dry-run now") {
            TelegramCommand::MessageIntent(prompt) => {
                let intent = channels::classify_channel_message(ExternalChannel::Telegram, &prompt);
                assert_eq!(
                    intent.event_type,
                    channels::ChannelEventType::CommandRejected
                );
                assert!(!intent.executes_runtime);
                assert!(!intent.calls_provider);
            }
            _ => panic!("expected message intent"),
        }
    }
}
