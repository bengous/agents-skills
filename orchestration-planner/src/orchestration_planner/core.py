from __future__ import annotations

import json
import os
import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal, TypeGuard, cast

STATE_FILE = ".orchestration-state.json"
STATE_VERSION = 1

type Stage = Literal[
    "intake",
    "clarification",
    "prd",
    "prd_review",
    "issues",
    "issues_review",
    "workflow",
    "workflow_review",
    "workflow_ready",
]

type JsonScalar = str | int | None
type JsonObject = dict[str, JsonScalar]

STAGES: tuple[Stage, ...] = (
    "intake",
    "clarification",
    "prd",
    "prd_review",
    "issues",
    "issues_review",
    "workflow",
    "workflow_review",
    "workflow_ready",
)

NEXT_STAGE: dict[Stage, Stage] = {
    "intake": "clarification",
    "clarification": "prd",
    "prd": "prd_review",
    "prd_review": "issues",
    "issues": "issues_review",
    "issues_review": "workflow",
    "workflow": "workflow_review",
    "workflow_review": "workflow_ready",
}

REQUIRED_SKILLS_BY_TARGET: dict[Stage, tuple[str, ...]] = {
    "clarification": ("grill-me",),
    "prd": ("to-prd",),
    "issues": ("to-issues", "tdd"),
    "workflow": ("tdd",),
}

PRD_SECTIONS = (
    "## Problem Statement",
    "## Solution",
    "## User Stories",
    "## Implementation Decisions",
)

ISSUE_FIELDS = (
    "Type:",
    "Depends on:",
    "Goal:",
    "Acceptance:",
    "TDD:",
    "Validation:",
    "Agent:",
)


class OrchError(Exception):
    pass


@dataclass(frozen=True)
class OrchestrationState:
    version: int
    stage: Stage
    created_at: str
    root: str
    session_id: str | None
    session_short: str | None
    cwd: str
    model: str | None
    transcript_path: str | None

    def to_json(self) -> JsonObject:
        return {
            "version": self.version,
            "stage": self.stage,
            "created_at": self.created_at,
            "root": self.root,
            "session_id": self.session_id,
            "session_short": self.session_short,
            "cwd": self.cwd,
            "model": self.model,
            "transcript_path": self.transcript_path,
        }

    def with_stage(self, stage: Stage) -> OrchestrationState:
        return OrchestrationState(
            version=self.version,
            stage=stage,
            created_at=self.created_at,
            root=self.root,
            session_id=self.session_id,
            session_short=self.session_short,
            cwd=self.cwd,
            model=self.model,
            transcript_path=self.transcript_path,
        )


@dataclass(frozen=True)
class Issue:
    number: int
    title: str
    body: str

    @property
    def slug(self) -> str:
        return f"issue-{self.number:02d}"

    @property
    def needs_review(self) -> bool:
        review_patterns = ("review gate", "review gates", "review:", "reviewer", "read-only review")
        body = self.body.lower()
        return any(pattern in body for pattern in review_patterns)


def is_stage(value: object) -> TypeGuard[Stage]:
    return isinstance(value, str) and value in STAGES


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")


def state_path(root: Path) -> Path:
    return root / STATE_FILE


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise OrchError(f"missing: {path.name}") from error


def write_if_missing(path: Path, content: str) -> None:
    if path.exists():
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def extract_optional_text(raw: Mapping[str, object], key: str) -> str | None:
    value = raw.get(key)
    if value is None:
        return None
    if isinstance(value, str):
        return value
    raise OrchError(f"invalid state field: {key}")


def load_state(root: Path) -> OrchestrationState:
    path = state_path(root)
    try:
        raw_text = path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise OrchError(f"missing: {STATE_FILE}") from error

    raw = cast(object, json.loads(raw_text))
    if not isinstance(raw, dict):
        raise OrchError("invalid state: expected object")
    state_json = cast(dict[str, object], raw)

    version = state_json.get("version")
    if not isinstance(version, int):
        raise OrchError("invalid state field: version")

    stage = state_json.get("stage")
    if not is_stage(stage):
        raise OrchError("invalid state field: stage")

    created_at = state_json.get("created_at")
    root_value = state_json.get("root")
    cwd = state_json.get("cwd")
    if (
        not isinstance(created_at, str)
        or not isinstance(root_value, str)
        or not isinstance(cwd, str)
    ):
        raise OrchError("invalid state: missing core string field")

    return OrchestrationState(
        version=version,
        stage=stage,
        created_at=created_at,
        root=root_value,
        session_id=extract_optional_text(state_json, "session_id"),
        session_short=extract_optional_text(state_json, "session_short"),
        cwd=cwd,
        model=extract_optional_text(state_json, "model"),
        transcript_path=extract_optional_text(state_json, "transcript_path"),
    )


