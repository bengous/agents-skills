---
name: slop-detector
description: >
  Detect, diagnose, score, and rewrite AI-generated or generic text.
  Trigger on: "check this for slop", "does this sound AI", "rewrite this",
  "make this sound human", "edit my LinkedIn post", "fix my cover letter",
  "this sounds generic", "clean up my writing", "audit this draft",
  "too much fluff", "sounds like ChatGPT", "deslop", "voice check",
  or any request to improve, critique, or humanize written text.
  Works in English, French, and Spanish — also trigger on French/Spanish
  cues like "ça fait IA", "détecte le slop", "rends ça plus humain",
  "trop d'anglicismes", "suena a IA", "esto suena a ChatGPT".
  Also trigger when reviewing any text output from another agent or skill
  that will be published, sent, or shared externally.
model: opus
effort: medium
allowed-tools:
  - Bash(python3 *)
---

# Slop Detector

Hybrid critic + rewriter. Finds AI-generated patterns, scores severity, explains what's wrong, and proposes line edits that preserve the writer's voice.

## Dependencies

Python 3 runtime: !`python3 --version`

## When to use this skill

- User wants text audited for AI-sounding patterns
- User wants a draft rewritten to sound more natural/human
- User asks for feedback on tone, voice, or genericness
- Text is being prepared for LinkedIn, cover letters, recruiter messages, or any external audience
- Another skill produced text that needs a voice/quality pass before delivery

## When NOT to use this skill

- Proofreading for grammar/spelling only (no voice or slop concern)
- Translation tasks
- Technical documentation where clinical tone is correct and intentional
- Code review
- The user explicitly says "don't change the voice" or "literal transcription"

## Core workflow

1. **Read the input text.** Identify the **language** (English, French, Spanish) and the genre (LinkedIn, cover letter, recruiter message, general).
2. **Run the pre-screen.** `python3 scripts/score.py --lang <en|fr|es|auto> --genre <genre> --file <path>` (or pipe via stdin). It returns a score, a `likely_ai` flag, and findings grouped by category. This is a baseline, not the verdict.
3. **Load the right heuristics.** Always apply `references/heuristics.md` (Parts I + II). For **French** also read `references/heuristics-fr.md`; for **Spanish**, `references/heuristics-es.md`. Then load the genre preset from `references/genre-presets.md`.
4. **Run the audit.** Apply the heuristics with the **two-tier gating** in mind: hard tells count alone, soft tells only in clusters. Don't flag isolated soft hits in otherwise strong writing.
5. **Produce the diagnosis.** Use the output format below.
6. **Propose line edits.** Be specific: quote the exact phrase, explain the problem, offer a replacement. For anglicisms, propose the correct native term.
7. **Optionally rewrite.** If the user asks for a full rewrite, or if the text is >70% contaminated, produce one. Otherwise, line edits only.
8. **Check voice config.** If `voice-profile.md` exists in this skill folder, compare rewrites against it. Read `references/voice-strategy.md` for how.

## Output format

```
## Slop Score: [X]/100
(0 = pristine human writing, 100 = pure AI slop)

## Diagnosis
[2-3 sentence summary of overall problems]

## Findings
### Category: [e.g., Buzzword Inflation]
- **Line/phrase**: "[exact quote]"
- **Problem**: [what's wrong, specifically]
- **Fix**: [replacement or instruction]
- **Severity**: high | medium | low

[repeat per finding, grouped by category]

## Structural Issues
[If any: paragraph rhythm, section symmetry, transition abuse, etc.]

## Line Edits
[Numbered list. Each: original → replacement, with short rationale]

## Rewrite (if requested or score > 70)
[Full improved version]

## Verdict
[One sentence: is this publishable, needs work, or needs a full rewrite?]
```

If the text scores below 20 and has no high-severity findings, say "Leave as is" and explain why it works. Do not manufacture problems to justify the skill's existence.

## Gotchas

These are real failure modes. Add to this list when you hit new ones.

