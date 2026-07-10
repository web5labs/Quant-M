use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    DocumentedUnprobed,
    PreviouslyObserved,
    AuthenticationPassed,
    EndpointReachable,
    ModelVisible,
    CapabilityCanaryPassed,
    Ready,
    Stale,
    Gated,
    SideEffectingUnverified,
    Contradicted,
    Quarantined,
}

impl fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DocumentedUnprobed => "documented_unprobed",
            Self::PreviouslyObserved => "previously_observed",
            Self::AuthenticationPassed => "authentication_passed",
            Self::EndpointReachable => "endpoint_reachable",
            Self::ModelVisible => "model_visible",
            Self::CapabilityCanaryPassed => "capability_canary_passed",
            Self::Ready => "ready",
            Self::Stale => "stale",
            Self::Gated => "gated",
            Self::SideEffectingUnverified => "side_effecting_unverified",
            Self::Contradicted => "contradicted",
            Self::Quarantined => "quarantined",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EndpointLifecycle {
    Production,
    Compatibility,
    Experimental,
    Beta,
    Deprecated,
    Privileged,
    Quarantined,
}

impl fmt::Display for EndpointLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Production => "production",
            Self::Compatibility => "compatibility",
            Self::Experimental => "experimental",
            Self::Beta => "beta",
            Self::Deprecated => "deprecated",
            Self::Privileged => "privileged",
            Self::Quarantined => "quarantined",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    ReadOnlyDiscovery,
    BillableInference,
    BillableEmbedding,
    BenchmarkMetadata,
    AccountAdministration,
    SideEffectingJob,
}

impl fmt::Display for AuthorityClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ReadOnlyDiscovery => "read_only_discovery",
            Self::BillableInference => "billable_inference",
            Self::BillableEmbedding => "billable_embedding",
            Self::BenchmarkMetadata => "benchmark_metadata",
            Self::AccountAdministration => "account_administration",
            Self::SideEffectingJob => "side_effecting_job",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    Generate,
    GenerateStructured,
    GenerateStream,
    Embed,
    Rerank,
    CountTokens,
    ListModels,
    InspectCredential,
    RetrieveUsage,
    RetrieveGeneration,
    RealtimeSession,
    ResearchSearch,
    BenchmarkMetadata,
    Administration,
}

