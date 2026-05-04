from __future__ import annotations

import json
import os
import re
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import cast

from orchestration_planner.core import OrchError, init_workflow_with_metadata, session_short_from

INVOCATION_RE = re.compile(
    r"^\s*\$orchestration-planner\b\s*(?P<intention>.*)\Z",
    re.IGNORECASE | re.DOTALL,
)


@dataclass(frozen=True)
class SkillInvocation:
    intention: str


def parse_skill_invocation(prompt: str) -> SkillInvocation | None:
    match = INVOCATION_RE.match(prompt)
    if match is None:
        return None
    return SkillInvocation(intention=match.group("intention").strip())


def prompt_from_payload(payload: object) -> str | None:
    if isinstance(payload, str):
        return payload
    if not isinstance(payload, dict):
        return None
    mapping = cast(Mapping[str, object], payload)

    for key in ("prompt", "user_prompt", "input"):
        value = mapping.get(key)
        if isinstance(value, str):
            return value

    return None


def text_from_payload(payload: object, *keys: str) -> str | None:
    if not isinstance(payload, dict):
        return None

    mapping = cast(Mapping[str, object], payload)
    nested = mapping.get("metadata")
    empty_mapping: Mapping[str, object] = {}
    nested_mapping = (
        cast(Mapping[str, object], nested) if isinstance(nested, dict) else empty_mapping
    )

    for key in keys:
        value = mapping.get(key)
        if isinstance(value, str) and value:
            return value
        nested_value = nested_mapping.get(key)
        if isinstance(nested_value, str) and nested_value:
            return nested_value

    return None


def root_for_hook(cwd: Path, session_id: str | None) -> Path:
    today = datetime.now().date().isoformat()
    session_short = session_short_from(session_id) or "unknown"
    return cwd / "orc" / f"{today}-{session_short}"


def main() -> int:
    if any(argument in ("-h", "--help") for argument in sys.argv[1:]):
        sys.stdout.write(
            "usage: orch-codex-user-prompt-submit < hook-payload.json\n"
            "Detects leading $orchestration-planner prompts and runs orch init.\n"
        )
        return 0

    raw = sys.stdin.read()
    if raw.strip() == "":
        return 0

    try:
        payload = cast(object, json.loads(raw))
    except json.JSONDecodeError:
        payload = raw

    prompt = prompt_from_payload(payload)
    if prompt is None:
        return 0

    invocation = parse_skill_invocation(prompt)
    if invocation is None:
        return 0

    session_id = (
        text_from_payload(payload, "session_id", "sessionId", "conversation_id")
        or os.environ.get("ORCH_SESSION_ID")
        or os.environ.get("CODEX_SESSION_ID")
    )
    cwd_text = text_from_payload(payload, "cwd") or os.environ.get("ORCH_CWD") or os.getcwd()
    cwd = Path(cwd_text)
    root = root_for_hook(cwd, session_id)
    try:
        output = init_workflow_with_metadata(
            root=root,
            intention=invocation.intention,
            entry_point="UserPromptSubmit",
            session_id=session_id,
            cwd=str(cwd),
            model=text_from_payload(payload, "model") or os.environ.get("ORCH_MODEL"),
            transcript_path=text_from_payload(payload, "transcript_path", "transcriptPath")
            or os.environ.get("ORCH_TRANSCRIPT_PATH"),
        )
    except OrchError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    sys.stdout.write(output)
    return 0
