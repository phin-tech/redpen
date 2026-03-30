# Theme & Color System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current border-heavy dark theme with a surface-elevation system and tiered accent hierarchy, establishing the visual foundation for all subsequent UI overhaul work.

**Architecture:** Update CSS custom properties in `app.css` to shift surface colors to GitHub-dark-inspired values, reduce border reliance in favor of background-color differentiation, and establish a clear accent tier (solid fill → underline → outlined → subtle). All changes are token-level — component-specific border removal happens in later plans.

**Tech Stack:** CSS custom properties, Tailwind CSS theme integration

---

### Task 1: Update Surface Tokens

**Files:**
- Modify: `src/app.css:4-11` (surface variables)
- Modify: `src/app.css:98-105` (Tailwind theme integration)

- [ ] **Step 1: Update surface color values**

In `src/app.css`, update the surface section:

```css
/* Surfaces */
--surface-base: #0D1117;
--surface-panel: #161B22;
--surface-editor: #0D1117;
--surface-elevated: #1C2129;
--surface-raised: #21262D;
--surface-highlight: #2A3039;
--surface-selection: #231B1A;
```

Key changes:
- `--surface-base`: `#0C0D10` → `#0D1117` (GitHub dark base)
- `--surface-panel`: `#121419` → `#161B22` (elevated surface for sidebars, cards, collapsed bubbles)
- `--surface-editor`: `#181B21` → `#0D1117` (editor matches base for maximum code contrast)
- New `--surface-elevated`: `#1C2129` (for popovers, modals, dropdowns — one step above panel)
- `--surface-raised`: `#20242C` → `#21262D` (slight adjustment to match GitHub dark scale)

- [ ] **Step 2: Add surface-elevated to Tailwind theme**

In the `@theme` block, add the new token:

```css
--color-surface-elevated: var(--surface-elevated);
```

- [ ] **Step 3: Build and verify no visual regressions**

Run: `npm run build`
Expected: Build succeeds with no errors. Colors shift darker/more GitHub-like across the app.

- [ ] **Step 4: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): update surface tokens to GitHub-dark elevation scale"
```

---

### Task 2: Soften Borders

**Files:**
- Modify: `src/app.css:18-21` (border variables)

- [ ] **Step 1: Update border tokens to be more subtle**

```css
/* Borders */
--border-default: rgba(255, 255, 255, 0.06);
--border-subtle: rgba(255, 255, 255, 0.03);
--border-emphasis: rgba(255, 255, 255, 0.12);
```

Key changes:
- `--border-default`: from opaque `#2A3038` to translucent `rgba(255, 255, 255, 0.06)` — much subtler, lets surface elevation do the work
- `--border-subtle`: from `0.05` to `0.03` — nearly invisible, just a hint
- `--border-emphasis`: from opaque `#3B4350` to translucent `rgba(255, 255, 255, 0.12)` — visible but not harsh

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Borders across the app become much subtler — panels, cards, and dividers now rely more on background-color contrast than border contrast.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): soften borders to support surface-elevation model"
```

---

### Task 3: Update Text Opacity Scale

**Files:**
- Modify: `src/app.css:13-16` (text variables)

- [ ] **Step 1: Adjust text tokens for ghost UI hierarchy**

```css
/* Text */
--text-primary: rgba(255, 255, 255, 0.9);
--text-secondary: rgba(255, 255, 255, 0.6);
--text-muted: rgba(255, 255, 255, 0.3);
--text-ghost: rgba(255, 255, 255, 0.15);
```

Key changes:
- `--text-primary`: from fixed `#F3F4F6` to `rgba(255, 255, 255, 0.9)` — still bright but blends better with surfaces
- `--text-secondary`: from `#C1C7D0` to `rgba(255, 255, 255, 0.6)` — readable but clearly secondary
- `--text-muted`: from `#8B93A1` to `rgba(255, 255, 255, 0.3)` — ghostly for timestamps, metadata
- New `--text-ghost`: `rgba(255, 255, 255, 0.15)` — for line numbers, dividers, ultra-dim elements

- [ ] **Step 2: Add text-ghost to Tailwind theme**

In the `@theme` block, add:

```css
--color-text-ghost: var(--text-ghost);
```

- [ ] **Step 3: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Text across the app shifts to a more consistent opacity-based hierarchy. Metadata and timestamps become dimmer, primary content stays readable.

- [ ] **Step 4: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): switch text tokens to opacity-based ghost UI scale"
```

---

### Task 4: Establish Accent Tiers

**Files:**
- Modify: `src/app.css:23-31` (accent variables)

- [ ] **Step 1: Add tiered accent tokens**

Replace the accent section:

```css
/* Accent — tiered usage:
   solid fill → --accent (Submit button ONLY)
   active indicator → --accent-active (2px underline)
   badge/outlined → --accent-badge-* (text + border, transparent bg)
   subtle → --accent-subtle (collapsed bubble left-border, highlights) */
