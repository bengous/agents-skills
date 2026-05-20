from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence

from intent_to_workflow import __version__
from intent_to_workflow.core import (
    LANGUAGE_NAMES,
    ItwError,
    advance_workflow,
    get_workflow,
    init_workflow,
    install_codex_agents,
    set_language_workflow,
    setup_fingerprint_status,
    setup_status,
    status_workflow,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="itw",
        description="Human-gated planner that turns intent into local workflow artifacts.",
    )
    parser.add_argument("--version", action="version", version=f"itw {__version__}")

    subparsers = parser.add_subparsers(
        dest="command", metavar="{init,status,get,advance,set-language,setup}"
    )

    init_parser = subparsers.add_parser("init", help="initialize a workflow")
    init_parser.add_argument("id")

    status_parser = subparsers.add_parser("status", help="print compact human status")
    status_parser.add_argument("id")

    get_parser = subparsers.add_parser("get", help="print the current agent-facing phase prompt")
    get_parser.add_argument("id")

    advance_parser = subparsers.add_parser("advance", help="advance exactly one phase")
    advance_parser.add_argument("id")

    language_parser = subparsers.add_parser("set-language", help="set workflow language")
    language_parser.add_argument("id")
    language_parser.add_argument("language", choices=tuple(LANGUAGE_NAMES))
    language_parser.add_argument("--force", action="store_true")

    setup_parser = subparsers.add_parser("setup", help="check or install runtime setup")
    setup_subparsers = setup_parser.add_subparsers(
        dest="setup_command", metavar="{status,install}"
    )
    setup_subparsers.add_parser("status", help="verify the CLI and Codex agent types")
    setup_subparsers.add_parser("install", help="install Codex agent types into CODEX_HOME")
    setup_subparsers.add_parser("fingerprint", help=argparse.SUPPRESS)

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
            sys.stdout.write(init_workflow(parsed.id))
        elif command == "status":
            sys.stdout.write(status_workflow(parsed.id))
        elif command == "get":
            sys.stdout.write(get_workflow(parsed.id))
        elif command == "advance":
            sys.stdout.write(advance_workflow(parsed.id))
        elif command == "set-language":
            sys.stdout.write(
                set_language_workflow(
                    parsed.id,
                    language=parsed.language,
                    force=parsed.force,
                )
            )
        elif command == "setup":
            if parsed.setup_command == "status":
                sys.stdout.write(setup_status())
            elif parsed.setup_command == "install":
                sys.stdout.write(install_codex_agents())
            elif parsed.setup_command == "fingerprint":
                sys.stdout.write(setup_fingerprint_status())
            else:
                parser.error("setup requires a subcommand: status or install")
        else:
            parser.error(f"unknown command: {command}")
    except ItwError as error:
        sys.stderr.write(f"error={error}\n")
        return 1

    return 0
