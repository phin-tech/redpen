# UI Overhaul Design Spec

## Goal

Modernize Red Pen's interface to reduce visual clutter, improve information density, and create a premium dark-theme aesthetic. Six coordinated changes that touch every major surface of the app.

## Scope

This is a large surface-area change. It decomposes into six independent sub-projects that can be implemented and shipped sequentially:

1. **Theme & Color System** — foundation that all other changes build on
2. **Header Consolidation** — merge two-row header into single row
3. **Collapsed Inline Bubbles + Navigation** — default-collapsed annotations with n/N jumping
4. **Collapsible Sidebars** — toggle left/right panels with keyboard shortcuts
5. **Review Page Overhaul** — proximity merging, tighter cards, hover-sync
6. **Launch Screen** — blank-slate landing page with hero PR input

Each sub-project gets its own implementation plan task group. They share the theme/color foundation but are otherwise independent.

---

## 1. Theme & Color System

### Surface Elevation (replaces borders)

| Token | Hex | Usage |
|-------|-----|-------|
| `--surface-base` | `#0D1117` | App background, editor background |
| `--surface-elevated` | `#161B22` | Cards, sidebars, collapsed bubbles, pill groups |
| `--surface-overlay` | `#1C2129` | Popovers, modals, dropdowns |

**Rule:** Use 1px of background-color difference instead of colored borders. Borders are reserved for interactive focus states only.

### Tiered Accent System

| Tier | Treatment | Usage |
|------|-----------|-------|
| **Primary action** | Solid amber fill (`#D9B15F` bg, dark text) | Submit review button only |
| **Active indicator** | 2px bottom border amber | Active tab underline |
| **Badge/count** | Outlined amber (text + border, transparent bg) | Review tab annotation count |
| **Subtle accent** | Left border 3px, 40% opacity amber | Collapsed bubble indicator |

### Ghost UI Opacity Scale

| Level | Opacity | Usage |
|-------|---------|-------|
| Ghostly | 0.15–0.2 | Timestamps, dividers, muted metadata |
| Dim | 0.3–0.4 | Secondary text, inactive tabs, placeholder |
| Readable | 0.6–0.7 | Body text, author names, descriptions |
| Bright | 0.85–0.9 | Primary text, headings, focused content |

### Semantic Colors

- **Success (Approve):** `#84C68D` — ghost button (text + icon only), never filled unless it's THE primary action
- **Danger (Request Changes):** `#dc6464` at 0.6 opacity — ghost button, clearly secondary
- **Unresolved dot:** amber `#D9B15F`
- **Resolved dot:** green `#84C68D`

### "Red Pen" Brand Consideration

Reserve a deep ink-red (`#C94040` or similar) for high-level alerts or unresolved markers in future iterations. Not in scope for this pass but noted for the color system.

---

## 2. Header Consolidation

### Current Problem

Two-row sandwich: ReviewWorkspaceHeader (PR context + actions) stacked above WorkspaceToolbar (view tabs + mode tools). Redundant "REVIEW" labels, grammar bugs ("1 files"), shortcut hints that look like glitches, actions scattered far from context.

### Design

**Single row** with three zones:

- **Left (flex:1, min-width:0, overflow:hidden):** PR context — `repo#number · PR title`. Title truncates first via text-overflow:ellipsis. Repo link is flex-shrink:0.
- **Center (flex-shrink:0):** View switcher pill group (`Code | Review [badge] | PR`). Active tab uses bottom-border underline, not fill. Badge always amber even in Code mode.
- **Right (flex-shrink:0):** Actions grouped. Secondary actions (Resync, Revert) are 28×28px icon-only buttons with tooltips. Primary action (Submit) is solid amber fill. Visual moat (12px gap) separates tabs from actions.

### Variants

- **Code view:** Right zone shows diff mode toggle (Split/Unified/Highlights) + Inline button
- **Local review:** No PR tab. Left shows "Agent change review · 3 files". Right shows ghost Approve (green text+icon) and Request Changes (red text+icon, dimmer)
- **No review context:** Header hidden entirely (just the workspace toolbar)

### Specific Fixes

- Pluralization: `${count} file${count === 1 ? '' : 's'}`
- ↩ revert icon replaces ✕ for discard (avoids "close window" confusion)
- Remove inline `<kbd>` shortcut hints from toolbar (keep in Help modal)
- Approve button: ghost style with checkmark icon, same padding as tab items (5px 12px)

---

## 3. Collapsed Inline Bubbles + Annotation Navigation

### Collapsed State (default)