- **False positive on intentional simplicity.** Short declarative sentences are not slop. "I built this. It works. Ship it." is strong writing. Don't flag brevity as a problem.
- **Overcorrecting into messiness.** "Humanizing" does not mean adding filler words, typos, or casual slang. It means removing the synthetic polish that signals machine origin.
- **Destroying precision.** If the original says "reduced latency by 40%", do not replace it with "made things faster." Specificity is human. Vagueness is slop.
- **Genre blindness.** A cover letter is allowed to be slightly formal. A LinkedIn post should NOT sound like a cover letter. Read the genre preset.
- **Rewriting when you should edit.** If the text is 60% good, propose line edits. Don't throw it out. Rewriting everything signals laziness, not craft.
- **Mirror slop.** Your own rewrites must pass the same audit. If your "improved" version uses "In today's fast-paced world" or "I'm thrilled to announce", you've failed.
- **Confusing voice with vocabulary.** Voice is about rhythm, sentence length variation, what gets emphasized, what gets cut. It's not about swapping "utilize" for "use."
- **Praising mediocrity.** If asked to audit, audit. Do not soften findings with "overall this is quite good!" unless it actually is.
- **Contraction false positives on formal genres.** Academic papers, legal text, and formal speeches legitimately avoid contractions. Don't flag "do not" in a legal brief. Check the genre first.
- **Vocabulary diversity in expert writing.** A domain expert using precise technical vocabulary (each term means something different) is not synonym cycling. Only flag when the variation serves no semantic purpose — when "important", "crucial", "vital", "essential" all mean exactly the same thing in context.
- **Emotional flatness in reports.** A quarterly earnings summary or technical postmortem is supposed to be neutral. Flatness is only a finding when the genre demands personality (LinkedIn, cover letters, blog posts, personal emails).
- **Penalizing real human flatness.** Some humans genuinely write in a flat, factual style. If the text has other human signals (contractions, specific details, uneven rhythm), don't double-penalize for low emotional markers.
- **Em-dash is not a binary tell.** A single em-dash means nothing — the em-dash signal is purely about density, and the "ChatGPT hyphen" (spaced ASCII hyphen) or double hyphen are em-dash surrogates left by humanizer tools. Flag the pattern, never one dash.
- **French typography is not slop.** A non-breaking space before `:` `;` `!` `?` and inside « » is *correct* French. Never flag it. The curly apostrophe in elisions and contractions (`l'homme`, `aujourd'hui`, `don't`) is a native apostrophe rather than a smart quote, so never flag it either. The Spanish dialogue raya (—) is correct per the RAE. The pre-screen guards these; you must too.
- **Anglicisms are gated for a reason.** One "digital" or "challenge" in French (or "aplicar"/"remover" in Latin-American Spanish) is normal human usage. The signal is the *accumulation* of calques plus other tics — never a single entrenched borrowing.
- **The statistical analyzers (contractions, vocab diversity, perplexity, flatness) are English-only.** They assume English morphology, so `score.py` skips them for French/Spanish. Lean on the pattern findings and your own judgment there.

## Strictness

Default mode is **brutal**. This means:
- Flag every instance, not just the worst ones
- Score honestly — most AI-generated text lands between 55-85
- Do not grade on a curve
- "Acceptable for a first draft" is not the bar

For alternative strictness modes, see `references/strictness-modes.md`.

## Reference files in this skill

| File | Read when |
|------|-----------|
| `references/heuristics.md` | Every audit. Core engine (Part I) + newer tics, gating, multilingual (Part II). |
| `references/heuristics-fr.md` | The input text is in French. |
| `references/heuristics-es.md` | The input text is in Spanish. |
| `references/genre-presets.md` | After identifying the genre of the input text. |
| `references/voice-strategy.md` | When a voice profile exists, or when rewriting. |
| `references/strictness-modes.md` | When user requests a different strictness level. |
| `examples/bad-to-good.md` | When you need calibration on what "good" looks like. |
| `scripts/score.py` | Deterministic pre-screen (EN/FR/ES). Use `--lang` and `--genre`. |
| `scripts/patterns.py` | Generated tic registry consumed by `score.py`. Not read directly. |
| `voice-profile.md` | Only if the user has created one. Not shipped by default. |
