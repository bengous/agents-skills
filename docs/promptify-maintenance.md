# Promptify maintenance

`promptify` is optimized for GPT-5.5 as of 2026-05-06. Revisit this document and the skill whenever OpenAI publishes a new primary model or materially changes GPT-5.5 prompting guidance.

## Source of truth

- Skill source: `promptify/SKILL.md`
- UI and invocation policy: `promptify/agents/openai.yaml`
- Human maintenance notes: this file

The skill body should stay lean. Do not add model research notes, migration history, long examples, or rationale to `SKILL.md` unless the agent must read them every time the skill runs.

## Official sources consulted

- OpenAI latest model guide: `https://developers.openai.com/api/docs/guides/latest-model`
- OpenAI GPT-5.5 prompt guidance: `https://developers.openai.com/api/docs/guides/prompt-guidance?model=gpt-5.5`
- OpenAI Codex Agent Skills docs: `https://developers.openai.com/codex/skills`

Relevant GPT-5.5 guidance used:

- Start from a small prompt that preserves the product contract.
- State expected outcome and success criteria.
- Reduce step-by-step process guidance unless the path matters.
- Make allowed side effects, evidence rules, output shape, and stopping rules explicit.
- Keep style and collaboration instructions short.
- Treat uncertainty as a clarification gate instead of hiding assumptions.

Relevant Codex skill guidance used:

- A skill is a focused `SKILL.md` plus optional resources.
- Skills use progressive disclosure: metadata first, then `SKILL.md`, then optional files.
- `agents/openai.yaml` can set `policy.allow_implicit_invocation: false`.
- `allow_implicit_invocation: false` keeps the skill explicit-only while preserving `$promptify` invocation.

## Design decisions

`promptify` has one job: rewrite rough dictated intent into a copy-pasteable GPT-5.5 prompt. It does not plan the work itself, execute side effects, or broaden the user's request.

The skill accepts one parameter, `prompt`, because its input should mirror the user's raw spoken instruction. If the prompt is missing, it asks for that single missing input instead of starting a questionnaire.

The final prompt shape is section-based but optional. GPT-5.5 benefits from explicit outcomes, constraints, allowed actions, validation, and stopping rules, but forcing every section into every output would add noise.

The ambiguity gate exists because prompt rewriting can silently invent intent. If blockers remain, the skill routes to `$grill-me` and asks one question at a time before generating the final prompt.

The manual-only invocation policy exists because this skill changes user intent before another model acts on it. It should run only when Augustin explicitly invokes `$promptify` or names the skill.

## Maintenance checklist

When OpenAI releases a new target model:

1. Read the latest model guide and model-specific prompt guidance from official OpenAI docs.
2. Check whether outcome-first prompting, verbosity, reasoning effort, tool-use, and clarification guidance changed.
3. Update `promptify/SKILL.md` only if the runtime behavior should change.
4. Update this file with the new target model, sources, and rationale.
5. Validate with `cargo run -p skills-tools -- validate frontmatter promptify/SKILL.md`.
6. Install from the local repo and verify source/live parity.
