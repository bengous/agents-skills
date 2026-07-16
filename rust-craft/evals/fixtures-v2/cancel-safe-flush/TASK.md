# Cancellation regression

Production reports duplicate records when an in-progress flush is cancelled
and retried. Repair the implementation without changing the documented
delivery contract or adding dependencies.

Run the project checks and record changed files, decisions, commands, results,
and remaining gaps in `EVAL_REPORT.md`.
