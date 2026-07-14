# Error Semantics

## Required Rules

- Define which non-zero statuses are expected domain results and which are failures. Preserve a producer's status before running another command.
- Check assignments containing command substitutions as commands: `if output=$(producer); then ... else status=$?; ... fi`.
- In Bash, enable `pipefail` only when the pipeline's contract requires failures from non-final commands. In POSIX `sh`, restructure the flow to capture each required status; do not emulate unsupported `pipefail` with brittle parsing.
- Treat `set -e` as control flow with documented exceptions, not exception handling. Audit commands in `if`, `while`, `until`, `&&`, `||`, `!`, pipelines, functions called from tested contexts, subshells, and command substitutions.
- Do not use process substitution when the producer status must decide success unless its PID is captured and waited for. A successful consumer does not prove `< <(producer)` succeeded.
- Wait for each relevant child and record its status. `wait` without operands can return success while hiding individual failures.
- Preserve the status that caused exit when cleanup explicitly controls termination. In Bash, an `EXIT` trap's final command does not by itself replace the pending status; do not report a missing save-and-exit sequence as a bug unless the handler can explicitly change that status.

## Contextual Decisions

- `set -e` can reduce repetition in a short, linear script after its ignored contexts are understood. Explicit checks remain clearer around recovery, cleanup, and domain statuses.
- `set -u` is useful only after optional and empty values have explicit defaults or guards.
- `-E` matters only for a deliberate Bash `ERR`-trap inheritance contract. An `ERR` trap is diagnostic, not a substitute for handling status.
- Bash command substitutions clear `-e` unless `inherit_errexit` or POSIX mode changes that behavior. Do not make correctness depend on an ambient shell option.
- Choose a deterministic aggregate status for concurrent work, such as the first non-zero result in launch order.

## Anti-Patterns

- Adding `set -Eeuo pipefail` mechanically to every script or sourced file.
- Hiding failures with `|| true` or checking `$?` after logging another command.
- `mapfile -t rows < <(producer)` when producer failure matters.
- Assuming an `ERR` trap runs in every function, subshell, or tested command.
- Using `! command` and then expecting `$?` to contain the original failure status.
- Calling bare `wait` and reporting the run successful.

## Official Sources

- [GNU Bash Reference Manual: The Set Builtin](https://www.gnu.org/software/bash/manual/bash.html#The-Set-Builtin)
- [GNU Bash Reference Manual: Pipelines](https://www.gnu.org/software/bash/manual/bash.html#Pipelines)
- [GNU Bash Reference Manual: Command Substitution](https://www.gnu.org/software/bash/manual/bash.html#Command-Substitution)
- [GNU Bash Reference Manual: Process Substitution](https://www.gnu.org/software/bash/manual/bash.html#Process-Substitution)
- [POSIX Shell Command Language: Exit Status and Errors](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html#tag_19_08)
- [ShellCheck SC2312: masked return values](https://www.shellcheck.net/wiki/SC2312)
