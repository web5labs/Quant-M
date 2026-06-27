# Local Alpha Release Checklist

Checkpoint: `LOCAL_ALPHA_RELEASE_CANDIDATE_15`.

Goal: produce release artifacts and proof without adding runtime authority.

## Required Artifacts

- [x] README quickstart
- [x] `docs/local-alpha-release-notes.md`
- [x] `docs/known-limitations.md`
- [x] `docs/security-boundaries.md`
- [x] `docs/validation-runs/local-alpha-release-candidate-2026-06-27.md`
- [x] release checklist
- [x] binary size record
- [x] feature matrix

## Required Local Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo test cluster --features core-full`
- [x] `cargo test pairing --features core-full`
- [x] `cargo test timing --features core-full`
- [x] `cargo test device_telemetry --features core-full`
- [x] `cargo test model_router --features core-full`
- [x] `cargo check --bin quant-m-child --no-default-features --features child-min`
- [x] `cargo clippy --bin quant-m-child --no-default-features --features child-min -- -D warnings`
- [x] `cargo clippy --all-targets --features dev-all -- -D warnings`
- [x] `python3 scripts/lint_project_onboarding.py --target .`

## Required Real-Device Proof

- [ ] Pi/DietPi or laptop fallback core on LAN
- [ ] Termux child on LAN
- [ ] pair
- [ ] approve
- [ ] heartbeat
- [ ] telemetry visible
- [ ] observe lease
- [ ] echo evidence
- [ ] scalar evidence
- [ ] cluster report
- [ ] `proposal_count=0`
- [ ] `execution=false`
- [ ] `provider_calls=false`

Current status: blocked until real devices are reachable. See `docs/validation-runs/pi-termux-lan-2026-06-27.md`.

This blocks fresh-device alpha, public beta, and production claims. It does not block shipping `v0.local-alpha` as a local-lab release when labeled honestly.

## Ship Decision

Allowed label:

```text
v0.local-alpha
```

Alternative allowed label:

```text
v0.1.0-local-alpha
```

Blocked labels:

- `v0.public-beta`
- production ready
- autonomous trading cluster
- deployment ready
