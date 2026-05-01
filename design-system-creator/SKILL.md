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

Create a design system anchored in the real identity of the subject — not in design trends. Every color, font, and component exists for a functional reason. The deliverable is a structured markdown document directly usable by a developer or AI agent for implementation.

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

Before starting, you need to understand the project. Ask the necessary questions to obtain:

1. **The subject** — Who or what? (a brand, a product, a person, a company, a concept)
2. **The concrete activity** — What does it do in the real world? What materials, tools, environments, gestures?
3. **The audience** — Who visits/uses it? (consumers, professionals, both? what level of familiarity?)
4. **The positioning** — High-end, accessible, technical, playful, institutional?
5. **The constraints** — Platform (web, mobile, Shopify, React...), primary content (photos, text, data, video), key features (e-commerce, portfolio, dashboard, blog...)
6. **What the user does NOT like** — Asking for examples of sites/styles they find bad or generic is often more revealing than asking what they like.

Don't start designing until you have these answers. If the user provides everything at once, perfect. Otherwise, ask the missing questions.

If the user only needs a subset (e.g., just colors, or just typography), the 6 steps can be scoped accordingly. But always start with Step 1 (anchoring) — even partial deliverables must be grounded.

## Method — 6 Sequential Steps

### Step 1 — Find the Real Anchoring

This is the most important step. Everything else flows from it.

Ask yourself: what makes this project unique in the physical or conceptual world? Not "what style would look nice" but "if I closed my eyes and thought about this subject, what materials, textures, lights, sounds, temperatures would I perceive?"

Anchoring examples:
- An artisan zinc worker — the metals themselves (grey-blue zinc, orange copper, golden brass), workshop light, patina
- A law firm — vellum paper, black ink, leather bindings, dark wood bookshelves
- A meditation app — stone, water, fog, dawn light
- A SaaS analytics platform — the grid, the electrical signal, the signal/noise contrast, the screen in the dark
- A Japanese restaurant — light wood, ceramics, emptiness (ma), black/white contrast
- A bike brand — asphalt, aluminum, speed, wind

The anchoring must be specific to the subject. If you can swap the project name for another in the same industry and the anchoring still holds, it's too generic. "Elegance and modernity" is not an anchoring. "The grey-blue of zinc that patinas over time" is.

Formulate the anchoring in one sentence: "The design system is anchored in [concrete anchoring]."

### Step 2 — Extract the Palette

From the anchoring, extract real colors. Not colors picked from a trend tool — colors that physically exist in the subject's universe.

**Required palette structure:**

**Foundation (4-5 neutral colors)**:
- A dark (primary background or primary text)
- A surface (cards, elevated blocks)
- A border (separators, subtle outlines)
- A light (alternate background or text on dark background)
- An optional off-white/paper (secondary background)

