# Process Lifecycle

## Required Rules

- Install signal handlers before starting children. Record each direct child PID immediately after launch.
- Keep signal handlers small: record the first signal, forward it to every tracked child, and let the main path reap them.
- Wait for every child separately and retain each status. Retry a Bash `wait` interrupted by a trapped signal while that child still exists.
- Define the aggregate result: first failure, fail-fast cancellation, all results, or signal status. Do not let loop order or the last `wait` decide accidentally.
- Distinguish direct children from process groups and descendants. Signaling one PID does not guarantee its grandchildren stop.
- Use `exec` when a wrapper has no cleanup or aggregation duty and should become the managed process.
- Ensure normal exit and signal exit both leave no owned children or temporary state behind.

## Contextual Decisions

- Direct-child supervision is enough when workers own their descendants. Otherwise establish process groups with an explicitly supported platform mechanism.
- Add fail-fast cancellation only when sibling results are no longer useful after one failure.
- Add TERM grace periods and KILL escalation only when shutdown deadlines are defined. Report forced termination.
- Use Bash `wait -n` only with a declared Bash version and when completion-order handling reduces complexity.
- A bounded worker pool is warranted only when input size or resource limits require it.

## Anti-Patterns

- Starting background work and exiting without reaping it.
- Bare `wait` followed by success.
- Checking `kill -0` and assuming the later `kill` cannot race; send the signal and handle an already-exited child.
- `kill 0` or a negative process-group ID without proving the supervisor is outside the target group.
- An `EXIT` trap that always exits zero or overwrites the child's status.
- Forwarding TERM but forgetting INT and HUP contracts.
- Spawning more workers after a shutdown signal was recorded.

## Official Sources

- [GNU Bash Reference Manual: Signals](https://www.gnu.org/software/bash/manual/bash.html#Signals)
- [GNU Bash Reference Manual: Job Control Basics](https://www.gnu.org/software/bash/manual/bash.html#Job-Control-Basics)
- [GNU Bash Reference Manual: Job Control Builtins (`wait`)](https://www.gnu.org/software/bash/manual/bash.html#Job-Control-Builtins)
- [POSIX `trap`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/trap.html)
- [POSIX `wait`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/wait.html)
- [ShellCheck SC2064: trap expansion timing](https://www.shellcheck.net/wiki/SC2064)
