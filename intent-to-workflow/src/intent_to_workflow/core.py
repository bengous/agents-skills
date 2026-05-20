from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tomllib
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from hashlib import sha256
from importlib.resources import files
from pathlib import Path
from typing import Literal, TypeGuard, cast

from intent_to_workflow import __version__

CLI_NAME = "itw"
WORKFLOW_DIR = ".itw"
STATE_FILE = ".itw-state.json"
STATE_VERSION = 1
GITIGNORE_ENTRY = (
    ".itw/\n"
    "# If you do want to track .itw, remove the line above. If you do not want to track "
    "it but not add this line to gitignore, move it to .git/info/exclude."
)

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

type Language = Literal[
    "en",
    "fr",
    "es",
    "de",
    "it",
    "pt",
    "nl",
    "pl",
    "ru",
    "uk",
    "ja",
    "ko",
    "zh",
    "ar",
    "hi",
    "tr",
    "sv",
    "da",
    "no",
    "fi",
    "cs",
    "ro",
]
type JsonScalar = str | int | None
type JsonObject = dict[str, JsonScalar]


@dataclass(frozen=True)
class ReviewerType:
    slug: str
    persona: str
    agent_type: str
    description: str
    capability: str
    focus: str
    usage: str


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
LOCALIZED_PLANNING_STAGES: frozenset[Stage] = frozenset(
    {"clarification", "prd", "prd_review"}
)
ENGLISH_EXECUTION_ARTIFACT_STAGES: frozenset[Stage] = frozenset(
    {"issues", "issues_review", "workflow", "workflow_review", "workflow_ready"}
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
LOCALIZED_PRD_SECTIONS: dict[Language, tuple[str, ...]] = {
    "fr": (
        "## Probleme",
        "## Solution",
        "## Parcours utilisateur",
        "## Decisions d'implementation",
    ),
}

TERMINOLOGY_FILE = "terminology.md"
TERMINOLOGY_SECTIONS = (
    "## Actors and Roles",
    "## Canonical Terms",
    "## Relationships",
    "## Flagged Ambiguities",
)
LOCALIZED_TERMINOLOGY_SECTIONS: dict[Language, tuple[str, ...]] = {
    "fr": (
        "## Acteurs et roles",
        "## Termes canoniques",
        "## Relations",
        "## Ambiguites signalees",
    ),
}

ISSUE_FIELDS = (
    "Type:",
    "Depends on:",
    "Goal:",
    "Acceptance:",
    "TDD:",
    "Validation:",
    "Agent:",
)

DEFAULT_LANGUAGE: Language = "en"
LOCALIZED_ARTIFACT_LANGUAGES: frozenset[Language] = frozenset({"fr"})

LANGUAGE_NAMES: dict[Language, str] = {
    "en": "English",
    "fr": "French",
    "es": "Spanish",
    "de": "German",
    "it": "Italian",
    "pt": "Portuguese",
    "nl": "Dutch",
    "pl": "Polish",
    "ru": "Russian",
    "uk": "Ukrainian",
    "ja": "Japanese",
    "ko": "Korean",
    "zh": "Chinese",
    "ar": "Arabic",
    "hi": "Hindi",
    "tr": "Turkish",
    "sv": "Swedish",
    "da": "Danish",
    "no": "Norwegian",
    "fi": "Finnish",
    "cs": "Czech",
    "ro": "Romanian",
}

LANGUAGE_ARTIFACT_POLICIES: dict[Language, str] = {
    "fr": (
        "Use French for human-facing prose and Markdown headings/body in "
        "`clarification.md`, `terminology.md`, and `prd.md`. Keep `issues.md`, "
        "`workflow.md`, `tracker.md`, and `prompts/*.md` in their established English "
        "structure. Keep file names, commands, code identifiers, required machine field "
        "labels, and canonical product/technical terms in English when those terms "
        "should map to code or international product vocabulary. In `terminology.md`, "
        "define those English canonical terms with French explanations and French "
        "aliases to avoid."
    ),
}

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
    "prd_review": "prd_review.md",
    "issues": "issues.md",
    "issues_review": "issues_review.md",
    "workflow": "workflow_types.md",
    "workflow_review": "workflow_types.md",
    "workflow_ready": None,
}

