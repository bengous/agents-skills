This reference takes the current conversation context, codebase understanding, and local terminology model and produces a local PRD in `prd.md`.

Do NOT interview the user - just synthesize what you already know.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already.

2. Sketch out the major modules you will need to build or modify to complete the implementation. Actively look for opportunities to extract deep modules that can be tested in isolation.

A deep module (as opposed to a shallow module) is one which encapsulates a lot of functionality in a simple, testable interface which rarely changes.

Check with the user that these modules match their expectations. Check with the user which modules they want tests written for.

3. Read `terminology.md`. Use its canonical actor and term names in the PRD. Do not duplicate the whole glossary in `prd.md`; keep `terminology.md` as the language source.

4. Before PRD review, finalize `terminology.md` by replacing placeholders with
   precise entries or the empty-section phrase required by the current scaffold.
   Instruction-only languages use the English empty-section phrase from their
   English scaffold.

5. Write the PRD to `prd.md` in the current ITW root. If `prd.md` already
   exists, preserve its current section headings and fill that scaffold. If no
   scaffold exists, ask the human to rerun `itw get <root>` or recreate the
   scaffold from the current phase language before writing. Use the template
   below only for native English or instruction-only language workflows.

<prd-template>

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each user story should be in the format of:

1. As an <actor>, I want a <feature>, so that <benefit>

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts, so that I can make better informed decisions about my spending
</user-story-example>

This list of user stories should be extremely extensive and cover all aspects of the feature.
Use actor names from `terminology.md`.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being outdated very quickly.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)

## Out of Scope

A description of the things that are out of scope for this PRD.

## Further Notes

Any further notes about the feature.

</prd-template>
