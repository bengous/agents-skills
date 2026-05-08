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
    ]
    assert (root / "intake").read_text(encoding="utf-8") == "do X because Y\n"
    assert not any(child.name == "intake" + ".md" for child in root.iterdir())

    state = json.loads((root / ".itw-state.json").read_text(encoding="utf-8"))
    assert state["stage"] == "clarification"
    assert state["root"] == str(root)
    assert state["language"] == "en"
    assert "# Phase Prompt" in result.stdout


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


def test_review_gates_create_expected_artifacts(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")

    assert "stage=prd_review" in run_cli(["advance", str(root)]).stdout
    assert not (root / "issues.md").exists()

    assert "stage=issues" in run_cli(["advance", str(root)]).stdout
    assert (root / "issues.md").exists()


def test_workflow_package_uses_self_contained_worker_prompts(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
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


def test_workflow_ready_prints_concise_handoff(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 0
    assert "stage=workflow_ready" in result.stdout
    assert "Execute the following workflow" in result.stdout
    assert "workflow.md" in result.stdout
    assert "tracker.md" in result.stdout


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


def test_all_packaged_phase_templates_render(tmp_path: Path) -> None:
    root = tmp_path / "itw" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    rendered = {stage: phase_prompt_for_stage(stage, root, ()) for stage in STAGES}

    assert all("# Phase Prompt" in prompt for prompt in rendered.values())
    assert all("{{" not in prompt and "}}" not in prompt for prompt in rendered.values())
    assert "grill.md" in rendered["clarification"]
    assert "prd.md" in rendered["prd"]
    assert "issues.md" in rendered["issues"]
    assert "artifacts.md" in rendered["workflow"]
    assert "grill.md" not in rendered["prd"]


def test_strict_template_renderer_fails_closed() -> None:
    with pytest.raises(ItwError, match="malformed"):
        render_template("{{root}}", {})

    with pytest.raises(ItwError, match="unknown"):
        render_template("{{BOGUS}}", {})

    with pytest.raises(ItwError, match="missing"):
        render_template("{{ROOT}}", {})

    assert render_template("{{ROOT}}", {"ROOT": "{{STAGE}}"}) == "{{STAGE}}"
