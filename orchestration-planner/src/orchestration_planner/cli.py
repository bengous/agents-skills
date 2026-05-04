from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from orchestration_planner import __version__
from orchestration_planner.core import OrchError, advance_workflow, init_workflow, status_workflow


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="orch",
        description="Human-gated planner for PRD-to-issues-to-workflow orchestration.",
    )
    parser.add_argument("--version", action="version", version=f"orch {__version__}")

    subparsers = parser.add_subparsers(dest="command", metavar="{init,status,advance}")

    init_parser = subparsers.add_parser("init", help="initialize a planning root")
    init_parser.add_argument("root")
    init_parser.add_argument("intention", nargs=argparse.REMAINDER)

    status_parser = subparsers.add_parser("status", help="print compact planning status")
    status_parser.add_argument("root")

    advance_parser = subparsers.add_parser("advance", help="advance exactly one phase")
    advance_parser.add_argument("root")

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
        elif command == "advance":
            sys.stdout.write(advance_workflow(Path(parsed.root)))
        else:
            parser.error(f"unknown command: {command}")
    except OrchError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    return 0
