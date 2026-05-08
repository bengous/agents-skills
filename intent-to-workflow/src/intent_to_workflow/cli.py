from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from intent_to_workflow import __version__
from intent_to_workflow.core import (
    ItwError,
    advance_workflow,
    get_workflow,
    init_workflow,
    set_language_workflow,
    status_workflow,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="itw",
        description="Human-gated planner that turns intent into local workflow artifacts.",
    )
    parser.add_argument("--version", action="version", version=f"itw {__version__}")

    subparsers = parser.add_subparsers(
        dest="command", metavar="{init,status,get,advance,set-language}"
    )

    init_parser = subparsers.add_parser("init", help="initialize a workflow root")
    init_parser.add_argument("root")
    init_parser.add_argument("intention", nargs=argparse.REMAINDER)

    status_parser = subparsers.add_parser("status", help="print compact human status")
    status_parser.add_argument("root")

    get_parser = subparsers.add_parser("get", help="print the current agent-facing phase prompt")
    get_parser.add_argument("root")

    advance_parser = subparsers.add_parser("advance", help="advance exactly one phase")
    advance_parser.add_argument("root")

    language_parser = subparsers.add_parser("set-language", help="set or infer workflow language")
    language_parser.add_argument("root")
    language_parser.add_argument("language", nargs="?", choices=("fr", "en", "unknown"))
    language_parser.add_argument("--from-intake", action="store_true")
    language_parser.add_argument("--force", action="store_true")

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    parsed = parser.parse_args(argv)
    command = parsed.command

    if command is None:
        parser.print_help()
        return 0

    try:
        if command == "init":
            sys.stdout.write(init_workflow(Path(parsed.root), parsed.intention))
        elif command == "status":
            sys.stdout.write(status_workflow(Path(parsed.root)))
        elif command == "get":
            sys.stdout.write(get_workflow(Path(parsed.root)))
        elif command == "advance":
            sys.stdout.write(advance_workflow(Path(parsed.root)))
        elif command == "set-language":
            sys.stdout.write(
                set_language_workflow(
                    Path(parsed.root),
                    language=parsed.language,
                    from_intake=parsed.from_intake,
                    force=parsed.force,
                )
            )
        else:
            parser.error(f"unknown command: {command}")
    except ItwError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    return 0
