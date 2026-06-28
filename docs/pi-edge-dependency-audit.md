# Raspberry Pi Edge Dependency Audit

This document records the dependency boundary for installing Quant-M on Raspberry Pi / DietPi during `v0.local-alpha`.

The goal is to separate:

- what is needed to prepare and build Quant-M from source
- what is needed to run Quant-M after a binary exists
- what is optional operator convenience
- what is not used by the edge child runtime

## Observed Pi Prep Tools

During the first DietPi install, the operator needed:

| Tool | Why it was used | Needed after build? | Notes |
| --- | --- | --- | --- |
| `openssh-server` | remote terminal access from laptop to Pi | optional, but recommended for headless Pi | Quant-M does not require SSH to run; it is operator access only |
| `curl` | install Rust via `rustup` | optional after Rust is installed | keep if you want easy updates; not needed by `quant-m-child` runtime |
| `git` | clone/pull the Quant-M repo | optional after binary exists | keep if the Pi will pull release branch updates |
| `cargo` / `rustc` from apt | first attempted Rust install | no, if rustup is installed | apt Rust 1.85 was too old for current dependencies |
| `rustup` cargo/rustc | current Rust toolchain | needed only to build/update from source | required for source installs; not needed to run an already-built binary |
| `gcc` / libc headers | build native Rust crate dependencies | needed only while building | core builds can need native compilation; child-min is much lighter |

## Preferred Rust Toolchain

Use `rustup` as the primary Rust installation:

```bash
source "$HOME/.cargo/env"
rustc --version
cargo --version
which rustc
which cargo
```

Expected:

```text
/root/.cargo/bin/rustc
/root/.cargo/bin/cargo
```

The Debian/DietPi `apt install cargo` path installed `rustc 1.85.0`, which is not sufficient for the current core dependency graph.

## Runtime-Only Requirements

After binaries are built, running the child requires only the binary plus normal Linux runtime libraries:

```text
target/release-child/quant-m-child
workspace/
```

The child-min runtime does not need:

- `git`
- `curl`
- `cargo`
- `rustc`
- `gcc`
- `openssh-server`

Those tools are useful for install, updates, or remote administration, not for child execution.

For a Pi core node, `quant-m` itself can run after build, but keeping `git`, `rustup`, and build tools is useful while local-alpha is still moving quickly.

## Cargo Feature Boundary

Child-min is intentionally small.

Child-min normal dependencies:

```text
anyhow
chrono
clap
serde
serde_json
toml
```

Child-min should not compile or require:

```text
tokio
reqwest
rusqlite
redb
ratatui
crossterm
postcard
rand
ring
image
qrcode
rqrr
provider adapters
model router
shared-state accept/reject storage
pairing server
execution adapters
trading/betting commands
```

Core-full may use heavier dependencies because the core owns pairing server, QR rendering, shared state, model stubs, timing, and governance surfaces.

## Safe Debloat Policy

Do not remove packages until the binary you need is built and tested.

Recommended order:

```bash
./quantm core-build        # if this Pi is the core
./quantm child-build       # if this Pi is a child worker
./quantm child doctor      # child sanity check
```

Then inspect:

```bash
bash scripts/pi_dependency_audit.sh
```

Conservative cleanup:

```bash
bash scripts/pi_lean_cleanup.sh --dry-run
bash scripts/pi_lean_cleanup.sh --apply
```

This removes repo-local build/runtime churn only. It does not purge apt packages.

## Optional Apt Purge Guidance

Only after `rustup` works and the needed binary has been built, the apt Rust packages can be removed because they are not the active toolchain:

```bash
apt purge cargo rustc libstd-rust-1.85 libstd-rust-dev
apt autoremove --purge
```

Keep `git` if the Pi will pull updates.

Keep `curl` if the Pi will refresh rustup or download install assets.

Keep `openssh-server` if the Pi is headless or managed over LAN.

Keep compiler/build packages while local-alpha is changing and source builds are expected.

## Quant-M Safety Boundary

Package cleanup does not change Quant-M authority.

Removing build tools must not enable:

- provider calls from children
- proposal approval
- execution
- trading
- betting
- canonical child writes

