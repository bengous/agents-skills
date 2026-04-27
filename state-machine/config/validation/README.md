# State-machine validation

This directory defines how the references in `SKILL.md` and `references/**/*.md`
are validated.

## Layout

| Path | Role |
|---|---|
| `manifest.json` | Human-readable validation summary: tools, pinned inputs, coverage counts, and command matrix. |
| `snippets.json` | Source-of-truth fence inventory. Every Markdown code fence is listed by file, index, language, hash, and validation mode. |
| `toolchains/npm/` | Locked npm toolchain for TypeScript, JavaScript, React, XState, Svelte, Mermaid, TOML, Markdown, and link validation. |
| `toolchains/cargo/` | Locked Cargo toolchain for Rust snippets that need `statig`. |

The `toolchains/npm/` name does not mean only JavaScript snippets are validated.
It means these validators are distributed through npm. The runner uses that
single locked package set for TypeScript, React, XState, Svelte, Mermaid, Taplo,
Markdown linting, and link checks.

## Runtime fixtures

The runner creates language-specific scratch projects under `/tmp`:

- C snippets are compiled with `gcc` and `clang`.
- Java snippets are compiled with `javac`.
- Rust snippets are copied into a scratch Cargo crate.
- Mermaid diagrams are rendered to scratch SVG files.
- Shell snippets are checked with `shellcheck`.
- TOML snippets are checked with Taplo from the npm toolchain.

Generated fixtures are intentionally not committed. If a snippet changes, update
`snippets.json` deliberately so the new hash and validation mode are reviewed.

## Command

Run from the repository root:

```sh
bash state-machine/scripts/validate-references.sh
```

Or from inside `state-machine/`:

```sh
bash scripts/validate-references.sh
```