def write_state(root: Path, state: OrchestrationState) -> None:
    state_path(root).write_text(json.dumps(state.to_json(), indent=2) + "\n", encoding="utf-8")


def compact_status(root: Path, state: OrchestrationState) -> str:
    next_command = "-"
    if state.stage in NEXT_STAGE:
        next_command = f"orch advance {root}"
    return f"stage={state.stage} root={root} next={next_command}\n"


def env_value(*names: str) -> str | None:
    for name in names:
        value = os.environ.get(name)
        if value:
            return value
    return None


def session_short_from(session_id: str | None) -> str | None:
    if session_id is None:
        return env_value("ORCH_SESSION_SHORT", "CODEX_SESSION_SHORT")
    compact = re.sub(r"[^A-Za-z0-9]", "", session_id)
    if compact == "":
        return None
    return compact[:8]


def init_workflow(root: Path, intention_parts: Sequence[str], entry_point: str = "manual") -> str:
    if state_path(root).exists():
        raise OrchError(f"already initialized: {root}")

    root.mkdir(parents=True, exist_ok=True)
    intention = " ".join(intention_parts).strip()
    session_id = env_value("ORCH_SESSION_ID", "CODEX_SESSION_ID")
    cwd = env_value("ORCH_CWD") or str(Path.cwd())
    model = env_value("ORCH_MODEL", "CODEX_MODEL")
    transcript_path = env_value("ORCH_TRANSCRIPT_PATH", "CODEX_TRANSCRIPT_PATH")
    return init_workflow_with_metadata(
        root=root,
        intention=intention,
        entry_point=entry_point,
        session_id=session_id,
        cwd=cwd,
        model=model,
        transcript_path=transcript_path,
    )


def init_workflow_with_metadata(
    root: Path,
    intention: str,
    entry_point: str,
    session_id: str | None,
    cwd: str,
    model: str | None,
    transcript_path: str | None,
) -> str:
    if state_path(root).exists():
        raise OrchError(f"already initialized: {root}")

    root.mkdir(parents=True, exist_ok=True)
    state = OrchestrationState(
        version=STATE_VERSION,
        stage="intake",
        created_at=utc_now(),
        root=str(root),
        session_id=session_id,
        session_short=session_short_from(session_id),
        cwd=cwd,
        model=model,
        transcript_path=transcript_path,
    )

    write_state(root, state)
    write_if_missing(root / "intake.md", intake_template(intention, entry_point))
    return compact_status(root, state) + instructions_for_stage("intake", root)


def status_workflow(root: Path) -> str:
    return compact_status(root, load_state(root))


def advance_workflow(root: Path) -> str:
    state = load_state(root)
    if state.stage == "workflow_ready":
        raise OrchError("already at workflow_ready")

    validate_current_stage(root, state.stage)
    target = NEXT_STAGE[state.stage]
    ensure_required_skills(target)

    if target == "clarification":
        write_if_missing(root / "clarification.md", clarification_template())
    elif target == "prd":
        write_if_missing(root / "prd.md", prd_template())
    elif target == "issues":
        write_if_missing(root / "issues.md", issues_template())
    elif target == "workflow":
        create_workflow_package(root)

    next_state = state.with_stage(target)
    write_state(root, next_state)

    output = compact_status(root, next_state)
    if target == "workflow_ready":
        return output + final_handoff_prompt(root)
    return output + instructions_for_stage(target, root)


def ensure_required_skills(target: Stage) -> None:
    missing = [
        skill for skill in REQUIRED_SKILLS_BY_TARGET.get(target, ()) if not skill_exists(skill)
    ]
    if missing:
        joined = ", ".join(f"${skill}" for skill in missing)
        raise OrchError(f"missing required companion skill(s): {joined}")


