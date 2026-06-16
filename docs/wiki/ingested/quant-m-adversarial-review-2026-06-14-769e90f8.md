# Ingested Wiki Source: Quant-M Adversarial Review 2026-06-14

## Metadata

- Source path: `docs/wiki/raw/project/quant-m-adversarial-review-2026-06-14.md`
- Ingested at: `2026-06-15T02:58:52.630969+00:00`
- Source extension: `.md`
- Source type: exported chat thread / adversarial roadmap
- Confidence: medium-high for product-roadmap signal; medium for claims about current implementation status

## Agent summary

- The source frames Quant-M as architecturally strong but not yet product-simple: architecture is scored higher than first-run usability.
- The recommended product identity is: "a governed multi-model orchestration runtime that transforms evidence into deterministic actions through FSMs."
- The largest adoption risks are onboarding complexity, provider setup confusion, lack of an obvious signature use case, operator overload, and too many knobs.
- The proposed signature feature is evidence-oriented multi-model consensus: multiple models produce evidence, shared state stores weighted results, FSMs gate recommendations/actions, and the operator approves.
- The source strongly recommends keeping public Quant-M research-first and delaying live trading until risk, market-state, policy, and approval gates are mature.
- The strongest new-feature themes are first-run onboarding, knowledge decay, channel isolation, agent budget limits, provider normalization, evidence packages, cost governance, trading safety, and an eventual control center.
- The definition of shippable is raised above "tests pass": a capability should be understandable, recoverable, observable, governed, replayable, and usable by a non-developer.
- The source repeatedly warns that shared state must become high-signal institutional memory, not a generic data dump.
- Telegram/chat channels are useful, but should remain notification, approval, denial, and escalation surfaces, never direct execution surfaces.
- Some source claims mention dashboards, previews, reconciliation reports, and cost tracking as if recently present; verify against repo state before treating those as implemented facts.

## Durable feature ideas

### First-run onboarding

Target experience:

- `quant-m init`
- `quant-m setup`
- `quant-m doctor`

Desired outcome:

- Install-to-usable target under 5 minutes.
- First useful workflow under 10 minutes.
- Fewer knobs on day one.
- Provider/tool detection that distinguishes model providers from local CLIs/harnesses.

Current implementation relevance:

- This aligns with the CLI onboarding/provider work in `src/main.rs` and `src/config.rs`.
- Future onboarding should keep `setup --non-interactive` stable for Staff OS, cmux, CI, and scripted workers.

### Knowledge decay and weighted shared state

Proposed state metadata:

- confidence
- reliability
- freshness
- decay rate
- source count
- contradiction count

Proposed memory classes:

- ephemeral: minutes to days
- tactical: days to weeks
- strategic: months
- canonical: years

Implementation relevance:

- This complements `context_decay`, shared-state history, and the LLM wiki separation between doctrine, evidence, current facts, and session causality.
- Avoid treating shared state as a catch-all database. Shared state should retain current reusable facts; session logs keep ordered evidence; docs/wiki keep doctrine.

### Channel hardening

Rules proposed by source:

- Channels cannot execute.
- Telegram is notification-only.
- Channel interactions can observe, approve, deny, or escalate.
- Research channels should be read-only.
- Execution channels should be restricted.
- Operator channels should be approval-only.

Implementation relevance:

- Preserve this boundary in future Telegram/chat adapter work.
- Do not let a channel message directly trigger trading, shell commands, model execution, or workflow mutation.

### Agent loop budgets

Each agent/task should eventually carry:

- max iterations
- max cost
- max time
- max context

Kill condition:

- stop and escalate when budget is exceeded.

Implementation relevance:

- This is a future runtime governance feature, not an onboarding feature.
- It belongs near workflow/session/runtime evidence rather than provider configuration.

### Provider normalization

The source recommends normalizing providers into:

- `ModelRequest`
- `ModelResponse`
- `ToolCall`
- `CostRecord`

Goal:

- Workflows should not depend on provider-specific response shapes above the adapter boundary.

Implementation relevance:

- This aligns with the provider registry introduced for onboarding, but the registry alone is not a runtime provider abstraction.
- Future model execution work should define typed provider contracts before adding more live calls.

### Evidence packages