impl fmt::Display for ProviderCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Generate => "generate",
            Self::GenerateStructured => "generate_structured",
            Self::GenerateStream => "generate_stream",
            Self::Embed => "embed",
            Self::Rerank => "rerank",
            Self::CountTokens => "count_tokens",
            Self::ListModels => "list_models",
            Self::InspectCredential => "inspect_credential",
            Self::RetrieveUsage => "retrieve_usage",
            Self::RetrieveGeneration => "retrieve_generation",
            Self::RealtimeSession => "realtime_session",
            Self::ResearchSearch => "research_search",
            Self::BenchmarkMetadata => "benchmark_metadata",
            Self::Administration => "administration",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Delete => "DELETE",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointEvidence {
    pub provider: String,
    pub method: HttpMethod,
    pub path_template: String,
    pub capability: ProviderCapability,
    pub lifecycle: EndpointLifecycle,
    pub authority: AuthorityClass,
    pub evidence_status: EvidenceStatus,
    pub documented_at: Option<String>,
    pub last_observed_at: Option<String>,
    pub last_canary_at: Option<String>,
    pub source_reference: String,
    pub schema_hash: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderEvidenceReport {
    pub verdict: String,
    pub ready_count: usize,
    pub total_count: usize,
    pub records: Vec<EndpointEvidence>,
}

pub fn registry() -> Vec<EndpointEvidence> {
    let mut records = vec![
        july(
            "openai",
            HttpMethod::Post,
            "/responses",
            ProviderCapability::Generate,
            EndpointLifecycle::Production,
            AuthorityClass::BillableInference,
            "OpenAI Responses is the primary production generation surface; canary evidence is still required before Ready.",
        ),
        july(
            "openai",
            HttpMethod::Post,
            "/embeddings",
            ProviderCapability::Embed,
            EndpointLifecycle::Production,
            AuthorityClass::BillableEmbedding,
            "Embeddings are production-capable only after credential and model-specific validation.",
        ),
        july(
            "openai",
            HttpMethod::Get,
            "/models",
            ProviderCapability::ListModels,
            EndpointLifecycle::Production,
            AuthorityClass::ReadOnlyDiscovery,
            "Model listing can prove visibility but not generation readiness.",
        ),
        july(
            "openai",
            HttpMethod::Post,
            "/responses/{response_id}/input_tokens/count",
            ProviderCapability::CountTokens,
            EndpointLifecycle::Experimental,
            AuthorityClass::ReadOnlyDiscovery,
            "Use only near context or cost limits after adapter contract tests.",
        ),
        july(
            "openai",
            HttpMethod::Post,
            "/moderations",
            ProviderCapability::Generate,
            EndpointLifecycle::Compatibility,
            AuthorityClass::ReadOnlyDiscovery,
            "Supported but inactive in default routing until policy semantics are explicit.",
        ),
        quarantined(
            "openai",
            HttpMethod::Get,
            "/organization/*",
            ProviderCapability::Administration,
            (
                EndpointLifecycle::Privileged,
                AuthorityClass::AccountAdministration,
                EvidenceStatus::SideEffectingUnverified,
            ),
            "Administration requires separate authority and credentials; never route through ordinary inference.",
        ),
        july(
            "gemini",
            HttpMethod::Post,
            "/v1beta/models/{model}:generateContent",
            ProviderCapability::Generate,
            EndpointLifecycle::Compatibility,
            AuthorityClass::BillableInference,
            "Stateless fallback for ordinary text generation; Interactions remains preferred for newer workflows.",
        ),
        july(
            "gemini",
            HttpMethod::Post,
            "/v1beta/models/{model}:streamGenerateContent",
            ProviderCapability::GenerateStream,
            EndpointLifecycle::Experimental,
            AuthorityClass::BillableInference,
            "Streaming stays inactive until stream fixture and cancellation behavior are tested.",
        ),
        july(
            "gemini",
            HttpMethod::Post,
            "/v1beta/models/{model}:embedContent",
            ProviderCapability::Embed,
            EndpointLifecycle::Production,
            AuthorityClass::BillableEmbedding,
            "Single embeddings need credential-specific canary evidence.",
        ),
        july(
            "gemini",
            HttpMethod::Post,
            "/v1beta/models/{model}:batchEmbedContents",
            ProviderCapability::Embed,
            EndpointLifecycle::Production,
            AuthorityClass::BillableEmbedding,
            "Preferred for council batches after offline adapter contracts pass.",
        ),
        july(
            "gemini",
            HttpMethod::Get,
            "/v1beta/models",
            ProviderCapability::ListModels,
            EndpointLifecycle::Production,
            AuthorityClass::ReadOnlyDiscovery,
            "Model listing can prove visibility but not readiness.",
        ),
        july(
            "openrouter",
            HttpMethod::Post,
            "/chat/completions",
            ProviderCapability::Generate,
            EndpointLifecycle::Production,
            AuthorityClass::BillableInference,
            "Primary OpenRouter inference adapter; unsupported required parameters must fail before send.",
        ),
        july(
            "openrouter",
            HttpMethod::Post,
            "/embeddings",
            ProviderCapability::Embed,
            EndpointLifecycle::Production,
            AuthorityClass::BillableEmbedding,
            "Embedding readiness is model and credential specific.",
        ),
        july(
            "openrouter",
            HttpMethod::Get,
            "/models",
            ProviderCapability::ListModels,
            EndpointLifecycle::Production,
            AuthorityClass::ReadOnlyDiscovery,
            "Model catalog visibility is not a Ready signal.",
        ),
        july(
            "openrouter",
            HttpMethod::Get,
            "/auth/key",
            ProviderCapability::InspectCredential,
            EndpointLifecycle::Production,
            AuthorityClass::ReadOnlyDiscovery,
            "Key status can authenticate the credential but cannot activate inference by itself.",
        ),
        july(
            "openrouter",
            HttpMethod::Get,
            "/generation?id={id}",
            ProviderCapability::RetrieveGeneration,
            EndpointLifecycle::Experimental,
            AuthorityClass::ReadOnlyDiscovery,
            "Generation retrieval is audit support, not hot-path routing.",
        ),
        quarantined(
            "openrouter",
            HttpMethod::Post,
            "/responses",
            ProviderCapability::Generate,
            (
                EndpointLifecycle::Beta,
                AuthorityClass::BillableInference,
                EvidenceStatus::Gated,
            ),
            "Responses beta remains disabled until contract tests and canaries prove it.",
        ),
        quarantined(
            "openrouter",
            HttpMethod::Post,
            "/keys",
            ProviderCapability::Administration,
            (
                EndpointLifecycle::Privileged,
                AuthorityClass::AccountAdministration,
                EvidenceStatus::SideEffectingUnverified,
            ),
            "Key management is privileged administration and never part of default routing.",
        ),
        july(
            "zai",
            HttpMethod::Post,
            "/paas/v4/chat/completions",
            ProviderCapability::Generate,
            EndpointLifecycle::Production,
            AuthorityClass::BillableInference,
            "Z.AI has OpenAI-style shape but requires its own DTO and business-code error parsing.",
        ),
        previously_observed(
            "zai",
            HttpMethod::Get,
            "/paas/v4/models",
            ProviderCapability::ListModels,
            EndpointLifecycle::Experimental,
            AuthorityClass::ReadOnlyDiscovery,
            "Observed in earlier probes; July credential must revalidate before use.",
        ),
        previously_observed(
            "zai",
            HttpMethod::Post,
            "/paas/v4/embeddings",
            ProviderCapability::Embed,
            EndpointLifecycle::Experimental,
            AuthorityClass::BillableEmbedding,
            "Previously observed only; disabled until a fresh embedding canary passes.",
        ),
        previously_observed(
            "zai",
            HttpMethod::Post,
            "/paas/v4/rerank",
            ProviderCapability::Rerank,
            EndpointLifecycle::Experimental,
            AuthorityClass::BillableInference,
            "Previously observed only; not active in production routing.",
        ),
        july(
            "artificial_analysis",
            HttpMethod::Get,
            "/api/v2/data/llms/models",
            ProviderCapability::BenchmarkMetadata,
            EndpointLifecycle::Production,
            AuthorityClass::BenchmarkMetadata,
            "Benchmark metadata is cached selection evidence, not inference authority.",
        ),
        quarantined(
            "artificial_analysis",
            HttpMethod::Post,
            "/api/v2/critpt/*",
            ProviderCapability::BenchmarkMetadata,
            (
                EndpointLifecycle::Experimental,
                AuthorityClass::SideEffectingJob,
                EvidenceStatus::SideEffectingUnverified,
            ),
            "CritPt is a batch evaluation surface and must stay outside ordinary routing.",
        ),
    ];
    records.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then(a.path_template.cmp(&b.path_template))
    });
    records
}

