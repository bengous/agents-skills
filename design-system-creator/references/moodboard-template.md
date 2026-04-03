# Moodboard Template — Widget Visualizer

This file contains the HTML template to use with the Visualizer (show_widget) to present a visual direction to the user before building out the full design system.

## When to use

At step 3 of the workflow, after proposing the anchoring and before producing the tokens. The user must **see** the direction, not just read it.

Create 2-3 variants of this moodboard for the user to choose from.

## How to use

1. Copy the HTML template below
2. Replace all values between `{{...}}` with the project's values
3. Call `visualize:show_widget` with the modified HTML
4. Repeat for each proposed direction

## Variables to replace

| Variable | Description | Example |
|---|---|---|
| `{{DIRECTION_NAME}}` | Name of the direction | "Patina" |
| `{{ANCHOR_TEXT}}` | Anchoring phrase | "The metals themselves..." |
| `{{FONT_DISPLAY_IMPORT}}` | Google Fonts / Fontshare URL for the display font | `https://fonts.googleapis.com/css2?family=...` |
| `{{FONT_BODY_IMPORT}}` | Google Fonts / Fontshare URL for the body font | `https://fonts.googleapis.com/css2?family=...` |
| `{{FONT_DISPLAY}}` | CSS name of the display font | `'Instrument Serif', serif` |
| `{{FONT_BODY}}` | CSS name of the body font | `'DM Sans', sans-serif` |
| `{{COLOR_FOND}}` | Main background color | `#1C1A17` |
| `{{COLOR_SURFACE}}` | Surface color | `#252320` |
| `{{COLOR_BORDURE}}` | Border color | `#3A3835` |
| `{{COLOR_TEXTE}}` | Main text color | `#F0EBE3` |
| `{{COLOR_TEXTE_MUTE}}` | Secondary text color | `#8A8580` |
| `{{COLOR_ACTION}}` | Action accent color | `#B87340` |
| `{{COLOR_ACTION_HOVER}}` | Action hover color | `#D4935A` |
| `{{COLOR_STRUCTURE}}` | Structure color (or empty if not used) | `#6B7D80` |
| `{{COLOR_DISTINCTION}}` | Distinction color (or empty if not used) | `#C9A84C` |
| `{{COLOR_FOND_ALT}}` | Alternating background | `#F5F0E8` |
| `{{COLOR_TEXTE_ALT}}` | Text on alternating background | `#1C1A17` |
| `{{HERO_TITLE}}` | Hero mockup title | `Metal artisan since 1987` |
| `{{HERO_SUBTITLE}}` | Subtitle | `Zinc, copper, brass — furniture and decoration` |
| `{{CTA_TEXT}}` | CTA button text | `Discover the workshop` |
| `{{RADIUS}}` | Border-radius of elements | `3px` |
| `{{RULE_1}}` | Guiding rule 1 | `Copper = clickable. Always.` |
| `{{RULE_2}}` | Guiding rule 2 | `No pure white. The lightest is #F0EBE3.` |
| `{{RULE_3}}` | Guiding rule 3 | `Dark background dominant — 70/30.` |

---

## HTML Template

