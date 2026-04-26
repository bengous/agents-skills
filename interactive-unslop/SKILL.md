---
name: interactive-unslop
description: Interactive prose review and de-slopping workflow for personal essays, blog posts, journal chapters, articles, README prose, public notes, and other authored text. Use when the user asks to unslopify, deslop, humanize, review tone, refine prose, continue an editorial pass, work section by section, preserve voice, discuss wording before edits, add source comments explaining editorial decisions, or coordinate with another agent before applying text changes.
---

# Interactive Unslop

Use this skill to review authored text interactively, one small section at a time. The goal is not to rewrite the author away; it is to remove generic AI prose, preserve factual grounding, and make decisions explicit before editing.

## Core Contract

- Do not edit before showing proposals unless the user explicitly says to apply a specific wording.
- Work in small chunks: one paragraph, section, list, heading, or local argument at a time.
- Preserve the author's voice, chronology, and intent. Do not replace lived experience with generic polish.
- Separate diagnosis from proposals: say what is wrong, why it matters, and what each rewrite changes.
- Treat comments, commit history, source repos, prior drafts, and adjacent chapters as evidence when the text depends on them.
- Commit narrowly when the user asks to commit. Do not stage unrelated files.

## Workflow

1. **Orient**
   - Read the target text and adjacent context.
   - If the user says they are continuing a previous pass, inspect recent history and existing inline comments.
   - Identify the active section and quote only the relevant current text.

2. **Diagnose**
   - Flag AI tells: slogans, deck-like headings, fake transitions, decorative emphasis, abstract nouns, unexplained buzzwords, too-clean conclusions, and generic examples.
   - Flag factual drift: chronology errors, modern framing applied to old work, source claims not grounded in the artifact.
   - Flag aging risk: raw numbers, tool names, or technical claims that need temporal context.

3. **Propose**
   - Offer 1-3 variants with clear tradeoffs.
   - Keep facts stable unless explicitly discussing a factual correction.
   - Prefer concrete source-grounded wording over broad claims.
   - Mark risky changes: tone shift, chronology shift, stronger claim, weaker claim, or information loss.

4. **Decide With The User**
   - Ask concise questions only when a real decision is needed.
   - If the user selects wording, apply exactly that direction with minimal cleanup.
   - If the user pushes back, treat it as signal and refine the proposal instead of defending the first draft.

5. **Edit And Verify**
   - Apply the smallest patch that implements the accepted wording.
   - Re-read the surrounding lines after editing.
   - For generated/public output, run the relevant build or preview check when the edit could affect rendering.

6. **Commit When Asked**
   - Stage only the scoped file(s).
   - Use the repo's commit style.
   - Mention leftover untracked/unrelated files after the commit.

## Technical Journal Mode

Use this mode for technical memoirs, engineering journals, and dated narratives of real work.

- Work chronologically. Do not turn chronological order into causal necessity.
- Separate the lived moment, retrospective interpretation, and current-state updates.
- Preserve failure modes when they demonstrate judgment. Do not sand them into generic success stories.
- When a tool or workflow is later abandoned, name what survived: primitives, discipline, constraints, or habits.
- Verify exact technical names before editing: commands, repos, commits, file paths, product names.
- For technical credibility, prefer one concrete incident over a list of abstract symptoms.
- Watch for "built too much, then abandoned it" readings. If that is the story, show the learning and durable practice.
- Track the narrative arc across chapters: experimentation, context management, orchestration, guardrails, stabilization.
- Make the author shine through judgment and tradeoffs, not through self-praise.
- Add editorial comments for decisions future agents are likely to undo.

## Peer Agent Use

Only spawn another agent when the user explicitly asks for agent discussion, brainstorming, Claude/Opus/Codex comparison, or independent review.

When spawning a peer:
- Give a bounded task and a clear reason.
- Ask for diagnosis and proposals, not edits.
- Do not leak your preferred answer unless the peer is validating a specific proposal.
- Reconcile disagreements before presenting proposals to the user.
- Keep the final recommendation as a human-readable editorial choice, not an agent debate transcript.

For long-form journal passes, use distinct reviewers when requested:
- one slop/voice reader;
- one domain reader, such as technical journal, recruiter, tech lead, or architecture reviewer.

Tell reviewers what has not been rewritten yet so they do not overfocus on unfinished chapters. Ask for critique, not praise.

## Source-Grounded Review

Use source artifacts when the prose describes prior work:
- Read the linked repo, draft, commit history, notes, or issue before rewriting concrete claims.
- Prefer exact mechanisms over generic examples.
- If a system is obsolete, frame it as a dated exploration instead of pretending it is current guidance.
- Use present-tense narration inside a dated phase when it helps the reader live the moment; use retrospective framing in explicit bilan/recul sections.

## Inline Comments

Add invisible source comments when the editorial decision is durable and likely to be challenged later.

Use the file's existing comment style. For MDX prose, prefer:

```mdx
{/* Editorial decision: explain why the wording is shaped this way and what future editors should preserve. */}
```

Good comment targets:
- chronology choices
- source-grounding decisions
- why a number is approximate or contextualized
- why a section uses present narration despite being historical
- why a phrase avoids a tempting but false modern framing
- why a later update is framed as retrospective context rather than a rewrite of the dated narrative
- why an obsolete tool is described through the primitive that survived

After adding comments to generated/public content, verify they do not leak into production HTML or JS when practical.

## Output Shape

For each section under review, prefer:

```md
Current text:
[short excerpt]

What does not work:
- [specific issue]

Proposals:
1. [proposal] — [tradeoff]
2. [proposal] — [tradeoff]

Recommendation:
[one clear choice]
```

Keep responses concise. The user is making live editorial decisions; do not bury the choice in a long essay.
