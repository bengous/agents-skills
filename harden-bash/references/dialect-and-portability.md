# Dialect And Portability

## Required Rules

- Identify the parser from the caller, not only the shebang. A shebang applies to direct execution; `sh script`, `bash script`, and sourcing choose different contracts.
- Use `#!/bin/sh` only for POSIX shell language. Exclude arrays, `[[ ... ]]`, process substitution, `pipefail`, `local`, `function`, here-strings, `${BASH_SOURCE[...]}`, and `&>`.
- Use a Bash shebang when Bash features materially simplify a Bash-only contract. Declare and test the minimum Bash version needed.
- Treat shell syntax and external utilities as separate portability surfaces. `mktemp`, `realpath`, `flock`, GNU option extensions, and utility-specific `--` support are not guaranteed by POSIX shell conformance.
- When a POSIX rewrite relies on non-POSIX utility behavior, state each dependency or extension in the delivered answer. Passing dash and ash tests does not make flags such as `grep -H` or utility-specific `--` portable.
- Keep sourced files from changing the caller's options, traps, working directory, `umask`, or positional parameters unless that mutation is the documented API.
- Run ShellCheck and shfmt with the selected dialect rather than whichever shell happens to be installed locally.

## Contextual Decisions

- Prefer a fixed interpreter path when deployment owns that path. Prefer `/usr/bin/env bash` when environment lookup is an intentional dependency.
- A Bash script may target Bash 5.3 without supporting macOS Bash 3.2 when the deployment matrix says so.
- For strict POSIX operands beginning with `-`, avoid utility option parsing through redirection or normalize relative paths to start with `./`; do not assume every utility accepts `--`.
- Test the actual shell and utility implementations. Bash POSIX mode is useful but does not replace dash and ash tests.

## Anti-Patterns

- `#!/bin/sh` above code tested only with Bash.
- Adding Bashisms to avoid a few explicit status checks in a portability-required script.
- Assuming a successful parse proves external flags exist.
- Treating GNU and BSD utilities as interchangeable.
- Sourcing a script that calls `exit`, installs global traps, or enables strict options for its caller.

## Official Sources

- [GNU Bash Reference Manual: Shell Scripts](https://www.gnu.org/software/bash/manual/bash.html#Shell-Scripts)
- [GNU Bash Reference Manual: Bash POSIX Mode](https://www.gnu.org/software/bash/manual/bash.html#Bash-POSIX-Mode)
- [POSIX Shell Command Language](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html)
- [ShellCheck SC3040: `pipefail` is undefined in POSIX sh](https://www.shellcheck.net/wiki/SC3040)
