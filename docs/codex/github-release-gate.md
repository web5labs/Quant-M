# GitHub Release Gate

## Purpose

This gate defines when Quant-M is ready to be initialized as a GitHub repository, pushed to `web5labs`, and prepared for remote deployment.

GitHub authentication is now a prerequisite that has been satisfied for this machine, but authentication alone does not make the project shippable.

## Current GitHub Auth State

- GitHub account: `web5labs`
- Target repository name: `Quant-M`
- Git author email: `wen5labs.llc@gmail.com`
- First deployment family: edge devices and laptops, with deployment docs expected to cover constrained nodes and ordinary desktop/laptop runs before heavier control-plane deployment.
- SSH auth: verified with `ssh -T git@github.com`
- Verified response: `Hi web5labs! You've successfully authenticated, but GitHub does not provide shell access.`
- Local repo state: `quantm` is not currently initialized as a git repository.

## Release Boundary

Do not push for remote deployment until:

- disk space is sufficient for Rust validation: passed after removing inactive Rust `1.92-aarch64-apple-darwin`
- Rust formatting passes: passed
- focused Context Firewall tests pass: passed
- full test suite passes, or any unavailable checks are explicitly accepted as release blockers: passed
- clippy passes, or release blockers are recorded: passed
- build passes: passed
- onboarding lint passes: passed
- top-level `.gitignore` protects runtime databases, queues, sessions, logs, local packet artifacts, build outputs, and local reference copies
- git identity is configured: selected as `web5labs <wen5labs.llc@gmail.com>`
- target repository name is selected: `web5labs/Quant-M`
- human approval is recorded for the first push

## Required Validation

Run:

```bash
cargo fmt --all -- --check
cargo test context_firewall
cargo test context_packet_command
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
python3 scripts/ingest_wiki.py --target .
python3 scripts/generate_goal_prompt.py --target .
python3 scripts/lint_project_onboarding.py --target .
```

If validation cannot run because of local disk limits, the release state remains blocked until disk space is repaired or the operator explicitly approves a reduced validation release.

Latest validation run:

- `cargo fmt --all -- --check`: passed
- `cargo test context_firewall`: passed
- `cargo test context_packet_command`: passed
- `cargo clippy --all-targets -- -D warnings`: passed
- `cargo test`: passed
- `cargo build`: passed
- `python3 scripts/ingest_wiki.py --target .`: passed
- `python3 scripts/generate_goal_prompt.py --target .`: passed
- `python3 scripts/lint_project_onboarding.py --target .`: passed

## Git Setup Steps

After validation passes:

```bash
git init
git config user.name "web5labs"
git config user.email "wen5labs.llc@gmail.com"
git add .
git commit -m "Prepare Quant-M shippable runtime"
git branch -M main
git remote add origin git@github.com:web5labs/Quant-M.git
git push -u origin main
```

Use a local repo config first unless the operator wants global git identity settings.

## Remote Deployment Preparation

Remote deployment is a separate milestone after GitHub push.

Before remote deployment:

- choose target: edge devices and laptops first, including Raspberry Pi/Android-Termux-style constrained nodes and ordinary desktop/laptop runs
- define required system packages
- define service user and install path
- define secrets policy
- define `quant-m.toml` production override strategy
- define systemd or terminal-cockpit process model
- add deployment smoke checks

## Current Blockers

- Human approval for `git init`, first commit, remote creation/selection, and first push is still required.
- Disk is still tight after validation, so avoid unnecessary rebuilds until more cache cleanup is approved.
- GitHub remote repository `web5labs/Quant-M` still needs to exist before first push.

## Next Safe Action

Initialize the repository and prepare the first commit without pushing until the remote repository exists and final human approval is recorded.