pub fn report() -> ProviderEvidenceReport {
    let records = registry();
    let ready_count = records
        .iter()
        .filter(|record| record.evidence_status == EvidenceStatus::Ready)
        .count();
    ProviderEvidenceReport {
        verdict: if ready_count == 0 {
            "provider_registry_01_documented_unprobed".to_string()
        } else {
            "provider_registry_01_has_ready_evidence".to_string()
        },
        total_count: records.len(),
        ready_count,
        records,
    }
}

pub fn docs_only_endpoints_are_never_ready(records: &[EndpointEvidence]) -> bool {
    records.iter().all(|record| {
        !matches!(
            record.evidence_status,
            EvidenceStatus::DocumentedUnprobed | EvidenceStatus::PreviouslyObserved
        ) || record.evidence_status != EvidenceStatus::Ready
    })
}

pub fn render_report(report: &ProviderEvidenceReport) -> String {
    let mut out = format!(
        "Quant-M provider evidence registry\nverdict: {}\nready: {}/{}\n\n",
        report.verdict, report.ready_count, report.total_count
    );
    for record in &report.records {
        out.push_str(&format!(
            "{} {} {} capability={} status={} lifecycle={} authority={}\n",
            record.provider,
            record.method,
            record.path_template,
            record.capability,
            record.evidence_status,
            record.lifecycle,
            record.authority
        ));
    }
    out.push_str(
        "\nGate: documented_unprobed and previously_observed endpoints are evidence only. They are not executable Ready routes.\n",
    );
    out
}