CODEX_WORKER_AGENT_TYPE = "itw-worker"
CODEX_AGENT_RESOURCE_DIR = ("agents", "codex")
CODEX_AGENT_RESOURCE_FILES = (
    "itw-worker.toml",
    "itw-simplification-reviewer.toml",
    "itw-security-reviewer.toml",
    "itw-contract-reviewer.toml",
)
CODEX_AGENT_CONFIG_EXAMPLE = "config.example.toml"
CODEX_AGENT_CONFIG_BEGIN = "# BEGIN intent-to-workflow codex agents"
CODEX_AGENT_CONFIG_END = "# END intent-to-workflow codex agents"
REVIEWER_TYPES: tuple[ReviewerType, ...] = (
    ReviewerType(
        slug="simplification",
        persona="Simplification Reviewer",
        agent_type="itw-simplification-reviewer",
        description="simplification reviewer in report-only mode",
        capability="Embedded report-only simplify-wip capability in the agent persona.",
        focus=(
            "Check whether the WIP has safe simplification opportunities: avoidable "
            "complexity, duplication, unnecessary abstractions, missed existing utilities, "
            "or local inefficiencies."
        ),
        usage=(
            "Use the embedded simplify-wip capability from your persona. Keep the review "
            "report-only: do not edit, commit, stage, or format files."
        ),
    ),
    ReviewerType(
        slug="security",
        persona="Security Reviewer",
        agent_type="itw-security-reviewer",
        description="security reviewer on WIP changes",
        capability="Embedded WIP security-review capability in the agent persona.",
        focus=(
            "Check whether the WIP introduces security risks: unsafe input handling, "
            "secret exposure, privilege boundaries, injection paths, or dependency risks."
        ),
        usage=(
            "Use the embedded security-review capability from your persona. Keep the review "
            "read-only and work from the embedded procedure directly."
        ),
    ),
    ReviewerType(
        slug="contract",
        persona="Contract Reviewer",
        agent_type="itw-contract-reviewer",
        description="contract reviewer against the PRD and current issue",
        capability="Embedded contract-review capability in the agent persona.",
        focus=(
            "Check whether the WIP drifts from the PRD, terminology, current issue scope, "
            "acceptance criteria, or validation contract."
        ),
        usage=(
            "Use the embedded contract-review capability from your persona. Keep the review "
            "read-only and work from the embedded procedure directly."
        ),
    ),
)


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

def is_stage(value: object) -> TypeGuard[Stage]:
    return isinstance(value, str) and value in STAGES


def is_language(value: object) -> TypeGuard[Language]:
    return isinstance(value, str) and value in LANGUAGE_NAMES


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")


def state_path(root: Path) -> Path:
    return root / STATE_FILE


def validate_workflow_id(value: str | Path) -> str:
    workflow_id = str(value).strip()
    path = Path(workflow_id)
    if (
        workflow_id == ""
        or path.is_absolute()
        or len(path.parts) != 1
        or workflow_id in {".", ".."}
    ):
        raise ItwError(
            "invalid workflow id; use a plain id like `etch-v2`, not a path. "
            f"Received: {value}"
        )
    return workflow_id


def git_root_for(start: Path) -> Path | None:
    current = start.resolve()
    for candidate in (current, *current.parents):
        if (candidate / ".git").exists():
            return candidate
    return None


def workspace_root() -> Path:
    return git_root_for(Path.cwd()) or Path.cwd()


def workflow_root_for_id(workflow_id: str | Path, base: Path | None = None) -> Path:
    root_base = workspace_root() if base is None else (git_root_for(base) or base)
    return root_base / WORKFLOW_DIR / validate_workflow_id(workflow_id)


def ensure_gitignore_excludes_workflow_dir(base: Path) -> None:
    if not (base / ".git").exists():
        return

    gitignore = base / ".gitignore"
    existing = gitignore.read_text(encoding="utf-8") if gitignore.exists() else ""
    for line in existing.splitlines():
        stripped = line.strip()
        if stripped == ".itw/" or stripped.startswith(".itw/ "):
            return

    separator = "" if existing == "" or existing.endswith("\n") else "\n"
    gitignore.write_text(existing + separator + GITIGNORE_ENTRY + "\n", encoding="utf-8")


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


def workflow_id_for_root(root: Path) -> str:
    return root.name


def compact_status(root: Path, state: WorkflowState) -> str:
    workflow_id = workflow_id_for_root(root)
    next_command = "-"
    if state.stage in NEXT_STAGE:
        next_command = f"{CLI_NAME} advance {workflow_id}"
    return f"stage={state.stage} id={workflow_id} root={root} next={next_command}\n"


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


def init_workflow(workflow_id: str | Path) -> str:
    root = workflow_root_for_id(workflow_id)
    if state_path(root).exists():
        raise ItwError(f"already initialized: {validate_workflow_id(workflow_id)}")

    session_id = env_value("ITW_SESSION_ID", "CODEX_SESSION_ID")
    cwd = env_value("ITW_CWD") or str(Path.cwd())
    model = env_value("ITW_MODEL", "CODEX_MODEL")
    transcript_path = env_value("ITW_TRANSCRIPT_PATH", "CODEX_TRANSCRIPT_PATH")
    return init_workflow_with_metadata(
        workflow_id=validate_workflow_id(workflow_id),
        session_id=session_id,
        cwd=cwd,
        model=model,
        transcript_path=transcript_path,
    )


