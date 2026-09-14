#!/usr/bin/env python3
"""Report likely Codex instruction drift since the last guidance change."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

GUIDANCE_PATHS = (
    "AGENTS.md",
    "AGENTS.override.md",
    ":(glob)**/AGENTS.md",
    ":(glob)**/AGENTS.override.md",
    ":(glob)**/SKILL.md",
    ".agents/skills/",
    ".codex/config.toml",
    ".codex/hooks.json",
)


@dataclass(frozen=True)
class Commit:
    hash: str
    date: str
    subject: str
    files: tuple[str, ...]


def run_git(args: list[str]) -> str:
    result = subprocess.run(
        ["git", *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {args[0]}: {result.stderr.strip()}")
    return result.stdout.strip()


def guidance_files() -> list[str]:
    files: set[str] = set()
    for pattern in ("AGENTS.md", "AGENTS.override.md"):
        out = run_git(["ls-files", f":(glob)**/{pattern}"])
        files.update(line for line in out.splitlines() if line)
    for path in (".codex/config.toml", ".codex/hooks.json"):
        if Path(path).exists():
            files.add(path)
    out = run_git(["ls-files", ":(glob)**/SKILL.md"])
    files.update(line for line in out.splitlines() if ".agents/skills/" in line or line.count("/") <= 1)
    for path in Path.cwd().glob("*/SKILL.md"):
        if ".git" not in path.parts and "target" not in path.parts:
            files.add(path.relative_to(Path.cwd()).as_posix())
    return sorted(files)


def find_baseline(explicit: str | None) -> tuple[str, str, str]:
    if explicit:
        line = run_git(["log", "--format=%H|%ai|%s", "-1", explicit])
        if not line:
            raise RuntimeError(f"baseline not found: {explicit}")
        commit, date, subject = parse_header(line)
        return commit, date, subject

    args = ["log", "--format=%H|%ai|%s", "-1", "--", *GUIDANCE_PATHS]
    line = run_git(args)
    if not line:
        line = run_git(["log", "--format=%H|%ai|%s", "-1"])
    if not line:
        raise RuntimeError("no commits found")
    commit, date, subject = parse_header(line)
    return commit, date, subject


def parse_header(line: str) -> tuple[str, str, str]:
    parts = line.split("|", 2)
    if len(parts) != 3:
        raise RuntimeError(f"unexpected git log format: {line}")
    return parts[0], parts[1], parts[2]


def commits_since(baseline: str) -> list[Commit]:
    out = run_git(["log", "--format=%H|%ai|%s", "--name-only", f"{baseline}..HEAD"])
    commits: list[Commit] = []
    current: tuple[str, str, str] | None = None
    files: list[str] = []
    after_header = False

    for line in out.splitlines():
        if is_header(line):
            if current is not None:
                commits.append(Commit(*current, tuple(files)))
            current = parse_header(line)
            files = []
            after_header = True
        elif not line:
            after_header = False
        elif current is not None and not after_header:
            files.append(line)

    if current is not None:
        commits.append(Commit(*current, tuple(files)))
    return commits


def is_header(line: str) -> bool:
    return len(line) > 42 and line[40] == "|" and all(c in "0123456789abcdef" for c in line[:40])


def classify(path: str) -> str:
    parts = path.split("/")
    if len(parts) == 1:
        return "(root)"
    if parts[0] == "src" and len(parts) >= 3:
        return f"{parts[1]}/{parts[2]}"
    if parts[0] == "src":
        return "src"
    if parts[0] == "client" and len(parts) >= 2:
        return f"client/{parts[1]}"
    if parts[0].startswith(".") and len(parts) >= 2:
        return f"{parts[0]}/{parts[1]}"
    return parts[0]


def zone_stats(commits: list[Commit]) -> list[dict[str, object]]:
    zones: dict[str, dict[str, set[str]]] = defaultdict(lambda: {"commits": set(), "files": set()})
    for commit in commits:
        for file in commit.files:
            zone = classify(file)
            zones[zone]["commits"].add(commit.hash)
            zones[zone]["files"].add(Path(file).name)
    rows = [
        {"zone": zone, "commits": len(data["commits"]), "key_files": sorted(data["files"])}
        for zone, data in zones.items()
    ]
    return sorted(rows, key=lambda row: (-int(row["commits"]), str(row["zone"])))


def render_markdown(baseline: tuple[str, str, str], commits: list[Commit], guidance: list[str]) -> str:
    commit, date, subject = baseline
    short = commit[:7]
    out = [
        "# Codex Docs Drift Audit",
        "",
        "## Context",
        "",
        f"- Baseline: `{short}` ({date}) {subject}",
        f"- Commits since baseline: {len(commits)}",
        f"- Candidate Codex guidance files: {len(guidance)}",
        "",
    ]
    if guidance:
        out.extend(["## Candidate Guidance Files", ""])
        out.extend(f"- `{path}`" for path in guidance)
        out.append("")
    if not commits:
        out.extend(["## Result", "", "No drift window: guidance is at HEAD.", ""])
        return "\n".join(out)

    out.extend(["## Changed Zones", "", "| Zone | Commits | Key files |", "|---|---:|---|"])
    for row in zone_stats(commits):
        files = ", ".join(row["key_files"][:8])
        suffix = "..." if len(row["key_files"]) > 8 else ""
        out.append(f"| {row['zone']} | {row['commits']} | {files}{suffix} |")
    out.extend(["", "## Commit Window", ""])
    for item in commits:
        out.append(f"- `{item.hash[:7]}` {item.subject}")
    out.extend(
        [
            "",
            "## Next Audit Steps",
            "",
            "1. Read each candidate guidance file above.",
            "2. Inspect changed zones with the highest commit counts.",
            "3. Report only guidance that can mislead future Codex agents.",
            "4. Patch only after explicit approval.",
            "",
        ]
    )
    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--since", help="Git revision to use as the baseline")
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    args = parser.parse_args()

    try:
        baseline = find_baseline(args.since)
        commits = commits_since(baseline[0])
        guidance = guidance_files()
    except RuntimeError as exc:
        print(f"docs-drift: {exc}", file=sys.stderr)
        return 1

    if args.json:
        print(
            json.dumps(
                {
                    "baseline": {"hash": baseline[0], "date": baseline[1], "subject": baseline[2]},
                    "commit_count": len(commits),
                    "guidance_files": guidance,
                    "zones": zone_stats(commits),
                    "commits": [commit.__dict__ for commit in commits],
                },
                indent=2,
            )
        )
    else:
        print(render_markdown(baseline, commits, guidance))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
