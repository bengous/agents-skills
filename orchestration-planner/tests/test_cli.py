from __future__ import annotations

import json
from collections.abc import Sequence
from contextlib import redirect_stderr, redirect_stdout
from dataclasses import dataclass
from io import StringIO
from pathlib import Path

import pytest

from orchestration_planner.cli import main
from orchestration_planner.hook import parse_skill_invocation

REQUIRED_TEST_SKILLS = ("grill-me", "to-prd", "to-issues", "tdd")


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


def configure_companion_skills(root: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    skill_root = root / "skills"
    for skill in REQUIRED_TEST_SKILLS:
        skill_dir = skill_root / skill
        skill_dir.mkdir(parents=True, exist_ok=True)
        (skill_dir / "SKILL.md").write_text(f"---\nname: {skill}\n---\n", encoding="utf-8")
    monkeypatch.setenv("ORCH_SKILL_ROOTS", str(skill_root))


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
    assert "usage: orch" in result.stdout
    assert "init" in result.stdout
    assert "status" in result.stdout
    assert "advance" in result.stdout


def test_init_with_raw_intention_creates_only_state_and_intake(tmp_path: Path) -> None:
    root = tmp_path / "orc" / "demo"
    result = run_cli(["init", str(root), "do", "X", "because", "Y"])

    assert result.exit_code == 0
    assert "stage=intake" in result.stdout
    assert sorted(child.name for child in root.iterdir()) == [
        ".orchestration-state.json",
        "intake.md",
    ]
    assert "do X because Y" in (root / "intake.md").read_text(encoding="utf-8")

    state = json.loads((root / ".orchestration-state.json").read_text(encoding="utf-8"))
    assert state["stage"] == "intake"
    assert state["root"] == str(root)


def test_intake_todo_blocks_advance(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "intake.md initial intention" in result.stderr
    assert "stage=intake" in run_cli(["status", str(root)]).stdout


def test_raw_todo_intention_blocks_advance(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "TODO"]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "intake.md initial intention" in result.stderr


def test_status_is_compact_text_not_json(tmp_path: Path) -> None:
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root)]).exit_code == 0

    result = run_cli(["status", str(root)])

    assert result.exit_code == 0
    assert result.stdout.startswith("stage=intake")
    assert "next=orch advance" in result.stdout
    assert not result.stdout.lstrip().startswith("{")


def test_advance_creates_next_skeleton_one_phase_at_a_time(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 0
    assert "stage=clarification" in result.stdout
    assert (root / "clarification.md").exists()
    assert not (root / "prd.md").exists()


def test_missing_companion_skill_blocks_advance(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("ORCH_SKILL_ROOTS", str(tmp_path / "empty-skills"))
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "missing required companion skill(s): $grill-me" in result.stderr
    assert "stage=intake" in run_cli(["status", str(root)]).stdout


def test_failed_validation_keeps_stage_unchanged(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "missing: clarification.md non-empty" in result.stderr
    assert "stage=clarification" in run_cli(["status", str(root)]).stdout


def test_untouched_clarification_skeleton_blocks_prd(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "clarification.md placeholder TODO" in result.stderr
    assert "stage=clarification" in run_cli(["status", str(root)]).stdout


def test_prd_and_issues_review_gates_create_expected_artifacts(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")

    assert "stage=prd_review" in run_cli(["advance", str(root)]).stdout
    assert not (root / "issues.md").exists()

    assert "stage=issues" in run_cli(["advance", str(root)]).stdout
    assert (root / "issues.md").exists()


def test_degraded_prd_review_blocks_issues(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "prd.md").write_text("# PRD\n\nTODO\n", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "prd.md placeholder TODO" in result.stderr
    assert "stage=prd_review" in run_cli(["status", str(root)]).stdout


def test_issue_validation_applies_to_each_issue(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
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


def test_workflow_package_generates_tracker_and_prompts(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 0
    assert "stage=workflow" in result.stdout
    assert (root / "workflow.md").exists()
    assert (root / "tracker.md").exists()
    assert (root / "prompts" / "issue-01-worker.md").exists()
    assert (root / "prompts" / "issue-01-reviewer.md").exists()
    assert "$tdd" in (root / "prompts" / "issue-01-worker.md").read_text(encoding="utf-8")
    assert "pending" in (root / "tracker.md").read_text(encoding="utf-8")


def test_degraded_issues_review_blocks_workflow(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_issues(root / "issues.md")
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "issues.md").write_text("# Issues\n\n### 1. TODO\n\nTODO\n", encoding="utf-8")

    result = run_cli(["advance", str(root)])

    assert result.exit_code == 1
    assert "issues.md issue 1 title" in result.stderr
    assert "stage=issues_review" in run_cli(["status", str(root)]).stdout


def test_workflow_ready_prints_concise_handoff(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
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


def test_missing_prompt_reference_blocks_workflow_review(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    configure_companion_skills(tmp_path, monkeypatch)
    root = tmp_path / "orc" / "demo"
    assert run_cli(["init", str(root), "plan something"]).exit_code == 0
    assert run_cli(["advance", str(root)]).exit_code == 0
    (root / "clarification.md").write_text("Q001 done", encoding="utf-8")
    assert run_cli(["advance", str(root)]).exit_code == 0
    write_prd(root / "prd.md")
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


def test_hook_parser_detects_skill_and_preserves_raw_intention() -> None:
    invocation = parse_skill_invocation("$orchestration-planner do X because --flag maybe")

    assert invocation is not None
    assert invocation.intention == "do X because --flag maybe"


def test_hook_parser_ignores_late_mentions() -> None:
    assert parse_skill_invocation("please use $orchestration-planner later") is None