Rule: no pure white (#FFFFFF) or pure black (#000000) unless the anchoring explicitly justifies it (e.g., a brand whose identity is absolute black/white contrast). Neutrals must have a temperature (warm or cool) consistent with the anchoring.

**Accent families (2-3 families, 3 stops each)**:
Each family has 3 variants: dark (pressed/active), medium (default), light (hover/luminous).
Each family has a unique functional role:
- **Action family** — CTAs, links, buttons, everything interactive. This is the most visible color. There must be only one.
- **Structure family** (optional) — Secondary elements, tertiary backgrounds, inactive icons. More subdued than action.
- **Distinction family** (optional) — Reserved for ONE type of premium/rare element (badge, label, special status). Used sparingly. If it is everywhere, it distinguishes nothing.

**Absolute rule**: every color has a job. If you can't say in one sentence "this color is used for [X]", remove it.

Deliver the palette as CSS tokens:
```
--color-[semantic-name]: #hex;
```

### Step 3 — Choose the Typography

Find a font pair that creates intentional contrast.

**Display font** (headings, taglines):
- Must have character — it should be recognizable
- Serif, slab, display sans-serif, monospace display... anything goes depending on the anchoring
- Elimination criterion: if it's the first font that comes to mind for this type of project, it's probably overused. Dig deeper.

**Body font** (running text, nav, buttons, forms):
- Must fade into the background — it shouldn't be noticed
- Readable at 14-16px, good screen legibility
- Sans-serif in 90% of cases

**Fonts to avoid unless strongly justified**: Inter, Roboto, Arial, system fonts (too generic), Playfair Display, Cormorant Garamond (overused in "artisan/luxury"), Space Grotesk (overused in "tech/startup").

**Define usage rules:**
- Which font for what (display NEVER goes to body, body NEVER goes to headings)
- Allowed weights (limit to 2-3 max)
- Italic usage (if allowed, under what conditions)
- Line-height per usage (tight for headings ~1.1-1.2, airy for body ~1.6-1.7)

Deliver as tokens:
```
--font-display: 'Font Name', category;
--font-body: 'Font Name', category;
```

Plus a complete typographic scale with responsive sizes (use `clamp()`).

### Step 4 — Set the Complete Tokens

Beyond colors and typography, define:

**Spacing** — a scale based on a multiple (4px or 8px):
```
--space-xs to --space-5xl
```

**Border radii** — consistent with the identity:
- Sharp corners = precision, rigor, technical
- Soft rounds = warmth, accessibility
- Pill (100px) = modern, organic
- No inconsistent mixing — pick a direction and commit

**Transitions** — one duration per type:
```
--transition-fast: 0.2s ease;   /* hover, focus */
--transition-medium: 0.4s ease; /* open/close */
--transition-slow: 0.6s ease;   /* scroll reveal */
```

**Breakpoints** — for responsive:
```
mobile / tablet / desktop / wide
```

Deliver a complete, copy-pasteable `:root` block.

### Step 5 — Derive the Components

Every component is justified by the project's identity. Don't copy generic UI patterns.

**Required components to define:**
- Buttons (primary, secondary, text link) — shape, padding, typography, hover/active/focus states
- Cards (if applicable) — background, border, radius, no shadow unless justified
- Navigation — structure, scroll behavior, responsive
- Forms (if applicable) — fields, labels, focus states
- Images — treatment (filters or not), radius, overlay if text on top

**For each component, provide the complete CSS** with interactive states.

**Rule**: every component choice must be justifiable by the anchoring. "Why are the buttons pill-shaped?" — "Because the rounded/angular contrast reflects metalwork that is both fluid and structured." If you can't justify it, it's an arbitrary choice.

### Step 6 — Write the Absolute Rules

This is what transforms a palette into a language. Without rules, the design system disintegrates at the first developer (or AI agent) who implements it.

Write between 8 and 12 rules. Each rule is:
- **Binary** — not "prefer warm tones" but "never use pure white, the lightest white is #F0EBE3"
- **Verifiable** — you can look at any element on the site and say whether the rule is respected or not
- **Justified** — each rule exists for a functional reason, not an aesthetic one

**Typical rule categories:**
- Color + interactivity: "This color = interactive. If it is this color, it is clickable."
- Color + rarity: "This color = reserved for [specific usage]. Nowhere else."
- Extremes: "No pure white / no pure black" or "No medium gray — either light or dark"
- Shadows: "No box-shadow" or "Shadow only on floating elements (modals, dropdowns)"
- Images: "No CSS filters" or "All images in B&W except [exception]"
- Typography: "Serif never goes to body" or "Maximum 2 font weights"
- Animation: "Translate max Xpx" or "No animation on body text"
- Accessibility: "The primary CTA is always visible" or "Minimum AA contrast on all text"

## Deliverable Format

The deliverable is a structured markdown file:

```markdown
# [Project Name] — Design System "[System Name]"

## Philosophy
[The anchoring in 3-4 sentences. Why these choices, where they come from.]

## Colors — Tokens

### Foundation
[Tokens --color-xxx: #hex; + 1 sentence explaining each color's role]

### [Accent family 1 — e.g., Copper]
[3 stops (default/hover/pressed) as tokens + unique functional role]

### [Accent family 2 if applicable — e.g., Zinc]
[3 stops as tokens + unique functional role]

### [Accent family 3 if applicable — e.g., Brass]
[3 stops as tokens + unique functional role — justify why a 3rd family is necessary]

### Semantics
[Error/success/warning/info tokens — may reuse existing families]

### Contrast Table
[Table validating EVERY text/background combination used on the site.
Format:
| Combination | Ratio | Status |
| --color-texte on --color-fond | X.X:1 | AA Pass / Fail |
Minimum: all pairs listed in accessibility-checklist.md section 1.]

### Application Rules
[How to use colors depending on dark/light background context.
Which colors change between the two contexts, which stay the same.]

## Typography

### Fonts
[Names + copy-pasteable @import line:
@import url('https://fonts.googleapis.com/css2?family=...');
Include fallback: --font-display: 'Name', category;]

### Typographic Scale
[Tokens with clamp() for each level:
display, h1, h2, h3, body, small, label
Each level with: font-size, line-height, font-weight, letter-spacing if applicable]

### Application Rules
[Which font for what (display = headings only, body = everything else).
Allowed weights (max 3). Italic: if yes, under what conditions.
Display NEVER goes to body. Body NEVER goes to headings.]

## Spacing
[Complete scale xs -> 5xl with values.
+ Usage summary:
- Micro (xs-sm): inside components
- Small (md): between elements within a block
- Medium (lg-xl): component padding
- Large (2xl-3xl): between sections
- Very large (4xl-5xl): structural breathing room]

## Components

### Buttons
[Complete CSS: primary, secondary, text link.
Each variant with :hover, :active, :focus-visible, :disabled.
Padding, typography, radius justified by the anchoring.]

### Cards
[Complete CSS: background, border, radius, padding, interactive states if applicable.
Variant on dark background AND on light background.]

### Navigation
[Complete CSS: desktop and mobile.
Scroll behavior (sticky, fixed, transparent -> opaque).
Mobile menu: structure, open animation, close on click.]

### Forms
[Complete CSS: field, label, placeholder, focus, error.
If the project has no forms, write "Not applicable" and justify.]

### Images
[Default treatment: radius, recommended aspect-ratio.
Overlay if text on top: gradient direction + opacity.
CSS filters: yes/no, justified by the anchoring.
Rule: photos of real content = never filtered.]

### Iconography
[Single chosen source (Lucide, Phosphor, Tabler, Heroicons...) + justification.
Single style: outline / filled / duotone.
Stroke width if outline.
Sizes on grid: --icon-sm (16px), --icon-md (20px), --icon-lg (24px), --icon-xl (32px).
Color by context: in a button = button color, standalone = muted text, interactive = action.]

## Animations

### Principles
[What gets animated and why. The three jobs: guide, confirm, create continuity.]

### Speed Tokens
[--transition-fast, --transition-medium, --transition-slow with duration + easing]

### Patterns
[Ready CSS for:
- Hover/focus (fast)
- Menu/modal open/close (medium)
- Scroll reveal (slow) — with JS IntersectionObserver
- Page load hero — staggered sequence
Each pattern justified by the anchoring: an artisan site does not move like a SaaS.]

## Responsive

### Breakpoints
[Exact values: mobile < 768px, tablet 768-1023px, desktop 1024-1439px, wide >= 1440px]

### Behavior per Breakpoint
[Summary table for each breakpoint: grid columns, section spacing, nav behavior, image sizing]

### Container
[--container-max, --container-padding (with clamp for mobile)]

## Accessibility

### Validated Contrasts
[Recap of the contrast table from the Colors section — or reference to it.
Confirmation that ALL pairs pass AA.]

### Focus
[CSS for global :focus-visible.
Variants if needed (dark background vs light background).]

### Reduced Motion
[CSS block @media (prefers-reduced-motion: reduce) — mandatory, copy-pasteable.]

### Touch Targets
[Reminder: min 44x44px on all clickable elements. List of at-risk elements in this specific project.]

## Complete CSS Tokens — Copy-Paste
[Single COMPLETE :root block. Everything above condensed into one copy-pasteable block.
Includes: colors, typography, spacing, radius, borders, shadows, transitions, layout, z-index, icons.
No superfluous comments — just tokens with a role comment per line.]

## Theme Switching (if applicable)

If the project requires both light and dark modes, structure tokens using `[data-theme]` attributes:

```css
:root, [data-theme="dark"] {
  /* dark tokens (default) */
}

[data-theme="light"] {
  /* light overrides */
}

@media (prefers-color-scheme: light) {
  :root:not([data-theme]) {
    /* auto-detect for users without explicit preference */
  }
}
```

Define which tokens change between themes and which remain constant. The action color family typically stays the same; foundation colors invert.

## Absolute Rules
[8-12 numbered rules. Each rule is:
- Binary (yes/no, never "prefer")
- Verifiable (any element can be audited)
- Justified (one sentence explaining why)
Example categories: color+interactivity, color+rarity, extremes (white/black),
shadows, images, typography, animation, accessibility.]
```

## Anti-Patterns — What This Skill NEVER Does

- **Start from a design trend** instead of the real subject. No "let's do brutalist" or "let's do glassmorphism" without the anchoring justifying it.
- **Propose colors without a functional role.** Every color has a job or it goes.
- **Use generic fonts** (Inter, Roboto, Arial) unless the project explicitly demands absolute neutrality.
- **Deliver a palette without usage rules.** Colors without rules are a Figma collecting dust.
- **Copy industry codes.** "Lawyer site = navy blue + gold" is a template, not design. The anchoring must be specific to the client, not their industry.
- **Mix temperatures without reason.** If the foundation is warm, cool accents must be justified (and vice versa).
- **Propose more than 3 accent families.** More colors = less coherence. Constraint is a tool.

## Creation Workflow

1. Collect inputs (ask questions if needed)
2. Propose the anchoring — validate it with the user before continuing
3. Propose 2-3 visual directions (moodboards via Visualizer if available) — the user picks one
4. Run through the 6 steps for the chosen direction
5. Deliver the complete markdown document
6. Iterate if the user wants adjustments

The moodboard in step 3 matters: show rather than describe. If a visualization tool is available (HTML widget, Visualizer, etc.), create a visual moodboard with the hero mockup, palette swatches, typography specimen, and guiding principles. The user must see before they validate.

If no visualization tool is available (no `visualize:show_widget` or similar), present the moodboard as:
- An HTML file written to disk that the user can open in their browser
- Or a structured comparison table in markdown showing: palette swatches (hex values), font names + specimen text, key rules, and a hero mockup description

## Specificity Test

Before delivering, run every design choice through this filter: can you replace the project name with a competitor's name and have the design system still work? If yes, the anchoring is too generic. Go back to Step 1.

## Reference Files

The `references/` folder contains support material. **Read the relevant file BEFORE the corresponding step**, not all at once at the start.

| File | When to read | Content |
|---|---|---|
| `references/anchoring-examples.md` | **Step 1** — before formulating the anchoring | 6 complete anchoring examples with thought process, red flags, and validation checklist |
| `references/font-pairings.md` | **Step 3** — before choosing typography | 20 pairs organized by universe (craft, architecture, gastronomy, tech, luxury, sport, wellness) with ready Google Fonts / Fontshare imports |
| `references/root-template.md` | **Step 4** — before setting tokens | Complete CSS `:root` skeleton with all categories (colors, typography, spacing, radius, shadows, transitions, layout, z-index) and completeness checklist |
| `references/moodboard-template.md` | **Step 3** — to create the visual moodboard | Ready HTML template for the Visualizer with variables to replace, showing palette + typography + hero + cards + rules in a single widget |
| `references/spacing-guide.md` | **Step 4** — after the root-template | When to use which spacing token. Mapping micro (xs-sm) -> structural (4xl-5xl) with concrete examples per context |
| `references/component-patterns.md` | **Step 5** — before deriving components | Complete CSS templates for buttons, cards, navigation, forms, images + iconography rules |
| `references/responsive-rules.md` | **Step 5** — in parallel with components | Behavior per breakpoint, grid patterns, calibrated clamp(), responsive navigation, touch targets |
| `references/motion-principles.md` | **Step 5** — after components | 5 motion patterns (hover, open, scroll reveal, page load, smooth scroll) with ready CSS. No tool prescribed. |
| `references/accessibility-checklist.md` | **All steps** — continuous verification | WCAG AA contrast, visible focus, touch targets, reduced motion, accessible typography, HTML semantics |
