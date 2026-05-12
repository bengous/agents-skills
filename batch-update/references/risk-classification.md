# Risk Classification

Keep classification small. Use one primary status plus a short reason.

## Statuses

- `low`: patch/minor/local update, clean research, no peer/runtime signal.
- `family`: coupled packages that must move together.
- `tooling`: runtime, package manager, compiler, build tool, test runner, linter, formatter, or tool-version declaration.
- `breaking`: major version, migration guide, breaking note, peer conflict, removed API, or broad behavior change.
- `blocked`: cannot safely apply now because of ambiguous research, failed install, failing validation, unsupported runtime, or unclear independence.

## Gates

Proceed after research for `low` and straightforward `family` batches.

Ask before applying:

- `breaking`;
- runtime or package-manager upgrades;
- CI/Docker changes;
- dependency replacement or removal;
- manual dependency-version edits because native tooling cannot express the update.

`tooling` can proceed only when impact is narrow and validation is obvious. Otherwise ask.

## Failure Handling

If a batch fails:

1. Attempt one minimal fix only when the cause is clear.
2. Re-run targeted validation.
3. If still failing, revert the batch.
4. Document package(s), current version, target version, command, error, hypothesis, and required decision.

Do not pile unrelated fixes into an update batch.
