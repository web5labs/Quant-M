# LLM Project Onboarding Framework

This file is a portable onboarding contract for Codex, IDE agents, CLI agents, and LLM coding systems.

Use it at the start of any project to create the foundational project architecture before development begins.

The goal is **not** to build the product immediately.

The goal is to create:

- LLM wiki library,
- raw file wiki folder,
- ingested wiki folder,
- project definition,
- project spec,
- definition of shippable,
- finite state machine execution model,
- Context7 fallback protocol,
- reference repo ingestion protocol,
- lint/validation scripts,
- reuse-scan and repair-loop rails,
- Codex `/goal` prompt,
- UI/UX handoff scaffold.

---

## Invocation

```text
@LLM_PROJECT_ONBOARDING.md

Use this onboarding framework for the current project.
Create the required project architecture, wiki scaffold, raw file wiki folder, ingested wiki folder, project spec scaffold, definition-of-shippable scaffold, FSM execution controller, Context7 lookup protocol, repo-ingestion protocol, lint/validation scripts, Codex goal prompt scaffold, and UI/UX handoff scaffold.
Do not implement application features yet.
Stop when the project is READY_FOR_FUNCTIONAL_BUILD_GOAL.
```

---

## Core principle

Do not ask the model to build from a giant unstructured prompt.

Force the project through this sequence:

```text
Idea / existing repo
  -> context discovery
  -> LLM wiki scaffold
  -> raw file ingestion
  -> normalized wiki summaries
  -> context gap audit
  -> Context7 docs fallback, if needed
  -> reference repo ingestion, if needed
  -> project definition
  -> project spec
  -> definition of shippable
  -> project execution FSM
  -> generated Codex /goal prompt
  -> functional MVP build
  -> validation
  -> UI/UX handoff
  -> separate UI/UX goal
```

---

## Agent role

You are the project onboarding agent.

Your job is to prepare the project foreground. You must avoid premature implementation. You must create clear rails, context, state machines, validation scripts, and handoff documents so the project can be built without drift.

Prioritize:

- project clarity,
- token efficiency,
- durable context,
- smallest reviewable slices,
- reuse before reinvention,
- implementation readiness,
- human review checkpoints,
- shippable definition,
- validation,
- UI/UX separation.

---

## Required output architecture

Create missing files and folders only. Preserve existing project docs.

```text
AGENTS.md
LLM_PROJECT_ONBOARDING.md

.agents/
  skills/
    model-project-onboarding/
      SKILL.md

scripts/
  bootstrap_project_onboarding.py
  ingest_wiki.py
  lint_project_onboarding.py

docs/
  README.md
  project-definition.md
  project-spec.md
  definition-of-shippable.md
  assumptions.md
  non-goals.md
  open-questions.md
  validation-plan.md

  wiki/
    MANIFEST.md
    raw/
      README.md
    ingested/
      README.md
    external-docs/
      README.md
    repo-ingest/
      README.md
    08-reference-repos.md

  fsm/
    project-execution-fsm.md

  codex/
    execution-plan.md
    goal-prompt.md
    reuse-scan.md
    repair-loop.md
    blockers.md
    handoff-to-ui-ux.md
```

---

## LLM wiki rules

The wiki is the project's persistent memory.

```text
docs/wiki/raw/          original source materials
docs/wiki/ingested/     normalized summaries
docs/wiki/MANIFEST.md   compressed context map
docs/wiki/external-docs/ Context7 summaries
docs/wiki/repo-ingest/  reference repo pattern summaries
```

Rules:

1. Read `docs/wiki/MANIFEST.md` first.
2. Load only the files needed for the active state.
3. Do not repeatedly load the full wiki.
4. If a task appears to need too many files, split the slice before implementation.
5. Preserve raw files exactly.
6. Normalize raw files into ingested summaries.
7. Every ingested file must include source path, date, summary, implementation relevance, risks, and open questions.
8. Update the manifest whenever new raw or ingested files are added.

---

## Context7 fallback protocol

Use Context7 only when local docs are not enough.

Before using Context7, check:

```text
docs/wiki/MANIFEST.md
docs/wiki/external-docs/
package.json
lockfiles
```

Use Context7 when documentation is:

- missing,
- stale,
- version-sensitive,
- integration-critical,
- or likely to have changed.

When used, summarize the relevant docs into:

```text
docs/wiki/external-docs/[library-or-framework].md
```

Each summary must include:

- library/framework name,
- version,
- lookup reason,
- date,
- relevant APIs,
- implementation notes,
- gotchas,
- validation commands,
- source/tool used.

Do not dump full external docs.

---

## Reference repo ingestion protocol

Use reference repos only for implementation patterns.

Do not copy code blindly.

If an approved repo needs local inspection, use a source-reference tool such as:

```bash
npx opensrc fetch <owner>/<repo>
npx opensrc path <owner>/<repo>
```

Treat fetched source as evidence. Summarize patterns and exact files to inspect; do not vendor the source or turn it into hidden scope.

Write findings to:

```text
docs/wiki/repo-ingest/[repo-name].md
```

Each repo-ingest summary must include:

- repo name and URL,
- license,
- why it is relevant,
- architecture patterns,
- folder structure,
- exact files to inspect first,
- routing/API patterns,
- data model patterns,
- testing patterns,
- security notes,
- what not to copy,
- how it affects the current project spec.

---

## Project execution FSM

Use this onboarding FSM:

```text
BOOTSTRAP_REQUESTED
-> REPO_INSPECTED
-> CONTEXT_DISCOVERED
-> WIKI_SCAFFOLDED
-> RAW_FILES_INGESTED
-> CONTEXT_GAPS_IDENTIFIED
-> EXTERNAL_DOCS_RESOLVED
-> REPO_PATTERNS_RESOLVED
-> PROJECT_SPEC_DRAFTED
-> SHIPPABLE_DEFINITION_DRAFTED
-> FSM_READY
-> VALIDATION_READY
-> GOAL_PROMPT_READY
-> READY_FOR_FUNCTIONAL_BUILD_GOAL
```

Do not advance states without exit criteria.

---

## Definition of shippable

A functional build is shippable only when:

- the core user flow works end-to-end,
- required data persists,
- API/server wiring exists,
- the AI workflow works or is safely stubbed,
- basic loading/error/empty states exist,
- validation checks pass or unavailable checks are documented,
- UI/UX polish is explicitly deferred,
- no final subjective design pass has been attempted.

---

## Functional build vs UI/UX pass

The first implementation goal wires the product functionally.

Allowed in functional build:

- routes/pages,
- forms,
- API/server actions,
- data model,
- auth/billing wiring if specified,
- AI workflow,
- dashboard/output surface,
- basic loading/error/empty states,
- tests and validation.

Deferred to UI/UX pass:

- final brand system,
- visual redesign,
- animation polish,
- advanced layout polish,
- subjective interaction design,
- final copy polish unless required by the spec.

---

## Required lint checks

Create and run `scripts/lint_project_onboarding.py`.

It must check:

- required files exist,
- manifest exists,
- raw and ingested wiki folders exist,
- project spec has required headings,
- definition of shippable has pass/fail criteria,
- FSM has states and exit criteria,
- goal prompt has objective, scope, validation, and stop condition,
- UI/UX polish is deferred,
- docs do not contain obvious raw secrets.

---

## Stop condition

Stop at:

```text
READY_FOR_FUNCTIONAL_BUILD_GOAL
```

Do not implement app features yet.
