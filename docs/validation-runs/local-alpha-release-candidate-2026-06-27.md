# Local Alpha Release Candidate Validation

Date: 2026-06-27

Checkpoint: `LOCAL_ALPHA_RELEASE_CANDIDATE_15`

Status: `v0.local-alpha` is shippable for local lab use. Real-device LAN smoke remains blocked.

## Scope

This validation creates release proof only. It does not add runtime authority, scheduling priority, provider access, proposal approval, canonical writes, live trading, or betting.

## Release Posture

| Target | Verdict |
| --- | --- |
| Internal/local alpha | yes |
| Controlled lab release | yes |
| Fresh-device alpha | almost, after real LAN smoke |
| Public beta | no |
| Production deployment | no |
| Autonomous trading/betting | no |

## Artifact Checklist

| Artifact | Status |
| --- | --- |
| README quickstart | added |
| local alpha release notes | added |
| known limitations | added |
| security boundaries | added |
| feature matrix | added |
| release checklist | added |
| binary size record | added |
| real-device fallback record | blocked, documented |

## Local Validation Commands

Run with temporary target storage so release cleanup can remove build artifacts:

```bash
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo fmt --all -- --check
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo test cluster --features core-full
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo test pairing --features core-full
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo test timing --features core-full
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo test device_telemetry --features core-full
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo test model_router --features core-full
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo check --bin quant-m-child --no-default-features --features child-min
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo clippy --bin quant-m-child --no-default-features --features child-min -- -D warnings
CARGO_TARGET_DIR=/private/tmp/quantm-target cargo clippy --all-targets --features dev-all -- -D warnings
python3 scripts/lint_project_onboarding.py --target .
```

## Results

| Check | Result |
| --- | --- |
| formatting | passed |
| cluster tests | passed: 53 lib tests, 54 main tests |
| pairing tests | passed: 34 lib tests, 34 main tests |
| timing tests | passed: 14 lib tests, 15 main tests |
| device telemetry tests | passed: 14 lib tests, 14 main tests |
| model router tests | passed: 9 lib tests, 9 main tests |
| child-min check | passed |
| child-min clippy | passed |
| dev-all clippy | passed |
| onboarding lint | passed: readiness score 100% |

The first `cargo test cluster --features core-full` attempt hit temporary disk pressure while linking in `/private/tmp/quantm-target`. The temp target was cleaned, freeing about 2 GB, and the validation suite was rerun successfully with `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-Cdebuginfo=0`.

## Binary Size

Latest recorded child-min release-child size:

```text
quant-m-child: 720,112 bytes
```

Measured with:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS=-Cdebuginfo=0 CARGO_TARGET_DIR=/private/tmp/quantm-target cargo build --bin quant-m-child --profile release-child --no-default-features --features child-min
wc -c /private/tmp/quantm-target/release-child/quant-m-child
```

## Real-Device Validation

Real Pi/DietPi plus Termux LAN smoke remains blocked until devices are reachable. The blocked artifact is `docs/validation-runs/pi-termux-lan-2026-06-27.md`.

This blocked artifact is acceptable as a fallback record for a local release candidate, but it is not pass evidence for fresh-device alpha, public beta, or production deployment.

## Final Local-Alpha Decision

`v0.local-alpha` is shippable as a local-lab release.

It must not be described as public beta, production ready, deployment ready, an autonomous trading cluster, or an autonomous betting system.

Release safety values:

| Field | Value |
| --- | --- |
| `proposal_count` | `0` |
| `execution` | `false` |
| `provider_calls_from_children` | `false` |
| `canonical_child_write` | `false` |
| `trading` | `false` |
| `betting` | `false` |

Next required milestone before fresh-device alpha/public beta: `REAL_DEVICE_LAN_SMOKE_15A`.
