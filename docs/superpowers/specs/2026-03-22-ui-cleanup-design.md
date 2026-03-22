# UI Consistency Cleanup — Design Spec

## Goal

Full-pass UI cleanup: consolidate the styling system for future theme support, raise visual consistency across all components, replace Flowbite with Bits UI + lightweight custom atoms.

## 1. Token Architecture

### Problem

Two parallel token systems exist — legacy `:root` CSS variables (`--bg-panel`, `--accent`, etc.) and Tailwind `@theme` tokens (`graphite-*`, `coral-*`, `surface-*`). Components mix both, making theming impossible and maintenance confusing.

### Solution

Single semantic CSS variable layer in `:root`, consumed by Tailwind `@theme`. To add a theme, swap the `:root` values — Tailwind classes follow automatically.

### Semantic Variables

**Surfaces:**
| Variable | Value (dark) | Usage |
|----------|-------------|-------|
| `--surface-base` | `#0E0F11` | App background |
| `--surface-panel` | `#131417` | Side panels |
| `--surface-editor` | `#1A1C21` | Editor area |
| `--surface-raised` | `#22252B` | Cards, secondary buttons |
| `--surface-highlight` | `#2A2D35` | Hover states |
| `--surface-selection` | `#2A1E1E` | Selected items |

**Text:**
| Variable | Value (dark) | Usage |
|----------|-------------|-------|
| `--text-primary` | `#F0F1F3` | Main content |
| `--text-secondary` | `#B3B7BF` | Labels, descriptions |
| `--text-muted` | `#6B7280` | Placeholders, disabled |

**Borders:**
| Variable | Value (dark) | Usage |
|----------|-------------|-------|
| `--border-default` | `#2B2E36` | Dividers |
| `--border-subtle` | `rgba(255,255,255,0.03)` | Faint inset borders |
| `--border-emphasis` | `#3B3F4A` | Stronger dividers |

**Accent:**
| Variable | Value (dark) | Usage |
|----------|-------------|-------|
| `--accent` | `#F47A63` | Primary action color |
| `--accent-hover` | `#FF9E87` | Hover state |
| `--accent-subtle` | `rgba(244,122,99,0.12)` | Tinted backgrounds |
| `--accent-annotation-border` | `rgba(244,122,99,0.45)` | CodeMirror annotation underline |

**Status:**
| Variable | Value | Usage |
|----------|-------|-------|
| `--color-success` | `#74B97A` | Success states |
| `--color-warning` | `#E3B341` | Warnings |
| `--color-danger` | `#D96B5F` | Errors, destructive actions |
| `--accent-purple` | `#B07ACC` | Git renamed |
| `--accent-teal` | `#6FA39B` | Git untracked |
| `--accent-blue` | `#5B9BD5` | Informational |

**Elevation:**
| Variable | Value | Usage |
|----------|-------|-------|
| `--shadow-xs` | `0 1px 2px rgba(0,0,0,0.2)` | Subtle depth |
| `--shadow-card` | `0 1px 2px rgba(0,0,0,0.3), 0 2px 6px rgba(0,0,0,0.15)` | Cards |
| `--shadow-card-hover` | `0 2px 4px rgba(0,0,0,0.35), 0 4px 12px rgba(0,0,0,0.2)` | Card hover |
| `--shadow-panel` | `0 1px 3px rgba(0,0,0,0.4), 0 4px 12px rgba(0,0,0,0.25)` | Side panels |
| `--shadow-popover` | `0 4px 16px rgba(0,0,0,0.5), 0 8px 32px rgba(0,0,0,0.3)` | Dialogs, menus |
| `--shadow-inset` | `inset 0 1px 2px rgba(0,0,0,0.25)` | Input fields |
| `--gradient-panel` | `linear-gradient(180deg, rgba(255,255,255,0.02) 0%, transparent 100%)` | Panel sheen |
| `--gradient-toolbar` | `linear-gradient(180deg, rgba(255,255,255,0.04) 0%, rgba(255,255,255,0.01) 100%)` | Toolbar sheen |

