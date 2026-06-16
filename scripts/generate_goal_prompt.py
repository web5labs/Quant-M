#!/usr/bin/env python3
"""Generate the Codex slash-goal prompt from current onboarding docs."""

from __future__ import annotations

import argparse
from pathlib import Path


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace").strip() if path.exists() else ""


def has_meaningful_content(text: str) -> bool:
    cleaned = text.replace("_TBD_", "").replace("TBD", "").strip()
    return len(cleaned) >= 80


def build_goal(target: Path) -> str:
    spec = read(target / "docs/project-spec.md")
    shippable = read(target / "docs/definition-of-shippable.md")
    validation = read(target / "docs/validation-plan.md")
    manifest = read(target / "docs/wiki/MANIFEST.md")
    refs = read(target / "docs/wiki/08-reference-repos.md")

    spec_note = (
        "Project spec appears to contain meaningful project detail."
        if has_meaningful_content(spec)
        else "Project spec still appears skeletal; resolve open questions before broad implementation."
    )
    validation_note = (
        "Use the validation commands in docs/validation-plan.md."
        if has_meaningful_content(validation)
        else "Detect available lint, typecheck, test, and build commands before claiming completion."
    )
    manifest_note = (
        "Use docs/wiki/MANIFEST.md as the context router."
        if has_meaningful_content(manifest)
        else "Run ingest before loading raw context if the manifest is empty."
    )
    reference_repo_note = (
        "Reference repo guidance exists; use repo manifests before opening upstream implementation files."
        if has_meaningful_content(refs)
        else "If implementation patterns are missing, record reference repo needs before pulling extra code context."
    )

    return f"""# Generated Codex Goal Prompt

Use this after onboarding is reviewed and approved.

```text
/goal Execute docs/project-spec.md through docs/fsm/project-execution-fsm.md until the functional build reaches READY_FOR_HUMAN_UI_UX_PASS.

Objective:
Build the smallest functional MVP that satisfies docs/project-spec.md and docs/definition-of-shippable.md.

Read first:
- AGENTS.md
- docs/wiki/MANIFEST.md
- docs/project-spec.md
- docs/definition-of-shippable.md
- docs/fsm/project-execution-fsm.md
- docs/codex/execution-plan.md
- docs/codex/reuse-scan.md
- docs/codex/repair-loop.md
- docs/open-questions.md
- docs/codex/blockers.md

Project readiness notes:
- {spec_note}
- {validation_note}
- {manifest_note}
- {reference_repo_note}

Scope includes:
- data model and persistence required by the spec
- API/server wiring required by the spec
- the core user flow
- AI workflow or safe documented stubs when explicitly allowed
- basic output/dashboard surface
- basic loading, error, and empty states
- validation checks and honest blocker reporting

Scope excludes:
- final UI/UX polish
- visual redesign
- animation polish
- unapproved paid APIs
- unapproved growth or marketing pages
- features not listed in the spec
- speculative abstractions not needed for the current shippable version

Development loop:
1. Read the minimum context needed for the current FSM state.
2. Run the reuse scan before adding new services, helpers, adapters, routes, or workers.
3. Define the smallest reviewable slice, list the files you need to inspect, and stop to split scope if the slice grows too large.
4. Implement only the approved slice.
5. Run a structure pass to consolidate duplicated runtime mechanics and improve service or adapter boundaries.
6. Update docs/codex/execution-plan.md and docs/codex/blockers.md with meaningful changes, blockers, and follow-up scope.
7. Run the relevant validation commands and leave behind a durable verifier.
8. If validation or review fails, enter the repair loop and patch only the failing scope.

Context rules:
- Read docs/wiki/MANIFEST.md before loading larger wiki files.
- Use local docs first.
- Use Context7 only if docs are missing, stale, version-sensitive, or needed for implementation correctness.
- Summarize any Context7 findings into docs/wiki/external-docs/.
- Use reference repo findings only as patterns, not copied code.
- Prefer repo manifests and exact-file references over loading entire upstream repos.
- If the slice appears to need more than 8 files, stop and propose a smaller PR boundary.
- Prefer a fresh thread over dragging a degraded, bloated context forward.

Validation:
- Run lint if available.
- Run typecheck if available.
- Run tests if available.
- Run build if available.
- Run python3 scripts/lint_project_onboarding.py --target .
- Add or update a durable verifier such as a unit test, integration test, regression test, smoke test, CLI check, or honest manual checklist.
- Do not claim a command passed unless it was run.

Stop when:
- the functional MVP satisfies docs/project-spec.md,
- docs/definition-of-shippable.md is satisfied,
- available validation commands pass or blockers are documented,
- the current slice has no unresolved duplicate runtime mechanics,
- docs/codex/handoff-to-ui-ux.md is complete,
- follow-up work is documented separately instead of implemented,
- no final UI/UX polish has been attempted.
```
"""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", default=".", help="Target repo root")
    args = parser.parse_args()

    target = Path(args.target).resolve()
    out = target / "docs/codex/goal-prompt.md"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(build_goal(target), encoding="utf-8")
    print(f"Goal prompt generated: {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
