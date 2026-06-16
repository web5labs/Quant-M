# Security Policy

Quant-M is a local-first runtime. Treat local config, workspace state, session logs, queues, and provider credentials as sensitive.

## Supported Status

Quant-M is currently a private-preview candidate. Security reports are reviewed for the current `main` branch.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability.

Report privately to the repository owner or Web5 Labs maintainer channel. Include:

- affected version or commit
- operating system
- steps to reproduce
- expected impact
- whether credentials, local files, shell commands, network calls, or worker queues are involved

## Safety Boundaries

By default Quant-M should preserve these boundaries:

- no hidden provider calls during onboarding
- no live trading authority
- no worker proposal auto-acceptance
- no channel message directly executes actions
- no secrets committed to git
- no generated local workspace state exported to GitHub
