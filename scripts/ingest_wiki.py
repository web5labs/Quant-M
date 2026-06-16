#!/usr/bin/env python3
"""Ingest raw wiki files into normalized markdown summaries.

This script is intentionally lightweight and dependency-free. It does not call an LLM.
It creates structured summary placeholders that Codex or another agent can fill in.
"""

from __future__ import annotations

import argparse
import hashlib
import re
from datetime import datetime, timezone
from pathlib import Path

SUPPORTED = {".md", ".txt", ".json", ".yaml", ".yml"}


def slugify(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "repo"


def safe_name(path: Path) -> str:
    stem = "".join(c if c.isalnum() or c in "-_" else "-" for c in path.stem).strip("-").lower()
    digest = hashlib.sha1(str(path).encode("utf-8")).hexdigest()[:8]
    return f"{stem}-{digest}.md"


def read_excerpt(path: Path, max_chars: int = 3000) -> str:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except Exception as exc:  # noqa: BLE001
        return f"[Could not read file: {exc}]"
    text = text.strip()
    return text[:max_chars] + ("\n\n[Truncated by ingest script.]" if len(text) > max_chars else "")


def build_ingested_doc(source: Path, raw_root: Path) -> str:
    rel = source.relative_to(raw_root.parent.parent) if raw_root.parent.parent in source.parents else source
    now = datetime.now(timezone.utc).isoformat()
    excerpt = read_excerpt(source)
    return f"""# Ingested Wiki Source: {source.name}

## Metadata

- Source path: `{rel}`
- Ingested at: `{now}`
- Source extension: `{source.suffix}`

## Agent summary

_TBD: Summarize the source in 5-10 bullets._

## Key facts

_TBD_

## Implementation relevance

_TBD: Explain how this source affects the project spec, architecture, data model, API plan, or UI/UX handoff._

## Risks / constraints

_TBD_

## Open questions

_TBD_

## Source excerpt

```text
{excerpt}
```
"""


def parse_reference_repos(reference_doc: Path) -> list[tuple[str, str]]:
    if not reference_doc.exists():
        return []

    repos: list[tuple[str, str]] = []
    section = ""

    for raw_line in reference_doc.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw_line.strip()
        lower = line.lower()

        if lower.startswith("## "):
            if "approved repos" in lower:
                section = "approved"
            elif "candidate repos" in lower:
                section = "candidate"
            else:
                section = ""
            continue

        if section and line.startswith(("-", "*")):
            entry = line[1:].strip()
            if not entry or entry.lower() == "_tbd_":
                continue
            if entry.startswith("`") and entry.endswith("`"):
                entry = entry[1:-1].strip()
            repos.append((section, entry))

    return repos


def build_repo_file(kind: str, repo_name: str) -> str:
    if kind == "repo-map":
        return f"""# Repo Map: {repo_name}

## Why this repo matters

_TBD_

## First files to inspect

- _TBD_

## Notable folders

- _TBD_

## License / constraints

_TBD_
"""

    if kind == "useful-patterns":
        return f"""# Useful Patterns: {repo_name}

## Patterns to borrow

- _TBD_

## Patterns to avoid copying blindly

- _TBD_

## Relevance to current project

_TBD_
"""

    return f"""# Files To Reference: {repo_name}

## Exact files for the current goal

- _TBD_

## Why these files matter

_TBD_
"""


def scaffold_reference_repo_docs(target: Path, repos: list[tuple[str, str]]) -> list[Path]:
    base = target / "docs/wiki/repo-ingest"
    base.mkdir(parents=True, exist_ok=True)
    created: list[Path] = []

    for status, repo_name in repos:
        slug = slugify(repo_name)
        repo_dir = base / slug
        repo_dir.mkdir(parents=True, exist_ok=True)

        status_path = repo_dir / "README.md"
        if not status_path.exists():
            status_path.write_text(
                f"# {repo_name}\n\nStatus: {status}\n\nUse this folder to capture repo-specific manifests and patterns.\n",
                encoding="utf-8",
            )
            created.append(status_path)

        for kind, filename in (
            ("repo-map", "repo-map.md"),
            ("useful-patterns", "useful-patterns.md"),
            ("files-to-reference", "files-to-reference.md"),
        ):
            out = repo_dir / filename
            if out.exists():
                continue
            out.write_text(build_repo_file(kind, repo_name), encoding="utf-8")
            created.append(out)

    return created


def update_manifest(
    target: Path,
    raw_files: list[Path],
    ingested_files: list[Path],
    requested_repos: list[tuple[str, str]],
    repo_summary_files: list[Path],
) -> None:
    manifest = target / "docs/wiki/MANIFEST.md"
    manifest.parent.mkdir(parents=True, exist_ok=True)

    raw_list = "\n".join(f"- `{p.relative_to(target)}`" for p in raw_files) or "- _None discovered._"
    ingested_list = "\n".join(f"- `{p.relative_to(target)}`" for p in ingested_files) or "- _None generated._"
    repo_request_list = (
        "\n".join(f"- {status}: `{name}`" for status, name in requested_repos) or "- _None requested._"
    )
    repo_summary_list = (
        "\n".join(f"- `{p.relative_to(target)}`" for p in repo_summary_files) or "- _None generated._"
    )

    existing = manifest.read_text(encoding="utf-8") if manifest.exists() else "# LLM Wiki Manifest\n"

    marker = "\n## Ingestion Index\n"
    before = existing.split(marker)[0].rstrip()
    updated = f"""{before}

## Ingestion Index

Updated by `scripts/ingest_wiki.py`.

### Raw files discovered

{raw_list}

### Ingested summaries

{ingested_list}

### Reference repos requested

{repo_request_list}

### Repo-ingest summaries

{repo_summary_list}
"""
    manifest.write_text(updated, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", default=".", help="Target repo root")
    args = parser.parse_args()

    target = Path(args.target).resolve()
    raw_root = target / "docs/wiki/raw"
    out_root = target / "docs/wiki/ingested"
    raw_root.mkdir(parents=True, exist_ok=True)
    out_root.mkdir(parents=True, exist_ok=True)

    raw_files = sorted(
        p for p in raw_root.rglob("*") if p.is_file() and p.suffix.lower() in SUPPORTED and p.name != "README.md"
    )
    ingested_files: list[Path] = []

    for source in raw_files:
        out_path = out_root / safe_name(source)
        if not out_path.exists():
            out_path.write_text(build_ingested_doc(source, raw_root), encoding="utf-8")
        ingested_files.append(out_path)

    reference_doc = target / "docs/wiki/08-reference-repos.md"
    requested_repos = parse_reference_repos(reference_doc)
    scaffold_reference_repo_docs(target, requested_repos)
    repo_summary_files = sorted(
        p
        for p in (target / "docs/wiki/repo-ingest").rglob("*")
        if p.is_file() and p.name != "README.md"
    )

    update_manifest(target, raw_files, ingested_files, requested_repos, repo_summary_files)
    print(
        "Ingest complete: "
        f"raw_files={len(raw_files)}, ingested_files={len(ingested_files)}, "
        f"reference_repos={len(requested_repos)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
