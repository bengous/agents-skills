# Slop Detector — Design Notes

This file is not read by the agent. It's for the skill author (you) to understand the design decisions.

---

## 1. Synthesis: What Matters from the Research and the Guide

**From the article on skills:**

The highest-value lessons applied here:

- **Skills are folders, not files.** The heuristics, genre presets, voice strategy, and examples are all separate files loaded on demand. SKILL.md is a hub that dispatches to them. This follows the progressive disclosure pattern: ~100 lines in the hub, details in reference files.
- **Description is a trigger, not a summary.** The description field lists specific phrases a user would say ("does this sound AI", "deslop", "fix my LinkedIn post") so the model can pattern-match against real user intent.
- **Gotchas are the highest-signal content.** The gotchas section in SKILL.md was written from real failure modes — false positives on brevity, overcorrecting into messiness, mirror-slop in rewrites.
- **Don't railroad.** The skill gives heuristics and calibration, not step-by-step scripts. The model decides how to apply them based on the input text.
- **Give it code.** The `score.py` script handles deterministic pattern matching so the model can focus on the qualitative, contextual work that regex can't do.
- **Store data.** The voice profile is a persistent memory mechanism — the skill learns the user's voice over time.

**From the research on AI writing patterns:**

The heuristics file distills these into 10 concrete, scoreable categories. The key insight: individual patterns aren't damning, but clusters are. A single "moreover" is fine. "Moreover" + "furthermore" + "let's dive in" + numbered takeaways in a 200-word text is a clear AI fingerprint.

The meta-check ("could anyone have written this?") catches the deepest form of slop: text that passes every individual heuristic but has no voice, no perspective, no specificity.

---

## 2. Architecture Rationale

### Why hybrid (critic + rewriter)?

