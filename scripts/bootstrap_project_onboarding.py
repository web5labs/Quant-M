#!/usr/bin/env python3
"""Bootstrap the Codex Project Onboarding Framework into a target repo.

The checked-in docs are the source of truth. This script copies those templates
instead of carrying a second copy of the same markdown in Python strings.
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

TEMPLATE_FILES = [
    "AGENTS.md",
    "LLM_PROJECT_ONBOARDING.md",
    "docs/README.md",
    "docs/project-definition.md",
    "docs/project-spec.md",
    "docs/definition-of-shippable.md",
    "docs/assumptions.md",
    "docs/non-goals.md",
    "docs/open-questions.md",
    "docs/validation-plan.md",
    "docs/wiki/MANIFEST.md",
    "docs/wiki/raw/README.md",
    "docs/wiki/ingested/README.md",
    "docs/wiki/external-docs/README.md",
    "docs/wiki/repo-ingest/README.md",
    "docs/wiki/08-reference-repos.md",
    "docs/contracts/README.md",
    "docs/contracts/ponboarding-to-staff-os-handoff.md",
    "docs/contracts/ponboarding-handoff.schema.json",
    "docs/contracts/repo-scorecard.schema.json",
    "docs/contracts/model-stack-recommendation.schema.json",
    "docs/fsm/project-execution-fsm.md",
    "docs/codex/execution-plan.md",
    "docs/codex/intake-summary.md",
    "docs/codex/goal-prompt.md",
    "docs/codex/blockers.md",
    "docs/codex/reuse-scan.md",
    "docs/codex/repair-loop.md",
    "docs/codex/handoff-to-ui-ux.md",
]

SUPPORT_SCRIPTS = [
    "bootstrap_project_onboarding.py",
    "ingest_wiki.py",
    "generate_goal_prompt.py",
    "lint_project_onboarding.py",
]


def copy_file(source: Path, destination: Path, force: bool) -> str:
    if source.resolve() == destination.resolve():
        return "skipped"
    if destination.exists() and not force:
        return "skipped"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    return "written"


def copy_tree(source: Path, destination: Path, force: bool) -> tuple[int, int]:
    created = skipped = 0
    for path in source.rglob("*"):
        if not path.is_file():
            continue
        result = copy_file(path, destination / path.relative_to(source), force)
        created += result == "written"
        skipped += result == "skipped"
    return created, skipped


def copy_template_files(source_root: Path, target: Path, force: bool) -> tuple[int, int]:
    created = skipped = 0
    for rel in TEMPLATE_FILES:
        source = source_root / rel
        if not source.exists():
            raise FileNotFoundError(f"Missing template file: {source}")
        result = copy_file(source, target / rel, force)
        created += result == "written"
        skipped += result == "skipped"
    return created, skipped


def copy_support_files(source_root: Path, target: Path, force: bool) -> tuple[int, int]:
    created = skipped = 0

    for name in SUPPORT_SCRIPTS:
        result = copy_file(source_root / "scripts" / name, target / "scripts" / name, force)
        created += result == "written"
        skipped += result == "skipped"

    return created, skipped


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", default=".", help="Target repo root")
    parser.add_argument("--force", action="store_true", help="Overwrite existing files")
    args = parser.parse_args()

    target = Path(args.target).resolve()
    source_root = Path(__file__).resolve().parents[1]

    template_created, template_skipped = copy_template_files(source_root, target, args.force)
    support_created, support_skipped = copy_support_files(source_root, target, args.force)

    created = template_created + support_created
    skipped = template_skipped + support_skipped
    print(f"Bootstrap complete: created/updated={created}, skipped={skipped}, target={target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
