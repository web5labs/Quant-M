# Side Effect Policy Gate Audit

Slice: `SIDE_EFFECT_POLICY_GATE_01`

Purpose: add one focused policy gate around existing side-effect paths without adding provider behavior, network behavior, shell behavior, trading execution, or auto-approval.

## Classification

| Path | Classification | Gate |
| --- | --- | --- |
| `quant-m llm ask` / `src/llm.rs` | wired/shared gate | `provider_call`; requires `llm.enabled` and `runtime.external_network_enabled` |
| Worker `http_get` / `src/worker.rs` | wired/shared gate | `network_http`; disabled by config, dry-run recorded, live requires `runtime.external_network_enabled` |
| Worker `shell` / `src/worker.rs` | wired/shared gate | `shell_command`; requires `worker.allow_shell_commands` |
| Skill shell execution / `src/skills.rs` | wired/shared gate | `shell_command`; requires `skills.allow_shell_commands` |
| Adapter webhook / `src/adapters.rs` | wired/shared gate | `webhook_send`; requires configured HTTPS webhook and `runtime.external_network_enabled` |
| Telegram poll/send / `src/telegram.rs` | wired/shared gate | `telegram_send`; requires `telegram.enabled` and `runtime.external_network_enabled` |
| Channel text classification / `src/channels.rs` | already safely gated | evidence only; no execution authority |
| Session replay / `src/sessions.rs` | dry-run/replay-only | replay remains inspect-only; gate models `replay_skipped` |
| Trading/broker/exchange execution | unavailable | no live execution surface exists |
| File writes/state mutation | audited not wired | existing local validation remains path-specific |

## Runtime Evidence

The gate wraps `PolicyApprovalFsm` and emits compact `AuditNote` evidence where sessions already exist. Worker blocks also emit `PolicyDecision` evidence. JSON fields use stable `snake_case` labels for `SideEffectKind` and `SideEffectDecision`.

## Authority Boundary

`policy_approval` remains `partially_wired` in `quant-m fsm authority --json`. The gate covers current high-risk paths, but it is not yet a universal sink for every file write or state mutation.
