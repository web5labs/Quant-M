---
name: model-project-onboarding
description: Bootstrap a new or existing software project for agentic development by creating an LLM wiki library, raw file wiki folder, ingested wiki folder, project spec, definition of shippable, FSM execution controller, Context7 fallback protocol, repo-ingestion protocol, lint/validation scripts, Codex goal prompt, and UI/UX handoff. Use before implementation. Do not use for ordinary coding tasks.
---

# Model Project Onboarding Skill

Use this skill when the user wants to prepare a project for Codex, an IDE agent, or a CLI coding agent before implementation begins.

Your job is to create the project foreground:

- durable context files,
- LLM wiki library,
- raw source folder,
- ingested/normalized wiki folder,
- project definition,
- source-of-truth project spec,
- definition of shippable,
- finite state machine execution controller,
- Context7 documentation fallback protocol,
- reference repo ingestion protocol,
- validation/lint plan,
- reuse-scan and repair-loop rails,
- generated Codex `/goal` prompt,
- UI/UX handoff scaffold.

Do not implement application features during this skill unless the user explicitly asks to move into the functional build phase.

## Required behavior

1. Inspect the repo.
2. Identify existing docs, source folders, package manager, stack, config, and context files.
3. Preserve existing work.
4. Create missing onboarding structure.
5. Use the wiki first.
6. Use Context7 only when framework/API docs are missing, stale, or version-sensitive.
7. Ingest reference repos only when necessary and summarize patterns instead of copying code.
8. Add reuse-scan and repair-loop rails so future build work stays bounded.
9. Separate functional UI wiring from final UI/UX polish.
10. Create a human-review checkpoint before implementation.
11. Run onboarding lint.
12. Stop after onboarding and report what was created.

## Required files

Create or update:

```text
AGENTS.md
LLM_PROJECT_ONBOARDING.md
docs/project-definition.md
docs/project-spec.md
docs/definition-of-shippable.md
docs/wiki/MANIFEST.md
docs/wiki/raw/README.md
docs/wiki/ingested/README.md
docs/wiki/external-docs/README.md
docs/wiki/repo-ingest/README.md
docs/fsm/project-execution-fsm.md
docs/fsm/product-state-machines.md
docs/codex/execution-plan.md
docs/codex/goal-prompt.md
docs/codex/reuse-scan.md
docs/codex/repair-loop.md
docs/codex/handoff-to-ui-ux.md
scripts/ingest_wiki.py
scripts/lint_project_onboarding.py
```

## Output contract

At completion, provide:

```text
Onboarding status:
Current FSM state:
Files created:
Files updated:
Wiki status:
Context gaps:
External docs needed:
Reference repos needed:
Lint results:
Generated /goal prompt location:
Human review required:
Recommended next command:
```
