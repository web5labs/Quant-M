use crate::config::Config;
use crate::side_effect_gate::{SideEffectKind, SideEffectRequest, evaluate_side_effect};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: String,
}

pub async fn ask(cfg: &Config, prompt: &str) -> Result<String> {
    let gate = evaluate_side_effect(
        SideEffectRequest::new(SideEffectKind::ProviderCall, "llm.ask")
            .config_allowed(cfg.llm.enabled && cfg.runtime.external_network_enabled)
            .policy_allowed(cfg.llm.enabled && cfg.runtime.external_network_enabled),
    );
    if !gate.is_allowed() {
        return Err(anyhow!(
            "code=side_effect_gate_blocked action={} decision={} reason={}",
            gate.action_label,
            gate.decision,
            gate.reason
        ));
    }
    let api_key = cfg
        .resolve_llm_api_key()
        .ok_or_else(|| anyhow!("missing LLM API key"))?;
    let base = cfg.llm.api_base.trim_end_matches('/');
    let url = format!("{base}/chat/completions");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            cfg.llm.request_timeout_seconds.max(5),
        ))
        .build()
        .context("failed to build LLM HTTP client")?;

    let body = ChatCompletionsRequest {
        model: cfg.llm.model.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.trim().to_string(),
        }],
        temperature: cfg.llm.temperature,
        max_tokens: cfg.llm.max_tokens,
    };

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("failed to call LLM endpoint {url}"))?
        .error_for_status()
        .with_context(|| format!("LLM endpoint returned non-success status {url}"))?
        .json::<ChatCompletionsResponse>()
        .await
        .context("failed to decode LLM response JSON")?;

    let content = resp
        .choices
        .first()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("LLM response was empty"))?;

    Ok(content)
}
