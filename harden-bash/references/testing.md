# Testing

## Required Rules

- Parse Bash files with the promised Bash version and run `bash -n` as a fast syntax check.
- Run ShellCheck with an explicit dialect: `-s bash` or `-s sh`. Suppress a diagnostic only at the narrowest location with a reason.
- Run shfmt with the same dialect and verify a clean formatting diff.
- Execute behavior under every promised interpreter and utility environment. Bash parsing is not a POSIX portability test.
- Cover missing values, empty values where meaningful, whitespace, option-like data, producer failure, cleanup, signals, child statuses, and partial-output prevention according to the script's risk surface.
- For an option-like path check, pass an operand whose argument itself begins with `-` from the test working directory; `/tmp/case/-name` does not exercise option parsing.
- Test destructive examples only inside a fresh temporary directory. Require no root, host mutation, application network, or writable host mount.
- Test sourced and executed modes separately when both are supported.
- Assert exit status and side effects, not only stdout.

## Contextual Decisions

- This skill's v1 example matrix is Bash 5.3, Debian bookworm dash, and BusyBox 1.37 ash. macOS Bash 3.2 is outside the guarantee.
- Add a test framework only when plain shell assertions become harder to understand than the behavior they check.
- Add fault injection for storage, permissions, timing, or signals only where those failures affect the contract.
- Syntax and static analysis may run on every change; container behavior tests may run at the repository's chosen validation boundary.

## Anti-Patterns

- Claiming portability after `bash script.sh` succeeds.
- Treating `bash -n`, ShellCheck, or shfmt as behavioral proof.
- Running a filesystem test against a developer's real home or project tree.
- Tests that require root or live services without that boundary being the subject under test.
- Checking only the happy path while advertising cleanup or atomicity.
- Hiding test failures behind a pipeline or bare `wait`.

## Official Sources

- [GNU Bash Reference Manual: Invoking Bash](https://www.gnu.org/software/bash/manual/bash.html#Invoking-Bash)
- [POSIX Shell Command Language](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html)
- [ShellCheck Wiki](https://www.shellcheck.net/wiki/)
- [shfmt documentation](https://github.com/mvdan/sh#shfmt)
