---
name: design-system-creator
disable-model-invocation: true
model: opus
effort: medium
description: >
  Create a complete, actionable design system for any project (website, app,
  product) grounded in physical/sensory anchoring rather than design trends.
  Delivers CSS tokens, typography scale, component patterns, accessibility
  checklist, and absolute rules as a single developer-ready markdown document.
  Use when the requested deliverable is a design system, style guide, visual
  identity system, CSS/theme tokens, color and typography system, component
  styling rules, or structured art direction. Do not use for one-off UI styling,
  generic look-and-feel feedback, page implementation, content architecture,
  PRDs, or implementation tickets.
compatibility:
  optional:
    - visualize:show_widget (for visual moodboards; falls back to HTML file or structured markdown)
---

# Design System Creator

Create a reusable design language anchored in the subject's real identity, not in trends. Every color, font, component, and rule must have a functional reason.

## Routing

Use this skill when the final deliverable is a reusable design language:

- Design system or style guide.
- Visual identity system, theme tokens, CSS variables, color/typography rules.
- Component styling rules with usage constraints.
- Structured art direction that must guide later implementation.

Do not use this skill for:

- Building, refactoring, or polishing a specific UI screen.
- Generic "make it look better" feedback without a design-system deliverable.
- Sitemap, screen inventory, navigation, or content architecture.
- PRDs, implementation issues, or product requirements.

## Required Inputs

Collect only missing high-impact inputs, but do not start designing until the essentials are known:

- Subject: brand, product, person, company, or concept.
- Concrete activity: real-world materials, tools, environments, gestures.
- Audience and familiarity level.
- Positioning: high-end, accessible, technical, playful, institutional, etc.
- Constraints: platform, content types, key features, existing brand rules.
- Dislikes: examples of styles the user finds bad or generic.

If essentials are missing, ask. In async contexts where you cannot ask, proceed only with explicit assumptions. If the user only needs a subset such as colors or typography, scope the output accordingly but still start from the anchoring.

## Workflow

1. Find the anchoring: a concrete physical or conceptual source specific to the subject.
2. For full design systems, present the anchoring for validation before continuing.
3. For full design systems, propose 2-3 visual directions or moodboards; the user picks one before tokens/components.
4. Extract the palette: foundation colors plus 1-3 functional accent families.
5. Choose typography: display/body roles, allowed weights, usage rules.
6. Define tokens, derive components, and write 8-12 absolute rules.
7. Produce the design-system document.

For full deliverables, load `references/design-system-template.md` before writing the final document.

## Rules

- Anchor choices in something concrete. If the same anchoring works for a competitor, it is too generic.
- Do not start from trends such as brutalism, glassmorphism, or "modern luxury" unless the anchoring justifies them.
- Every color needs a job. If you cannot state where it is used, remove it.
- Use one action color family. More accent families reduce coherence unless each has a distinct role.
- Do not deliver colors, fonts, or components without usage rules.
- Avoid generic fonts unless the project explicitly demands neutrality.
- Components must follow the same visual logic as the tokens.
- Accessibility is part of the system: contrast, focus, reduced motion, and touch targets are required.

## Output

For scoped requests, return only the requested subset plus the anchoring and usage rules.

For full design systems, include:

- Philosophy and anchoring.
- Color tokens with roles, semantics, contrast, and application rules.
- Typography imports, tokens, scale, and usage rules.
- Spacing, radius, motion, layout, and icon tokens.
- Component rules and CSS for common states.
- Responsive, accessibility, and theme-switching guidance when applicable.
- Complete copy-pasteable `:root` token block.
- 8-12 absolute rules.

For full design systems, a moodboard matters: show before finalizing. Prefer a tool/widget if available. Otherwise produce an HTML file or a structured markdown comparison.

## Reference Files

Read only the files needed for the current step.

| File | When to read | Content |
|---|---|---|
| `references/design-system-template.md` | Full deliverable | Complete markdown structure for a finished design system |
| `references/anchoring-examples.md` | Step 1 | Anchoring examples, red flags, validation checklist |
| `references/font-pairings.md` | Step 3 | Font pairs by universe with import guidance |
| `references/root-template.md` | Step 4 | Complete CSS `:root` skeleton and token checklist |
| `references/moodboard-template.md` | Moodboard needed | HTML moodboard template with palette, typography, hero, cards, rules |
| `references/spacing-guide.md` | Step 4 | Spacing token usage from micro to structural scale |
| `references/component-patterns.md` | Step 5 | CSS templates for buttons, cards, navigation, forms, images, iconography |
| `references/responsive-rules.md` | Step 5 | Breakpoints, grid patterns, calibrated `clamp()`, responsive navigation |
| `references/motion-principles.md` | Step 5 | Motion patterns and timing guidance |
| `references/accessibility-checklist.md` | All steps | WCAG AA contrast, focus, touch targets, reduced motion, semantic HTML |
