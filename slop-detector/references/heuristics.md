# Heuristics Reference

This is the detection engine. Apply these checks systematically during every audit. Each category includes specific patterns to search for, why they're problems, and how to score them.

## Scoring rules

Each finding gets a severity: **high** (5 pts), **medium** (3 pts), **low** (1 pt). Sum all findings. Cap at 100. A clean text with zero findings scores 0.

Severity assignment:
- **High**: Pattern that would make a reader think "this is AI-generated" within 2 seconds
- **Medium**: Pattern that contributes to a generic/synthetic feel but isn't immediately damning
- **Low**: Pattern that's mildly formulaic but could appear in competent human writing

---

## Category 1: Buzzword Inflation

The text uses words that sound important but add zero information. AI models overindex on these because they appear frequently in training data as "good writing."

### Watch for
- "leverage", "utilize", "facilitate", "streamline", "optimize", "empower"
- "delve", "navigate", "synthesize", "unveil", "harness", "spearhead"
- "innovative", "cutting-edge", "state-of-the-art", "next-generation", "world-class"
- "holistic", "synergy", "ecosystem", "paradigm", "landscape", "disruptive"
- "robust", "scalable", "seamless", "end-to-end"
- "drive impact", "create value", "deliver results", "move the needle"
- "passionate about", "deeply committed to", "excited to share"
- "game-changer", "transformative", "revolutionary"
- "In today's [adjective] world/landscape/environment"
- "aims to explore", "navigating the landscape/complexities", "at its core"

### Why it's slop
These words are compression artifacts. The model learned that humans use them in contexts that got upvoted/praised, so it scatters them everywhere. They signal nothing because they could describe anything.

