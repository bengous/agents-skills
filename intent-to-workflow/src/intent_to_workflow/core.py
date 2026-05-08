from __future__ import annotations

import json
import os
import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from importlib.resources import files
from pathlib import Path
from typing import Literal, TypeGuard, cast

CLI_NAME = "itw"
STATE_FILE = ".itw-state.json"
STATE_VERSION = 1

type Stage = Literal[
    "clarification",
    "prd",
    "prd_review",
    "issues",
    "issues_review",
    "workflow",
    "workflow_review",
    "workflow_ready",
]

type Language = Literal["fr", "en", "unknown"]
type JsonScalar = str | int | None
type JsonObject = dict[str, JsonScalar]

STAGES: tuple[Stage, ...] = (
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
    "clarification": "prd",
    "prd": "prd_review",
    "prd_review": "issues",
    "issues": "issues_review",
    "issues_review": "workflow",
    "workflow": "workflow_review",
    "workflow_review": "workflow_ready",
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

TEMPLATE_TOKENS = frozenset(
    {
        "ARTIFACT_STATUS",
        "BLOCKERS",
        "LANGUAGE_INSTRUCTION",
        "INITIAL_INTENTION",
        "NEXT_COMMAND",
        "READ_FIRST",
        "REFERENCE_CONTENT",
        "REFERENCE_TITLE",
        "ROOT",
        "STAGE",
    }
)

TEMPLATE_TOKEN_RE = re.compile(r"{{([A-Z][A-Z0-9_]*)}}")
ANY_TEMPLATE_TOKEN_RE = re.compile(r"{{(.*?)}}", flags=re.DOTALL)

REFERENCE_BY_STAGE: dict[Stage, str | None] = {
    "clarification": "grill.md",
    "prd": "prd.md",
    "prd_review": "prd.md",
    "issues": "issues.md",
    "issues_review": "issues.md",
    "workflow": "artifacts.md",
    "workflow_review": "artifacts.md",
    "workflow_ready": None,
}


class ItwError(Exception):
    pass


@dataclass(frozen=True)
class WorkflowState:
    version: int
    stage: Stage
    created_at: str
    root: str
    session_id: str | None
    session_short: str | None
    cwd: str
    model: str | None
    transcript_path: str | None
    language: Language

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
            "language": self.language,
        }

    def with_stage(self, stage: Stage) -> WorkflowState:
        return WorkflowState(
            version=self.version,
            stage=stage,
            created_at=self.created_at,
            root=self.root,
            session_id=self.session_id,
            session_short=self.session_short,
            cwd=self.cwd,
            model=self.model,
            transcript_path=self.transcript_path,
            language=self.language,
        )

    def with_language(self, language: Language) -> WorkflowState:
        return WorkflowState(
            version=self.version,
            stage=self.stage,
            created_at=self.created_at,
            root=self.root,
            session_id=self.session_id,
            session_short=self.session_short,
            cwd=self.cwd,
            model=self.model,
            transcript_path=self.transcript_path,
            language=language,
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


def is_language(value: object) -> TypeGuard[Language]:
    return value in ("fr", "en", "unknown")


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")


def state_path(root: Path) -> Path:
    return root / STATE_FILE


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ItwError(f"missing: {path.name}") from error


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
    raise ItwError(f"invalid state field: {key}")


def load_state(root: Path) -> WorkflowState:
    path = state_path(root)
    try:
        raw_text = path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ItwError(f"missing: {STATE_FILE}") from error

    raw = cast(object, json.loads(raw_text))
    if not isinstance(raw, dict):
        raise ItwError("invalid state: expected object")
    state_json = cast(dict[str, object], raw)

    version = state_json.get("version")
    if not isinstance(version, int):
        raise ItwError("invalid state field: version")

    stage = state_json.get("stage")
    if not is_stage(stage):
        raise ItwError("invalid state field: stage")

    created_at = state_json.get("created_at")
    root_value = state_json.get("root")
    cwd = state_json.get("cwd")
    if (
        not isinstance(created_at, str)
        or not isinstance(root_value, str)
        or not isinstance(cwd, str)
    ):
        raise ItwError("invalid state: missing core string field")

    language = state_json.get("language")
    if not is_language(language):
        raise ItwError("invalid state field: language")

    return WorkflowState(
        version=version,
        stage=stage,
        created_at=created_at,
        root=root_value,
        session_id=extract_optional_text(state_json, "session_id"),
        session_short=extract_optional_text(state_json, "session_short"),
        cwd=cwd,
        model=extract_optional_text(state_json, "model"),
        transcript_path=extract_optional_text(state_json, "transcript_path"),
        language=language,
    )


def write_state(root: Path, state: WorkflowState) -> None:
    state_path(root).write_text(json.dumps(state.to_json(), indent=2) + "\n", encoding="utf-8")


def compact_status(root: Path, state: WorkflowState) -> str:
    next_command = "-"
    if state.stage in NEXT_STAGE:
        next_command = f"{CLI_NAME} advance {root}"
    return f"stage={state.stage} root={root} next={next_command}\n"


def env_value(*names: str) -> str | None:
    for name in names:
        value = os.environ.get(name)
        if value:
            return value
    return None


def session_short_from(session_id: str | None) -> str | None:
    if session_id is None:
        return env_value("ITW_SESSION_SHORT", "CODEX_SESSION_SHORT")
    compact = re.sub(r"[^A-Za-z0-9]", "", session_id)
    if compact == "":
        return None
    return compact[:8]


def init_workflow(root: Path, intention_parts: Sequence[str]) -> str:
    if state_path(root).exists():
        raise ItwError(f"already initialized: {root}")

    intention = " ".join(intention_parts).strip()
    session_id = env_value("ITW_SESSION_ID", "CODEX_SESSION_ID")
    cwd = env_value("ITW_CWD") or str(Path.cwd())
    model = env_value("ITW_MODEL", "CODEX_MODEL")
    transcript_path = env_value("ITW_TRANSCRIPT_PATH", "CODEX_TRANSCRIPT_PATH")
    return init_workflow_with_metadata(
        root=root,
        intention=intention,
        session_id=session_id,
        cwd=cwd,
        model=model,
        transcript_path=transcript_path,
    )


def init_workflow_with_metadata(
    root: Path,
    intention: str,
    session_id: str | None,
    cwd: str,
    model: str | None,
    transcript_path: str | None,
) -> str:
    if state_path(root).exists():
        raise ItwError(f"already initialized: {root}")

    captured_intention = intention.strip()
    if captured_intention == "":
        raise ItwError("initial intention required")

    if root.exists() and not root.is_dir():
        raise ItwError(f"root is not a directory: {root}")
    if root.exists() and any(root.iterdir()):
        raise ItwError(f"root is not empty: {root}")

    root.mkdir(parents=True, exist_ok=True)
    state = WorkflowState(
        version=STATE_VERSION,
        stage="clarification",
        created_at=utc_now(),
        root=str(root),
        session_id=session_id,
        session_short=session_short_from(session_id),
        cwd=cwd,
        model=model,
        transcript_path=transcript_path,
        language=detect_language(captured_intention),
    )

    (root / "intake").write_text(captured_intention + "\n", encoding="utf-8")
    write_if_missing(root / "clarification.md", clarification_template())
    write_state(root, state)
    return compact_status(root, state) + phase_prompt_for_stage(
        "clarification",
        root,
        validation_errors_for_stage(root, "clarification"),
    )


def status_workflow(root: Path) -> str:
    return compact_status(root, load_state(root))


def set_language_workflow(
    root: Path,
    language: str | None,
    from_intake: bool,
    force: bool,
) -> str:
    if language is not None and from_intake:
        raise ItwError("choose language or --from-intake, not both")
    if language is None and not from_intake:
        raise ItwError("language or --from-intake required")

    state = load_state(root)
    if state.language != "unknown" and not force:
        raise ItwError(f"language already set: {state.language}; use --force to override")

    next_language: Language
    if from_intake:
        errors = validate_intake(root)
        if errors:
            raise ItwError("missing: " + ", ".join(errors))
        next_language = detect_language(read_text(root / "intake"))
    else:
        if not is_language(language):
            raise ItwError("invalid language")
        next_language = language

    write_state(root, state.with_language(next_language))
    return f"language={next_language} root={root}\n"


def get_workflow(root: Path) -> str:
    state = load_state(root)
    return compact_status(root, state) + phase_prompt_for_stage(
        state.stage,
        root,
        blockers_for_state(root, state),
    )


def advance_workflow(root: Path) -> str:
    state = load_state(root)
    if state.stage == "workflow_ready":
        raise ItwError("already at workflow_ready")

    validate_current_stage(root, state.stage)
    target = NEXT_STAGE[state.stage]

    if target == "prd":
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
    return output + phase_prompt_for_stage(target, root, validation_errors_for_stage(root, target))


def validate_current_stage(root: Path, stage: Stage) -> None:
    state = load_state(root)
    if state.stage != stage:
        raise ItwError(f"state changed while validating: {state.stage}")
    missing = blockers_for_state(root, state)
    if missing:
        raise ItwError("missing: " + ", ".join(missing))


def blockers_for_state(root: Path, state: WorkflowState) -> tuple[str, ...]:
    errors = list(validation_errors_for_stage(root, state.stage))
    if state.language == "unknown":
        errors.append("language")
    return tuple(dict.fromkeys(errors))


def validation_errors_for_stage(root: Path, stage: Stage) -> tuple[str, ...]:
    validators: dict[Stage, tuple[str, ...]] = {
        "clarification": validate_clarification(root),
        "prd": validate_prd(root),
        "prd_review": validate_prd_review(root),
        "issues": validate_issues(root),
        "issues_review": validate_issues_review(root),
        "workflow": validate_workflow(root),
        "workflow_review": validate_workflow_review(root),
        "workflow_ready": (),
    }
    return (*validate_intake(root), *validators[stage])


def validate_intake(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "intake", require_non_empty=True))
    text = (
        (root / "intake").read_text(encoding="utf-8").strip() if (root / "intake").exists() else ""
    )
    if text == "" or text == "TODO" or text.startswith("TODO:"):
        errors.append("intake initial intention")
    return tuple(errors)