--accent: #D9B15F;
--accent-hover: #E4C06E;
--accent-subtle: rgba(217, 177, 95, 0.12);
--accent-active: #D9B15F;
--accent-badge-text: #D9B15F;
--accent-badge-border: rgba(217, 177, 95, 0.3);
--accent-annotation-border: rgba(217, 177, 95, 0.4);
--view-active: rgba(255, 255, 255, 0.9);
--view-active-hover: rgba(255, 255, 255, 1);
--view-active-subtle: transparent;
--view-active-border: transparent;
```

Key changes:
- `--accent` stays amber `#D9B15F` but is now documented as "solid fill for primary action only"
- `--accent-hover` adjusted from salmon `#FFAF9B` to lighter amber `#E4C06E`
- New `--accent-active`: same amber, used for 2px tab underlines
- New `--accent-badge-*`: for outlined badge counts
- `--accent-annotation-border` toned down from 0.58 to 0.4 opacity
- `--view-active`: changed from blue `#72AEE6` to white `0.9` — active tab text is just brighter, not a different hue. The amber underline provides the accent.
- `--view-active-subtle`/`--view-active-border`: cleared to transparent — no more blue fill on active tabs

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Active tabs may look different (white text instead of blue fill). This is intentional — the amber underline will be added in the Header Consolidation plan.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): establish tiered accent system — solid, underline, outlined, subtle"
```

---

### Task 5: Clean Up Deprecated Tokens

**Files:**
- Modify: `src/app.css:87-89` (gradient variables)

- [ ] **Step 1: Remove gradient token comment and values**

Replace:
```css
/* Gradients (disabled – flat panels) */
--gradient-panel: none;
--gradient-toolbar: none;
```

With:
```css
/* Gradients — removed, surfaces use flat colors */
```

The `none` values were left as compatibility shims from the previous gradient removal. Any remaining references to `var(--gradient-panel)` or `var(--gradient-toolbar)` should have been cleaned up in the prior commit. If the build fails, search for remaining references and replace with the appropriate surface color.

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds. If any component still references `--gradient-panel` or `--gradient-toolbar`, it will show as `unset` which is fine (transparent).

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "chore(theme): remove deprecated gradient token shims"
```

---

### Task 6: Update Shadow Tokens

**Files:**
- Modify: `src/app.css:79-85` (shadow variables)

- [ ] **Step 1: Adjust shadows for darker base**

With the darker base color, shadows need slightly more opacity to register:

```css
/* Elevation */
--shadow-xs: 0 1px 3px rgba(0, 0, 0, 0.3);
--shadow-card: 0 2px 4px rgba(0, 0, 0, 0.4), 0 4px 8px rgba(0, 0, 0, 0.2);
--shadow-card-hover: 0 4px 8px rgba(0, 0, 0, 0.5), 0 8px 16px rgba(0, 0, 0, 0.25);
--shadow-panel: 0 2px 6px rgba(0, 0, 0, 0.5), 0 8px 16px rgba(0, 0, 0, 0.3);
--shadow-popover: 0 8px 24px rgba(0, 0, 0, 0.6), 0 16px 48px rgba(0, 0, 0, 0.4);
--shadow-inset: inset 0 1px 2px rgba(0, 0, 0, 0.3);
```

Shadows are now slightly stronger to compensate for the darker base, and spread values are increased for a softer, more premium feel.

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Elevated elements (popovers, cards on hover) have a more pronounced but softer shadow.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): strengthen shadows for darker base surface"
```

---

### Task 7: Update Scrollbar and Body Styles

**Files:**
- Modify: `src/app.css:156-172` (scrollbar styles)
- Modify: `src/app.css:137-147` (body styles)

- [ ] **Step 1: Update scrollbar to match new tokens**

```css
::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.08);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.15);
}
```

The scrollbar thumb was using `--border-default` (now very subtle). Switch to explicit rgba values that are slightly more visible than borders but still unobtrusive.

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Scrollbars are subtle but visible when scrolling.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): update scrollbar styling for new surface palette"
```

---

### Task 8: Visual Smoke Test

This is a manual verification step — no code changes.

- [ ] **Step 1: Run the app and check all major surfaces**

Run: `cargo tauri dev`

Check these surfaces against the new theme:
- App background (should be `#0D1117`)
- Left sidebar / file tree (should be `#161B22`)
- Editor area (should match base `#0D1117`)
- Right sidebar / annotations (should be `#161B22`)
- Annotation cards (should be on `#161B22` surface)
- Popovers / command palette (should be `#1C2129`)
- Text hierarchy: primary bright, secondary readable, muted ghostly, timestamps nearly invisible
- Borders: barely visible, surfaces do the separation work
- Scrollbars: subtle but findable

- [ ] **Step 2: Check for any components that look broken**

Look for:
- Text that's now invisible (too low opacity against the new background)
- Borders that disappeared entirely where they're still needed (focus rings, input fields)
- Cards or panels that lost all visual separation

If issues are found, fix them by adjusting the specific component's styles — do NOT change the global tokens.

- [ ] **Step 3: Commit any component-level fixes**

```bash
git add -A
git commit -m "fix(theme): adjust component styles for new surface palette"
```