def skill_roots() -> tuple[Path, ...]:
    configured = os.environ.get("ORCH_SKILL_ROOTS")
    if configured:
        return tuple(Path(value).expanduser() for value in configured.split(os.pathsep) if value)

    home = Path.home()
    return (
        Path.cwd() / ".agents" / "skills",
        home / ".agents" / "skills",
        home / ".codex" / "skills",
        home / ".claude" / "skills",
    )


def skill_exists(skill_name: str) -> bool:
    return any((root / skill_name / "SKILL.md").is_file() for root in skill_roots())


def validate_current_stage(root: Path, stage: Stage) -> None:
    validators: dict[Stage, tuple[str, ...]] = {
        "intake": validate_intake(root),
        "clarification": validate_clarification(root),
        "prd": validate_prd(root),
        "prd_review": validate_prd_review(root),
        "issues": validate_issues(root),
        "issues_review": validate_issues_review(root),
        "workflow": validate_workflow(root),
        "workflow_review": validate_workflow_review(root),
        "workflow_ready": (),
    }
    missing = validators[stage]
    if missing:
        raise OrchError("missing: " + ", ".join(missing))


def validate_intake(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "intake.md", require_non_empty=True))
    text = (root / "intake.md").read_text(encoding="utf-8") if (root / "intake.md").exists() else ""
    intention = section_body(text, "## Initial Intention").strip()
    if intention == "" or intention == "TODO" or intention.startswith("TODO:"):
        errors.append("intake.md initial intention")
    return tuple(errors)


def validate_clarification(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "clarification.md", require_non_empty=True))
    if (root / "clarification.md").exists():
        errors.extend(placeholder_errors("clarification.md", read_text(root / "clarification.md")))
    return tuple(errors)


def validate_prd(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "prd.md", require_non_empty=True))
    text = (root / "prd.md").read_text(encoding="utf-8") if (root / "prd.md").exists() else ""
    errors.extend(section for section in PRD_SECTIONS if section not in text)
    errors.extend(placeholder_errors("prd.md", text))
    return tuple(errors)


def validate_prd_review(root: Path) -> tuple[str, ...]:
    return validate_prd(root)


def validate_issues(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "issues.md", require_non_empty=True))
    text = (root / "issues.md").read_text(encoding="utf-8") if (root / "issues.md").exists() else ""
    issues = parse_issues(text)
    if not issues:
        errors.append("issues.md issue heading")
    for issue in issues:
        if "TODO" in issue.title:
            errors.append(f"issues.md issue {issue.number} title")
        errors.extend(
            f"issues.md issue {issue.number} {field}"
            for field in ISSUE_FIELDS
            if field not in issue.body
        )
        errors.extend(
            f"issues.md issue {issue.number} {error}"
            for error in placeholder_errors("body", issue.body)
        )
    return tuple(errors)


def validate_issues_review(root: Path) -> tuple[str, ...]:
    return validate_issues(root)


def validate_workflow(root: Path) -> tuple[str, ...]:
    errors: list[str] = []
    errors.extend(file_errors(root, "workflow.md", require_non_empty=True))
    errors.extend(file_errors(root, "tracker.md", require_non_empty=True))
    errors.extend(validate_prompt_references(root))
    if (root / "workflow.md").exists():
        errors.extend(placeholder_errors("workflow.md", read_text(root / "workflow.md")))
    if (root / "tracker.md").exists():
        errors.extend(placeholder_errors("tracker.md", read_text(root / "tracker.md")))
    return tuple(errors)


def validate_workflow_review(root: Path) -> tuple[str, ...]:
    return validate_workflow(root)


def file_errors(root: Path, relative: str, require_non_empty: bool = False) -> tuple[str, ...]:
    path = root / relative
    if not path.exists():
        return (relative,)
    if require_non_empty and path.read_text(encoding="utf-8").strip() == "":
        return (f"{relative} non-empty",)
    return ()


def placeholder_errors(label: str, text: str) -> tuple[str, ...]:
    placeholders = ("TODO", "[TODO", "Add your description here")
    return tuple(f"{label} placeholder {value}" for value in placeholders if value in text)