def init_workflow_with_metadata(
    workflow_id: str,
    session_id: str | None,
    cwd: str,
    model: str | None,
    transcript_path: str | None,
    base: Path | None = None,
) -> str:
    root = workflow_root_for_id(workflow_id, base=base)
    if state_path(root).exists():
        raise ItwError(f"already initialized: {workflow_id}")

    if root.exists() and not root.is_dir():
        raise ItwError(f"root is not a directory: {root}")
    if root.exists() and any(root.iterdir()):
        raise ItwError(f"root is not empty: {root}")

    root.mkdir(parents=True, exist_ok=True)
    ensure_gitignore_excludes_workflow_dir(root.parent.parent)
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
        language=DEFAULT_LANGUAGE,
    )

    (root / "intake").write_text("", encoding="utf-8")
    write_if_missing(root / "clarification.md", clarification_template(DEFAULT_LANGUAGE))
    write_if_missing(root / TERMINOLOGY_FILE, terminology_template(DEFAULT_LANGUAGE))
    write_state(root, state)
    return compact_status(root, state)


def status_workflow(workflow_id: str | Path, base: Path | None = None) -> str:
    root = workflow_root_for_id(workflow_id, base=base)
    return compact_status(root, load_state(root))


def set_language_workflow(
    workflow_id: str | Path,
    language: str | None,
    force: bool,
    base: Path | None = None,
) -> str:
    if language is None:
        raise ItwError("language required")

    root = workflow_root_for_id(workflow_id, base=base)
    state = load_state(root)
    if state.language != DEFAULT_LANGUAGE and not force:
        raise ItwError(f"language already set: {state.language}; use --force to override")

    if not is_language(language):
        supported = ", ".join(LANGUAGE_NAMES)
        raise ItwError(f"unsupported language: {language}; supported: {supported}")
    next_language = language

    write_state(root, state.with_language(next_language))
    refresh_placeholder_artifacts_for_language(root, next_language)
    return (
        f"language={next_language} name={LANGUAGE_NAMES[next_language]} "
        f"id={workflow_id_for_root(root)} root={root}\n"
    )


def refresh_placeholder_artifacts_for_language(root: Path, language: Language) -> None:
    target_language = language if language in LOCALIZED_ARTIFACT_LANGUAGES else DEFAULT_LANGUAGE
    replacements = (
        (
            "clarification.md",
            known_clarification_templates(),
            clarification_template(target_language),
        ),
        (
            TERMINOLOGY_FILE,
            known_terminology_templates(),
            terminology_template(target_language),
        ),
        ("prd.md", known_prd_templates(), prd_template(target_language)),
    )
    for relative, known_placeholders, target_content in replacements:
        path = root / relative
        if path.exists() and path.read_text(encoding="utf-8") in known_placeholders:
            path.write_text(target_content, encoding="utf-8")


def known_clarification_templates() -> frozenset[str]:
    languages = (DEFAULT_LANGUAGE, *LOCALIZED_ARTIFACT_LANGUAGES)
    return frozenset(
        clarification_template(language)
        for language in languages
    )


def known_terminology_templates() -> frozenset[str]:
    languages = (DEFAULT_LANGUAGE, *LOCALIZED_ARTIFACT_LANGUAGES)
    return frozenset(
        terminology_template(language)
        for language in languages
    )


def known_prd_templates() -> frozenset[str]:
    languages = (DEFAULT_LANGUAGE, *LOCALIZED_ARTIFACT_LANGUAGES)
    return frozenset(
        prd_template(language)
        for language in languages
    )


def get_workflow(workflow_id: str | Path, base: Path | None = None) -> str:
    root = workflow_root_for_id(workflow_id, base=base)
    state = load_state(root)
    return compact_status(root, state) + phase_prompt_for_stage(
        state.stage,
        root,
        blockers_for_state(root, state),
    )


def advance_workflow(workflow_id: str | Path, base: Path | None = None) -> str:
    root = workflow_root_for_id(workflow_id, base=base)
    state = load_state(root)
    if state.stage == "workflow_ready":
        raise ItwError("already at workflow_ready")

    validate_current_stage(root, state.stage)
    target = NEXT_STAGE[state.stage]

    if target == "prd":
        write_if_missing(root / "prd.md", prd_template(state.language))
    elif target == "issues":
        write_if_missing(root / "issues.md", issues_template())
    elif target == "workflow":
        create_workflow_package(root)

    next_state = state.with_stage(target)
    write_state(root, next_state)

    output = compact_status(root, next_state)
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