Every action should answer:

- why
- what evidence
- which policy
- which model
- which source
- which version

Replay package should include:

- inputs
- models
- outputs
- FSM state
- policies
- result

Implementation relevance:

- This reinforces existing session evidence and replay direction.
- Do not mark a capability shippable if it cannot explain itself through evidence.

### Cost governance

Future cost controls:

- estimated cost before model invocation
- actual cost after model invocation
- per-agent accounting
- per-workflow accounting
- per-desk accounting
- per-day accounting
- soft, hard, and kill budget limits

Implementation relevance:

- This should be designed before large-scale multi-model orchestration.
- Provider onboarding should not imply cost governance exists yet.

### Trading safety

The source recommends public Quant-M remain research-only until stronger controls exist.

Required before live execution:

- market reachability
- spread health
- latency health
- market-open validation
- max drawdown checks
- exposure checks
- position conflict checks
- operator approval
- desk approval
- risk approval

Implementation relevance:

- This matches the current conservative stance around paper/sandbox handoffs and no implicit live trading.
- Any future trading execution slice must pass a higher gate than ordinary research workflows.

### Operator control center

The source eventually wants one operator surface for:

- agents
- channels
- evidence
- shared state
- costs
- approvals
- alerts

Implementation relevance:

- Treat this as future UX/cockpit direction.
- Do not let the control center become required for the v0.1 proof path or scripted worker paths.

## Product identity notes

The source argues Quant-M should not present as "trading system", "agent framework", "research tool", "FSM runtime", and "multi-agent platform" all at once.

Recommended identity:

> Quant-M is a governed multi-model orchestration runtime that turns evidence into deterministic actions.

Recommended first public product frame:

> Quant-M Research Runtime

Feature emphasis:

- multi-model orchestration
- shared state
- evidence system
- channel alerts
- FSM approvals
- cost tracking
- replay

Explicitly de-emphasize for early public release:

- live trading
- full desk ecosystem
- complex workflow builders
- autonomous execution
- massive plugin systems

## Shippable definition proposed by source

A feature is shippable when a new user can successfully use it, understand it, recover from failure, and trust the results without requiring the developer.

Required traits:

- works correctly
- understandable
- recoverable
- observable
- governed
- evidence-producing
- replayable
- policy-respecting
- cost-aware when model calls are involved
- fits onboarding

Four maturity levels:

- Level 1: functional
- Level 2: usable
- Level 3: governed
- Level 4: operational

Suggested release-gate questions:

1. Does it work?
2. Can a new user understand it?
3. Can it recover from failure?
4. Is it observable?
5. Does it generate evidence?
6. Can it be replayed?
7. Does it respect policy?
8. Does it respect cost controls?
9. Does it fit onboarding?
10. Would it be trusted at 3 AM without the developer?

## Risks / constraints

- Do not treat the roadmap as permission to broaden scope immediately.
- Do not add governance surfaces before a signature workflow exists.
- Do not turn Telegram/chat channels into execution surfaces.
- Do not turn shared state into a global junk drawer.
- Do not present provider setup as model execution permission.
- Do not move public Quant-M toward live trading before research, replay, evidence, approvals, and risk controls are mature.
- Do not count "architecture completeness" as product readiness.

## Contradictions and verification notes

- The source says recent work includes cost tracking, dashboards, runtime evidence previews, import previews, and reconciliation reports. Some of these may be product vision rather than current repo state. Verify before using them as implementation evidence.
- The source proposes `quantm init`; the actual binary/README convention is `quant-m init`.
- The source recommends first-run wizard work. As of this ingest, Quant-M has begun implementing CLI onboarding inside the existing `init/setup/doctor/config` lane, not as a new onboarding runtime.

## Open questions

- Should "Quant-M Research Runtime" become the official v0.1/v1 public product frame?
- What is the first signature workflow that demonstrates evidence-oriented multi-model consensus?
- Which shared-state fields should become first-class schema versus derived context-decay metadata?
- Where should cost accounting live: provider adapter, execution runtime, session evidence, or shared-state history?
- What is the minimum useful operator control center that does not undermine terminal-first automation?
- What must be true before a trading desk can progress from research/paper handoff to any live execution adapter?
