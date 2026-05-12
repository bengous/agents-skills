# Ecosystem Commands

Use native tooling. Do not manually edit lockfiles. Do not manually edit dependency version declarations unless native tooling cannot express the update and the user explicitly approves.

These are starting points; inspect the repo before choosing exact commands.

## JavaScript

Inventory:

```sh
bun outdated -r
npm outdated --workspaces
pnpm outdated -r
yarn outdated
```

Apply with the package manager used by the repo. Prefer workspace-aware commands. Keep catalog/source-of-truth declarations centralized; do not edit generated or derived consumers directly.

Validation examples:

```sh
bun install
bun test
bun run lint
bun run build
```

Use repo scripts over generic commands when available.

## Go

Inventory per module:

```sh
go list -m -u all
```

Apply with Go tooling:

```sh
go get <module>@<version>
go mod tidy
go test ./...
```

Do not manually edit `go.sum`.

## Python

Detect the project manager first: uv, Poetry, pip-tools, plain pip, or another tool. Use that tool's native upgrade command. Do not rewrite lockfiles manually.

Validation examples:

```sh
uv run pytest
uv run ruff check .
uv run pyright
```

Use commands present in the repo.

## Rust

Inventory/apply with Cargo:

```sh
cargo update
cargo update -p <package>
cargo test
cargo clippy --all-targets --all-features
```

Do not manually edit `Cargo.lock`.

## mise And Runtime Tools

Inventory with `mise` when available. Runtime/tool updates are `tooling` batches and must be separate from application dependency updates.

Check related declarations together:

- `mise.toml` and `.mise.toml`;
- `packageManager`;
- `engines`;
- `go.mod` language versions;
- Python `requires-python`.

Avoid creating mismatches between installed tool versions and declared runtime constraints.

## CI And Docker

Detect and report in V1. Do not auto-bump unless the user explicitly approves the CI/Docker batch.
