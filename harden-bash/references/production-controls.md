# Production Controls

## Required Rules

- Add logging only when an operator contract defines audience, destination, severity, format, and redaction. Send diagnostics to stderr and keep machine output stable.
- Add dry-run only when every mutation, including redirections, builtins, remote calls, and implicit side effects, passes through a faithfully simulable path.
- Retry only a classified transient failure for an operation safe to replay. Bound attempts, preserve the final failure, and stop retrying permanent errors.
- Make a script idempotent only when rerun is part of its contract. Define the desired state and distinguish already-correct state from drift or partial state.
- Lock only around shared mutable state. Match lock scope to state scope and define acquisition failure, stale ownership, and cleanup.
- Keep secrets out of argv, logs, traces, world-readable environment or temporary files. Prefer protected stdin or file descriptors and disable tracing before secret handling.
- Require the least privilege possible. Do not embed broad `sudo` escalation into otherwise unprivileged logic.

## Contextual Decisions

| Control | Add when | Minimum contract |
| --- | --- | --- |
| Logging | Humans or collectors operate the script | Stable destination, levels, redaction |
| Dry-run | Every mutation can be simulated | Same validation and decision path as real mode |
| Retry | Failure is transient and replay is safe | Bounded attempts, delay policy, final status |
| Idempotence | Callers intentionally rerun | Desired state and drift behavior |
| Lock | Concurrent actors share mutable state | Atomic acquisition, owner, timeout/stale policy |

- `flock` can simplify Linux-only locking but is not POSIX; declare it as a dependency.
- Add jitter only when synchronized clients create a real load problem.
- Environment variables may be acceptable for lower-risk configuration, but they are not a universal secret channel.

## Anti-Patterns

- Shipping timestamped log helpers, `--dry-run`, retry, locks, and idempotence in every script template.
- Logging commands with `$*`, which loses argument boundaries and may expose secrets.
- A dry-run wrapper that misses output redirections or mutations outside the wrapper.
- Retrying authentication, validation, permission, or syntax failures.
- Calling `mkdir -p` proof of whole-script idempotence.
- Treating a lock file's existence as proof that its owner is alive.
- Running `set -x` across credential reads or secret-bearing commands.

## Official Sources

- [GNU Bash Reference Manual: The Set Builtin (`-x`)](https://www.gnu.org/software/bash/manual/bash.html#The-Set-Builtin)
- [GNU Bash Reference Manual: Redirections](https://www.gnu.org/software/bash/manual/bash.html#Redirections)
- [POSIX Shell Command Language](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html)
- [ShellCheck Wiki](https://www.shellcheck.net/wiki/)
