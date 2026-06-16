# Project Execution FSM

This FSM controls project onboarding and functional build execution.

## Transition rule

Do not advance to the next state until the current state's exit criteria are met.

If validation fails, enter a repair loop for the current state.

If human judgment is required, document it in `docs/open-questions.md` or `docs/codex/blockers.md`.

## Repair loop rule

When a slice fails validation or review:

1. Read the failing diff or failing scope only.
2. Read test failures and reviewer feedback.
3. Patch only the failing scope.
4. Add or update a durable verifier when practical.
5. Re-run validation.
6. Stop when the threshold is met or document the blocker.

## States

### BOOTSTRAP_REQUESTED

Goal: Start onboarding.

Exit criteria:
- Framework file has been read.
- Repo root has been identified.

### REPO_INSPECTED

Goal: Inspect current repo.

Exit criteria:
- Existing docs, source folders, package files, and config files are listed.
- Package manager and likely stack are identified when possible.

### CONTEXT_DISCOVERED

Goal: Identify existing project context.

Exit criteria:
- Existing project docs are mapped.
- Missing context is listed.

### WIKI_SCAFFOLDED

Goal: Create LLM wiki structure.

Exit criteria:
- `docs/wiki/MANIFEST.md` exists.
- raw, ingested, external-docs, and repo-ingest folders exist.

### RAW_FILES_INGESTED

Goal: Normalize raw source materials.

Exit criteria:
- Raw files are preserved.
- Ingested summaries exist or placeholders document what must be summarized.
- Manifest is updated.

### CONTEXT_GAPS_IDENTIFIED

Goal: Identify what is missing before spec creation.

Exit criteria:
- Context gaps are listed in the manifest and open questions.

### EXTERNAL_DOCS_RESOLVED

Goal: Resolve missing/stale framework/API docs.

Exit criteria:
- Context7 needed/not needed is documented.
- If used, summaries are saved to `docs/wiki/external-docs/`.

### REPO_PATTERNS_RESOLVED

Goal: Ingest reference repo patterns if needed.

Exit criteria:
- Reference repo needs are documented.
- Repo manifests exist for approved repos.

### PROJECT_SPEC_DRAFTED

Goal: Create source-of-truth project spec.

Exit criteria:
- `docs/project-spec.md` contains required sections.
- Unknowns are listed.

### SHIPPABLE_DEFINITION_DRAFTED

Goal: Define done/shippable.

Exit criteria:
- `docs/definition-of-shippable.md` has pass/fail criteria.
- Human review checkpoint exists.

### FSM_READY

Goal: Prepare execution controls.

Exit criteria:
- Project execution FSM exists.
- Product/runtime FSM candidates are listed.

### VALIDATION_READY

Goal: Prepare lint and validation.

Exit criteria:
- `scripts/lint_project_onboarding.py` exists.
- `scripts/ingest_wiki.py` exists.
- `docs/validation-plan.md` exists.

### GOAL_PROMPT_READY

Goal: Generate Codex build goal.

Exit criteria:
- `docs/codex/goal-prompt.md` exists.
- It includes objective, scope, validation, stop condition, reuse scan, and UI/UX deferral.

### READY_FOR_FUNCTIONAL_BUILD_GOAL

Goal: Stop onboarding and prepare for human review.

Exit criteria:
- Onboarding lint passes or blockers are documented.
- No application features were implemented.
- Recommended next command is provided.

### GOAL_CONTEXT_BOUNDED

Goal: Start a safe feature slice.

Exit criteria:
- Required files for the slice are listed.
- Context boundary is small enough to review confidently.

### REUSE_SCAN_COMPLETE

Goal: Identify existing reusable logic before implementation.

Exit criteria:
- Existing services, helpers, adapters, workers, and routes were checked.
- New parallel runtime mechanics are justified if introduced.

### FEATURE_SLICE_PLANNED

Goal: Define the smallest reviewable implementation slice.

Exit criteria:
- Files likely to change are listed.
- Validation plan for the slice is listed.
- Follow-up work is separated from current scope.

### FEATURE_SLICE_IMPLEMENTED

Goal: Implement the approved slice.

Exit criteria:
- Only the approved slice was implemented.
- Decisions and blockers are updated.

### STRUCTURE_PASS_COMPLETE

Goal: Remove duplication and improve re-entry quality.

Exit criteria:
- Repeated runtime mechanics were consolidated or intentionally documented.
- Service, adapter, and domain boundaries were reviewed.

### VALIDATION_PASSED

Goal: Verify the slice.

Exit criteria:
- Relevant validation commands were run.
- The slice leaves behind a durable verifier or an honest manual checklist.

### REPAIR_LOOP_READY

Goal: Enter bounded repair only if needed.

Exit criteria:
- Failing scope is identified.
- Repair work is limited to the failing scope.

### READY_FOR_HUMAN_UI_UX_PASS

Goal: Finish the functional build phase and hand off polish separately.

Exit criteria:
- Functional shippable definition is satisfied.
- Follow-up work is documented but not implemented.
- UI/UX handoff is complete.
