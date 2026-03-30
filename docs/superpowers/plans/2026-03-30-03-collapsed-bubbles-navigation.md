# Collapsed Inline Bubbles + Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Default inline annotation bubbles to collapsed state and add n/N keyboard navigation that auto-expands the focused bubble, auto-collapses the previous, and shows a position counter.

**Architecture:** Change the bubble expansion model from "manually expanded set" to "single focused line" — only one bubble expanded at a time. Add a `focusedBubbleLine` StateField that drives expansion. Navigation commands cycle through annotated lines, dispatch the focus effect, and scroll the editor. The AnnotationBubble component gains a `focused` prop to show the position counter.

**Tech Stack:** CodeMirror 6 StateEffects/StateFields, Svelte 5, TypeScript

---

### Task 1: Change Bubble Expansion to Single-Focus Model

**Files:**
- Modify: `src/lib/codemirror/bubbles.ts`

- [ ] **Step 1: Replace bubbleExpansionField with focusedBubbleLineField**

Replace the `bubbleExpansionField` (which tracks a Set of expanded lines) with a new field that tracks a single focused line number (or null for none expanded):

```typescript
// Replace toggleBubbleExpansionEffect with these two:
export const setFocusedBubbleEffect = StateEffect.define<number | null>();

const focusedBubbleLineField = StateField.define<number | null>({
  create() {
    return null; // All collapsed by default
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setFocusedBubbleEffect)) {
        // Toggle: if clicking the already-focused line, collapse it
        return effect.value === value ? null : effect.value;
      }
    }
    return value;
  },
});
```

- [ ] **Step 2: Update AnnotationBubbleWidget to use focusedBubbleLineField**

In the `bubbleDecorations` compute function, change:
```typescript
// Old:
const expandedLines = state.field(bubbleExpansionField);
// ...
expandedLines.has(lineNum),

// New:
const focusedLine = state.field(focusedBubbleLineField);
// ...
focusedLine === lineNum,
```

- [ ] **Step 3: Update the widget's onToggle callback**

In `AnnotationBubbleWidget.toDOM()`, change the toggle dispatch:
```typescript
onToggle: () => {
  activeViewRef.view?.dispatch({
    effects: setFocusedBubbleEffect.of(lineNum),
  });
},
```

- [ ] **Step 4: Update bubbleExtensions to use new field**

Replace `bubbleExpansionField` with `focusedBubbleLineField` in the extensions array.

- [ ] **Step 5: Update the compute deps**

In `bubbleDecorations`, update the deps array to reference `focusedBubbleLineField` instead of `bubbleExpansionField`.

- [ ] **Step 6: Export setFocusedBubbleEffect**