### Tailwind `@theme` Integration

The `@theme` block maps to semantic vars:

```css
@theme {
  --color-surface-base: var(--surface-base);
  --color-surface-panel: var(--surface-panel);
  --color-surface-editor: var(--surface-editor);
  --color-surface-raised: var(--surface-raised);
  --color-surface-highlight: var(--surface-highlight);
  --color-surface-selection: var(--surface-selection);
  --color-accent: var(--accent);
  --color-accent-hover: var(--accent-hover);
  --color-accent-subtle: var(--accent-subtle);
  /* ...etc */
}
```

Components use Tailwind classes like `bg-surface-panel`, `text-accent`, `border-default`.

### What Gets Removed

- All legacy `--bg-*` variables
- All `--font-ui`, `--font-size-*` variables
- The `graphite-*` and `coral-*` color scales from `@theme` (replaced by semantic tokens)
- The `primary-*` Flowbite mappings
- All `--color-surface-*` duplicates currently in `@theme` (they move to `:root`)

## 2. Typography Scale

### Problem

5+ arbitrary pixel sizes scattered across components: `text-[10px]`, `text-[11px]`, `text-[12px]`, `text-[13px]`, `text-[14px]`.

### Solution

4 semantic steps defined as CSS variables and Tailwind utilities:

| Token | Size | Usage |
|-------|------|-------|
| `--text-xs` / `text-xs` | 11px | Badges, counters, status indicators |
| `--text-sm` / `text-sm` | 12px | Labels, tree items, secondary UI |
| `--text-base` / `text-base` | 14px | Body text, annotation content, inputs |
| `--text-lg` / `text-lg` | 16px | Dialog titles, section headings |

### Migration

- `text-[10px]` → `text-xs`
- `text-[11px]` → `text-xs`
- `text-[12px]` → `text-sm`
- `text-[13px]` → `text-base` (annotation body is content-level text)
- `text-[14px]` / `text-sm` (Tailwind default) → `text-base`

## 3. Component Strategy

### Drop Flowbite

Flowbite is used for 3 components (`Button`, `ButtonGroup`, `Kbd`), all with `!important` overrides. Remove the dependency entirely.

### Adopt Bits UI

Add `bits-ui` (^1.x) as a dependency. Bits UI provides headless, accessible, Svelte 5-native primitives. Adopt for:

| Bits UI Primitive | Replaces | Used In |
|-------------------|----------|---------|
| `Dialog` | Hand-rolled backdrop + focus trap | SettingsDialog, AnnotationPopover |
| `Context Menu` | Hand-rolled context menu | FileTree |
| `Tooltip` | CSS `.icon-tooltip` | FileTree icon buttons |
| `Toggle Group` | Flowbite `ButtonGroup` | FilterBar |

### Custom Atoms

New `src/components/ui/` directory for lightweight shared components:

**`Button.svelte`** — Variants: primary, secondary, ghost, danger. Sizes: sm, md.

| Variant | Styles |
|---------|--------|
| primary | `bg-accent text-surface-base font-semibold`, hover: `bg-accent-hover` |
| secondary | `bg-surface-raised text-secondary border border-subtle`, hover: `bg-surface-highlight` |
| ghost | `transparent text-secondary`, hover: `bg-surface-highlight` |
| danger | `text-danger`, hover: `bg-danger/10` |

| Size | Padding |
|------|---------|
| sm | `px-2.5 py-1 text-xs` |
| md | `px-3 py-1.5 text-sm` |

**`Kbd.svelte`** — Keyboard shortcut display. Styled with token colors, no Flowbite dependency.

**`IconButton.svelte`** — Replaces the `.icon-btn` pattern in FileTree. Wraps Bits UI Tooltip for hover labels.

## 4. Consistency Fixes

### Spacing

