# Benchmarks

Quant-M publishes conservative local benchmark evidence for release candidates. These numbers are proof points, not cross-machine performance guarantees.

## v0.1.0-beta Local Snapshot

Measured on 2026-06-16 from a clean temp checkout of the current worktree on macOS. Cargo dependencies were already available on the machine, but the release target directory was rebuilt from scratch.

| Check | Command | Result |
| --- | --- | --- |
| Export repo size | `du -sh .` after removing `target/` and generated workspace state | 3.7M |
| Release binary size | `du -sh target/release/quant-m` | 4.2M |
| Fresh release build time | `/usr/bin/time -p cargo build --release` | 70.83s real |
| Startup/help time | `/usr/bin/time -p ./target/release/quant-m --help` | 0.44s real |
| Init time | `/usr/bin/time -p ./target/release/quant-m init` | 0.45s real |
| Setup time | `/usr/bin/time -p ./target/release/quant-m setup --non-interactive --runtime-profile laptop --context-guardian true` | 0.01s real |
| Consensus dry run | `/usr/bin/time -p ./target/release/quant-m consensus --dry-run "..."` | 0.04s real |
| Replay time | `/usr/bin/time -p ./target/release/quant-m replay <session_id>` | 0.00s real |
| Compact time | `/usr/bin/time -p ./target/release/quant-m compact <session_id>` | 0.00s real |
| Guardian time | `/usr/bin/time -p ./target/release/quant-m context guard --json` | 0.00s real |
| Cost summary time | `/usr/bin/time -p ./target/release/quant-m cost summary` | 0.00s real |
| Test count | `cargo test` | 236 library tests + 303 binary tests |

The sub-0.01s commands are shown as `0.00s` by `/usr/bin/time -p` on this machine.

## First-Use Proof Session

The beta walkthrough produced:

- local session evidence in `workspace/state/sessions/`
- compact packet files in `workspace/state/compacted/`
- context guardian metadata and handoff in `workspace/state/context-guardian/`
- cost ledger record in `workspace/state/cost/cost-ledger.jsonl`
- replay status: `ValidatedEvidenceOnly`
- cost summary: one mock `consensus_dry_run` record, zero actual cost

## Benchmark Posture

- Keep generated benchmark output out of git unless curated.
- Record hardware, OS, commit, command, and result summary when sharing numbers.
- Do not compare provider/model performance without naming configuration and network conditions.
- Prefer local, repeatable CLI checks over synthetic claims.

## Peer Harness Comparison Plan

Quant-M should be compared with adjacent agent harnesses and local runtimes, not with coding tools that can run beside it. Codex, Claude Code, Antigravity CLI, and similar tools are part of the operator/tool ecosystem; they are not the direct benchmark lane.

The direct comparison set for the public beta is:

- OpenClaw
- Hermes Agent
- ZeroClaw
- Quant-M

Peer numbers should stay marked as pending until each project is measured from a fresh checkout on the same machine, with the same timing method.

| Metric | OpenClaw | Hermes Agent | ZeroClaw | Quant-M v0.1.0-beta |
| --- | --- | --- | --- | --- |
| Repo size | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 3.7M clean export |
| Release/binary size | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 4.2M release binary |
| Fresh build speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 70.83s release build |
| Startup/help speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 0.44s |
| Proof-loop speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | consensus dry run 0.04s; replay/compact/guardian/cost below local timer precision |
| Tool-use posture | Pending same-harness review | Pending same-harness review | Pending same-harness review | gated, recorded, replay-aware |
| Channel posture | Pending same-harness review | Pending same-harness review | Pending same-harness review | CLI now; channels stay policy-gated |
| Token-saving posture | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | compact packets and handoffs reduce context reload; exact saved tokens pending model/channel benchmarks |
| API consistency | Pending same-harness review | Pending same-harness review | Pending same-harness review | typed payload normalization before state, replay, cost, and handoff writes |

## API Payload Normalization Benchmark Direction

Quant-M treats model and tool output as untrusted until it is parsed into typed local records. Future benchmark packets should measure:

- malformed payload rejection
- schema/version acceptance
- replay determinism after normalized writes
- compact packet size
- estimated tokens saved by handoff packets versus raw session reloads
- channel consistency across CLI, worker, model, and replay payloads

## Available Helpers

```bash
scripts/bench_worker_runtime.sh
scripts/benchmark_agent_frameworks.sh
```

Generated benchmark reports should stay local unless explicitly curated into `docs/`.
