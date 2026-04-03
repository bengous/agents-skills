# Voice Preservation Strategy

The hardest part of rewriting isn't fixing slop — it's keeping the writer's voice intact while you do it. This file explains how.

## What is voice?

Voice is NOT vocabulary. Swapping "utilize" for "use" doesn't change voice. Voice is the combination of:

- **Rhythm**: Sentence length variation. Short/long patterns. Where the writer breathes.
- **Emphasis**: What gets its own sentence. What gets buried in a clause. What gets cut entirely.
- **Stance**: How the writer positions themselves. Confident? Self-deprecating? Observational? Argumentative?
- **Specificity patterns**: Does the writer use numbers? Anecdotes? Analogies? Abstract principles?
- **What they skip**: What a writer chooses NOT to say is as diagnostic as what they do say. Some writers never explain their credentials. Some never use examples. Some never hedge.
- **Punctuation habits**: Em dashes vs parentheses. Semicolons vs periods. Exclamation points or never.

## Using a voice profile

If `voice-profile.md` exists in this skill folder, it contains a description of the target voice. Use it as a comparison:

1. Before rewriting, identify which voice traits from the profile are present in the original (even if buried under slop).
2. Preserve those traits in your rewrite.
3. If the original conflicts with the profile (e.g., the profile says "never uses exclamation points" but the original has three), follow the profile.
4. If no profile exists, infer voice from whatever non-slop portions of the text you can find.

## Creating a voice profile

When the user asks to set up their voice, or when you encounter enough of their writing to characterize it, create `voice-profile.md` in this skill folder. Structure:

```markdown
# Voice Profile

## Rhythm
[Description of typical sentence length patterns, paragraph length, pacing]

## Stance
[How the writer positions themselves — confident, understated, direct, etc.]

## Signature moves
[2-3 specific habits: e.g., "leads with the conclusion", "uses one-word sentences for emphasis", "never explains acronyms"]

## Avoids
[Things the writer clearly doesn't do: e.g., "never uses emoji", "avoids first person in professional writing"]

## Register
[Typical formality level and when it shifts]

## Sample sentences
[3-5 real sentences from the user's writing that exemplify their voice]
```

## Rewriting without a profile

When no voice profile exists and you're rewriting:

1. **Identify the strongest sentence** in the original. The one that sounds most like a specific person wrote it. Use that as your calibration point.
2. **Match that sentence's energy** across the rewrite. Not its structure — its energy. If the best sentence is blunt and short, the rewrite should be blunt and short.
3. **When in doubt, cut rather than rephrase.** Removing slop reveals voice. Adding words risks burying it further.
4. **Preserve weird choices.** If the writer used an unusual word or structure, it's probably intentional. Keep it unless it's clearly AI-generated.

## Anti-patterns in voice preservation

- **Flattening to "professional."** The writer might be casual, blunt, or irreverent. Don't sand those edges off.
- **Adding your own voice.** Your rewrite should sound like THEM, not like you. If you don't have enough signal on their voice, stay close to the original structure and just remove slop.
- **Consistency theater.** Real human writing isn't perfectly consistent. Don't force every sentence into the same register if the original had natural variation.
- **Thesaurus swaps.** Don't replace the writer's vocabulary with "better" words. Their word choices are their voice.