def detect_language(text: str) -> Language:
    normalized = f" {text.lower()} "
    french_markers = (
        " je ",
        " tu ",
        " nous ",
        " vous ",
        " le ",
        " la ",
        " les ",
        " des ",
        " une ",
        " pour ",
        " avec ",
        " dans ",
        " est-ce ",
        " que ",
        " qui ",
        " pas ",
    )
    if any(marker in normalized for marker in french_markers) or re.search(
        r"[àâçéèêëîïôùûü]",
        normalized,
    ):
        return "fr"
    if normalized.strip():
        return "en"
    return "unknown"


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


def phase_prompt_for_stage(stage: Stage, root: Path, blockers: Sequence[str]) -> str:
    reference_name = REFERENCE_BY_STAGE[stage]
    reference_title = "None"
    reference_content = "No current-phase reference."
    if reference_name is not None:
        reference_title = reference_name
        reference_content = read_package_text("references", reference_name).strip()

    return render_template(
        read_package_text("templates", f"{stage}.md"),
        {
            "ARTIFACT_STATUS": artifact_status(root),
            "BLOCKERS": blocker_text(blockers),
            "INITIAL_INTENTION": read_text(root / "intake").strip(),
            "LANGUAGE_INSTRUCTION": language_instruction(load_state(root).language),
            "NEXT_COMMAND": next_command_for(stage, root),
            "READ_FIRST": read_first_for(stage, root),
            "REFERENCE_CONTENT": reference_content,
            "REFERENCE_TITLE": reference_title,
            "ROOT": str(root),
            "STAGE": stage,
        },
    )


