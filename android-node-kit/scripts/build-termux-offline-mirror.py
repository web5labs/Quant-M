#!/usr/bin/env python3
"""Build a small offline Termux apt mirror for Android edge devices."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import re
import shutil
import subprocess
import sys
import urllib.request
import urllib.error
from pathlib import Path


DEFAULT_REPO = "https://packages.termux.dev/apt/termux-main"
DEFAULT_PACKAGES = [
    "openssh",
    "git",
    "curl",
    "termux-tools",
    "termux-api",
    "rust",
    "rsync",
]
DEFAULT_ARCHES = ["aarch64", "arm"]


def fetch_bytes(url: str) -> bytes:
    try:
        with urllib.request.urlopen(url) as response:
            return response.read()
    except urllib.error.URLError:
        return subprocess.check_output(["curl", "-fsSL", url])


def parse_packages(text: str) -> dict[str, dict[str, str]]:
    packages: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    last_key: str | None = None

    for line in text.splitlines():
        if not line:
            if "Package" in current:
                packages[current["Package"]] = current
            current = {}
            last_key = None
            continue
        if line.startswith(" ") and last_key:
            current[last_key] = current[last_key] + "\n" + line
            continue
        key, sep, value = line.partition(":")
        if sep:
            current[key] = value.strip()
            last_key = key

    if "Package" in current:
        packages[current["Package"]] = current
    return packages


def package_names_from_depends(depends: str) -> list[str]:
    names: list[str] = []
    for dep in depends.split(","):
        first_alt = dep.split("|", 1)[0].strip()
        match = re.match(r"([A-Za-z0-9+_.-]+)", first_alt)
        if match:
            names.append(match.group(1))
    return names


def resolve_closure(index: dict[str, dict[str, str]], roots: list[str]) -> list[str]:
    resolved: list[str] = []
    seen: set[str] = set()
    stack = list(reversed(roots))

    while stack:
        name = stack.pop()
        if name in seen:
            continue
        if name not in index:
            raise KeyError(f"package not found in Termux index: {name}")
        seen.add(name)
        resolved.append(name)
        depends = index[name].get("Depends", "")
        for dep in reversed(package_names_from_depends(depends)):
            if dep not in seen:
                stack.append(dep)
    return resolved


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_arch(repo: str, out_root: Path, arch: str, roots: list[str]) -> dict[str, object]:
    repo_name = repo.rstrip("/").rsplit("/", 1)[-1]
    binary_dir = f"dists/stable/main/binary-{arch}"
    packages_url = f"{repo}/{binary_dir}/Packages.gz"
    print(f"[{arch}] fetching package index: {packages_url}")
    packages_text = gzip.decompress(fetch_bytes(packages_url)).decode("utf-8")
    index = parse_packages(packages_text)
    closure = resolve_closure(index, roots)
    print(f"[{arch}] resolved {len(closure)} packages")

    repo_root = out_root / repo_name
    metadata_dir = repo_root / binary_dir
    metadata_dir.mkdir(parents=True, exist_ok=True)

    release_url = f"{repo}/{binary_dir}/Release"
    release_path = metadata_dir / "Release"
    release_path.write_bytes(fetch_bytes(release_url))

    selected_stanzas: list[str] = []
    downloaded: list[dict[str, object]] = []

    for name in closure:
        stanza = index[name]
        filename = stanza["Filename"]
        target = repo_root / filename
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            url = f"{repo}/{filename}"
            print(f"[{arch}] downloading {name}: {filename}")
            target.write_bytes(fetch_bytes(url))
        else:
            print(f"[{arch}] already present {name}: {filename}")
        digest = sha256_file(target)
        selected_stanzas.append("\n".join(f"{k}: {v}" for k, v in stanza.items()))
        downloaded.append(
            {
                "package": name,
                "filename": filename,
                "size": target.stat().st_size,
                "sha256": digest,
            }
        )

    packages_path = metadata_dir / "Packages"
    packages_path.write_text("\n\n".join(selected_stanzas) + "\n", encoding="utf-8")
    with gzip.open(metadata_dir / "Packages.gz", "wb") as gz:
        gz.write(packages_path.read_bytes())

    return {"arch": arch, "package_count": len(closure), "packages": downloaded}


def write_summary(out_root: Path, repo: str, roots: list[str], results: list[dict[str, object]]) -> None:
    repo_name = repo.rstrip("/").rsplit("/", 1)[-1]
    lines = [
        "# Termux Offline Mirror",
        "",
        f"Source repo: `{repo}`",
        "",
        "Root packages:",
        "",
    ]
    lines.extend(f"- `{name}`" for name in roots)
    lines.extend(["", "Architectures:", ""])
    for result in results:
        lines.append(f"- `{result['arch']}`: {result['package_count']} packages")
    lines.extend(
        [
            "",
            "Use from Termux with:",
            "",
            "```bash",
            "bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh",
            "```",
        ]
    )
    (out_root / f"README-{repo_name}.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--out", default="android-node-kit/bundles/quant-m-edge-bundle/offline")
    parser.add_argument("--arch", action="append", choices=["aarch64", "arm", "i686", "x86_64"])
    parser.add_argument("--package", action="append")
    parser.add_argument("--clean", action="store_true")
    args = parser.parse_args()

    out_root = Path(args.out)
    arches = args.arch or DEFAULT_ARCHES
    packages = args.package or DEFAULT_PACKAGES

    if args.clean and out_root.exists():
        shutil.rmtree(out_root)
    out_root.mkdir(parents=True, exist_ok=True)

    results = [build_arch(args.repo.rstrip("/"), out_root, arch, packages) for arch in arches]
    write_summary(out_root, args.repo.rstrip("/"), packages, results)
    print(f"offline mirror written to {out_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
