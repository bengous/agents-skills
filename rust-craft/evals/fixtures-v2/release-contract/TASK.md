# Release repair

The `wire-core` release candidate regressed after a partial refactor. Restore
the repository's documented release contract with the smallest compatible
change. Do not change the supported matrix or intended public behavior.

Run the project release gate and record changed files, decisions, commands,
results, and remaining gaps in `EVAL_REPORT.md`.
