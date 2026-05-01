# Design System Deliverable Template

Use this structure for a complete design-system deliverable.

````markdown
# [Project Name] -- Design System "[System Name]"

## Philosophy
[Anchoring in 3-4 sentences. Explain why these choices exist and where they come from.]

## Colors -- Tokens

### Foundation
[Tokens `--color-xxx: #hex;` plus one sentence explaining each color's role.]

### [Accent Family 1]
[3 stops: default, hover, pressed. State the unique functional role.]

### [Accent Family 2 If Applicable]
[3 stops plus unique functional role.]

### [Accent Family 3 If Applicable]
[3 stops plus justification for why a third family is necessary.]

### Semantics
[Error, success, warning, info tokens. Reuse existing families when appropriate.]

### Contrast Table

| Combination | Ratio | Status |
|-------------|-------|--------|
| --color-text on --color-bg | X.X:1 | AA Pass / Fail |

Validate every text/background combination used by the system.

### Application Rules
[How colors behave on dark/light backgrounds. Which colors change between contexts, which stay constant.]

## Typography

### Fonts
[Names, copy-pasteable import, and fallbacks.]

```css
@import url('https://fonts.googleapis.com/css2?family=...');

:root {
  --font-display: 'Name', category;
  --font-body: 'Name', category;
}
```

### Typographic Scale
[Tokens with `clamp()` for display, h1, h2, h3, body, small, label. Include size, line-height, weight, and letter-spacing if applicable.]

### Application Rules
[Which font is used where. Display never goes to body. Body never goes to headings. Max 2-3 weights.]

## Spacing
[Complete scale xs -> 5xl plus usage summary: micro, element, component, section, structural.]

## Components

### Buttons
[Complete CSS for primary, secondary, text link; hover, active, focus-visible, disabled.]

### Cards
[Background, border, radius, padding, and interactive states if applicable. Include dark/light usage when needed.]

### Navigation
[Desktop and mobile structure, scroll behavior, menu behavior.]

### Forms
[Field, label, placeholder, focus, error. If not applicable, say so.]

### Images
[Default treatment, aspect ratio, overlay rules, filter rules.]

### Iconography
[Chosen source, style, stroke/fill, sizes, color rules.]

## Animations

### Principles
[What gets animated and why: guide, confirm, create continuity.]

### Speed Tokens
[Fast, medium, slow duration and easing.]

### Patterns
[Hover/focus, menu/modal, scroll reveal, page load when applicable.]

## Responsive

### Breakpoints
[Mobile, tablet, desktop, wide values.]

### Behavior Per Breakpoint
[Grid columns, section spacing, nav behavior, image sizing.]

### Container
[Max width and responsive padding tokens.]

## Accessibility

### Validated Contrasts
[Recap or link to the contrast table. Confirm all pairs pass AA.]

### Focus
[Global focus-visible CSS and any dark/light variants.]

### Reduced Motion
[Mandatory reduced-motion CSS.]

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Touch Targets
[Confirm min 44x44px for interactive elements and list project-specific risks.]

## Complete CSS Tokens -- Copy-Paste
[Single complete `:root` block covering colors, typography, spacing, radius, borders, shadows, transitions, layout, z-index, icons.]

## Theme Switching If Applicable

```css
:root,
[data-theme="dark"] {
  /* dark tokens */
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

Define which tokens change between themes and which remain constant. The action color family usually stays constant; foundation colors invert.

## Absolute Rules
[8-12 numbered rules. Each must be binary, verifiable, and justified.]
````