def section_body(text: str, heading: str) -> str:
    start = text.find(heading)
    if start == -1:
        return ""
    body_start = start + len(heading)
    next_heading = text.find("\n## ", body_start)
    if next_heading == -1:
        return text[body_start:]
    return text[body_start:next_heading]


def validate_prompt_references(root: Path) -> tuple[str, ...]:
    tracker = root / "tracker.md"
    prompts_dir = root / "prompts"
    if not tracker.exists():
        return ()

    errors: list[str] = []
    if not prompts_dir.is_dir():
        errors.append("prompts/")
        return tuple(errors)

    prompt_refs = sorted(set(re.findall(r"prompts/[A-Za-z0-9_.-]+\.md", read_text(tracker))))
    if not prompt_refs:
        errors.append("tracker.md prompt references")
        return tuple(errors)

    for prompt_ref in prompt_refs:
        if not (root / prompt_ref).is_file():
            errors.append(prompt_ref)

    return tuple(errors)


def intake_template(intention: str, entry_point: str) -> str:
    initial = intention if intention else "TODO: initial intention"
    return f"""# Intake

Entry point: {entry_point}

## Initial Intention

{initial}

## Context

TODO

## Assumptions

- TODO
"""


def clarification_template() -> str:
    return """# Clarification

Record each question before asking the next one.

## Q001

Question: TODO

Reco: TODO

Answer: TODO

Decision: TODO
"""


def prd_template() -> str:
    return """# PRD

## Problem Statement

TODO

## Solution

TODO

## User Stories

- TODO

## Implementation Decisions

- TODO

## Out of Scope

- TODO
"""


def issues_template() -> str:
    return """# Issues

### 1. TODO

Type: AFK

Depends on: none

Goal:

TODO

Acceptance:

- [ ] TODO

TDD:

- First behavior test: TODO

Validation:

- TODO

Agent:

- TODO
"""


def instructions_for_stage(stage: Stage, root: Path) -> str:
    command = f"orch advance {root}"
    instructions: dict[Stage, str] = {
        "intake": f"next: fill intake.md, then human runs `{command}`\n",
        "clarification": (
            "phase: clarification\n"
            "use: $grill-me\n"
            "write: clarification.md before each next question\n"
            f"done: ask human to run `{command}`\n"
        ),
        "prd": (
            "phase: prd\n"
            "use: $to-prd\n"
            "write: prd.md from intake.md + clarification.md\n"
            f"done: ask human to run `{command}`\n"
        ),
        "prd_review": (
            "phase: prd_review\n"
            "human validates prd.md; do not create issues yet\n"
            f"approved: human runs `{command}`\n"
        ),
        "issues": (
            "phase: issues\n"
            "use: $to-issues; design worker prompts to invoke $tdd\n"
            "write: issues.md with vertical AFK/HITL slices\n"
            f"done: ask human to run `{command}`\n"
        ),
        "issues_review": (
            "phase: issues_review\n"
            "human validates issues.md; do not create workflow yet\n"
            f"approved: human runs `{command}`\n"
        ),
        "workflow": (
            "phase: workflow\n"
            "write/finalize: workflow.md, tracker.md, prompts/*.md\n"
            "use: $deepen-codebase-architecture only if boundaries are unclear\n"
            f"done: ask human to run `{command}`\n"
        ),
        "workflow_review": (
            "phase: workflow_review\n"
            "human validates workflow package; no execution launch\n"
            f"approved: human runs `{command}`\n"
        ),
        "workflow_ready": "phase: workflow_ready\n",
    }
    return instructions[stage]


def parse_issues(text: str) -> tuple[Issue, ...]:
    matches = list(re.finditer(r"^###\s+(?:(\d+)[.)]\s*)?(.+)$", text, flags=re.MULTILINE))
    issues: list[Issue] = []
    for index, match in enumerate(matches):
        number = int(match.group(1)) if match.group(1) else index + 1
        title = match.group(2).strip()
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        issues.append(Issue(number=number, title=title, body=text[start:end].strip()))
    return tuple(issues)


