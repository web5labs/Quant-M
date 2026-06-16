---
type: external-doc-summary
date: 2026-05-31
source_count: 3
tags:
  - ironclaw
  - rust
  - security
  - sandbox
---

# IronClaw Security Runtime

## Sources

- [IronClaw introduction](https://docs.ironclaw.com/)
- [IronClaw WASM tools](https://docs.ironclaw.com/capabilities/sandboxed-tools)
- [NEAR AI GitHub org](https://github.com/nearai) showing `nearai/ironclaw`

## What matters for Quant-M

- IronClaw is explicitly Rust-based and security-first.
- Its documented security story is architectural: safety layer, WASM sandboxing, Docker isolation, encrypted secrets, and explicit capability declarations.
- Parallel jobs, self-repair, persistent memory, and heartbeat-based behavior are described as core capabilities.
- Tool execution is framed as capability-scoped rather than “trust the tool because it exists.”

## Borrow

- Security boundaries as a runtime architecture decision, not a marketing add-on.
- Capability declaration for risky integrations.
- Layered isolation story for untrusted tools.
- State-machine language around concurrent jobs and recovery behavior.

## Avoid

- Importing cloud-specific or platform-specific assumptions into Quant-M’s local-first core.
- Rebuilding a full multi-channel security platform before the local worker runtime is fully shippable.
- Over-promising sandbox strength if Quant-M only has config guards and not actual isolation.

## Rails implication

- Quant-M should keep “unsafe by exception” as a core policy.
- If custom tools expand beyond trusted local commands, a capability-scoped interface is the right future direction.
- Approval boundaries and sandbox posture should become explicit design sections in shippable criteria, not just implementation details.