- Single-line summary: `💬 author · truncated body [N replies]`
- Background: `--surface-elevated` (#161B22)
- Left border: 3px amber at 40% opacity
- Height: ~28px (vs current ~80px+ expanded)
- Click to expand

### Expanded State

- Full thread with replies inline (not hidden behind click-trap)
- Background: `--surface-elevated` with subtle box-shadow
- Left border: 3px solid amber
- Delete button, kind badge, timestamp visible
- Choices and labels shown

### Navigation (n/N keys)

- `n` jumps to next annotation, `N` to previous
- Current annotation auto-expands, previous auto-collapses
- Editor scrolls to center the focused annotation
- Position counter shown: `◀ 2/6` with `n next · N prev` hint
- State tracked via new `focusedBubbleLine` field in bubbles StateField
- `selectAnnotation()` called on focus for sidebar sync

### Implementation

- Change `bubbleExpansionField` default from empty Set to: only the focused line is expanded
- Add `setFocusedBubbleEffect` StateEffect
- Navigation commands added to `commands.ts`: `annotations.next`, `annotations.previous`
- Keyboard handling added to `appShell.svelte.ts` for `n`/`N` keys (only when not in input/editor)
- `AnnotationBubble.svelte` gains `focused` prop for the counter UI

---

## 4. Collapsible Sidebars

### Mechanism

- Toggle button in each panel header (◀/▶ arrow)
- Collapsed state = width 0px, same as existing split-diff auto-collapse
- Expand restores saved width (reuse `savedLeftPanelWidth` pattern, add `savedRightPanelWidth`)
- Keyboard: `Cmd+B` toggles left, `Cmd+Shift+B` toggles right

### State Persistence

- When navigating to a file from Review feed while tree is collapsed, the file tree should highlight and reveal that file when re-expanded
- This is already handled: `openFile()` sets `editor.currentFilePath` which FileTree uses for `selectedPath`

### Implementation

- Add `leftCollapsed`/`rightCollapsed` boolean state to App.svelte
- Toggle functions that save/restore width (same pattern as split-diff)
- Add commands to registry: `sidebar.toggleLeft`, `sidebar.toggleRight`
- Add shortcuts to `appShell.svelte.ts`
- FileTree header: add collapse arrow button
- AnnotationSidebar header: add collapse arrow button

---

## 5. Review Page Overhaul

### Smart Proximity Merging

**Rule:** If annotations A and B are within 5 lines of each other, they share one context block.

**Algorithm:**
1. Sort annotations by line number
2. Walk sorted list, start a new group when `currentLine - groupEndLine > 5`
3. Each group gets one shared snippet: `min(lines) - 1` to `max(lines) + 1`
4. Threads stacked below the snippet, each prefixed with `L{n}` tag

### Snippet Changes

- Show only the merged range (not whole function)
- Existing `h`/`l` expand/contract keys still work
- Annotated lines get dot indicators in the gutter: amber for unresolved, green for resolved

### Hover-Sync

- Hovering an `L{n}` tag in a thread highlights the corresponding line in the snippet above (CSS class toggle)
- Hovering a line in the snippet highlights the corresponding thread below

### Click-to-Reply

- Clicking anywhere in a thread's whitespace area activates the reply input for that thread
- Reply input hidden by default (no empty void), appears on click or `r` key

### Card Cleanup

- "Open file →" consolidated to file header only (removed from individual cards)
- Resolve button: proper button with text label, not tiny icon
- Reply input: hidden until focused
- Timestamps: relative ("2h ago") by default
- Surface elevation for cards, no heavy borders
- 10px margin between cards for breathing room

### Keyboard Navigation

- `j`/`k` navigates between threads within a merged block, then jumps to next block
- All existing review shortcuts preserved

---

## 6. Launch Screen

### Layout

- **No sidebars** when no session is active — full blank-slate centered layout
- **Hero PR input** top-center: large input with search icon, amber focus glow (1px amber border + subtle outer glow on focus)
- "or drag a folder anywhere" hint below
- **Two-column pathing** below hero:
  - Left: "Local Review" — Open Folder button + recent folders list
  - Right: "Recent Sessions" — history cards with ghost Resume buttons
- Subtle vertical divider between columns (1px rgba line)

### Progressive Disclosure

- If no history exists: just the hero input + Open Folder button in a tight centered cluster (no empty lists)
- Review queue only shown if it has items
- Subtle tip at bottom for first-time users

### Micro-Polish

- PR input: amber outer-glow on focus (`box-shadow: 0 0 0 2px rgba(217,177,95,0.2)`)
- Empty state: Open Folder button pulled up close to PR input (tight cluster)
- Red Pen logo/wordmark in top-left as home anchor

### Session Cards

- Surface elevation (#161B22) on base (#0D1117)
- No borders — just background difference
- Ghost "Resume" button (outline, no fill)
- PR sessions show repo#number in amber, title in dim text
- Local sessions show "Agent review" + verdict badge if completed
- Proper pluralization on file/annotation counts

---

## What's NOT in Scope

- Full color palette redesign (ink-red brand color, syntax highlighting overhaul) — noted for future pass
- Mobile/responsive breakpoints — desktop-only for now
- Accessibility audit beyond basic kbd styling
- Animation/transition polish beyond existing 150ms transitions
