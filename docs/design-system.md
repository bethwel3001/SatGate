# Design System

## Fonts

Use Comfortaa variants for brand and prominent display text. Use Calibri or a metrically compatible fallback for body text.

Recommended CSS stack:

```css
:root {
  --font-brand: "Comfortaa", "Calibri", system-ui, sans-serif;
  --font-body: "Calibri", "Segoe UI", system-ui, sans-serif;
}
```

Apply `--font-brand` to:

- logo text
- major page headings
- primary metric labels

Apply `--font-body` to:

- forms
- tables
- dashboard content
- helper text

## Colors

Use a focused palette:

- black: primary text and high-contrast surfaces
- white: main background and inverse text
- blue: primary actions and active states
- red: destructive errors
- yellow: warnings and invoice pending states
- green: success and paid states

Suggested tokens:

```css
:root {
  --color-black: #0b0f14;
  --color-white: #ffffff;
  --color-blue: #1d4ed8;
  --color-red: #dc2626;
  --color-yellow: #eab308;
  --color-green: #16a34a;
}
```

## UI Rules

- Responsive design is required for the dashboard, widget, and demo page.
- Buttons should use clear text or icon plus text where needed.
- Error states should use red for failure and yellow for warning or pending payment.
- Success states should use green.
- Do not hardcode emojis in source code.
- Avoid decorative comments and explain only non-obvious implementation logic.
- Keep the widget compact because it will live inside other websites.
- Keep dashboard layouts dense, readable, and operational rather than marketing-heavy.

## Accessibility

- Ensure color states also include text labels.
- Use semantic form labels.
- Preserve keyboard navigation.
- Keep payment fallback available for users without WebLN.