def validate_clarification(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "clarification.md", require_non_empty=True))
    errors.extend(file_errors(root, TERMINOLOGY_FILE))
    if (root / "clarification.md").exists():
        errors.extend(placeholder_errors("clarification.md", read_text(root / "clarification.md")))
    return tuple(errors)


def validate_prd(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, "prd.md", require_non_empty=True))
    errors.extend(validate_terminology(root))
    text = (root / "prd.md").read_text(encoding="utf-8") if (root / "prd.md").exists() else ""
    errors.extend(section for section in prd_sections_for(root) if section not in text)
    errors.extend(placeholder_errors("prd.md", text))
    return tuple(errors)


def validate_prd_review(root: Path) -> tuple[str, ...]:
    return validate_prd(root)


def validate_terminology(root: Path) -> tuple[str, ...]:
    errors = list(file_errors(root, TERMINOLOGY_FILE, require_non_empty=True))
    if (root / TERMINOLOGY_FILE).exists():
        text = read_text(root / TERMINOLOGY_FILE)
        errors.extend(section for section in terminology_sections_for(root) if section not in text)
        errors.extend(placeholder_errors(TERMINOLOGY_FILE, text))
    return tuple(errors)


def prd_sections_for(root: Path) -> tuple[str, ...]:
    return LOCALIZED_PRD_SECTIONS.get(load_state(root).language, PRD_SECTIONS)


def terminology_sections_for(root: Path) -> tuple[str, ...]:
    return LOCALIZED_TERMINOLOGY_SECTIONS.get(load_state(root).language, TERMINOLOGY_SECTIONS)


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


def clarification_template(language: Language = DEFAULT_LANGUAGE) -> str:
    if language == "fr":
        return """# Clarification

Consigner chaque question avant de poser la suivante.
Garder `terminology.md` a jour quand une reponse clarifie un acteur, un role,
un terme canonique, une relation ou une ambiguite de langue. Ne pas le mettre
a jour mecaniquement apres chaque reponse.

## Q001

Question: TODO

Recommandation: TODO

Reponse: TODO

Decision: TODO
"""
    return """# Clarification

Record each question before asking the next one.
Keep `terminology.md` current when an answer clarifies an actor, role,
canonical term, relationship, or language ambiguity. Do not update it
mechanically after every answer.

## Q001

Question: TODO

Reco: TODO

Answer: TODO

Decision: TODO
"""


def terminology_template(language: Language = DEFAULT_LANGUAGE) -> str:
    if language == "fr":
        return """# Terminologie

Modele de langage local pour l'intention. Garder ce fichier a jour pendant la
clarification quand la comprehension change; le finaliser avant la revue PRD.
Utiliser `Aucun identifie` quand une section requise n'a pas d'entree utile.
Les termes canoniques produit/techniques peuvent rester en anglais quand ils
doivent se traduire en code ou en vocabulaire produit international.

## Acteurs et roles

| Acteur | Definition | Responsabilites | Pouvoir de decision | Contraintes |
| --- | --- | --- | --- | --- |
| TODO | TODO | TODO | TODO | TODO |

## Termes canoniques

| Terme | Definition | Alias a eviter |
| --- | --- | --- |
| TODO | TODO | TODO |

## Relations

- TODO

## Ambiguites signalees

- TODO
"""
    return """# Terminology

Working language model for the intention. Keep this file current during
clarification when the understanding changes; finalize it before PRD review.
Use `None identified` when a required section has no useful entry.

## Actors and Roles

| Actor | Definition | Responsibilities | Decision Power | Constraints |
| --- | --- | --- | --- | --- |
| TODO | TODO | TODO | TODO | TODO |

## Canonical Terms

| Term | Definition | Aliases to Avoid |
| --- | --- | --- |
| TODO | TODO | TODO |

## Relationships

- TODO

## Flagged Ambiguities

- TODO
"""


def prd_template(language: Language = DEFAULT_LANGUAGE) -> str:
    if language == "fr":
        return """# PRD

## Probleme

TODO

## Solution

TODO

## Parcours utilisateur

- TODO

## Decisions d'implementation

- TODO

## Hors perimetre

- TODO
"""
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
            "LANGUAGE_INSTRUCTION": language_instruction(load_state(root).language, root, stage),
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


def read_package_resource(path_parts: Sequence[str]) -> str:
    try:
        return files(__package__).joinpath(*path_parts).read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ItwError(f"missing packaged resource: {'/'.join(path_parts)}") from error


def codex_home() -> Path:
    return Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser()


