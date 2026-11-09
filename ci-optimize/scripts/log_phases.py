# /// script
# requires-python = ">=3.11"
# ///
r"""Phase durations of a CI job log whose lines start with a timestamp.

    python3 log_phases.py job.log --mark install='npm ci' \
        --mark lint='npm run lint' --mark test='npm test' --end 'Post job cleanup'

Needs Python 3.11 or later. Each --mark NAME=REGEX starts a phase at the first
line it matches after the previous mark; a phase ends where the next one
starts, at the --end line, or at the log's last timestamped line. A leading
byte-order mark and terminal escape codes are stripped first. The timestamp
is the line's first field, ISO 8601 (GitHub Actions writes
2026-01-01T12:00:00.0000000Z).
"""

import argparse
import re
import sys
from datetime import UTC, datetime
from pathlib import Path

ESCAPE = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
STAMP = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d+))?(Z|[+-]\d{2}:?\d{2})?(?:\s|$)")


def stamp(line: str) -> datetime | None:
    match = STAMP.match(line)
    if match is None:
        return None
    base, fraction, zone = match.groups()
    micro = int((fraction or "0")[:6].ljust(6, "0"))
    moment = datetime.fromisoformat(base).replace(microsecond=micro)
    if zone in (None, "Z"):
        return moment.replace(tzinfo=UTC)
    return datetime.fromisoformat(f"{base}{zone}").replace(microsecond=micro)


def mark(text: str) -> tuple[str, re.Pattern[str]]:
    name, separator, pattern = text.partition("=")
    if not separator or not name or not pattern:
        raise argparse.ArgumentTypeError(f"expected NAME=REGEX, got {text!r}")
    return name, re.compile(pattern)


def phases(
    lines: list[str], marks: list[tuple[str, re.Pattern[str]]], end: re.Pattern[str] | None
) -> list[tuple[str, float]]:
    stamped = [(moment, line) for line in lines if (moment := stamp(line)) is not None]
    if not stamped:
        raise SystemExit("no timestamped line: the log has no leading ISO 8601 times")

    starts: list[tuple[str, datetime, int]] = []
    position = 0
    for name, pattern in marks:
        found = next((index for index in range(position, len(stamped)) if pattern.search(stamped[index][1])), None)
        if found is None:
            after = f"after mark {starts[-1][0]!r}" if starts else "in the log"
            raise SystemExit(f"mark {name!r} matches no timestamped line {after}")
        starts.append((name, stamped[found][0], found))
        position = found + 1

    finish = stamped[-1][0]
    if end is not None:
        found = next((index for index in range(position, len(stamped)) if end.search(stamped[index][1])), None)
        if found is None:
            raise SystemExit("the --end pattern matches no line after the last mark")
        finish = stamped[found][0]

    bounds = [moment for _, moment, _ in starts[1:]] + [finish]
    return [(name, (stop - start).total_seconds()) for (name, start, _), stop in zip(starts, bounds, strict=True)]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("log", type=Path)
    parser.add_argument("--mark", type=mark, action="append", required=True, metavar="NAME=REGEX")
    parser.add_argument("--end", type=re.compile, metavar="REGEX")
    args = parser.parse_args()

    text = args.log.read_text(encoding="utf-8-sig", errors="replace")
    lines = [ESCAPE.sub("", line) for line in text.splitlines()]
    result = phases(lines, args.mark, args.end)
    width = max(len("total"), *(len(name) for name, _ in result))
    for name, seconds in result:
        print(f"{name:<{width}}  {seconds:8.1f} s")
    print(f"{'total':<{width}}  {sum(seconds for _, seconds in result):8.1f} s")


if __name__ == "__main__":
    sys.exit(main())
