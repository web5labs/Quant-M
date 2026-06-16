# Reference Repos

Use this file to declare which external repos should be mined for patterns.

## Approved repos

- `openclaw/openclaw`
- `nearai/ironclaw`
- `paperclipai/paperclip`
- `NousResearch/hermes-agent`

## Candidate repos

- Additional lightweight Rust automation runtimes, if the current module boundaries prove insufficient.

## Current note

The approved references are public upstreams or ecosystem anchors to inspect when Quant-M needs runtime-pattern evidence. They should remain pattern references, not scope mandates.

Local copied repos, private workspace folders, and raw source dumps must not be committed as reference material. Summarize relevant patterns in `docs/wiki/repo-ingest/` instead.

## Source-code fetch protocol

For approved repos, prefer a local source-reference snapshot instead of ad-hoc browser context:

```bash
npx opensrc fetch <owner>/<repo>
npx opensrc path <owner>/<repo>
```

Use the returned path only to inspect patterns. Summarize findings in `docs/wiki/repo-ingest/`; do not copy implementation wholesale or vendor the reference repo into the project.

## Per-repo checklist

For each repo, record:

- repo name
- URL
- license
- why it is relevant
- patterns to borrow
- files to inspect first
- what not to copy

## Repo manifest structure

Each approved repo should get:

```text
docs/wiki/repo-ingest/<repo-slug>/
  repo-map.md
  useful-patterns.md
  files-to-reference.md
```