def codex_agents_dir() -> Path:
    return codex_home() / "agents"


def codex_config_path() -> Path:
    return codex_home() / "config.toml"


def codex_agent_resource_text(name: str) -> str:
    return read_package_resource((*CODEX_AGENT_RESOURCE_DIR, name))


def codex_agent_config_example() -> str:
    return codex_agent_resource_text(CODEX_AGENT_CONFIG_EXAMPLE).strip()


def setup_fingerprint() -> str:
    digest = sha256()
    digest.update(f"intent-to-workflow\0{__version__}\0".encode())
    for name in (CODEX_AGENT_CONFIG_EXAMPLE, *CODEX_AGENT_RESOURCE_FILES):
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(codex_agent_resource_text(name).encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()


def setup_fingerprint_status() -> str:
    return f"setup_fingerprint={setup_fingerprint()} version={__version__}\n"


def parse_toml_config(label: str, text: str) -> dict[str, object]:
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise ItwError(f"invalid TOML in {label}: {error}") from error
    return cast(dict[str, object], raw)


def expected_codex_agent_config() -> dict[str, dict[str, str]]:
    raw = parse_toml_config(CODEX_AGENT_CONFIG_EXAMPLE, codex_agent_config_example())
    agents = raw.get("agents")
    if not isinstance(agents, dict):
        raise ItwError(f"invalid {CODEX_AGENT_CONFIG_EXAMPLE}: missing agents table")

    expected: dict[str, dict[str, str]] = {}
    agent_items = cast(dict[str, object], agents)
    for name, value in agent_items.items():
        if not isinstance(value, dict):
            raise ItwError(f"invalid {CODEX_AGENT_CONFIG_EXAMPLE}: agents table")
        values = cast(dict[str, object], value)
        description = values.get("description")
        config_file = values.get("config_file")
        if not isinstance(description, str) or not isinstance(config_file, str):
            raise ItwError(f"invalid {CODEX_AGENT_CONFIG_EXAMPLE}: {name}")
        expected[name] = {"description": description, "config_file": config_file}
    return expected


def expected_codex_agent_names() -> tuple[str, ...]:
    return tuple(expected_codex_agent_config().keys())


def codex_agent_config_block(names: Iterable[str] | None = None) -> str:
    expected = expected_codex_agent_config()
    selected = tuple(expected) if names is None else tuple(names)
    lines = [CODEX_AGENT_CONFIG_BEGIN]
    for name in selected:
        values = expected[name]
        lines.extend(
            [
                "",
                f"[agents.{name}]",
                f"description = {json.dumps(values['description'])}",
                f"config_file = {json.dumps(values['config_file'])}",
            ]
        )
    lines.append(CODEX_AGENT_CONFIG_END)
    return "\n".join(lines) + "\n"


def codex_config_text() -> str:
    path = codex_config_path()
    return path.read_text(encoding="utf-8") if path.exists() else ""


def unique_backup_path(path: Path) -> Path:
    timestamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    candidate = path.with_name(f"{path.name}.itw-backup-{timestamp}")
    if not candidate.exists():
        return candidate
    for index in range(1, 1000):
        next_candidate = path.with_name(f"{path.name}.itw-backup-{timestamp}-{index}")
        if not next_candidate.exists():
            return next_candidate
    raise ItwError(f"could not allocate backup path for {path}")


def backup_existing_file(path: Path) -> Path | None:
    if not path.exists():
        return None
    backup_path = unique_backup_path(path)
    backup_path.write_text(path.read_text(encoding="utf-8"), encoding="utf-8")
    return backup_path


def codex_config_data() -> dict[str, object]:
    text = codex_config_text()
    if text.strip() == "":
        return {}
    return parse_toml_config(str(codex_config_path()), text)


def codex_config_agent_table() -> dict[str, object]:
    raw = codex_config_data().get("agents", {})
    if raw == {}:
        return {}
    if not isinstance(raw, dict):
        raise ItwError(f"invalid TOML in {codex_config_path()}: agents is not a table")
    return cast(dict[str, object], raw)


def codex_agent_role_errors() -> tuple[str, ...]:
    expected = expected_codex_agent_config()
    agents = codex_config_agent_table()
    errors: list[str] = []
    for name, values in expected.items():
        existing = agents.get(name)
        if not isinstance(existing, dict):
            errors.append(f"missing config role [agents.{name}]")
            continue
        existing_values = cast(dict[str, object], existing)
        description = existing_values.get("description")
        config_file = existing_values.get("config_file")
        if description != values["description"]:
            errors.append(f"stale description for [agents.{name}]")
        if config_file != values["config_file"]:
            errors.append(f"stale config_file for [agents.{name}]")
    return tuple(errors)


def codex_agent_file_errors() -> tuple[str, ...]:
    errors: list[str] = []
    target_dir = codex_agents_dir()
    for name in CODEX_AGENT_RESOURCE_FILES:
        target = target_dir / name
        expected = codex_agent_resource_text(name)
        if not target.exists():
            errors.append(f"missing {target}")
        elif target.read_text(encoding="utf-8") != expected:
            errors.append(f"stale {target}")
    return tuple(errors)


def status_codex_agents() -> str:
    errors = [*codex_agent_file_errors(), *codex_agent_role_errors()]
    if errors:
        raise ItwError(
            "codex agents not installed: "
            + "; ".join(errors)
            + "; run `itw setup install`"
        )
    names = ", ".join(expected_codex_agent_names())
    return f"codex_agents=installed home={codex_home()} agents={names}\n"


def global_itw_fingerprint_error(itw_path: str) -> str | None:
    expected = setup_fingerprint_status().strip()
    try:
        completed = subprocess.run(
            [itw_path, "setup", "fingerprint"],
            capture_output=True,
            check=False,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"could not verify `{CLI_NAME}` fingerprint at {itw_path}: {error}"

    actual = completed.stdout.strip()
    if completed.returncode != 0:
        detail = completed.stderr.strip() or actual or f"exit {completed.returncode}"
        return f"could not verify `{CLI_NAME}` fingerprint at {itw_path}: {detail}"
    if actual != expected:
        return f"stale or mismatched `{CLI_NAME}` command at {itw_path}"
    return None


def setup_status() -> str:
    errors: list[str] = []
    itw_path = shutil.which(CLI_NAME)
    if itw_path is None:
        errors.append(f"missing global `{CLI_NAME}` command")
    else:
        fingerprint_error = global_itw_fingerprint_error(itw_path)
        if fingerprint_error is not None:
            errors.append(fingerprint_error)
    errors.extend(codex_agent_file_errors())
    errors.extend(codex_agent_role_errors())
    if errors:
        raise ItwError(
            "setup incomplete: "
            + "; ".join(errors)
            + "; run the installed skill setup script: `<skill-dir>/scripts/setup`"
        )

    names = ", ".join(expected_codex_agent_names())
    return f"setup=ok command={itw_path} codex_home={codex_home()} agents={names}\n"


def replace_marked_codex_agent_block(config_text: str, block: str) -> str | None:
    pattern = re.compile(
        rf"(?ms)^{re.escape(CODEX_AGENT_CONFIG_BEGIN)}$.*?"
        rf"^{re.escape(CODEX_AGENT_CONFIG_END)}$\n?"
    )
    if pattern.search(config_text) is None:
        return None
    return pattern.sub(block, config_text)


def missing_codex_agent_role_names() -> tuple[str, ...]:
    expected = expected_codex_agent_config()
    agents = codex_config_agent_table()
    missing: list[str] = []
    for name, values in expected.items():
        existing = agents.get(name)
        if existing is None:
            missing.append(name)
            continue
        if not isinstance(existing, dict):
            raise ItwError(f"invalid config role [agents.{name}]")
        existing_values = cast(dict[str, object], existing)
        if (
            existing_values.get("description") != values["description"]
            or existing_values.get("config_file") != values["config_file"]
        ):
            raise ItwError(
                f"existing [agents.{name}] does not match intent-to-workflow; "
                f"edit {codex_config_path()} or remove that role before reinstalling"
            )
    return tuple(missing)


def install_codex_agents() -> str:
    target_dir = codex_agents_dir()
    target_dir.mkdir(parents=True, exist_ok=True)
    for name in CODEX_AGENT_RESOURCE_FILES:
        source_text = codex_agent_resource_text(name)
        target = target_dir / name
        target.write_text(source_text, encoding="utf-8")

    config_path = codex_config_path()
    config_path.parent.mkdir(parents=True, exist_ok=True)
    existing_text = codex_config_text()
    full_block = codex_agent_config_block()
    next_text = replace_marked_codex_agent_block(existing_text, full_block)
    if next_text is None:
        missing = missing_codex_agent_role_names()
        if missing:
            separator = "" if existing_text == "" or existing_text.endswith("\n") else "\n"
            next_text = existing_text + separator
            if existing_text.strip() != "":
                next_text += "\n"
            next_text += codex_agent_config_block(missing)
        else:
            next_text = existing_text

    backup_path: Path | None = None
    if next_text != existing_text:
        backup_path = backup_existing_file(config_path)
        config_path.write_text(next_text, encoding="utf-8")

    # Fail closed if the write produced invalid or incomplete config.
    status_codex_agents()
    files = ", ".join(CODEX_AGENT_RESOURCE_FILES)
    backup_text = f" backup={backup_path}" if backup_path is not None else ""
    return (
        f"codex_agents=installed home={codex_home()} files={files} "
        f"config={config_path}{backup_text}\n"
    )


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


def language_instruction(language: Language, root: Path, stage: Stage) -> str:
    name = LANGUAGE_NAMES[language]
    workflow_id = workflow_id_for_root(root)
    if language == DEFAULT_LANGUAGE:
        return (
            f"Default language is {name}. If the initial intention is not in {name}, "
            f"first run `itw set-language {workflow_id} <language-code>` and then "
            f"respond in that language; otherwise respond in {name}."
        )
    if language in LOCALIZED_ARTIFACT_LANGUAGES and stage in ENGLISH_EXECUTION_ARTIFACT_STAGES:
        return (
            f"Use English for generated execution artifacts in this phase, including "
            f"`issues.md`, `workflow.md`, `tracker.md`, and `prompts/*.md` prose/body, "
            f"headings, and machine-facing labels. You may answer the human in {name} "
            f"outside those artifacts."
        )
    artifact_policy = (
        LANGUAGE_ARTIFACT_POLICIES[language]
        if language in LOCALIZED_ARTIFACT_LANGUAGES and stage in LOCALIZED_PLANNING_STAGES
        else (
            f"{name} is instruction-only for artifacts: use {name} for human-facing prose "
            "inside artifact sections, but keep English Markdown headings, file names, "
            "commands, code identifiers, required machine field labels, and canonical "
            "product/technical terms."
        )
    )
    return f"Respond in {name}. {artifact_policy}"


def next_command_for(stage: Stage, root: Path) -> str:
    if stage == "workflow_ready":
        return "-"
    return f"{CLI_NAME} advance {workflow_id_for_root(root)}"


def read_first_for(stage: Stage, root: Path) -> str:
    common = [root / "intake"]
    phase_files: dict[Stage, tuple[str, ...]] = {
        "clarification": ("clarification.md", TERMINOLOGY_FILE),
        "prd": ("clarification.md", TERMINOLOGY_FILE, "prd.md"),
        "prd_review": ("clarification.md", TERMINOLOGY_FILE, "prd.md"),
        "issues": (TERMINOLOGY_FILE, "prd.md", "issues.md"),
        "issues_review": ("prd.md", "issues.md"),
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
        TERMINOLOGY_FILE,
        "prd.md",
        "issues.md",
        "workflow.md",
        "tracker.md",
        "prompts/",
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
        (root / "reviews").mkdir(exist_ok=True)
        for reviewer_type in REVIEWER_TYPES:
            write_if_missing(
                root / "prompts" / f"{issue.slug}-{reviewer_type.slug}-reviewer.md",
                reviewer_prompt(issue, root, reviewer_type),
            )

    write_if_missing(root / "workflow.md", workflow_template(root, issues))
    write_if_missing(root / "tracker.md", tracker_template(issues))


def issue_lines(issues: Iterable[Issue]) -> str:
    return "\n".join(f"- {issue.number}. {issue.title}" for issue in issues)


def codex_agent_type_lines() -> str:
    lines = [f"- Worker: `{CODEX_WORKER_AGENT_TYPE}`"]
    lines.extend(
        f"- {reviewer.persona}: `{reviewer.agent_type}`"
        for reviewer in REVIEWER_TYPES
    )
    return "\n".join(lines)


def workflow_template(root: Path, issues: Sequence[Issue]) -> str:
    return f"""# Workflow

## Sources

- `intake`
- `clarification.md`
- `prd.md`
- `issues.md`
- `tracker.md`

## Workflow Type Decision

Selected type: PRD-to-slices reviewed execution

Recommendation basis:

- The planning process produced a PRD and independently grabbable vertical-slice issues.
- The future execution should preserve a visible PRD/issue contract while
  implementation proceeds slice by slice.
- Each slice benefits from explicit validation plus clean post-implementation review before commit.

Available workflow types:

- PRD-to-slices reviewed execution: recommended default for fuzzy intentions
  turned into agent implementation work.
- Tuned workflow: start from a built-in workflow type and record the changes in this file.
- Pair-designed custom workflow: define the workflow from scratch with the
  human when no built-in type fits.

Tuning notes:

- Base workflow: PRD-to-slices reviewed execution.
- Changes from base: none yet.
- If this workflow is tuned or replaced, keep `issues.md` as the build scope,
  `workflow.md` as the execution method, and `tracker.md` as execution authority.

## Execution Contract

- Read this file, `tracker.md`, and `issues.md` before changing code.
- Treat `tracker.md` as execution authority.
- Execute one issue at a time unless the workflow explicitly allows parallel work.
- Worker prompts live under `prompts/`.
- Workers follow the local TDD contract embedded in their prompts.
- Before spawning Codex subagents with `itw-*` `agent_type`s, verify
  `itw setup status`.
- After each worker implementation, run slice validation, ask the three clean
  reviewers, accept/reject/defer each finding with rationale, apply accepted
  fixes, revalidate, then commit the scoped slice locally.
- Keep execution artifacts local unless the human explicitly asks otherwise.
- Do not execute work outside this workflow root's scope.

## Codex Agent Types and Capabilities

Use Codex `agent_type` for persona/capability selection. Keep subagent
`message` content limited to the assigned task, scope, context, and requested
output.

Codex agent types:

{codex_agent_type_lines()}

- Worker:
  - Agent type: `{CODEX_WORKER_AGENT_TYPE}`.
  - Capability: implement exactly one issue.
  - Rules: edit only in issue scope; validate before review; do not commit until
    accepted review fixes are revalidated.
- Simplification Reviewer:
  - Agent type: `itw-simplification-reviewer`.
  - Capability: embedded report-only simplify-wip capability in the persona.
  - Rules: read-only; review WIP changes for safe simplification opportunities;
    propose fixes without applying them.
- Security Reviewer:
  - Agent type: `itw-security-reviewer`.
  - Capability: embedded WIP security-review capability in the persona.
  - Rules: read-only; review WIP changes for security risks; report only
    findings that should block or change the slice.
- Contract Reviewer:
  - Agent type: `itw-contract-reviewer`.
  - Capability: embedded contract-review capability in the persona.
  - Rules: read-only; flag scope creep, missed acceptance criteria, terminology
    drift, and validation gaps.

## Issue Order

{issue_lines(issues)}

## Gates

- [ ] Confirm branch policy before first implementation commit.
- [ ] Confirm `itw setup status` passes before spawning Codex subagents.
- [ ] Keep tracker current after each handoff.
- [ ] Run each issue's validation before review.
- [ ] Record simplification, security, and contract review outcomes for each issue.
- [ ] Revalidate after accepted review fixes before marking completed.
- [ ] Record commit hashes in `tracker.md`.

## Final Report

Report completed issues, commits, validation, blockers, and any human follow-up.

Root: `{root}`
"""


def tracker_template(issues: Sequence[Issue]) -> str:
    rows = [
        (
            "| Seq | Issue | Status | Worker Agent Type | Prompt | Review Agent Types | "
            "Commit | Validation |"
        ),
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for issue in issues:
        reviews = "<br>".join(
            f"{reviewer.agent_type} -> prompts/{issue.slug}-{reviewer.slug}-reviewer.md"
            for reviewer in REVIEWER_TYPES
        )
        rows.append(
            "| "
            f"{issue.number} | {issue.title} | pending | {CODEX_WORKER_AGENT_TYPE} | "
            f"prompts/{issue.slug}-worker.md | {reviews} | - | - |"
        )
    return "\n".join(
        [
            "# Tracker",
            "",
            "Execution authority for this workflow.",
            "",
            "Statuses: pending, in_progress, validating, reviewing, fixing, completed, blocked.",
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
- `{root / "terminology.md"}`
- `{root / "issues.md"}`

Issue:
{issue.number}. {issue.title}

Scope:
- Implement only this issue.
- Keep `tracker.md` current.
- Run the issue validation.
- Commit the scoped slice locally after validation and accepted reviews.
- Keep all execution artifacts local unless the human explicitly asks otherwise.

## TDD Contract

{read_package_text("references", "tdd.md").strip()}

## Issue Body

{issue.body}
"""


def reviewer_prompt(
    issue: Issue,
    root: Path,
    reviewer: ReviewerType,
) -> str:
    return f"""Read-only {reviewer.description}.

Review issue {issue.number}: {issue.title}

Persona:
{reviewer.persona}

Capability:
{reviewer.capability}

Read:
- `{root / "workflow.md"}`
- `{root / "tracker.md"}`
- `{root / "intake"}`
- `{root / "prd.md"}`
- `{root / "terminology.md"}`
- `{root / "issues.md"}`

Rules:
- Do not edit files.
- {reviewer.usage}
- Focus: {reviewer.focus}
- No findings is a valid outcome when the implementation satisfies this
  reviewer role's contract.
- If findings exist, prioritize bugs, contract drift, missed validation, scope
  creep, and issues that should block the slice commit.
- For each reported finding, include file/line references when possible.
- Return the review to the workflow owner. The workflow owner records it under
  `{root / "reviews" / f"{issue.slug}-{reviewer.slug}-review.md"}`.
"""