fn july(
    provider: &str,
    method: HttpMethod,
    path_template: &str,
    capability: ProviderCapability,
    lifecycle: EndpointLifecycle,
    authority: AuthorityClass,
    note: &str,
) -> EndpointEvidence {
    EndpointEvidence {
        provider: provider.to_string(),
        method,
        path_template: path_template.to_string(),
        capability,
        lifecycle,
        authority,
        evidence_status: EvidenceStatus::DocumentedUnprobed,
        documented_at: Some("2026-07-10".to_string()),
        last_observed_at: None,
        last_canary_at: None,
        source_reference: "2026-07 provider documentation review".to_string(),
        schema_hash: None,
        notes: vec![note.to_string()],
    }
}

fn previously_observed(
    provider: &str,
    method: HttpMethod,
    path_template: &str,
    capability: ProviderCapability,
    lifecycle: EndpointLifecycle,
    authority: AuthorityClass,
    note: &str,
) -> EndpointEvidence {
    EndpointEvidence {
        provider: provider.to_string(),
        method,
        path_template: path_template.to_string(),
        capability,
        lifecycle,
        authority,
        evidence_status: EvidenceStatus::PreviouslyObserved,
        documented_at: None,
        last_observed_at: Some("2026-04".to_string()),
        last_canary_at: None,
        source_reference: "2026-04 authenticated probe notes".to_string(),
        schema_hash: None,
        notes: vec![note.to_string()],
    }
}

fn quarantined(
    provider: &str,
    method: HttpMethod,
    path_template: &str,
    capability: ProviderCapability,
    classification: (EndpointLifecycle, AuthorityClass, EvidenceStatus),
    note: &str,
) -> EndpointEvidence {
    let (lifecycle, authority, evidence_status) = classification;
    EndpointEvidence {
        provider: provider.to_string(),
        method,
        path_template: path_template.to_string(),
        capability,
        lifecycle,
        authority,
        evidence_status,
        documented_at: Some("2026-07-10".to_string()),
        last_observed_at: None,
        last_canary_at: None,
        source_reference: "2026-07 provider documentation review".to_string(),
        schema_hash: None,
        notes: vec![note.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_no_docs_only_ready_endpoints() {
        let records = registry();
        assert!(docs_only_endpoints_are_never_ready(&records));
        assert_eq!(report().ready_count, 0);
    }

    #[test]
    fn registry_keeps_privileged_and_beta_surfaces_gated() {
        let records = registry();
        let admin = records
            .iter()
            .find(|record| {
                record.provider == "openai"
                    && record.authority == AuthorityClass::AccountAdministration
            })
            .expect("openai admin record");
        assert_eq!(
            admin.evidence_status,
            EvidenceStatus::SideEffectingUnverified
        );

        let openrouter_responses = records
            .iter()
            .find(|record| record.provider == "openrouter" && record.path_template == "/responses")
            .expect("openrouter responses beta");
        assert_eq!(openrouter_responses.evidence_status, EvidenceStatus::Gated);
    }
}
