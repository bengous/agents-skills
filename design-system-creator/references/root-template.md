# CSS :root Template — Complete Skeleton

Copy this block and fill in each token. Do not delete any category — if a token does not apply, justify it and explicitly remove it in the absolute rules.

The `/* FILL */` comments indicate values to define. The `/* e.g.: ... */` comments give a typical value example.

---

```css
/* ============================================
   [PROJECT NAME] — Design System "[NAME]"
   Generated on [DATE]
   ============================================ */

:root {

  /* ----------------------------------------
     COLORS — Foundation
     ---------------------------------------- */

  /* Main background (most present on the site) */
  --color-fond:            /* FILL */;  /* e.g.: #1C1A17 (dark) or #F5F0E8 (light) */

  /* Elevated surface (cards, modals, raised blocks) */
  --color-surface:         /* FILL */;  /* e.g.: #252320 */

  /* Borders and separators */
  --color-bordure:         /* FILL */;  /* e.g.: #3A3835 */

  /* Main text */
  --color-texte:           /* FILL */;  /* e.g.: #F0EBE3 (on dark bg) or #1C1A17 (on light bg) */

  /* Secondary / muted text */
  --color-texte-mute:      /* FILL */;  /* e.g.: #8A8580 */

  /* Alternating background (sections alternating with main bg) */
  --color-fond-alt:        /* FILL */;  /* e.g.: #F5F0E8 if main bg is dark */

  /* Text on alternating background (if different from main text) */
  --color-texte-alt:       /* FILL */;  /* e.g.: #1C1A17 if fond-alt is light */


  /* ----------------------------------------
     COLORS — Action Family (interactive)
     Role: CTA, links, buttons, focus rings
     ---------------------------------------- */

  --color-action:          /* FILL */;  /* default state */
  --color-action-hover:    /* FILL */;  /* lighter / more luminous */
  --color-action-pressed:  /* FILL */;  /* darker / more saturated */


  /* ----------------------------------------
     COLORS — Structure Family (optional)
     Role: secondary elements, tertiary backgrounds, inactive icons
     ---------------------------------------- */

  --color-structure:       /* FILL or REMOVE if not used */;
  --color-structure-hover: /* FILL */;
  --color-structure-mute:  /* FILL */;


  /* ----------------------------------------
     COLORS — Distinction Family (optional)
     Role: badges, premium labels, special statuses
     Usage: RARE — if it is everywhere, it distinguishes nothing
     ---------------------------------------- */

  --color-distinction:       /* FILL or REMOVE if not used */;
  --color-distinction-hover: /* FILL */;
  --color-distinction-mute:  /* FILL */;


  /* ----------------------------------------
     COLORS — Semantic (feedback)
     ---------------------------------------- */

  --color-erreur:          /* FILL */;  /* e.g.: #D94040 */
  --color-succes:          /* FILL */;  /* e.g.: #3A8A5C */
  --color-warning:         /* FILL */;  /* e.g.: #C4873A */
  --color-info:            /* FILL */;  /* e.g.: same as --color-action or dedicated */


  /* ----------------------------------------
     TYPOGRAPHY — Fonts
     ---------------------------------------- */

  --font-display:          /* FILL */;  /* e.g.: 'Instrument Serif', serif */
  --font-body:             /* FILL */;  /* e.g.: 'DM Sans', sans-serif */


  /* ----------------------------------------
     TYPOGRAPHY — Scale (with responsive clamp)
     ---------------------------------------- */

  /* Display — large titles, hero */
  --text-display:          /* FILL */;  /* e.g.: clamp(2.5rem, 5vw, 4rem) */
  --text-display-lh:       /* FILL */;  /* e.g.: 1.1 */
  --text-display-weight:   /* FILL */;  /* e.g.: 400 */
  --text-display-tracking: /* FILL */;  /* e.g.: -0.02em */

  /* H1 */
  --text-h1:               /* FILL */;  /* e.g.: clamp(2rem, 4vw, 3rem) */
  --text-h1-lh:            /* FILL */;  /* e.g.: 1.15 */
  --text-h1-weight:        /* FILL */;  /* e.g.: 400 */

  /* H2 */
  --text-h2:               /* FILL */;  /* e.g.: clamp(1.5rem, 3vw, 2.25rem) */
  --text-h2-lh:            /* FILL */;  /* e.g.: 1.2 */
  --text-h2-weight:        /* FILL */;  /* e.g.: 400 */

  /* H3 */
  --text-h3:               /* FILL */;  /* e.g.: clamp(1.25rem, 2.5vw, 1.75rem) */
  --text-h3-lh:            /* FILL */;  /* e.g.: 1.25 */
  --text-h3-weight:        /* FILL */;  /* e.g.: 500 */

  /* Body — running text */
  --text-body:             /* FILL */;  /* e.g.: clamp(0.9375rem, 1.5vw, 1.0625rem) */
  --text-body-lh:          /* FILL */;  /* e.g.: 1.65 */
  --text-body-weight:      /* FILL */;  /* e.g.: 400 */

  /* Small — captions, footnotes, metadata */
  --text-small:            /* FILL */;  /* e.g.: clamp(0.8125rem, 1vw, 0.875rem) */
  --text-small-lh:         /* FILL */;  /* e.g.: 1.5 */

  /* Label — buttons, nav, form labels */
  --text-label:            /* FILL */;  /* e.g.: 0.875rem */
  --text-label-lh:         /* FILL */;  /* e.g.: 1.2 */
  --text-label-weight:     /* FILL */;  /* e.g.: 500 */
  --text-label-tracking:   /* FILL */;  /* e.g.: 0.03em */


  /* ----------------------------------------
     SPACING — Scale (base 4px or 8px)
     ---------------------------------------- */

  --space-unit:  /* FILL */;  /* e.g.: 4px or 8px */
  --space-xs:    /* FILL */;  /* e.g.: 4px   (1 unit) */
  --space-sm:    /* FILL */;  /* e.g.: 8px   (2 units) */
  --space-md:    /* FILL */;  /* e.g.: 16px  (4 units) */
  --space-lg:    /* FILL */;  /* e.g.: 24px  (6 units) */
  --space-xl:    /* FILL */;  /* e.g.: 32px  (8 units) */
  --space-2xl:   /* FILL */;  /* e.g.: 48px  (12 units) */
  --space-3xl:   /* FILL */;  /* e.g.: 64px  (16 units) */
  --space-4xl:   /* FILL */;  /* e.g.: 96px  (24 units) */
  --space-5xl:   /* FILL */;  /* e.g.: 128px (32 units) */


  /* ----------------------------------------
     BORDERS
     ---------------------------------------- */

  --radius-sm:     /* FILL */;  /* e.g.: 2px */
  --radius-md:     /* FILL */;  /* e.g.: 4px */
  --radius-lg:     /* FILL */;  /* e.g.: 8px */
  --radius-pill:   /* FILL */;  /* e.g.: 100px — or remove if not used */

  --border-width:  /* FILL */;  /* e.g.: 1px */
  --border-style:  /* FILL */;  /* e.g.: solid */
  --border:        var(--border-width) var(--border-style) var(--color-bordure);


  /* ----------------------------------------
     SHADOWS
     Justify each level. If "no shadow" is a rule, remove this block.
     ---------------------------------------- */

  --shadow-sm:     /* FILL or REMOVE */;  /* e.g.: 0 1px 2px rgba(0,0,0,0.08) */
  --shadow-md:     /* FILL or REMOVE */;  /* e.g.: 0 4px 12px rgba(0,0,0,0.12) */
  --shadow-lg:     /* FILL or REMOVE */;  /* e.g.: 0 8px 24px rgba(0,0,0,0.16) */


  /* ----------------------------------------
     TRANSITIONS
     ---------------------------------------- */

  --transition-fast:    /* FILL */;  /* e.g.: 0.15s ease    — hover, focus */
  --transition-medium:  /* FILL */;  /* e.g.: 0.3s ease     — open, close */
  --transition-slow:    /* FILL */;  /* e.g.: 0.6s ease-out — scroll reveal */


  /* ----------------------------------------
     BREAKPOINTS (for reference — used in @media)
     ---------------------------------------- */

  /* --bp-mobile:   480px;  */
  /* --bp-tablet:   768px;  */
  /* --bp-desktop:  1024px; */
  /* --bp-wide:     1440px; */


  /* ----------------------------------------
     LAYOUT
     ---------------------------------------- */

  --container-max:     /* FILL */;  /* e.g.: 1200px */
  --container-padding: /* FILL */;  /* e.g.: clamp(1rem, 4vw, 2rem) */
  --grid-gap:          /* FILL */;  /* e.g.: var(--space-lg) */


  /* ----------------------------------------
     Z-INDEX (fixed scale)
     ---------------------------------------- */

  --z-base:      1;
  --z-sticky:    100;
  --z-dropdown:  200;
  --z-overlay:   300;
  --z-modal:     400;
  --z-toast:     500;
}


/* ============================================
   BREAKPOINTS — Media queries
   ============================================ */

/* Breakpoints cannot be CSS variables.
   Define them here as reference for media queries. */

/* @media (min-width: 480px)  → mobile landscape / small tablet */
/* @media (min-width: 768px)  → tablet */
/* @media (min-width: 1024px) → desktop */
/* @media (min-width: 1440px) → wide */


/* ============================================
   TYPOGRAPHY — Utility classes (optional)
   ============================================ */

/*
.text-display {
  font-family: var(--font-display);
  font-size: var(--text-display);
  line-height: var(--text-display-lh);
  font-weight: var(--text-display-weight);
  letter-spacing: var(--text-display-tracking);
}

.text-h1 {
  font-family: var(--font-display);
  font-size: var(--text-h1);
  line-height: var(--text-h1-lh);
  font-weight: var(--text-h1-weight);
}

.text-h2 {
  font-family: var(--font-display);
  font-size: var(--text-h2);
  line-height: var(--text-h2-lh);
  font-weight: var(--text-h2-weight);
}

.text-h3 {
  font-family: var(--font-display);
  font-size: var(--text-h3);
  line-height: var(--text-h3-lh);
  font-weight: var(--text-h3-weight);
}

.text-body {
  font-family: var(--font-body);
  font-size: var(--text-body);
  line-height: var(--text-body-lh);
  font-weight: var(--text-body-weight);
}

.text-small {
  font-family: var(--font-body);
  font-size: var(--text-small);
  line-height: var(--text-small-lh);
}

.text-label {
  font-family: var(--font-body);
  font-size: var(--text-label);
  line-height: var(--text-label-lh);
  font-weight: var(--text-label-weight);
  letter-spacing: var(--text-label-tracking);
  text-transform: uppercase;
}
*/
```

---

## Pre-delivery checklist

Verify that the delivered `:root` block contains:

- [ ] **Foundation**: at least fond, surface, bordure, texte, texte-mute (5 minimum)
- [ ] **Action**: 3 stops (default, hover, pressed)
- [ ] **Structure or Distinction**: at least one additional family if the anchoring justifies it
- [ ] **Semantic**: erreur, succes, warning (even if identical to an existing family)
- [ ] **Fonts**: display + body declared
- [ ] **Type scale**: at minimum display, h1, h2, h3, body, small, label — with line-height and weight
- [ ] **Spacing**: 7+ stops from xs to 4xl minimum
- [ ] **Radius**: at least 2 levels
- [ ] **Transitions**: fast, medium, slow
- [ ] **Layout**: container-max, container-padding, grid-gap
- [ ] **Z-index**: complete scale

If a token is intentionally absent, it must be justified in the absolute rules (e.g.: "No box-shadow → no --shadow-* tokens").