Pure critics frustrate users — they point out problems without fixing them. Pure rewriters destroy voice — they replace the original wholesale. The hybrid approach diagnoses first (so the user understands what's wrong), then proposes targeted fixes (so the user keeps control).

The skill defaults to line edits, not full rewrites. A rewrite is only produced when the text is >70% contaminated or the user explicitly asks. This respects the principle that the user's text is theirs.

### Why a scoring script?

The script is a pre-screen, not the full detector. It handles the deterministic portion of findings:

- **Pattern matching** (~30%): exact phrase detection for buzzwords, template language, emotional inflation, transitions, hedging
- **Contraction analysis**: counts formal vs. contracted forms and flags suspiciously formal ratios in non-academic text
- **Vocabulary diversity**: type-token ratio measurement to catch synonym cycling
- **Bigram predictability**: filler bigram frequency as a proxy for the low-perplexity signal research identifies in AI text
- **Emotional flatness**: absence of voice markers (opinions, hedges, first-person reactions) in genres that demand personality
- **Structural analysis**: sentence length variance, paragraph uniformity, numbered list patterns

The model handles the ~40% that requires context, judgment, and voice awareness: rhetorical intent, genre-appropriate exceptions, voice preservation, and the meta-check.

### Why separate genre presets?

Genres have wildly different tolerance levels. "Thrilled to announce" is a death sentence in a LinkedIn post but wouldn't appear in a cover letter. "I am writing to express my interest" is the worst cover letter opening but irrelevant for blog posts. Keeping presets separate means each genre gets calibrated independently.

### Why the voice profile is optional?

Most first-time uses won't have a voice profile. The skill needs to work well without one. The profile is a bonus that improves over time — it's the skill's memory layer.

---

## 3. Folder Structure

```
slop-detector/
├── SKILL.md                          ← Hub. Workflow, output format, gotchas, file table.
├── voice-profile-template.md         ← Template for user voice profile (not active until populated)
├── references/
│   ├── heuristics.md                 ← Core detection engine. 13 categories + meta-check.
│   ├── genre-presets.md              ← Per-genre scoring adjustments (LinkedIn, cover letter, recruiter, general).
│   ├── voice-strategy.md             ← How to preserve voice during rewrites. How to build a profile.
│   └── strictness-modes.md           ← Brutal / firm / gentle / calibration-only.
├── examples/
│   └── bad-to-good.md               ← 5 worked examples at different severity levels.
├── scripts/
│   └── score.py                      ← Deterministic pre-screen. Regex + statistical analysis (6 analyzers).
├── evals/
│   └── evals.json                    ← 5 test prompts with expected outcomes.
└── DESIGN.md                         ← This file. Not read by the agent.
```

---

## 4. Triggering Strategy

The description field was designed for the model's scanning pass, not for human readability. It includes:

- **Action phrases the user would actually type**: "check this for slop", "does this sound AI", "rewrite this", "make this sound human"
- **Genre-specific triggers**: "edit my LinkedIn post", "fix my cover letter"
- **Colloquial variants**: "deslop", "voice check", "too much fluff", "sounds like ChatGPT"
- **Agent-to-agent trigger**: "reviewing any text output from another agent or skill that will be published" — this lets other skills in a pipeline invoke slop-detector as a quality gate

The scope and non-scope sections prevent false triggers. "Grammar-only proofreading" and "code review" are explicitly excluded so the skill doesn't activate for tasks where a different tool is more appropriate.

---

## 5. Evaluation Plan

### Pre-screen script validation

Run `score.py` against the 5 eval prompts. Expected:
- Eval 1 (LinkedIn slop): 20+ pre-screen score, 7+ findings
- Eval 2 (cover letter template): 20+ pre-screen score, 4+ findings
- Eval 3 (clean technical writing): 0 pre-screen score, 0 findings
- Eval 4 (recruiter spam): 15+ pre-screen score, 3+ findings
- Eval 5 (mild corporate padding): 0-5 pre-screen score, 0-2 findings

### Full skill validation

Run the agent with the skill on each eval prompt. Check:
1. **Score calibration**: Does the score match the expected range in evals.json?
2. **Finding completeness**: Are the expected patterns flagged?
3. **False positive rate**: Are clean sentences left alone?
4. **Rewrite quality**: Do rewrites pass the same audit? (Mirror-slop check)
5. **Voice preservation**: If a voice profile exists, do rewrites match it?
6. **Genre awareness**: Does the agent load the correct preset?
7. **"Leave as is" detection**: Does eval 3 correctly get a low score and no rewrite?

### Ongoing evaluation

After each use, ask: did the skill flag something that shouldn't have been flagged? Did it miss something obvious? Add any new failure modes to the gotchas section and any new patterns to the heuristics.

---

## 6. Future Extensions (highest-value only)

1. **French language support.** You write in French for mpzinc.fr and likely for LinkedIn in French. The heuristics are English-only right now. A `references/heuristics-fr.md` file with French-specific patterns (overuse of "en effet", "il convient de noter", "force est de constater", "dans le cadre de", em dashes in French AI output, "afin de" instead of "pour") would extend coverage. The contraction analysis would need adaptation too — French doesn't contract the same way.

2. **Adversarial self-check.** After producing a rewrite, automatically re-run the audit on the rewrite itself. If the rewrite scores above 25, flag it and revise. This prevents mirror-slop without manual verification.

3. **Comparative scoring over time.** Store scores in a log file (`${CLAUDE_PLUGIN_DATA}/audit-log.jsonl`). Track whether the user's writing is getting cleaner or whether the same patterns keep recurring. Surface trends when running an audit.

4. **Multi-file batch mode.** Run the detector across all markdown files in a directory (e.g., a blog content folder) and produce a summary report ranking files by slop score.

5. **Pre-commit hook.** A `hooks/pre-publish.sh` that runs `score.py` on any text file being committed and blocks if the score exceeds a threshold. Useful as a CI gate for content repos.

6. **N-gram perplexity model.** The current bigram predictability check is a rough proxy. A more accurate approach would train a simple trigram or 4-gram model on a corpus of human writing and compute actual perplexity scores against it. Probably overkill for most uses, but would be the gold standard for the "predictability" axis the research emphasizes.
