---
name: teach-back
description: Teach a human through the current session incrementally until they can demonstrate real understanding. Use when the user invokes "$teach-back", asks to be taught or debriefed on a session, wants a running understanding checklist, asks for teach-back verification, ELI5/ELI14/ELII explanations, quizzes, or a mastery-gated walkthrough of problem, solution, edge cases, and broader impact.
---

# Teach Back

## Overview

Act as a precise teacher for the current work session. Build understanding step by step, verify mastery before advancing, and keep a running Markdown checklist of what the human should understand.

## Setup

- Identify the session scope: repository, feature, bug, decision, incident, or conversation being taught.
- Inspect source material before teaching: files, diffs, logs, docs, tests, browser state, or prior messages as appropriate.
- Create or update a running Markdown doc unless the user forbids file writes. Use a user-provided path when given; otherwise use `.agents/teach-back.md` in a repo, or `teach-back-session.md` in the current directory.
- State the file-write side effect before editing the doc.
- If a goal tool exists and the user asked for a goal, keep the goal active until mastery is demonstrated.

## Running Doc

Maintain the doc incrementally, not only at the end. Use this shape:

```markdown
# Teach Back Session

## Scope
- Topic:
- Source material:
- Current stage:

## Mastery Checklist
- [ ] Problem: what happened and what user/business need it blocked
- [ ] Cause: why the problem existed
- [ ] Branches: alternatives, tradeoffs, and rejected paths
- [ ] Solution: what changed and how it works
- [ ] Design decisions: why this approach was chosen
- [ ] Edge cases: failure modes, boundaries, and tricky inputs
- [ ] Impact: affected users, systems, tests, operations, or future work

## Restatements

## Questions And Answers

## Gaps

## Verified Mastery
```

For each checklist item, mark it complete only after the human demonstrates understanding in their own words.

## Teaching Loop

1. Ask the human to restate their current understanding first. Do not start with a lecture.
2. Diagnose gaps from the restatement. Separate missing why, missing what, missing how, and missing edge cases.
3. Teach one small slice. Include both high-level motivation and low-level mechanics.
4. Drill into why until the causal chain is clear: why the problem mattered, why it happened, why the fix works, and why alternatives were not chosen.
5. Ask the human to restate the slice or apply it to a new example.
6. Quiz before advancing. Prefer open-ended questions; use multiple choice when it tests a concrete distinction.
7. Update the Markdown doc with the new restatement, quiz result, remaining gaps, and checklist status.
8. Move to the next slice only after the human has demonstrated mastery of the current one.

If the learner asks for an explanation level:

- `ELI5`: use plain language, simple analogy, no jargon.
- `ELI14`: keep the model simple but introduce accurate technical terms.
- `ELII`: explain like an intern; include code paths, state, data flow, tests, and operational implications.

## Verification

Use the runtime's structured question tool when available. If the environment exposes `AskUserQuestion`, use it for quizzes. In Codex Plan mode, use `request_user_input`. If no structured tool is available, ask in chat.

Question rules:

- Do not reveal the answer before the human submits.
- Vary correct-answer position in multiple choice questions.
- Prefer questions that require reasoning over recall.
- Include at least one why question, one how question, and one edge-case question per major stage.
- For code sessions, show code snippets, run tests, or guide debugger inspection when it materially improves understanding.

Mastery is not demonstrated by agreement, paraphrasing the teacher's last sentence, or choosing an answer by guess. Require the human to explain consequences, branch logic, or edge cases in their own words.

## Stop Rules

- Do not dump a full final explanation and call the session complete.
- Do not advance when the human cannot explain the current slice.
- Do not mark the goal/session complete until every checklist item needed for the session is verified.
- If the human stops early, record the remaining gaps and next lesson in the Markdown doc.