def create_workflow_package(root: Path) -> None:
    issues_text = read_text(root / "issues.md")
    issues = parse_issues(issues_text)
    if not issues:
        raise OrchError("missing: issues.md issue heading")

    (root / "prompts").mkdir(exist_ok=True)

    for issue in issues:
        write_if_missing(root / "prompts" / f"{issue.slug}-worker.md", worker_prompt(issue, root))
        if issue.needs_review:
            (root / "reviews").mkdir(exist_ok=True)
            write_if_missing(
                root / "prompts" / f"{issue.slug}-reviewer.md",
                reviewer_prompt(issue, root),
            )

    write_if_missing(root / "workflow.md", workflow_template(root, issues))
    write_if_missing(root / "tracker.md", tracker_template(issues))


def issue_lines(issues: Iterable[Issue]) -> str:
    return "\n".join(f"- {issue.number}. {issue.title}" for issue in issues)


def workflow_template(root: Path, issues: Sequence[Issue]) -> str:
    return f"""# Workflow

## Sources

- `intake.md`
- `clarification.md`
- `prd.md`
- `issues.md`
- `tracker.md`

## Execution Contract

- Read this file, `tracker.md`, and `issues.md` before changing code.
- Treat `tracker.md` as execution authority.
- Execute one issue at a time unless the workflow explicitly allows parallel work.
- Worker prompts live under `prompts/`.
- Workers use `$tdd`.
- Workers commit local scoped slices after validation and accepted reviews.
- Push only after the whole workflow is complete.
- Do not execute work outside this workflow root's scope.

## Issue Order

{issue_lines(issues)}

## Gates

- [ ] Confirm branch policy before first implementation commit.
- [ ] Keep tracker current after each handoff.
- [ ] Run each issue's validation before marking completed.
- [ ] Record commit hashes in `tracker.md`.

## Final Report

Report completed issues, commits, validation, blockers, and any human follow-up.

Root: `{root}`
"""


def tracker_template(issues: Sequence[Issue]) -> str:
    rows = [
        "| Seq | Issue | Status | Agent | Prompt | Review | Commit | Validation |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for issue in issues:
        review = f"prompts/{issue.slug}-reviewer.md" if issue.needs_review else "-"
        rows.append(
            "| "
            f"{issue.number} | {issue.title} | pending | worker | "
            f"prompts/{issue.slug}-worker.md | {review} | - | - |"
        )
    return "\n".join(
        [
            "# Tracker",
            "",
            "Execution authority for this workflow.",
            "",
            "Statuses: pending, in_progress, reviewing, completed, blocked.",
            "",
            *rows,
            "",
            "Resume note: keep this short; reference files instead of duplicating context.",
            "",
        ]
    )


def worker_prompt(issue: Issue, root: Path) -> str:
    return f"""Use $tdd.

You are implementing one scoped issue from an existing workflow.

Why:
This workflow was prepared through intake, clarification, PRD, and issue review.

Read first:
- `{root / "workflow.md"}`
- `{root / "tracker.md"}`
- `{root / "prd.md"}`
- `{root / "issues.md"}`

Issue:
{issue.number}. {issue.title}

Scope:
- Implement only this issue.
- Keep `tracker.md` current.
- Run the issue validation.
- Commit the scoped slice locally after validation and accepted reviews.
- Do not push.

Issue body:
{issue.body}
"""


def reviewer_prompt(issue: Issue, root: Path) -> str:
    return f"""Read-only review.

Review issue {issue.number}: {issue.title}

Read:
- `{root / "workflow.md"}`
- `{root / "tracker.md"}`
- `{root / "prd.md"}`
- `{root / "issues.md"}`

Rules:
- Do not edit files.
- Prioritize bugs, contract drift, missed validation, and scope creep.
- Write findings with file/line references when possible.
- Save the review to `{root / "reviews" / f"{issue.slug}-review.md"}` if possible.
- Otherwise report it to the orchestrator.
"""


def final_handoff_prompt(root: Path) -> str:
    return f"""Execute the following workflow:
>>>
Read `{root / "workflow.md"}`, `{root / "tracker.md"}`, and `{root / "issues.md"}`.
Treat `tracker.md` as the execution authority.
Use prompts under `{root / "prompts"}`.
Keep scope to the active issue.
Workers commit local scoped slices after validation/review.
Do not push until the workflow is complete.
Final report: completed issues, commits, validation, blockers.
<<<
"""
