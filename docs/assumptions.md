# Assumptions

- Quant-M should remain a local-first Rust CLI/runtime unless the spec is explicitly expanded.
- The copies of `Ponboarding` and `Staff-OS` inside the repo are planning references, not runtime dependencies that Quant-M must mirror.
- Optional LLM, webhook, and Telegram features are user-configurable and must work with user-supplied secrets only.
- Paper or sandbox execution is the safe default posture for desk workflows until live behavior is explicitly approved.
- Portable relative paths plus env overrides are the preferred config convention for cross-machine use.
- Workspace markdown files are part of the runtime contract and should be preserved when changing memory or heartbeat behavior.
- OpenClaw, IronClaw, Paperclip, and Hermes are design influences, not replacement product specs for Quant-M.