Standardize padding patterns across all components:

| Context | Padding |
|---------|---------|
| Panel header | `px-3 py-2` |
| Panel content area | `px-2 py-1.5` |
| Card padding | `px-3 py-2.5` |
| Input padding | `px-2.5 py-1.5` |
| Dialog padding | `p-5` |

### Focus & Selection

Unified across all interactive elements:

| State | Style |
|-------|-------|
| Focus ring | `ring-1 ring-accent/30` |
| Selection background | `bg-surface-selection` |
| Active indicator | `border-l-accent` (3px left border on cards/tree items) |

### Inline Hardcoded Values

Replace ~30 inline `rgba()` values in `style=` attributes with semantic tokens:

- `rgba(255,255,255,0.03)` → `var(--border-subtle)`
- `rgba(0,0,0,0.1)` → `var(--shadow-xs)`
- `box-shadow: inset -1px 0 0 rgba(...)` → `var(--border-subtle)` as border
- `box-shadow: 0 1px 4px rgba(...)` → `var(--shadow-xs)`

Where `style=` attributes remain necessary (dynamic widths, positions), use token references instead of raw values.

### Button Normalization

Currently primary buttons inconsistently use `bg-coral-400`, `bg-coral-500`, `bg-coral-400 text-graphite-950`, `bg-coral-500 text-white`. All primary buttons will use the `Button` atom with `variant="primary"`.

### Git Status Colors

Move from hardcoded Tailwind classes in `FileTreeItem.svelte` to semantic tokens:

```
M (modified)  → --color-warning (text-warning)
A (added)     → --color-success (text-success)
? (untracked) → --accent-teal (text-accent-teal)
D (deleted)   → --color-danger (text-danger)
R (renamed)   → --accent-purple (text-accent-purple)
```

## 5. CodeMirror Theme Alignment

Update `src/lib/codemirror/theme.ts` to reference the new semantic variables:

| Current | New |
|---------|-----|
| `var(--bg-editor)` | `var(--surface-editor)` |
| `var(--accent)` | `var(--accent)` (unchanged) |
| `var(--border-color)` | `var(--border-default)` |
| `var(--text-muted)` | `var(--text-muted)` (unchanged) |
| `var(--text-secondary)` | `var(--text-secondary)` (unchanged) |
| `var(--bg-selection)` | `var(--surface-selection)` |
| `rgba(34, 37, 43, 0.5)` | `var(--surface-highlight)` with opacity |
| `rgba(244, 122, 99, 0.10)` | `var(--accent-subtle)` |
| `rgba(244, 122, 99, 0.45)` | `var(--accent-annotation-border)` — new token, keeps rgba for CodeMirror JS compat |

## 6. File Structure

```
src/
  components/
    ui/                       ← new shared atoms
      Button.svelte
      Kbd.svelte
      IconButton.svelte
    Editor.svelte             ← unchanged
    FileTree.svelte           ← Bits UI ContextMenu + Tooltip, use IconButton
    FileTreeItem.svelte       ← token updates, semantic git status colors
    AnnotationSidebar.svelte  ← token updates, use Button atom
    AnnotationCard.svelte     ← token updates, unified selection style
    AnnotationPopover.svelte  ← Bits UI Dialog, use Button/Kbd atoms
    SettingsDialog.svelte     ← Bits UI Dialog, use Button atom
    FilterBar.svelte          ← Bits UI ToggleGroup replaces Flowbite ButtonGroup
    Toolbar.svelte            ← use IconButton atom
    ResizeHandle.svelte       ← token updates only
  app.css                     ← rebuilt: semantic :root vars + @theme mapping
  lib/
    codemirror/
      theme.ts                ← updated to use semantic vars
```

## Out of Scope

- Light theme implementation (tokens enable it; building it is a separate effort)
- Command bar / command palette (Bits UI Command primitive is available for future work)
- New features or functionality changes
- Rust/backend changes
