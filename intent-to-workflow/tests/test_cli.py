from __future__ import annotations

import json
import sys
from collections.abc import Sequence
from contextlib import redirect_stderr, redirect_stdout
from dataclasses import dataclass
from io import StringIO
from pathlib import Path

import pytest

from intent_to_workflow.cli import main
from intent_to_workflow.core import STAGES, ItwError, phase_prompt_for_stage, render_template
from intent_to_workflow.hook import main as hook_main
from intent_to_workflow.hook import parse_skill_invocation, root_for_hook


@dataclass(frozen=True)
class CliRun:
    exit_code: int
    stdout: str
    stderr: str


def normalize_exit_code(code: object) -> int:
    if code is None:
        return 0
    if isinstance(code, int):
        return code
    return 1


def run_cli(argv: Sequence[str]) -> CliRun:
    stdout = StringIO()
    stderr = StringIO()

    with redirect_stdout(stdout), redirect_stderr(stderr):
        try:
            exit_code = main(argv)
        except SystemExit as error:
            exit_code = normalize_exit_code(error.code)

    return CliRun(exit_code=exit_code, stdout=stdout.getvalue(), stderr=stderr.getvalue())


def run_hook(payload: object) -> CliRun:
    stdout = StringIO()
    stderr = StringIO()
    original_stdin = sys.stdin
    sys.stdin = StringIO(json.dumps(payload))
    try:
        with redirect_stdout(stdout), redirect_stderr(stderr):
            exit_code = hook_main()
    finally:
        sys.stdin = original_stdin

    return CliRun(exit_code=exit_code, stdout=stdout.getvalue(), stderr=stderr.getvalue())


def write_prd(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "# PRD",
                "",
                "## Problem Statement",
                "Need a planner.",
                "",
                "## Solution",
                "Build it.",
                "",
                "## User Stories",
                "- As a user, I want staged planning.",
                "",
                "## Implementation Decisions",
                "- Keep gates manual.",
            ]
        ),
        encoding="utf-8",
    )


def write_terminology(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "# Terminology",
                "",
                "## Actors and Roles",
                "",
                "| Actor | Definition | Responsibilities | Decision Power | Constraints |",
                "| --- | --- | --- | --- | --- |",
                "| Planner user | Workflow planner | Clarifies intent | Approves gates | "
                "Preserves intake |",
                "",
                "## Canonical Terms",
                "",
                "| Term | Definition | Aliases to Avoid |",
                "| --- | --- | --- |",
                "| Intake | Raw captured initial intention | intake.md, rewritten prompt |",
                "| Clarification | Question-driven understanding log | glossary phase |",
                "",
                "## Relationships",
                "",
                "- A Planner user approves gates before the workflow advances.",
                "- Clarification derives understanding from Intake without rewriting it.",
                "",
                "## Flagged Ambiguities",
                "",
                "- None identified",
            ]
        ),
        encoding="utf-8",
    )


def write_issues(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "# Issues",
                "",
                "### 1. Build state machine",
                "",
                "Type: AFK",
                "",
                "Depends on: none",
                "",
                "Goal:",
                "Implement phase transitions.",
                "",
                "Acceptance:",
                "- [ ] Advances one phase.",
                "",
                "TDD:",
                "- First behavior test: init creates state.",
                "",
                "Validation:",
                "- uv run pytest",
                "",
                "Agent:",
                "- generalist worker",
                "",
                "Review Gates:",
                "- read-only review after implementation",
            ]
        ),
        encoding="utf-8",
    )


def test_help_lists_supported_commands() -> None:
    result = run_cli(["--help"])

    assert result.exit_code == 0
    assert "usage: itw" in result.stdout
    assert "init" in result.stdout
    assert "status" in result.stdout
    assert "get" in result.stdout
    assert "advance" in result.stdout