### Scoring
- Single instance in a long text: low
- 3+ instances: medium per instance
- Opening sentence uses one: high (it contaminates the reader's trust immediately)

### What to do
Replace with the specific claim. "Leverage AI" → "use GPT-4 to classify tickets." "Drive impact" → "cut response time from 48h to 4h." If there's no specific claim behind the buzzword, the sentence probably shouldn't exist.

---

## Category 2: Hollow Transitions

AI text overuses transitions to create the illusion of logical flow where none exists. The model chains sentences that sound connected but each one could be deleted without breaking meaning.

### Watch for
- "Moreover", "Furthermore", "Additionally", "In addition"
- "That being said", "With that in mind", "It's worth noting that"
- "This is where [X] comes in", "This is why [X] matters"
- "But here's the thing", "And that's exactly why"
- "Let's dive in", "Let's explore", "Let's unpack"
- "At the end of the day", "When all is said and done"
- "Needless to say" (then why say it?)

### Why it's slop
Real transitions earn their place by connecting genuinely different ideas. AI transitions are spackling paste — they fill gaps between sentences that were generated independently. Delete the transition and read the two sentences: if they flow fine without it, the transition was fake.

### Scoring
- 1-2 in a long piece: low
- Transition-heavy paragraph (3+ in 5 sentences): medium per excess
- "Let's dive in" or "Let's unpack" as an opening: high

### What to do
Delete most of them. For the ones that serve a real purpose, replace with shorter, more specific connectors or restructure the paragraph so the connection is implicit.

---

## Category 3: Symmetry Addiction

AI loves parallel structure to an unnatural degree. Lists of three. Mirrored sentence pairs. Every paragraph the same length. Every section the same structure. Real writing is asymmetric — some ideas get more space because they deserve it.

### Watch for
- Rule of three applied everywhere: "X, Y, and Z" repeated across paragraphs
- Mirrored constructions: "Not only... but also", "Whether... or..."
- Every bullet point starting with the same part of speech
- Sections that all have the same number of paragraphs
- Parallel sentence lengths across adjacent sentences (within ±5 words consistently)
- Opening with a question, then immediately answering it, repeatedly

### Why it's slop
Symmetry in moderation is a rhetorical tool. Symmetry as a default is a machine artifact. Humans naturally vary their rhythm — some sentences are 4 words, some are 30. AI tends to keep everything in a 12-20 word band.

### Scoring
- Occasional parallelism: not a finding
- Persistent structural mirroring across 3+ instances: medium
- Every section follows identical structure: high

### What to do
Break the pattern. Vary sentence length deliberately. Let one bullet be two words and another be a full sentence. Let one section be three paragraphs and the next be one.

---

## Category 4: Hedging Contamination

AI hedges constantly because it was trained to be cautious. The result is text that qualifies every claim into meaninglessness.

### Watch for
- "It's important to note that", "It should be noted that"
- "Arguably", "Potentially", "Essentially", "Fundamentally"
- "In many ways", "To some extent", "In a sense"
- "While there are many factors", "There are various considerations"
- "This can vary depending on", "Results may differ"
- "It's not without its challenges"
- Double hedges: "It could potentially", "might perhaps"

### Why it's slop
Human experts hedge when they genuinely mean it. AI hedges reflexively, even on things it should state plainly. "The sky is arguably blue" is AI hedging. The sky IS blue.

### Scoring
- 1-2 genuine hedges on uncertain claims: not a finding
- Hedging on factual/obvious claims: high per instance
- 4+ hedges in a short text: medium for the pattern, even if individual hedges are defensible

### What to do
Delete the hedge and read the sentence. If the claim is true without qualification, keep it unhedged. If genuine uncertainty exists, use a single precise qualifier: "preliminary data suggests" is better than "it could potentially be argued that."

---

## Category 5: Emotional Distortion

AI gets emotion wrong in two opposite ways. It either overperforms emotion (inflation) or produces text that's eerily flat and neutral (flatness). Both are AI signals — just different failure modes.

### 5a: Emotional Inflation

AI is "thrilled" when a human would be "glad." It's "deeply moved" when a human would just say "thanks." This is especially toxic in LinkedIn posts and cover letters.

### Watch for (inflation)
- "Thrilled", "Honored", "Humbled", "Incredibly grateful"
- "Passionate about", "Deeply committed to"
- "I couldn't be more excited"
- "This journey has been nothing short of [superlative]"
- "Grateful for this incredible opportunity"
- Exclamation points on non-exclamatory content
- Emoji clusters used as emotional amplifiers

### 5b: Emotional Flatness

AI text often reads like an encyclopedia entry — "aggressively neutral," as one researcher put it. No opinion, no stance, no warmth, no frustration. Every sentence is a balanced declarative statement. Research shows AI outputs score closer to neutral on sentiment analysis than human writing, which tends toward stronger positive or negative signals.

### Watch for (flatness)
- Complete absence of first-person opinion ("I think", "I believe", "in my experience")
- Every sentence is a declarative statement — no questions, no exclamations, no asides
- No hedges that signal genuine uncertainty ("probably", "honestly", "I'm not sure but")
- Uniform emotional register throughout — no peaks or valleys
- Text reads like a Wikipedia summary: factually toned, carefully balanced, devoid of stance
- Absence of humor, surprise, frustration, or any recognizable human reaction

### Why it's slop
Real humans rarely announce their emotions in professional writing (inflation), but they also don't write like detached observers of their own lives (flatness). Human text has emotional texture — it's not uniformly excited and not uniformly flat. It varies. A paragraph might be dry, then the next one carries genuine frustration, then back to analytical.

### Scoring
- "Thrilled to announce" or "Humbled to share": high (instant AI signal)
- 2+ emotional amplifiers in a paragraph: medium per excess
- Genuine emotion with specific context: not a finding
- Text longer than 200 words with zero emotional markers (no opinion, no first person, no hedges): medium
- Text that reads like a product description or encyclopedia entry when it shouldn't: high
- Flat tone in a genre that demands personality (LinkedIn, cover letters): high

### What to do
For inflation: cut the emotion word, replace with the reason behind the emotion. If there's no reason, the sentence is padding.

For flatness: identify the one place where the writer clearly has an opinion or reaction, and let it show. Add a single "I think" or "honestly" where appropriate. Don't overcorrect into inflation — the goal is human texture, not performed enthusiasm.

---

## Category 6: Fake Specificity

AI sometimes generates details that sound specific but are actually generic. Numbers without sources. Names of techniques without explaining how they apply. Percentages that seem plausible but aren't grounded in anything.

### Watch for
- Suspiciously round numbers: "increased engagement by 50%"
- Technique name-dropping without application: "using a microservices architecture"
- Vague-but-concrete-sounding time references: "over the past several years"
- Lists of tools/technologies that read like a keyword dump
- "Real-world examples" that are actually hypothetical

### Why it's slop
Real specificity is messy. "We went from 12.3% to 18.7% over 6 months" is human. "We achieved a 50% improvement" is AI filling in a plausible-sounding number.

### Scoring
- Ungrounded statistics: high
- Technology keyword dump: medium
- Vague time references in place of real ones: low

### What to do
If the number is real, keep it and add context. If it's fabricated, either find the real number or cut the claim entirely. An honest "we improved significantly" is better than a fake "by 47%."

---

## Category 7: Structural Templates

AI writing follows recognizable templates. The most common: hook → context → 3 points → conclusion → CTA. This structure isn't inherently bad, but when every output follows it, the template becomes the signal.

### Watch for
- Opening with a provocative question or contrarian claim
- Exactly 3 main points (not 2, not 5 — always 3)
- Each point gets the same treatment (same length, same structure)
- Conclusion that restates the opening
- Final line is a question to the reader or a call to action
- "Here's what I learned" as a framing device
- Numbered "takeaways" or "lessons"

### Why it's slop
The template creates predictability. By the second paragraph, the reader knows exactly what's coming. Real writing surprises — it might not have a clean conclusion, or it might front-load all the insight and trail off, or it might be one long paragraph.

### Scoring
- Following the standard template closely: medium
- Opening with "Here's what I learned" + numbered takeaways: high
- Ending with a generic question ("What do you think?"): medium

### What to do
Break the template. Start with the strongest point instead of building to it. Merge two weak points into one strong one. End when you've said what you need to say — don't force a wrap-up.

---

## Category 8: Redundancy and Padding

AI restates the same idea in slightly different words to fill space. Sentences that add no new information. Phrases that exist purely to extend the text.

### Watch for
- Sentence pairs where the second rephrases the first
- "In other words" followed by the same idea
- Paragraphs where removing any sentence doesn't change the meaning
- Conclusions that just summarize what was already clear
- Introductions that preview what the reader is about to read
- "As mentioned earlier" or "As I said before" (if it was clear the first time, don't repeat it)

### Why it's slop
AI generates tokens sequentially and has no global awareness of whether it already said something. It also tends toward longer outputs because training data rewards thoroughness. Padding is the most fixable category — just delete it.

### Scoring
- Single redundant sentence: low
- Redundant paragraph: medium
- Conclusion that restates every point already made: high

### What to do
Delete. No replacement needed. Shorter is almost always better.

---

## Category 9: Register Mismatch

The text switches between formal and casual in ways that feel unearned, or uses a register inappropriate to the context.

### Watch for
- Academic phrasing in a LinkedIn post: "It is imperative that we consider"
- Forced casualness in formal context: "So yeah, our Q3 earnings were solid"
- Inconsistent contractions: "I'm excited" → "It is important" in adjacent sentences
- Slang mixed with corporate jargon
- First person switching to passive voice mid-paragraph

### Why it's slop
AI doesn't have a stable "self" driving its register. It samples from different training distributions within a single text. Humans have a voice; AI has a blend.

### Scoring
- 1-2 register shifts: low
- Persistent instability (shifts every 2-3 sentences): high
- Inappropriate register for genre: medium

### What to do
Pick the register that fits the genre and audience. Stabilize it. If the text is a LinkedIn post from a startup founder, casual-with-substance is the target — not academic, not bro-y.

---

## Category 10: Rhetorical Crutches

Repeated rhetorical devices that the model overuses. These individually aren't damning but in combination create an unmistakable AI fingerprint.

### Watch for
- Starting sentences with "And" or "But" more than twice in a paragraph (overcompensating for perceived formality)
- Em dashes used more than twice per paragraph
- Colons used to introduce every list or elaboration
- Parenthetical asides in every other sentence
- "The truth is" / "The reality is" / "Here's the thing" as paragraph openers
- Ending paragraphs with short punchy fragments: "And it worked." "Full stop." "Period."

### Why it's slop
Each device is fine in isolation. The pattern is the problem. If every paragraph ends with a short punchy fragment, the punchiness disappears. If every idea gets an em dash aside, the asides stop adding anything.

### Scoring
- Any single device used 3+ times: medium
- Multiple devices all overused: high
- Occasional use: not a finding

### What to do
Vary the devices. Use a colon once, then a comma splice next time. Let some paragraphs end mid-thought. The goal is irregular rhythm that feels like a person writing, not a model performing.

---

## Category 11: Contraction Avoidance

AI defaults to formal uncontracted forms ("do not", "it is", "I am", "they are", "we have") far more than humans do in non-academic writing. Research identifies this as one of the strongest single signals of machine origin. Humans contract almost reflexively in anything less formal than a legal brief.

### Watch for
- "do not" where "don't" is natural
- "it is" where "it's" fits
- "I am" where "I'm" works
- "they are" / "we are" / "you are" where contractions are normal
- "cannot" where "can't" fits the register
- "I would" / "I have" / "we will" in casual contexts
- Consistent absence of ANY contractions across an entire text

### Why it's slop
Contraction usage is a strong register signal. In casual or semi-formal writing (which covers LinkedIn, emails, cover letters, blog posts), humans contract by default. AI avoids contractions because its training data includes a disproportionate amount of formal/edited text, and because RLHF tends to reward "proper" grammar. The result: text that sounds like a college admissions essay when it should sound like a person talking.

### Scoring
- Occasional uncontracted form in casual text: low
- Consistent non-contraction across 5+ opportunities in a casual/semi-formal text: medium
- Zero contractions in a 200+ word text that isn't academic or legal: high
- Mixed: some contracted, some not, inconsistently: see Category 9 (Register Mismatch) instead

### What to do
Contract. In almost every genre except academic papers, legal documents, and formal speeches, contractions are the human default. Read the sentence aloud — if you'd contract it when speaking, contract it in writing.

---

## Category 12: Suspicious Vocabulary Diversity

AI uses a wider range of synonyms than humans do. Where a human will say "said" five times in a paragraph, AI will cycle through "stated", "noted", "remarked", "observed", "articulated." Research confirms this: AI produces higher type-token ratios (more unique words per total words) than human writers, who tend to repeat favorite words unevenly.

### Watch for
- Synonym cycling: the same concept described with a different word each time it appears
- Unusually high vocabulary diversity in a short text — every noun and verb is different
- Thesaurus-like substitutions: "important" → "crucial" → "vital" → "essential" → "pivotal" across 5 sentences
- Absence of repeated words that would be natural (humans repeat "good", "thing", "really", "actually", "just")
- Every sentence uses a different transition or connector

### Why it's slop
Humans have a limited active vocabulary. They repeat words they like. They use "thing" when a fancier word would work. They say "good" three times instead of cycling through "excellent, superb, outstanding." This unevenness is a core human fingerprint. AI's relentless synonym rotation is the opposite: it signals a system that has access to the entire dictionary and no preference within it.

### Scoring
- Noticeable synonym cycling on 2+ concepts: medium
- Zero word repetition in a 200+ word text (outside articles and prepositions): medium
- Thesaurus escalation (each synonym fancier than the last): high
- Natural repetition of common words: not a finding (this is actually human)

### What to do
Let words repeat. If you said "important" once and mean the same thing again, say "important" again. Don't reach for "crucial" just because you already used "important." Repetition with purpose is a human trait, not a flaw.

---

## Category 13: Predictability (Low Perplexity)

AI text is statistically "smoother" than human text. Research shows it has significantly lower perplexity (each word is more predictable given the previous context) and lower burstiness (less variance in sentence length). The text reads like a river at constant speed — no rapids, no pools, no surprises.

This is the hardest category to assess qualitatively, but you can develop an intuition for it.

### Watch for
- Every sentence follows subject-verb-object structure with no variation
- Word choices are always the "obvious" next word — no surprises, no unusual collocations
- The text could be completed by autocomplete at almost any point
- No unexpected juxtapositions, metaphors, or word pairings
- Reading the text feels like sliding — nothing snags your attention
- Sentence openings follow predictable patterns (noun, noun, transition + noun, noun...)

### Why it's slop
Human writing surprises. It uses a word you didn't expect. It puts a short sentence after three long ones. It starts a paragraph with "Look." or ends one mid-thought. AI writing optimizes for the most probable next token, which means it converges on the most average, expected phrasing. The result is text that's technically correct but has no texture.

### Scoring
- Moderate predictability in an otherwise voiced text: low
- Persistent autopilot feel across 3+ paragraphs: medium
- Entire text could be autocompleted by a phone keyboard: high

### What to do
Introduce surprise. Use a word that's slightly unexpected but precise. Vary sentence openings. Put the object before the subject occasionally. Break a long clause with a dash. The goal isn't randomness — it's the kind of unpredictability that comes from a specific person making specific choices.

---

## Meta-check: The "Could Anyone Have Written This?" Test

After categorizing findings, ask one final question: **if you removed the author's name, could you tell who wrote this?**

If the answer is "no, this could have been written by anyone," that's the deepest form of slop. It means the text has no voice, no perspective, no specificity. Even if it passes every individual heuristic, it fails the meta-check.

Score +10 for texts that fail this test. Note it in the diagnosis.

---

## Calibration notes

- A well-written human text with one buzzword is not slop. Don't flag isolated instances in otherwise strong writing.
- Some genres (academic abstracts, legal briefs, corporate earnings calls) have legitimate reasons for formality. Adjust expectations via genre presets.
- Intentional repetition (anaphora, callback) is a rhetorical choice, not slop. Check if there's a pattern or just noise.
- Non-native English speakers may produce patterns that look like AI but aren't. If the text has genuine voice despite some formulaic phrasing, note it but don't penalize as hard. In particular: non-native speakers may avoid contractions deliberately, and their vocabulary diversity may be lower (not higher) than native speakers — don't confuse ESL patterns with AI patterns.
- Contraction avoidance is only a signal in genres where contractions are normal. Academic papers, legal text, and formal reports legitimately avoid contractions.
- High vocabulary diversity is only suspicious if it looks like synonym cycling. A writer who genuinely knows many words and uses them precisely is not the same as AI rotating through a thesaurus. Check whether the word choices add nuance or just avoid repetition.
- Emotional flatness is only a finding when the genre demands personality. A quarterly earnings summary is supposed to be neutral. A LinkedIn post about a career change is not.