def read_package_text(group: str, name: str) -> str:
    try:
        return files(__package__).joinpath(group, name).read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ItwError(f"missing packaged resource: {group}/{name}") from error


def render_template(template: str, values: Mapping[str, str]) -> str:
    malformed = [
        match.group(0)
        for match in ANY_TEMPLATE_TOKEN_RE.finditer(template)
        if TEMPLATE_TOKEN_RE.fullmatch(match.group(0)) is None
    ]
    if malformed:
        raise ItwError(f"malformed template token(s): {', '.join(sorted(set(malformed)))}")

    used_tokens = set(TEMPLATE_TOKEN_RE.findall(template))
    unknown_tokens = used_tokens - TEMPLATE_TOKENS
    if unknown_tokens:
        raise ItwError(f"unknown template token(s): {', '.join(sorted(unknown_tokens))}")

    missing_values = used_tokens - values.keys()
    if missing_values:
        raise ItwError(f"missing template value(s): {', '.join(sorted(missing_values))}")

    rendered = TEMPLATE_TOKEN_RE.sub(lambda match: values[match.group(1)], template)
    return rendered


def blocker_text(blockers: Sequence[str]) -> str:
    if not blockers:
        return "- none"
    return "\n".join(f"- {blocker}" for blocker in blockers)


