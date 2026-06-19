use crate::config::{AdapterConfig, Config};
use crate::side_effect_gate::{SideEffectKind, SideEffectRequest, evaluate_side_effect};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterEvent {
    pub kind: String,
    pub message: String,
    pub node_id: String,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

#[derive(Clone)]
pub struct AdapterHub {
    cfg: AdapterConfig,
    node_id: String,
    client: reqwest::Client,
    external_network_enabled: bool,
}

impl AdapterHub {
    pub fn new(cfg: &Config) -> Result<Self> {
        if let Some(url) = &cfg.adapters.webhook_url {
            let parsed =
                reqwest::Url::parse(url).with_context(|| format!("invalid webhook URL '{url}'"))?;
            if parsed.scheme() != "https" {
                anyhow::bail!("webhook_url must use https");
            }
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                cfg.adapters.webhook_timeout_seconds.max(1),
            ))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            cfg: cfg.adapters.clone(),
            node_id: cfg.node_id.clone(),
            client,
            external_network_enabled: cfg.runtime.external_network_enabled,
        })
    }

    pub async fn send_simple(&self, kind: &str, message: &str) -> Result<()> {
        self.send(AdapterEvent {
            kind: kind.to_string(),
            message: message.to_string(),
            node_id: self.node_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            payload: serde_json::json!({}),
        })
        .await
    }

    pub async fn send(&self, event: AdapterEvent) -> Result<()> {
        if self.cfg.terminal_enabled {
            let as_json =
                serde_json::to_string(&event).context("failed to serialize adapter event")?;
            println!("{as_json}");
        }

        if let Some(url) = &self.cfg.webhook_url {
            let gate = evaluate_side_effect(
                SideEffectRequest::new(SideEffectKind::WebhookSend, "adapter.webhook_send")
                    .config_allowed(self.external_network_enabled)
                    .policy_allowed(self.external_network_enabled),
            );
            if !gate.is_allowed() {
                anyhow::bail!(
                    "code=side_effect_gate_blocked action={} decision={} reason={}",
                    gate.action_label,
                    gate.decision,
                    gate.reason
                );
            }
            self.client
                .post(url)
                .json(&event)
                .send()
                .await
                .with_context(|| format!("failed to POST adapter event to {url}"))?
                .error_for_status()
                .with_context(|| format!("webhook returned non-success status from {url}"))?;
        }

        Ok(())
    }
}