def test_init_requires_explicit_intention_and_does_not_create_root(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"

    result = run_cli(["init", str(root)])

    assert result.exit_code == 1
    assert "initial intention required" in result.stderr
    assert not root.exists()


def test_init_refuses_non_empty_root_before_writing_intake(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    root.mkdir(parents=True)
    (root / "intake").write_text("preexisting text\n", encoding="utf-8")

    result = run_cli(["init", str(root), "new intention"])

    assert result.exit_code == 1
    assert "root is not empty" in result.stderr
    assert (root / "intake").read_text(encoding="utf-8") == "preexisting text\n"


def test_init_captures_raw_intake_and_starts_at_clarification(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    result = run_cli(["init", str(root), "do", "X", "because", "Y"])

    assert result.exit_code == 0
    assert "stage=clarification" in result.stdout
    assert sorted(child.name for child in root.iterdir()) == [
        ".itw-state.json",
        "clarification.md",
        "intake",
        "terminology.md",
    ]
    assert (root / "intake").read_text(encoding="utf-8") == "do X because Y\n"
    assert not any(child.name == "intake" + ".md" for child in root.iterdir())
    assert "## Actors and Roles" in (root / "terminology.md").read_text(encoding="utf-8")

    state = json.loads((root / ".itw-state.json").read_text(encoding="utf-8"))
    assert state["stage"] == "clarification"
    assert state["root"] == str(root)
    assert state["language"] == "en"
    assert "# Phase Prompt" in result.stdout


def test_set_language_updates_get_prompt(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["set-language", str(root), "fr"])

    assert result.exit_code == 0
    assert "language=fr name=French" in result.stdout
    assert "Respond in French." in run_cli(["get", str(root)]).stdout

    state = json.loads((root / ".itw-state.json").read_text(encoding="utf-8"))
    assert state["language"] == "fr"


def test_set_language_rejects_unsupported_code(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["set-language", str(root), "xx"])

    assert result.exit_code == 2
    assert "invalid choice" in result.stderr


def test_set_language_refuses_overwrite_without_force(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["set-language", str(root), "fr"]).exit_code == 0

    blocked = run_cli(["set-language", str(root), "es"])
    forced = run_cli(["set-language", str(root), "es", "--force"])

    assert blocked.exit_code == 1
    assert "language already set: fr" in blocked.stderr
    assert forced.exit_code == 0
    assert "Respond in Spanish." in run_cli(["get", str(root)]).stdout


def test_set_language_from_intake_flag_is_not_supported(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["set-language", str(root), "--from-intake"])

    assert result.exit_code == 2
    assert "the following arguments are required: language" in result.stderr


def test_invalid_state_language_fails_closed(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    state_path = root / ".itw-state.json"
    state = json.loads(state_path.read_text(encoding="utf-8"))
    state["language"] = "xx"
    state_path.write_text(json.dumps(state), encoding="utf-8")

    result = run_cli(["status", str(root)])

    assert result.exit_code == 1
    assert "invalid state field: language" in result.stderr


def test_status_is_compact_human_output(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["status", str(root)])

    assert result.exit_code == 0
    assert result.stdout.startswith("stage=clarification")
    assert "next=itw advance" in result.stdout
    assert "# Phase Prompt" not in result.stdout
    assert not result.stdout.lstrip().startswith("{")


def test_get_is_read_only_and_agent_facing(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    before = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*")
        if path.is_file()
    }

    result = run_cli(["get", str(root)])

    after = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*")
        if path.is_file()
    }
    assert result.exit_code == 0
    assert before == after
    assert "# Phase Prompt" in result.stdout
    assert "## Blockers" in result.stdout
    assert "clarification.md placeholder TODO" in result.stdout


def test_advance_blocks_on_untouched_clarification(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "clarification.md placeholder TODO" in result.stderr
    assert "stage=clarification" in run_cli(["status", str(root)]).stdout


def test_advance_does_not_require_companion_skills(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 0
    assert "stage=prd" in result.stdout
    assert (root / "prd.md").exists()


def test_clarification_blocks_if_terminology_is_missing(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    (root / "terminology.md").unlink()

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "terminology.md" in result.stderr
    assert "stage=clarification" in run_cli(["status", str(root)]).stdout


def test_prd_blocks_on_incomplete_terminology(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "terminology.md placeholder TODO" in result.stderr
    assert "stage=prd" in run_cli(["status", str(root)]).stdout


def test_prd_review_rechecks_terminology_before_issues(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "terminology.md").write_text("# Terminology\n\nTODO\n", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "terminology.md placeholder TODO" in result.stderr
    assert "stage=prd_review" in run_cli(["status", str(root)]).stdout


def test_review_gates_create_expected_artifacts(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")

    assert "stage=prd_review" in run_cli(["advance", str(root)]).stdout
    assert not (root / "issues.md").exists()

    assert "stage=issues" in run_cli(["advance", str(root)]).stdout
    assert (root / "issues.md").exists()


def test_degraded_prd_review_blocks_issues(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "prd.md").write_text("# PRD\n\nTODO\n", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "prd.md placeholder TODO" in result.stderr
    assert "stage=prd_review" in run_cli(["status", str(root)]).stdout


def test_issue_validation_applies_to_each_issue(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "issues.md").write_text(
        "\n".join(
            [
                "# Issues",
                "",
                "### 1. Missing fields",
                "",
                "No required fields here.",
                "",
                "### 2. Complete issue",
                "",
                "Type: AFK",
                "",
                "Depends on: none",
                "",
                "Goal:",
                "Do it.",
                "",
                "Acceptance:",
                "- [ ] Done.",
                "",
                "TDD:",
                "- First behavior test: done.",
                "",
                "Validation:",
                "- uv run pytest",
                "",
                "Agent:",
                "- worker",
            ]
        ),
        encoding="utf-8",
    )

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "issues.md issue 1 Type:" in result.stderr
    assert "stage=issues" in run_cli(["status", str(root)]).stdout


def test_degraded_issues_review_blocks_workflow(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "issues.md").write_text("# Issues\n\n### 1. TODO\n\nTODO\n", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "issues.md issue 1 title" in result.stderr
    assert "stage=issues_review" in run_cli(["status", str(root)]).stdout


def test_workflow_package_uses_self_contained_worker_prompts(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    worker_prompt = (root / "prompts" / "issue-01-worker.md").read_text(encoding="utf-8")
    assert result.exit_code == 0
    assert "stage=workflow" in result.stdout
    assert (root / "workflow.md").exists()
    assert (root / "tracker.md").exists()
    assert "## TDD Contract" in worker_prompt
    assert "$" + "tdd" not in worker_prompt


def test_missing_prompt_reference_blocks_workflow_review(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "prompts" / "issue-01-worker.md").unlink()

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "prompts/issue-01-worker.md" in result.stderr
    assert "stage=workflow" in run_cli(["status", str(root)]).stdout


def test_workflow_ready_prints_packaged_phase_prompt(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    write_terminology(root / "terminology.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 0
    assert "stage=workflow_ready" in result.stdout
    assert "# Phase Prompt" in result.stdout
    assert "Stage: `workflow_ready`" in result.stdout
    assert "workflow.md" in result.stdout
    assert "tracker.md" in result.stdout
    assert "Execute the following workflow" not in result.stdout

    before = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*")
        if path.is_file()
    }
    status = run_cli(["status", str(root)])
    get = run_cli(["get", str(root)])
    advance = run_cli(["advance", str(root)])
    after = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*")
        if path.is_file()
    }

    assert status.exit_code == 0
    assert "stage=workflow_ready" in status.stdout
    assert "next=-" in status.stdout
    assert get.exit_code == 0
    assert "Stage: `workflow_ready`" in get.stdout
    assert advance.exit_code == 1
    assert "already at workflow_ready" in advance.stderr
    assert before == after


def test_hook_detects_only_leading_current_skill_token() -> None:
    invocation = parse_skill_invocation("$intent-to-workflow do X because --flag maybe")

    assert invocation is not None
    assert invocation.intention == "do X because --flag maybe"
    assert parse_skill_invocation("please use $intent-to-workflow later") is None
    assert parse_skill_invocation("$intent_to_workflow do X") is None


def test_empty_hook_invocation_creates_no_root_unless_resuming(tmp_path: Path) -> None:
    payload = {"prompt": "$intent-to-workflow", "cwd": str(tmp_path), "session_id": "abc-123"}
    root = root_for_hook(tmp_path, "abc-123")

    empty_result = run_hook(payload)

    assert empty_result.exit_code == 0
    assert "initial intention" in empty_result.stdout
    assert not root.exists()

    init_result = run_hook(
        {
            "prompt": "$intent-to-workflow build a local handoff planner",
            "cwd": str(tmp_path),
            "session_id": "abc-123",
        }
    )
    resume_result = run_hook(payload)

    assert init_result.exit_code == 0
    assert root.exists()
    assert resume_result.exit_code == 0
    assert "# Phase Prompt" in resume_result.stdout
    assert "stage=clarification" in resume_result.stdout


def test_hook_requires_stable_session_identity(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    for name in ("ITW_SESSION_ID", "CODEX_SESSION_ID", "ITW_SESSION_SHORT", "CODEX_SESSION_SHORT"):
        monkeypatch.delenv(name, raising=False)

    result = run_hook({"prompt": "$intent-to-workflow build it", "cwd": str(tmp_path)})

    assert result.exit_code == 1
    assert "missing session id" in result.stderr
    assert not (tmp_path / "itw").exists()


def test_hook_refuses_new_intention_when_session_root_exists(tmp_path: Path) -> None:
    payload = {
        "prompt": "$intent-to-workflow build a local handoff planner",
        "cwd": str(tmp_path),
        "session_id": "abc-123",
    }
    root = root_for_hook(tmp_path, "abc-123")
    assert run_hook(payload).exit_code == 0

    result = run_hook(
        {
            "prompt": "$intent-to-workflow build a different workflow",
            "cwd": str(tmp_path),
            "session_id": "abc-123",
        }
    )

    assert result.exit_code == 1
    assert "workflow already active for this session" in result.stderr
    assert (root / "intake").read_text(encoding="utf-8") == "build a local handoff planner\n"


def test_hook_preserves_structured_intake(tmp_path: Path) -> None:
    prompt = """$intent-to-workflow
# Build the thing

Keep this code sample:

```ts
const value = 1;
```
"""
    root = root_for_hook(tmp_path, "abc-123")

    result = run_hook({"prompt": prompt, "cwd": str(tmp_path), "session_id": "abc-123"})

    assert result.exit_code == 0
    assert (root / "intake").read_text(encoding="utf-8") == (
        "# Build the thing\n"
        "\n"
        "Keep this code sample:\n"
        "\n"
        "```ts\n"
        "const value = 1;\n"
        "```\n"
    )


def test_all_packaged_phase_templates_render(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    rendered = {stage: phase_prompt_for_stage(stage, root, ()) for stage in STAGES}

    assert all("# Phase Prompt" in prompt for prompt in rendered.values())
    assert all("{{" not in prompt and "}}" not in prompt for prompt in rendered.values())
    assert "grill.md" in rendered["clarification"]
    assert "prd.md" in rendered["prd"]
    assert "prd_review.md" in rendered["prd_review"]
    assert "issues.md" in rendered["issues"]
    assert "issues_review.md" in rendered["issues_review"]
    assert "artifacts.md" in rendered["workflow"]
    assert "grill.md" not in rendered["prd"]
    assert "Do not continue PRD generation" in rendered["prd_review"]
    assert "Write the PRD using the template" not in rendered["prd_review"]
    assert "Do not create `workflow.md`" in rendered["issues_review"]
    assert "Write the local issues" not in rendered["issues_review"]
    assert "prior context should be included" in rendered["clarification"]
    assert "terminology.md" in rendered["clarification"]
    assert "actor, role" in rendered["clarification"]
    assert "canonical actor and term names" in rendered["prd"]
    assert "against `terminology.md`" in rendered["prd_review"]
    assert "from `prd.md` and `terminology.md`" in rendered["issues"]

    rendered_text = "\n".join(rendered.values())
    assert "UBIQUITOUS_LANGUAGE.md" not in rendered_text
    assert "$" + "ubiquitous-language" not in rendered_text
    assert "GitHub" not in rendered_text
    assert "publish" not in rendered_text
    assert "remote" not in rendered_text


def test_strict_template_renderer_fails_closed() -> None:
    with pytest.raises(ItwError, match="malformed"):
        render_template("{{root}}", {})

    with pytest.raises(ItwError, match="unknown"):
        render_template("{{BOGUS}}", {})

    with pytest.raises(ItwError, match="missing"):
        render_template("{{ROOT}}", {})

    assert render_template("{{ROOT}}", {"ROOT": "{{STAGE}}"}) == "{{STAGE}}"
