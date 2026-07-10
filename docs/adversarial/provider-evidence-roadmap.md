# Provider Evidence Roadmap

## Verdict

The July 10, 2026 provider scan is a documentation baseline, not live capability evidence. Quant-M should preserve documented endpoints and older probe observations, but it must not convert them into executable routing by default.

Current implementation verdict:

```text
provider_registry_01_documented_unprobed
```

Run:

```bash
quant-m provider evidence
```

## Contract

Provider URLs remain implementation details behind a small internal capability surface:

- `generate`
- `generate_structured`
- `generate_stream`
- `embed`
- `rerank`
- `count_tokens`
- `list_models`
- `inspect_credential`
- `retrieve_usage`
- `retrieve_generation`
- `realtime_session`
- `research_search`
- `benchmark_metadata`

Documentation evidence, authenticated evidence, model visibility, and canary evidence are separate states. A provider endpoint may be installed and expected to work while still being unverified for the current credential, model, endpoint, and capability.

## Evidence States

Quant-M records endpoint evidence with these states:

- `documented_unprobed`
- `previously_observed`
- `authentication_passed`
- `endpoint_reachable`
- `model_visible`
- `capability_canary_passed`
- `ready`
- `stale`
- `gated`
- `side_effecting_unverified`
- `contradicted`
- `quarantined`

Hard gate:

```text
No documented_unprobed or previously_observed endpoint may report Ready.
```

## Initial Endpoint Disposition

Initial production candidates:

- OpenAI: Responses, Embeddings, Models.
- Gemini: Interactions or `generateContent` fallback, embeddings, Models.
- OpenRouter: Chat Completions, Embeddings, Models, key status.
- Z.AI: Chat Completions only until fresh model and embedding evidence exists.
- Artificial Analysis: cached benchmark metadata only.

Quarantined or inactive by default:

- OpenAI Administration.
- OpenRouter key, workspace, organization, BYOK, OAuth, guardrail, and Responses beta surfaces.
- Artificial Analysis CritPt in ordinary routing.
- Z.AI media, reader, search, tokenizer, moderation, rerank, and embeddings until revalidated.
- Any endpoint that creates keys, purchases credits, changes provider account state, or starts a privileged side-effecting job.

## Next Slices

`PROVIDER_CORE_RUST_02` should add canonical provider contracts, required-versus-preferred parameter semantics, translation reports, and a provider error taxonomy.

`PROVIDER_FIXTURE_HARNESS_03` should run adapters against offline fixtures and mock servers with no keys.

`PROVIDER_SECRET_ONBOARDING_04` should keep keys core-side, secret-store backed, redacted, and represented by installation-specific HMAC fingerprints.

`ARTIFICIAL_ANALYSIS_CACHE_09` should refresh benchmark metadata at most once daily into SQLite. The cache may influence weighted model selection, but it must never become a per-prompt dependency or imply provider entitlement.

`CAPABILITY_CANARY_10` should promote readiness only after explicit low-cost generation or embedding canaries prove the current credential, endpoint, model, and capability.

## Non-Goals

This roadmap does not add broker APIs, live trading, shadow trading, model sprawl, dashboard expansion, child provider keys, or automatic endpoint activation.