Remove the export of `toggleBubbleExpansionEffect` and ensure `setFocusedBubbleEffect` is exported (it's needed by Editor.svelte for navigation).

- [ ] **Step 7: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Bubbles now default to collapsed. Clicking a bubble expands it; clicking again collapses. Only one bubble can be expanded at a time.

- [ ] **Step 8: Commit**

```bash
git add src/lib/codemirror/bubbles.ts
git commit -m "feat: change bubble expansion to single-focus model — collapsed by default"
```

---

### Task 2: Add Navigation Effects and Commands

**Files:**
- Modify: `src/lib/codemirror/bubbles.ts` (add navigation helper)
- Modify: `src/lib/commands.ts` (register navigation commands)
- Modify: `src/lib/controllers/appShell.svelte.ts` (add n/N keyboard handling)

- [ ] **Step 1: Add navigation helper to bubbles.ts**

Add a function that computes the next/previous annotated line given the current focused line:

```typescript
export function getAnnotatedLines(state: EditorState): number[] {
  const annotations = state.field(annotationsField);
  const kindFilter = state.field(bubbleKindFilterField);
  const filtered = annotations.filter((a) => !a.replyTo && kindFilter.has(a.kind));
  const lines = [...new Set(filtered.map((a) => a.anchor.range.startLine))].sort((a, b) => a - b);
  return lines;
}

export function getFocusedBubbleLine(state: EditorState): number | null {
  return state.field(focusedBubbleLineField);
}
```

- [ ] **Step 2: Add navigation commands to commands.ts**

Add two new commands:

```typescript
{
  id: "annotations.next",
  title: "Next Annotation",
  section: "Annotations",
  keywords: ["next", "annotation", "forward"],
  shortcut: ["n"],
  closeOnRun: true,
  isEnabled: (ctx) => ctx.hasAnnotations(),
  run: (ctx) => ctx.navigateAnnotation(1),
},
{
  id: "annotations.previous",
  title: "Previous Annotation",
  section: "Annotations",
  keywords: ["previous", "annotation", "back"],
  shortcut: ["Shift", "n"],
  closeOnRun: true,
  isEnabled: (ctx) => ctx.hasAnnotations(),
  run: (ctx) => ctx.navigateAnnotation(-1),
},
```

Add `navigateAnnotation` to the `AppCommandContext` interface:
```typescript
navigateAnnotation: (direction: 1 | -1) => void;
```

- [ ] **Step 3: Implement navigateAnnotation in appShell.svelte.ts**

```typescript
function navigateAnnotation(direction: 1 | -1) {
  const editorView = editorRef()?.getView?.();
  if (!editorView) return;

  const lines = getAnnotatedLines(editorView.state);
  if (lines.length === 0) return;

  const currentFocused = getFocusedBubbleLine(editorView.state);
  let nextIndex: number;

  if (currentFocused === null) {
    // Nothing focused yet — go to first (forward) or last (backward)
    nextIndex = direction === 1 ? 0 : lines.length - 1;
  } else {
    const currentIndex = lines.indexOf(currentFocused);
    if (currentIndex === -1) {
      nextIndex = direction === 1 ? 0 : lines.length - 1;
    } else {
      nextIndex = currentIndex + direction;
      // Wrap around
      if (nextIndex >= lines.length) nextIndex = 0;
      if (nextIndex < 0) nextIndex = lines.length - 1;
    }
  }

  const targetLine = lines[nextIndex];

  // Focus the bubble (expands it, collapses previous)
  editorView.dispatch({
    effects: setFocusedBubbleEffect.of(targetLine),
  });

  // Scroll to the line
  const lineObj = editorView.state.doc.line(Math.min(targetLine, editorView.state.doc.lines));
  editorView.dispatch({
    effects: EditorView.scrollIntoView(lineObj.from, { y: "center" }),
  });

  // Select annotation in sidebar
  const annotations = editorView.state.field(annotationsField);
  const rootOnLine = annotations.find(
    (a) => !a.replyTo && a.anchor.range.startLine === targetLine
  );
  if (rootOnLine) {
    selectAnnotation(rootOnLine.id);
  }
}
```

- [ ] **Step 4: Add n/N keyboard handling to appShell**

In the `handleKeydown` function, add (before the general command matching, after the `isShortcutInputTarget` check):

```typescript
// Annotation navigation — n/N keys (only when not in an input)
if (!ignoreGlobalShortcuts) {
  if (event.key === "n" && !event.metaKey && !event.ctrlKey && !event.altKey) {
    if (event.shiftKey) {
      navigateAnnotation(-1);
    } else {
      navigateAnnotation(1);
    }
    event.preventDefault();
    return;
  }
}
```

- [ ] **Step 5: Expose getView on Editor ref**

The Editor component's ref interface needs to expose the raw CodeMirror EditorView so appShell can call `getAnnotatedLines()` and dispatch effects. Check if `getView` is already exposed. If not, add it:

```typescript
// In Editor.svelte's ref:
getView: () => view,
```

- [ ] **Step 6: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 7: Commit**

```bash
git add src/lib/codemirror/bubbles.ts src/lib/commands.ts src/lib/controllers/appShell.svelte.ts src/components/Editor.svelte
git commit -m "feat: add n/N annotation navigation — auto-expand focused, scroll into view"
```

---

### Task 3: Add Position Counter to Focused Bubble

**Files:**
- Modify: `src/components/AnnotationBubble.svelte`
- Modify: `src/lib/codemirror/bubbles.ts` (pass position info to widget)

- [ ] **Step 1: Add focusPosition prop to AnnotationBubble**

```typescript
let {
  annotations,
  expanded = false,
  focusPosition, // New: { current: number, total: number } | null
  onToggle,
  onSelect,
  onDelete,
  onChoiceToggle,
}: {
  annotations: Annotation[];
  expanded?: boolean;
  focusPosition?: { current: number; total: number } | null;
  onToggle: () => void;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onChoiceToggle: (annotationId: string, choiceIndex: number) => void;
} = $props();
```

- [ ] **Step 2: Show position counter in expanded view**

At the bottom of the expanded thread, add a footer with the counter and navigation hints:

```svelte
{#if expanded && focusPosition}
  <div class="rp-bubble-nav-footer">
    <span class="rp-bubble-nav-position">◀ {focusPosition.current}/{focusPosition.total}</span>
    <span class="rp-bubble-nav-hint">
      <kbd>n</kbd> next · <kbd>N</kbd> prev
    </span>
  </div>
{/if}
```

- [ ] **Step 3: Compute focusPosition in the widget**

In `bubbleDecorations` compute function, calculate position data:

```typescript
// After getting sortedLines:
const totalAnnotatedLines = sortedLines.length;

for (const [lineNum, group] of sortedLines) {
  const lineIndex = sortedLines.findIndex(([l]) => l === lineNum);
  const isFocused = focusedLine === lineNum;

  const widget = new AnnotationBubbleWidget(
    group,
    lineNum,
    isFocused,
    callbacks,
    isFocused ? { current: lineIndex + 1, total: totalAnnotatedLines } : null,
  );
  // ...
}
```

Update the `AnnotationBubbleWidget` constructor and `toDOM()` to pass `focusPosition` through:

```typescript
constructor(
  readonly annotations: Annotation[],
  readonly lineNum: number,
  readonly expanded: boolean,
  readonly callbacks: BubbleCallbacks,
  readonly focusPosition: { current: number; total: number } | null,
) {
  super();
}
```

And in `toDOM()`:
```typescript
props: {
  annotations: this.annotations,
  expanded: this.expanded,
  focusPosition: this.focusPosition,
  // ...other props
},
```

Update `eq()` to compare `focusPosition`:
```typescript
eq(other: AnnotationBubbleWidget) {
  if (this.annotations.length !== other.annotations.length) return false;
  if (this.expanded !== other.expanded) return false;
  if (this.focusPosition?.current !== other.focusPosition?.current) return false;
  if (this.focusPosition?.total !== other.focusPosition?.total) return false;
  // ...rest
}
```

- [ ] **Step 4: Add CSS for the navigation footer**

In `src/lib/codemirror/theme.ts`, add styles:

```typescript
".rp-bubble-nav-footer": {
  display: "flex",
  alignItems: "center",
  gap: "6px",
  marginTop: "8px",
  paddingTop: "6px",
  borderTop: "1px solid var(--border-subtle)",
},
".rp-bubble-nav-position": {
  color: "var(--accent)",
  fontSize: "10px",
  fontWeight: "500",
},
".rp-bubble-nav-hint": {
  marginLeft: "auto",
  color: "var(--text-ghost)",
  fontSize: "10px",
},
".rp-bubble-nav-hint kbd": {
  background: "var(--surface-raised)",
  border: "1px solid var(--border-default)",
  borderRadius: "3px",
  padding: "0 4px",
  fontSize: "10px",
  fontFamily: "inherit",
},
```

- [ ] **Step 5: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Focused bubble shows "◀ 2/6" counter with "n next · N prev" hint.

- [ ] **Step 6: Commit**

```bash
git add src/components/AnnotationBubble.svelte src/lib/codemirror/bubbles.ts src/lib/codemirror/theme.ts
git commit -m "feat: show position counter and keyboard hints on focused annotation bubble"
```

---

### Task 4: Update Collapsed Bubble Styling

**Files:**
- Modify: `src/lib/codemirror/theme.ts`

- [ ] **Step 1: Restyle collapsed bubbles for surface elevation**

Update the collapsed bubble styles to match the design spec — surface elevation instead of bordered boxes:

```typescript
// Replace the .rp-bubble-container styles:
".rp-bubble-container": {
  maxWidth: "560px",
  background: "var(--surface-panel)",
  borderLeft: "3px solid var(--accent-annotation-border)",
  borderRadius: "4px",
  padding: "4px 10px",
  fontSize: "12px",
  fontFamily: "var(--font-sans, system-ui, sans-serif)",
  color: "var(--text-primary)",
  cursor: "pointer",
  transition: "box-shadow 150ms, border-color 150ms",
  position: "relative",
},
".rp-bubble-container:hover": {
  boxShadow: "0 2px 8px rgba(0, 0, 0, 0.25)",
},
// Expanded state gets more prominence
".rp-bubble-container:not(.rp-bubble-collapsed)": {
  padding: "10px 12px",
  fontSize: "13px",
  borderLeft: "3px solid var(--accent)",
  boxShadow: "0 0 0 1px var(--border-subtle), 0 4px 12px rgba(0, 0, 0, 0.3)",
},
```

Also remove the notch arrow (it's visual noise for collapsed bubbles):
```typescript
".rp-bubble-notch": {
  display: "none",  // Remove notch — surface elevation does the separation
},
```

Update kind-specific collapsed styles to only change the left border color:
```typescript
".rp-bubble-kind-explanation": {
  borderLeftColor: "var(--kind-explanation)",
},
".rp-bubble-kind-explanation:not(.rp-bubble-collapsed)": {
  borderColor: "var(--kind-explanation-border)",
  background: "color-mix(in srgb, var(--kind-explanation-subtle) 60%, var(--surface-raised))",
},
// ... same pattern for linenote and label
```

- [ ] **Step 2: Reduce collapsed bubble padding**

The widget wrapper also needs tighter padding for collapsed state:
```typescript
".rp-bubble-widget": {
  padding: "3px 0 3px 56px",  // Reduced from 6px
  position: "relative",
},
```

- [ ] **Step 3: Build and verify**

Run: `npm run build`
Expected: Build succeeds. Collapsed bubbles are compact single-line elements with subtle left border. Expanded bubbles have more padding, shadow, and prominence.

- [ ] **Step 4: Commit**

```bash
git add src/lib/codemirror/theme.ts
git commit -m "feat: restyle collapsed bubbles — surface elevation, tighter padding, no notch"
```
