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

from intent_to_workflow.core import (
    ItwError,
    get_workflow,
    init_workflow_with_metadata,
    session_short_from,
    state_path,
)

INVOCATION_RE = re.compile(
    r"^\s*\$intent-to-workflow\b\s*(?P<intention>.*)\Z",
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
    # TODO: non-Codex harnesses should pass ITW_SESSION_ID or ITW_SESSION_SHORT explicitly.
    session_short = session_short_from(session_id)
    if session_short is None:
        raise ItwError(
            "missing session id; run in a Codex session with hook metadata or use "
            "manual `itw init <root> <initial intention>`"
        )
    return cwd / "itw" / f"{today}-{session_short}"


def empty_invocation_message(root: Path) -> str:
    return (
        "intent-to-workflow requires an explicit initial intention.\n"
        "No intent-to-workflow root exists for this session.\n"
        "Ask the human to invoke `$intent-to-workflow <initial intention>`.\n"
        f"Expected root after init: `{root}`\n"
    )


def main() -> int:
    if any(argument in ("-h", "--help") for argument in sys.argv[1:]):
        sys.stdout.write(
            "usage: itw-codex-user-prompt-submit < hook-payload.json\n"
            "Detects leading $intent-to-workflow prompts and runs itw init/get.\n"
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
        or os.environ.get("ITW_SESSION_ID")
        or os.environ.get("CODEX_SESSION_ID")
    )
    cwd_text = text_from_payload(payload, "cwd") or os.environ.get("ITW_CWD") or os.getcwd()
    cwd = Path(cwd_text)
    try:
        root = root_for_hook(cwd, session_id)
        if state_path(root).exists():
            if invocation.intention != "":
                raise ItwError(
                    "workflow already active for this session; invoke `$intent-to-workflow` "
                    "to resume or start a new session for a new intention"
                )
            output = get_workflow(root)
        elif invocation.intention == "":
            output = empty_invocation_message(root)
        else:
            output = init_workflow_with_metadata(
                root=root,
                intention=invocation.intention,
                session_id=session_id,
                cwd=str(cwd),
                model=text_from_payload(payload, "model") or os.environ.get("ITW_MODEL"),
                transcript_path=text_from_payload(payload, "transcript_path", "transcriptPath")
                or os.environ.get("ITW_TRANSCRIPT_PATH"),
            )
    except ItwError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    sys.stdout.write(output)
    return 0
