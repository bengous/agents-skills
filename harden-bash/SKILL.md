---
name: harden-bash
description: Write, review, and harden non-trivial Bash and POSIX sh scripts for production. Use for shell files and CI, deployment, administration, or automation code involving untrusted input, destructive filesystem operations, privileges, pipelines, traps, signals, concurrency, portability, or failure handling. Skip trivial interactive commands and tasks better served by an existing project tool.
---

# Harden Bash

## Gate

Read the script and every caller before changing it. Determine whether it is executed or sourced, which inputs cross trust boundaries, which privileges and side effects it has, and which shell actually parses it.

## Contracts

- Choose Bash or POSIX `sh` explicitly. Never mix their language or runtime contracts.
- Control expansions and preserve argument boundaries.
- Build commands as argv, never as executable strings.
- Make expected and unexpected failures visible with useful status and context.
- Keep secrets out of argv, logs, permissive temporary files, and tracing.
- Validate destructive scope before mutation.
- Add no abstraction or operational feature without a demonstrated contract.

## Workflow

1. Inspect callers, execution mode, inputs, privileges, side effects, dependencies, and target shells.
2. Classify the relevant risks with the table below.
3. Load only the references and examples for those risks.
4. In an audit, remain read-only and report findings by severity with location, root cause, and minimum correction. In a write or fix task, change the correct common point rather than patching callers independently.
5. Produce the smallest correction that preserves the intended behavior.
6. Validate syntax, diagnostics, behavior, failure paths, and every promised shell/runtime boundary.

| Risk | Reference | Example |
| --- | --- | --- |
| Dialect, shebang, versions, utilities | [`references/dialect-and-portability.md`](references/dialect-and-portability.md) | [`bash/safe-cli.sh`](examples/bash/safe-cli.sh), [`posix/safe-cli.sh`](examples/posix/safe-cli.sh) |
| Inputs, expansions, argv, injection | [`references/input-and-command-construction.md`](references/input-and-command-construction.md) | [`bash/safe-cli.sh`](examples/bash/safe-cli.sh), [`posix/safe-cli.sh`](examples/posix/safe-cli.sh) |
| `set -e`, statuses, pipelines, substitutions | [`references/error-semantics.md`](references/error-semantics.md) | [`safe-cli.sh`](examples/bash/safe-cli.sh), [`atomic-replace.sh`](examples/posix/atomic-replace.sh), [`supervise-jobs.sh`](examples/bash/supervise-jobs.sh) |
| Temporary files, traps, atomicity, permissions, deletion | [`references/filesystem-safety.md`](references/filesystem-safety.md) | [`posix/atomic-replace.sh`](examples/posix/atomic-replace.sh) |
| Signals, children, concurrency, `wait` | [`references/process-lifecycle.md`](references/process-lifecycle.md) | [`bash/supervise-jobs.sh`](examples/bash/supervise-jobs.sh) |
| Logs, dry-run, retry, locks, idempotence, secrets | [`references/production-controls.md`](references/production-controls.md) | Read conditionally |
| Syntax, lint, tests, shell matrix | [`references/testing.md`](references/testing.md) | [`scripts/validate-examples.sh`](scripts/validate-examples.sh) |
