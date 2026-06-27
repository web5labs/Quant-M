# EDGE_RUNTIME_MINIMIZATION_10

Quant-M now treats edge runtime minimization as a design constraint.

Core can be rich. Child must be tiny.

## Design Law

Every new feature must answer:

1. Does this belong on the core or the child?
2. Can the child avoid compiling this code?
3. Can this be represented as data instead of logic?
4. Can this reuse an existing envelope instead of adding a new type?
5. Can this be feature-gated?
6. Can this be append-only JSON instead of a database dependency?
7. Can it be tested with fixtures instead of live services?

## Binary Split

| Binary | Purpose | Command Surface |
| --- | --- | --- |
| `quant-m` | Core/operator CLI | full CLI, pairing server, playbooks, model handoff, shared-state validation, reports |
| `quant-m-child` | Edge child CLI | `pair`, `pair-scan`, `identity`, `doctor`, `heartbeat`, `run-once` |

`quant-m-child` is intentionally standalone and does not import the core crate modules for model routing, playbook authoring, shared-state acceptance, or pairing server runtime.

## Feature Matrix

| Feature | Intended Target | Notes |
| --- | --- | --- |
| `default` | minimal package baseline | kept empty |
| `core-full` | core/operator | core storage, terminal UI, network client, pairing, QR, scan image, playbooks, model handoff/router, local stub, compute |
| `child-min` | smallest child | no model router, no provider adapters, no shared-state accept/reject, no pairing server, no QR image scan |
| `child-pairing` | child pairing metadata | pairing-client feature alias |
| `child-scan-image` | child image scan | explicitly enables QR image scan dependencies |
| `child-compute` | scalar compute child | compute feature alias |
| `dev-all` | developer validation | all core features plus provider stubs and bench gates |

## CHILD_DEPENDENCY_PRUNING_10A

This cleanup converted core-only dependencies to explicit feature groups and added a `child-min` library cutout so the rich core modules are not compiled when building the child binary.

Feature groups:

- `async-runtime`
- `binary-codec`
- `core-storage-redb`
- `core-storage-sqlite`
- `network-client`
- `terminal-ui`
- `crypto-hash`
- `random`
- `pairing-client`
- `pairing-server`

`core-full` enables the rich operator stack. `child-min` enables none of these core groups.

## Child-Min Command Surface

`quant-m-child` exposes only:

- `quant-m-child pair`
- `quant-m-child pair-scan`
- `quant-m-child identity`
- `quant-m-child doctor`
- `quant-m-child heartbeat`
- `quant-m-child run-once`

It does not expose:

- model commands
- shared-state accept/reject commands
- pairing server commands
- provider commands
- playbook authoring commands
- proposal approval
- execution adapters
- trading or betting commands

## Child Storage

Child storage is file-first:

- `workspace/child/identity.toml`
- `workspace/child/core.toml`
- `workspace/child/playbook-cache/`
- `workspace/child/outbox/`
- `workspace/child/logs/`
- `workspace/child/outbox/heartbeats.jsonl`
- `workspace/child/outbox/job-receipts.jsonl`

No SQLite/redb storage is required by the child command surface.

## Dependency Budget

Target budget:

| Profile | Intended Dependencies |
| --- | --- |
| `child-min` | `anyhow`, `clap`, `serde`, `serde_json`, `toml`, time/hash utility already present |
| `child-scan-image` | `image`, `rqrr` |
| core QR | `qrcode`, `image` |
| core model router | no provider client by default |

Current measured package-level tree for `child-min`:

```text
quant-m
├── anyhow
├── chrono
├── clap
├── serde
├── serde_json
└── toml
```

The following core dependencies are no longer compiled by `child-min`:

- `crossterm`
- `postcard`
- `rand`
- `ratatui`
- `redb`
- `reqwest`
- `ring`
- `rusqlite`
- `tokio`
- `image`
- `qrcode`
- `rqrr`

## Size Measurement

Measured with:

```bash
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo build --bin quant-m-child --profile release-child --no-default-features --features child-min
wc -c /private/tmp/quantm-target/release-child/quant-m-child

CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo build --bin quant-m --features core-full
wc -c /private/tmp/quantm-target/debug/quant-m
```

Current binary sizes:

| Binary | Bytes |
| --- | ---: |
| `quant-m-child` child-min dev before pruning | `4,233,888` |
| `quant-m-child` release-child after pruning | `653,120` |
| `quant-m-child` release-child after device telemetry | `669,728` |
| `quant-m` core CLI dev before pruning | `34,693,752` |
| `quant-m` core-full dev after explicit feature split | `53,214,040` |

## Validation Commands

```bash
cargo fmt --all -- --check
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo test --bin quant-m-child --no-default-features --features child-min
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo check --bin quant-m-child --no-default-features --features child-min
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo clippy --bin quant-m-child --no-default-features --features child-min -- -D warnings
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo build --bin quant-m-child --profile release-child --no-default-features --features child-min
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo tree --no-default-features --features child-min -e normal --depth 1
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo tree --no-default-features --features child-min -e features --depth 2
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo test cluster --features core-full
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo test model_router --features core-full
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/quantm-target CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo clippy --all-targets --features dev-all -- -D warnings
```

Smoke:

```bash
quant-m-child --workspace /private/tmp/quantm-child-smoke identity --create --name tablet-01
quant-m-child --workspace /private/tmp/quantm-child-smoke heartbeat
quant-m-child --workspace /private/tmp/quantm-child-smoke doctor
quant-m-child --workspace /private/tmp/quantm-child-smoke run-once --job reject-job.json
```

The `run-once` smoke must reject non-echo jobs such as `model_handoff`.

## Boundary

The child is a sensor/runner. It can create identity metadata, store pairing metadata, heartbeat to a file-first outbox, and run a tiny echo job. It cannot become a model router, shared-state acceptor, pairing authority, execution adapter, provider caller, desk strategist, or FSM authority.
