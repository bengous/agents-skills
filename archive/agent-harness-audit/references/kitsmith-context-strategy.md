# Kitsmith Context Strategy

Use this reference when auditing Kitsmith itself or a repo that follows its
generated agent-tooling model.

## Nominal Model

- `CLAUDE.md` and `.claude/rules/**` are editable source context.
- Root and nested `AGENTS.md` files are generated agent projections.
- Agent sync tooling turns source context into generated projections.
- Generated projections are runtime surfaces to verify for drift, size, scope,
  and precedence, not the default place to edit policy.
- Local `.agents/skills/**` in the Kitsmith parent are maintainer/local skills
  unless explicitly routed into generated output.

## Common Commands

For source/projection changes:

```bash
bun run agents:sync
bun run agents:check
```

For parent-managed copied tooling:

```bash
bun run parent-tooling:sync
bun run parent-tooling:check
```

For normal local validation:

```bash
bun run check
bun run validate
```

For release-grade validation, Kitsmith has historically used:

```bash
bun run release:prepare
```

Do not run heavy gates by default during audit. Use them as known validation
surfaces and run only when the user asks or the audit needs cheap proof.

## Generated Project Boundary

Kitsmith treats generated project output as product behavior. If a recommendation
would affect generated projects, route it explicitly:

- engine behavior: `src/`
- dynamic rendered files: `templates/`
- stable copied presets: `template-sources/`
- generated dependency data: `config/generated-dependencies/`
- product contracts: `docs/product/`
- generated-project validation: `scripts/testing/` and contract tests

Do not recommend editing a generated artifact directly when a source, template,
or sync process owns it.

## Skill Distribution Boundary

In the Kitsmith parent repo, `.agents/skills/**` is not automatically included in
generated projects. Shipping a skill to scaffolded/adopted projects is a separate
product change that needs template/manifest/generation and contract-test updates.

For a first repo-local skill implementation, creating `.agents/skills/<name>` is
only the canonical/local source. Treat delivery to generated projects as a
follow-up unless the user explicitly asks for it.
