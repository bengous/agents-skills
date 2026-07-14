# Input And Command Construction

## Required Rules

- Validate argument count and option values before reading positional parameters. Distinguish an absent value from an allowed empty value.
- Quote expansions unless splitting or glob expansion is the explicit operation. Use `"$@"` to preserve positional arguments.
- In Bash, build variable commands with arrays: one element per argument. In POSIX `sh`, use direct argument lists or deliberately rebuild positional parameters with `set --`.
- Keep untrusted text as data. Never pass it through `eval`, `sh -c`, or an unquoted command variable.
- Use constant `printf` formats, `IFS= read -r` for line input, and NUL-delimited protocols when filenames may contain newlines and every tool in the path supports them.
- Prevent option injection. Use a supported `--`, an option such as `grep -e`, redirection instead of a filename operand, or a normalized `./relative-path`.
- Validate semantic constraints at the boundary: allowed values, path type, ownership, range, encoding, and whether empty input is meaningful.
- Remember that shell variables cannot contain NUL bytes. Do not claim arbitrary-byte safety for data stored in variables.

## Contextual Decisions

- Parse long options manually only when the CLI contract needs them; otherwise use the shell's native `getopts` contract.
- Reject filenames containing newlines when downstream line-oriented tools cannot preserve them; state that limitation instead of silently misparsing.
- Use `find ... -exec ... {} +` or a NUL-safe consumer rather than parsing `ls` output.
- Use `sh -c` only when a shell program is the explicit interface. Pass data as positional parameters, never interpolate it into the program text.

## Anti-Patterns

- `command="$tool $user_arg"; $command` or `eval "$command"`.
- Unquoted `$*`, `$@`, command substitution, or path expansion.
- `for file in $(find ...)` and parsing `ls`.
- `printf "$untrusted"`.
- Assuming quotes alone prevent option injection.
- Accepting `--option` without first checking that a value remains.

## Official Sources

- [GNU Bash Reference Manual: Quoting](https://www.gnu.org/software/bash/manual/bash.html#Quoting)
- [GNU Bash Reference Manual: Arrays](https://www.gnu.org/software/bash/manual/bash.html#Arrays)
- [POSIX Shell Command Language: Word Expansions](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html#tag_19_06)
- [ShellCheck SC2086: quote to prevent splitting and globbing](https://www.shellcheck.net/wiki/SC2086)
- [ShellCheck SC2059: keep `printf` formats constant](https://www.shellcheck.net/wiki/SC2059)