```html
<style>
  @import url('{{FONT_DISPLAY_IMPORT}}');
  @import url('{{FONT_BODY_IMPORT}}');

  * { margin: 0; padding: 0; box-sizing: border-box; }

  .moodboard {
    font-family: {{FONT_BODY}};
    color: var(--text);
    background: transparent;
    max-width: 720px;
  }

  .moodboard {
    --fond: {{COLOR_FOND}};
    --surface: {{COLOR_SURFACE}};
    --bordure: {{COLOR_BORDURE}};
    --text: {{COLOR_TEXTE}};
    --text-mute: {{COLOR_TEXTE_MUTE}};
    --action: {{COLOR_ACTION}};
    --action-hover: {{COLOR_ACTION_HOVER}};
    --structure: {{COLOR_STRUCTURE}};
    --distinction: {{COLOR_DISTINCTION}};
    --fond-alt: {{COLOR_FOND_ALT}};
    --text-alt: {{COLOR_TEXTE_ALT}};
  }

  /* --- Header --- */
  .mb-header {
    padding: 24px 32px;
    background: var(--fond);
    border-radius: 8px 8px 0 0;
    border-bottom: 1px solid var(--bordure);
  }
  .mb-direction-name {
    font-family: {{FONT_BODY}};
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--action);
    margin-bottom: 8px;
  }
  .mb-anchor {
    font-family: {{FONT_DISPLAY}};
    font-size: 20px;
    line-height: 1.35;
    color: var(--text);
  }

  /* --- Palette --- */
  .mb-palette {
    display: flex;
    gap: 0;
    background: var(--fond);
  }
  .mb-swatch-group {
    flex: 1;
    padding: 16px 12px;
  }
  .mb-swatch-group-label {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-mute);
    margin-bottom: 8px;
  }
  .mb-swatches {
    display: flex;
    gap: 4px;
  }
  .mb-swatch {
    flex: 1;
    aspect-ratio: 1;
    border-radius: {{RADIUS}};
    border: 1px solid var(--bordure);
    position: relative;
  }
  .mb-swatch-label {
    position: absolute;
    bottom: -16px;
    left: 0;
    font-size: 8px;
    color: var(--text-mute);
    white-space: nowrap;
  }

  /* --- Typography --- */
  .mb-type-section {
    padding: 32px;
    background: var(--fond);
    border-top: 1px solid var(--bordure);
  }
  .mb-type-display {
    font-family: {{FONT_DISPLAY}};
    font-size: 36px;
    line-height: 1.1;
    color: var(--text);
    margin-bottom: 4px;
  }
  .mb-type-display-meta {
    font-size: 10px;
    color: var(--text-mute);
    margin-bottom: 20px;
  }
  .mb-type-body-sample {
    font-family: {{FONT_BODY}};
    font-size: 15px;
    line-height: 1.65;
    color: var(--text-mute);
    max-width: 520px;
  }
  .mb-type-body-meta {
    font-size: 10px;
    color: var(--text-mute);
    margin-top: 4px;
  }

  /* --- Hero mockup --- */
  .mb-hero {
    background: var(--fond);
    padding: 48px 32px;
    border-top: 1px solid var(--bordure);
    text-align: center;
  }
  .mb-hero-title {
    font-family: {{FONT_DISPLAY}};
    font-size: 32px;
    line-height: 1.15;
    color: var(--text);
    margin-bottom: 12px;
  }
  .mb-hero-subtitle {
    font-family: {{FONT_BODY}};
    font-size: 14px;
    color: var(--text-mute);
    margin-bottom: 24px;
  }
  .mb-hero-cta {
    display: inline-block;
    font-family: {{FONT_BODY}};
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding: 12px 28px;
    background: var(--action);
    color: var(--fond);
    border-radius: {{RADIUS}};
    text-decoration: none;
    transition: background 0.2s ease;
  }
  .mb-hero-cta:hover {
    background: var(--action-hover);
  }

  /* --- Light section mockup --- */
  .mb-light-section {
    background: var(--fond-alt);
    padding: 32px;
    border-top: 1px solid var(--bordure);
  }
  .mb-light-title {
    font-family: {{FONT_DISPLAY}};
    font-size: 22px;
    color: var(--text-alt);
    margin-bottom: 8px;
  }
  .mb-light-body {
    font-family: {{FONT_BODY}};
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-alt);
    opacity: 0.7;
    max-width: 480px;
  }

  /* --- Card mockup --- */
  .mb-cards {
    display: flex;
    gap: 12px;
    padding: 24px 32px;
    background: var(--fond);
    border-top: 1px solid var(--bordure);
  }
  .mb-card {
    flex: 1;
    background: var(--surface);
    border: 1px solid var(--bordure);
    border-radius: {{RADIUS}};
    padding: 20px 16px;
  }
  .mb-card-title {
    font-family: {{FONT_DISPLAY}};
    font-size: 16px;
    color: var(--text);
    margin-bottom: 6px;
  }
  .mb-card-body {
    font-family: {{FONT_BODY}};
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-mute);
    margin-bottom: 12px;
  }
  .mb-card-link {
    font-family: {{FONT_BODY}};
    font-size: 12px;
    font-weight: 500;
    color: var(--action);
    text-decoration: none;
  }

  /* --- Rules --- */
  .mb-rules {
    padding: 24px 32px;
    background: var(--surface);
    border-radius: 0 0 8px 8px;
    border-top: 1px solid var(--bordure);
  }
  .mb-rules-title {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--action);
    margin-bottom: 12px;
  }
  .mb-rule {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-mute);
    padding: 4px 0;
    border-bottom: 1px solid var(--bordure);
  }
  .mb-rule:last-child {
    border-bottom: none;
  }
  .mb-rule strong {
    color: var(--text);
  }
</style>

<div class="moodboard">
  <!-- Header -->
  <div class="mb-header">
    <div class="mb-direction-name">Direction — {{DIRECTION_NAME}}</div>
    <div class="mb-anchor">{{ANCHOR_TEXT}}</div>
  </div>

  <!-- Palette -->
  <div class="mb-palette">
    <div class="mb-swatch-group">
      <div class="mb-swatch-group-label">Foundation</div>
      <div class="mb-swatches">
        <div class="mb-swatch" style="background:var(--fond)"></div>
        <div class="mb-swatch" style="background:var(--surface)"></div>
        <div class="mb-swatch" style="background:var(--bordure)"></div>
        <div class="mb-swatch" style="background:var(--text)"></div>
        <div class="mb-swatch" style="background:var(--fond-alt)"></div>
      </div>
    </div>
    <div class="mb-swatch-group">
      <div class="mb-swatch-group-label">Action</div>
      <div class="mb-swatches">
        <div class="mb-swatch" style="background:var(--action)"></div>
        <div class="mb-swatch" style="background:var(--action-hover)"></div>
      </div>
    </div>
    <div class="mb-swatch-group">
      <div class="mb-swatch-group-label">Structure</div>
      <div class="mb-swatches">
        <div class="mb-swatch" style="background:var(--structure)"></div>
      </div>
    </div>
    <div class="mb-swatch-group">
      <div class="mb-swatch-group-label">Distinction</div>
      <div class="mb-swatches">
        <div class="mb-swatch" style="background:var(--distinction)"></div>
      </div>
    </div>
  </div>

  <!-- Typography -->
  <div class="mb-type-section">
    <div class="mb-type-display">Aa — Display Specimen</div>
    <div class="mb-type-display-meta">{{FONT_DISPLAY}} — titles, taglines, hero</div>
    <div class="mb-type-body-sample">
      Here is a sample of running text. The body font should fade into the background — you do not notice it, you read through it. Good readability at 15px, generous spacing, nothing that catches the eye.
    </div>
    <div class="mb-type-body-meta">{{FONT_BODY}} — body, nav, buttons, forms</div>
  </div>

  <!-- Hero mockup -->
  <div class="mb-hero">
    <div class="mb-hero-title">{{HERO_TITLE}}</div>
    <div class="mb-hero-subtitle">{{HERO_SUBTITLE}}</div>
    <a class="mb-hero-cta" href="#">{{CTA_TEXT}}</a>
  </div>

  <!-- Light section -->
  <div class="mb-light-section">
    <div class="mb-light-title">Section on light background</div>
    <div class="mb-light-body">Dark/light background alternation for reading rhythm. Text and accents adapt to the context.</div>
  </div>

  <!-- Cards -->
  <div class="mb-cards">
    <div class="mb-card">
      <div class="mb-card-title">Example card</div>
      <div class="mb-card-body">Elevated surface with subtle border. The content breathes.</div>
      <a class="mb-card-link" href="#">Learn more →</a>
    </div>
    <div class="mb-card">
      <div class="mb-card-title">Example card</div>
      <div class="mb-card-body">Same structure, same rhythm. Coherence lives in the tokens.</div>
      <a class="mb-card-link" href="#">Learn more →</a>
    </div>
  </div>

  <!-- Rules -->
  <div class="mb-rules">
    <div class="mb-rules-title">Guiding rules</div>
    <div class="mb-rule"><strong>01</strong> — {{RULE_1}}</div>
    <div class="mb-rule"><strong>02</strong> — {{RULE_2}}</div>
    <div class="mb-rule"><strong>03</strong> — {{RULE_3}}</div>
  </div>
</div>
```

---

## Usage notes

**Adapt the template, do not follow it blindly.** If the anchoring calls for a light-dominant design (e.g.: Japanese restaurant), invert the dark/light sections. If there is no structure or distinction family, remove the corresponding swatches.

**Font @imports in the widget**: the Visualizer supports Google Fonts and Fontshare imports. Verify that the URLs are correct before rendering.

**Recommended size**: the template is designed for ~720px wide. The Visualizer adapts automatically.

**For multiple directions**: call show_widget once per direction with a different title (e.g.: `moodboard_direction_patina`, `moodboard_direction_raw`). The user sees both side by side in the conversation.
