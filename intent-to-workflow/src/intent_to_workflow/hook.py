from __future__ import annotations

import json
import os
import re
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from intent_to_workflow.core import (
    ItwError,
    get_workflow,
    git_root_for,
    init_workflow_with_metadata,
    state_path,
    validate_workflow_id,
    workflow_root_for_id,
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


def workflow_id_for_hook(cwd: Path) -> str:
    base = git_root_for(cwd) or cwd
    return validate_workflow_id(base.name)


def root_for_hook(cwd: Path, session_id: str | None = None) -> Path:
    del session_id
    return workflow_root_for_id(workflow_id_for_hook(cwd), base=cwd)


def empty_invocation_message(root: Path) -> str:
    return (
        "intent-to-workflow requires an explicit initial intention.\n"
        "No intent-to-workflow root exists for this repo.\n"
        "Ask the human to invoke `$intent-to-workflow <initial intention>`.\n"
        f"Expected root after init: `{root}`\n"
    )


def intake_edit_message(workflow_id: str, root: Path) -> str:
    return (
        f"stage=clarification id={workflow_id} root={root} next=edit {root / 'intake'}\n"
        "Intent-to-workflow scaffold created.\n"
        f"Edit `{root / 'intake'}` with the raw initial intention, then run "
        f"`itw get {workflow_id}`.\n"
        f"Do not edit `{root / '.itw-state.json'}`.\n"
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
        workflow_id = workflow_id_for_hook(cwd)
        root = root_for_hook(cwd, session_id)
        if state_path(root).exists():
            if invocation.intention != "":
                raise ItwError(
                    f"workflow already active for `{workflow_id}`; invoke "
                    "`$intent-to-workflow` to resume or start a different repo for a new intention"
                )
            output = get_workflow(workflow_id, base=cwd)
        elif invocation.intention == "":
            output = empty_invocation_message(root)
        else:
            init_workflow_with_metadata(
                workflow_id=workflow_id,
                session_id=session_id,
                cwd=str(cwd),
                model=text_from_payload(payload, "model") or os.environ.get("ITW_MODEL"),
                transcript_path=text_from_payload(payload, "transcript_path", "transcriptPath")
                or os.environ.get("ITW_TRANSCRIPT_PATH"),
                base=cwd,
            )
            output = intake_edit_message(workflow_id, root)
    except ItwError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    sys.stdout.write(output)
    return 0
