#!/usr/bin/env python3
"""Lint the Codex Project Onboarding Framework files.

Checks that the onboarding docs are complete enough to hand to a coding agent.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

REQUIRED_FILES = [
    "AGENTS.md",
    "LLM_PROJECT_ONBOARDING.md",
    "docs/project-definition.md",
    "docs/project-spec.md",
    "docs/definition-of-shippable.md",
    "docs/wiki/MANIFEST.md",
    "docs/wiki/raw/README.md",
    "docs/wiki/ingested/README.md",
    "docs/wiki/external-docs/README.md",
    "docs/wiki/repo-ingest/README.md",
    "docs/wiki/08-reference-repos.md",
    "docs/fsm/project-execution-fsm.md",
    "docs/codex/execution-plan.md",
    "docs/codex/intake-summary.md",
    "docs/codex/goal-prompt.md",
    "docs/codex/reuse-scan.md",
    "docs/codex/repair-loop.md",
    "docs/codex/handoff-to-ui-ux.md",
    "scripts/ingest_wiki.py",
    "scripts/generate_goal_prompt.py",
    "scripts/lint_project_onboarding.py",
]

PROJECT_SPEC_HEADINGS = [
    "Product summary",
    "Target user",
    "Core problem",
    "MVP outcome",
    "User stories",
    "Core user flow",
    "Human intent lock",
    "Functional requirements",
    "Non-functional requirements",
    "Tech stack",
    "API and integration plan",
    "Data model",
    "Routes and pages",
    "AI workflow",
    "Tests and validation",
    "Preservation rules",
    "Non-goals",
    "UI/UX deferred scope",
    "Definition of shippable",
    "Deferred follow-ups",
]

SECRET_PATTERNS = [
    re.compile(r"sk-[A-Za-z0-9_\-]{20,}"),
    re.compile(r"(?i)(api[_-]?key|secret|token)\s*[:=]\s*['\"]?[A-Za-z0-9_\-]{24,}"),
    re.compile(r"(?i)password\s*[:=]\s*['\"]?[^\s]{8,}"),
]


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def check_required_files(target: Path) -> list[str]:
    errors = []
    for rel in REQUIRED_FILES:
        if not (target / rel).exists():
            errors.append(f"Missing required file: {rel}")
    return errors


def check_project_spec(target: Path) -> list[str]:
    text = read(target / "docs/project-spec.md")
    errors = []
    for heading in PROJECT_SPEC_HEADINGS:
        if heading.lower() not in text.lower():
            errors.append(f"project-spec.md missing heading containing: {heading}")
    return errors


def check_shippable(target: Path) -> list[str]:
    text = read(target / "docs/definition-of-shippable.md").lower()
    required = ["functional shippable", "not shippable", "human review", "durable verifier"]
    return [f"definition-of-shippable.md missing section or concept: {r}" for r in required if r not in text]


def check_fsm(target: Path) -> list[str]:
    text = read(target / "docs/fsm/project-execution-fsm.md").lower()
    required = [
        "state",
        "exit criteria",
        "ready_for_functional_build_goal",
        "ready_for_human_ui_ux_pass",
        "reuse_scan_complete",
        "repair_loop_ready",
    ]
    return [f"project-execution-fsm.md missing concept: {r}" for r in required if r not in text]


def check_goal_prompt(target: Path) -> list[str]:
    text = read(target / "docs/codex/goal-prompt.md").lower()
    required = ["/goal", "objective", "scope", "validation", "stop", "reuse scan", "durable verifier"]
    return [f"goal-prompt.md missing concept: {r}" for r in required if r not in text]


def check_execution_plan(target: Path) -> list[str]:
    text = read(target / "docs/codex/execution-plan.md").lower()
    required = ["smallest reviewable slice", "reuse scan", "context budget", "structure pass", "stop conditions"]
    return [f"execution-plan.md missing concept: {r}" for r in required if r not in text]


def check_repair_docs(target: Path) -> list[str]:
    errors = []
    reuse_scan = read(target / "docs/codex/reuse-scan.md").lower()
    if "duplicate" not in reuse_scan and "reuse" not in reuse_scan:
        errors.append("reuse-scan.md should explain duplicate-risk or reuse behavior")

    repair_loop = read(target / "docs/codex/repair-loop.md").lower()
    required = ["failing scope", "patch only", "regression", "re-run"]
    errors.extend([f"repair-loop.md missing concept: {item}" for item in required if item not in repair_loop])
    return errors


def check_reference_repo_doc(target: Path) -> list[str]:
    text = read(target / "docs/wiki/08-reference-repos.md").lower()
    required = ["approved repos", "candidate repos", "repo-map", "files-to-reference"]
    return [f"08-reference-repos.md missing concept: {r}" for r in required if r not in text]


def check_wiki_manifest(target: Path) -> list[str]:
    text = read(target / "docs/wiki/MANIFEST.md").lower()
    required = ["raw", "ingested", "external-docs", "repo-ingest", "context budget", "reference repos"]
    return [f"MANIFEST.md missing wiki area or concept: {r}" for r in required if r not in text]


def check_uiux_deferred(target: Path) -> list[str]:
    combined = "\n".join(
        read(target / rel).lower()
        for rel in [
            "docs/project-spec.md",
            "docs/definition-of-shippable.md",
            "docs/codex/goal-prompt.md",
            "docs/codex/handoff-to-ui-ux.md",
        ]
    )
    if "ui/ux" not in combined or "defer" not in combined:
        return ["UI/UX deferral is not explicit across spec/shippable/goal/handoff docs"]
    return []


def check_secrets(target: Path) -> list[str]:
    errors = []
    for path in target.rglob("*"):
        if not path.is_file():
            continue
        if any(part in {".git", "node_modules", ".venv", "venv", "dist", "build"} for part in path.parts):
            continue
        if path.suffix.lower() not in {".md", ".txt", ".py", ".json", ".yml", ".yaml", ".toml", ".env"}:
            continue
        text = read(path)
        for pattern in SECRET_PATTERNS:
            if pattern.search(text):
                errors.append(f"Possible raw secret in {path.relative_to(target)}")
                break
    return errors


def is_placeholder(text: str) -> bool:
    stripped = text.strip()
    if not stripped:
        return True
    placeholders = ["_TBD_", "TBD", "_None yet._", "_Run ingest._"]
    return any(marker in stripped for marker in placeholders)


def readiness_report(target: Path) -> tuple[int, list[str]]:
    checks = [
        ("project definition has content", not is_placeholder(read(target / "docs/project-definition.md"))),
        ("project spec has content", not is_placeholder(read(target / "docs/project-spec.md"))),
        (
            "definition of shippable has pass/fail criteria",
            not is_placeholder(read(target / "docs/definition-of-shippable.md")),
        ),
        ("wiki manifest has ingestion index", "ingestion index" in read(target / "docs/wiki/MANIFEST.md").lower()),
        ("reference repo protocol exists", not is_placeholder(read(target / "docs/wiki/08-reference-repos.md"))),
        ("reuse scan exists", (target / "docs/codex/reuse-scan.md").exists()),
        ("repair loop exists", (target / "docs/codex/repair-loop.md").exists()),
        ("open questions reviewed", not is_placeholder(read(target / "docs/open-questions.md"))),
        ("assumptions reviewed", not is_placeholder(read(target / "docs/assumptions.md"))),
        ("validation plan has commands", "```" in read(target / "docs/validation-plan.md")),
        ("goal prompt generated", "/goal" in read(target / "docs/codex/goal-prompt.md")),
        ("UI/UX handoff exists", (target / "docs/codex/handoff-to-ui-ux.md").exists()),
    ]
    passed = sum(1 for _, ok in checks if ok)
    score = round((passed / len(checks)) * 100)
    missing = [label for label, ok in checks if not ok]
    return score, missing


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", default=".", help="Target repo root")
    args = parser.parse_args()

    target = Path(args.target).resolve()
    errors: list[str] = []
    errors.extend(check_required_files(target))
    errors.extend(check_wiki_manifest(target))
    errors.extend(check_reference_repo_doc(target))
    errors.extend(check_project_spec(target))
    errors.extend(check_shippable(target))
    errors.extend(check_fsm(target))
    errors.extend(check_goal_prompt(target))
    errors.extend(check_execution_plan(target))
    errors.extend(check_repair_docs(target))
    errors.extend(check_uiux_deferred(target))
    errors.extend(check_secrets(target))
    score, missing = readiness_report(target)

    if errors:
        print("Onboarding lint failed:\n")
        for error in errors:
            print(f"- {error}")
        print(f"\nReadiness score: {score}%")
        if missing:
            print("Readiness gaps:")
            for item in missing:
                print(f"- {item}")
        return 1

    print(f"Readiness score: {score}%")
    if missing:
        print("Readiness gaps:")
        for item in missing:
            print(f"- {item}")
    print("Onboarding lint passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