def language_instruction(language: Language) -> str:
    if language == "fr":
        return "Respond in French."
    if language == "en":
        return "Respond in English."
    return "Respond in the same language as the initial intention."


def next_command_for(stage: Stage, root: Path) -> str:
    if stage == "workflow_ready":
        return "-"
    return f"{CLI_NAME} advance {root}"


def read_first_for(stage: Stage, root: Path) -> str:
    common = [root / "intake"]
    phase_files: dict[Stage, tuple[str, ...]] = {
        "clarification": ("clarification.md",),
        "prd": ("clarification.md", "prd.md"),
        "prd_review": ("prd.md",),
        "issues": ("prd.md", "issues.md"),
        "issues_review": ("issues.md",),
        "workflow": ("prd.md", "issues.md", "workflow.md", "tracker.md"),
        "workflow_review": ("workflow.md", "tracker.md", "prompts/"),
        "workflow_ready": ("workflow.md", "tracker.md", "prompts/"),
    }
    paths = [*common, *(root / relative for relative in phase_files[stage])]
    seen: set[str] = set()
    lines: list[str] = []
    for path in paths:
        value = str(path)
        if value not in seen:
            seen.add(value)
            lines.append(f"- `{value}`")
    return "\n".join(lines)


def artifact_status(root: Path) -> str:
    artifacts: tuple[str, ...] = (
        "intake",
        "clarification.md",
        "prd.md",
        "issues.md",
        "workflow.md",
        "tracker.md",
        "prompts/",
        STATE_FILE,
    )
    lines: list[str] = []
    for artifact in artifacts:
        path = root / artifact
        exists = path.is_dir() if artifact.endswith("/") else path.is_file()
        lines.append(f"- `{artifact}`: {'present' if exists else 'missing'}")
    return "\n".join(lines)


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
        raise ItwError("missing: issues.md issue heading")

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

- `intake`
- `clarification.md`
- `prd.md`
- `issues.md`
- `tracker.md`

## Execution Contract

- Read this file, `tracker.md`, and `issues.md` before changing code.
- Treat `tracker.md` as execution authority.
- Execute one issue at a time unless the workflow explicitly allows parallel work.
- Worker prompts live under `prompts/`.
- Workers follow the local TDD contract embedded in their prompts.
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
    return f"""You are implementing one scoped issue from an existing workflow.

Why:
This workflow was prepared through intake, clarification, PRD, and issue review.

Read first:
- `{root / "workflow.md"}`
- `{root / "tracker.md"}`
- `{root / "intake"}`
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

## TDD Contract

{read_package_text("references", "tdd.md").strip()}

## Issue Body

{issue.body}
"""


def reviewer_prompt(issue: Issue, root: Path) -> str:
    return f"""Read-only review.

Review issue {issue.number}: {issue.title}

Read:
- `{root / "workflow.md"}`
- `{root / "tracker.md"}`
- `{root / "intake"}`
- `{root / "prd.md"}`
- `{root / "issues.md"}`

Rules:
- Do not edit files.
- Prioritize bugs, contract drift, missed validation, and scope creep.
- Write findings with file/line references when possible.
- Save the review to `{root / "reviews" / f"{issue.slug}-review.md"}` if possible.
- Otherwise report it to the workflow owner.
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
