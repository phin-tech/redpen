import { EditorView } from "@codemirror/view";

export const redPenTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px",
    fontFamily: "var(--font-mono)",
    backgroundColor: "var(--surface-editor)",
  },
  ".cm-content": {
    padding: "8px 0",
    caretColor: "var(--accent)",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--accent)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--surface-editor)",
    borderRight: "1px solid var(--border-default)",
    color: "var(--text-muted)",
    fontSize: "12px",
    minWidth: "48px",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    opacity: "0.5",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: "var(--text-secondary)",
    opacity: "1",
  },
  ".cm-activeLine": {
    backgroundColor: "color-mix(in srgb, var(--surface-highlight) 50%, transparent)",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--surface-selection) !important",
  },
  // Annotation highlight (default / comment)
  ".rp-annotation": {
    backgroundColor: "var(--kind-comment-subtle)",
    borderBottom: "2px solid var(--kind-comment-border)",
  },
  ".rp-annotation-orphaned": {
    backgroundColor: "var(--orphaned-subtle)",
    borderBottom: "2px solid var(--orphaned-border)",
  },
  // Kind-specific annotation highlights
  ".rp-annotation-explanation": {
    backgroundColor: "var(--kind-explanation-subtle)",
    borderBottom: "2px solid var(--kind-explanation-border)",
  },
  ".rp-annotation-linenote": {
    backgroundColor: "var(--kind-linenote-subtle)",
    borderBottom: "2px solid var(--kind-linenote-border)",
  },
  ".rp-annotation-label": {
    backgroundColor: "var(--kind-label-subtle)",
    borderBottom: "2px solid var(--kind-label-border)",
  },
  ".rp-annotation-question": {
    backgroundColor: "var(--kind-question-subtle)",
    borderBottom: "2px solid var(--kind-question-border)",
  },
  // Gutter markers
  ".rp-annotation-gutter": {
    width: "14px",
  },
  ".rp-gutter-marker": {
    width: "3px",
    height: "100%",
    display: "block",
    backgroundColor: "var(--kind-comment-border)",
    borderRadius: "1px",
    marginLeft: "5px",
    transition: "background-color 150ms",
  },
  ".rp-gutter-marker:hover": {
    backgroundColor: "var(--kind-comment)",
  },
  ".rp-gutter-marker-orphaned": {
    backgroundColor: "var(--orphaned-border)",
  },
  ".rp-gutter-marker-explanation": {
    backgroundColor: "var(--kind-explanation-border)",
  },
  ".rp-gutter-marker-explanation:hover": {
    backgroundColor: "var(--kind-explanation)",
  },
  ".rp-gutter-marker-linenote": {
    backgroundColor: "var(--kind-linenote-border)",
  },
  ".rp-gutter-marker-linenote:hover": {
    backgroundColor: "var(--kind-linenote)",
  },
  ".rp-gutter-marker-label": {
    backgroundColor: "var(--kind-label-border)",
  },
  ".rp-gutter-marker-label:hover": {
    backgroundColor: "var(--kind-label)",
  },
  ".rp-gutter-marker-question": {
    backgroundColor: "var(--kind-question-border)",
  },
  ".rp-gutter-marker-question:hover": {
    backgroundColor: "var(--kind-question)",
  },
  // Search highlights
  ".rp-search-match": {
    backgroundColor: "var(--search-match-bg)",
    borderBottom: "2px solid var(--search-match-border)",
  },
  ".rp-search-current": {
    backgroundColor: "var(--search-current-bg)",
    borderBottom: "2px solid var(--search-current-border)",
  },
  // Annotation bubbles
  ".rp-bubble-widget": {
    padding: "3px 0 3px 56px",
    position: "relative",
  },
  ".rp-bubble-container": {
    maxWidth: "560px",
    background: "var(--surface-panel)",
    border: "none",
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
  ".rp-bubble-container:not(.rp-bubble-collapsed)": {
    padding: "10px 12px",
    fontSize: "13px",
    borderLeft: "3px solid var(--accent)",
    boxShadow: "0 0 0 1px var(--border-subtle), 0 4px 12px rgba(0, 0, 0, 0.3)",
  },
  ".rp-bubble-orphaned": {
    borderLeftColor: "var(--color-danger)",
    opacity: "0.7",
  },
  ".rp-bubble-orphaned:not(.rp-bubble-collapsed)": {
    background: "color-mix(in srgb, var(--orphaned-subtle) 60%, var(--surface-raised))",
  },
  ".rp-bubble-kind-explanation": {
    borderLeftColor: "var(--kind-explanation)",
  },
  ".rp-bubble-kind-explanation:not(.rp-bubble-collapsed)": {
    background: "color-mix(in srgb, var(--kind-explanation-subtle) 60%, var(--surface-raised))",
  },
  ".rp-bubble-kind-linenote": {
    borderLeftColor: "var(--kind-linenote)",
  },
  ".rp-bubble-kind-linenote:not(.rp-bubble-collapsed)": {
    background: "color-mix(in srgb, var(--kind-linenote-subtle) 60%, var(--surface-raised))",
  },
  ".rp-bubble-kind-label": {
    borderLeftColor: "var(--kind-label)",
  },
  ".rp-bubble-kind-label:not(.rp-bubble-collapsed)": {
    background: "color-mix(in srgb, var(--kind-label-subtle) 60%, var(--surface-raised))",
  },
  ".rp-bubble-kind-question": {
    borderLeftColor: "var(--kind-question)",
  },
  ".rp-bubble-kind-question:not(.rp-bubble-collapsed)": {
    background: "color-mix(in srgb, var(--kind-question-subtle) 60%, var(--surface-raised))",
  },
  ".rp-bubble-notch": {
    display: "none",
  },
  // Bubble collapsed summary
  ".rp-bubble-summary": {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    overflow: "hidden",
  },
  ".rp-bubble-collapsed .rp-bubble-body": {
    whiteSpace: "nowrap",
    overflow: "hidden",
    textOverflow: "ellipsis",
    flex: "1",
    minWidth: "0",
    color: "var(--text-secondary)",
  },
  // Expanded bubble body wrapping
  ".rp-bubble-thread .rp-bubble-body": {
    whiteSpace: "pre-wrap",
    wordBreak: "break-word",
    color: "var(--text-secondary)",
    lineHeight: "1.5",
  },
  ".rp-bubble-author": {
    fontSize: "12px",
    fontWeight: "600",
    color: "var(--text-primary)",
    flexShrink: "0",
    display: "flex",
    alignItems: "center",
    gap: "4px",
  },
  ".rp-bubble-agent-icon": {
    width: "14px",
    height: "14px",
    color: "var(--kind-explanation)",
    flexShrink: "0",
  },
  ".rp-bubble-reply-count": {
    fontSize: "11px",
    color: "var(--text-muted)",
    flexShrink: "0",
    padding: "1px 6px",
    borderRadius: "8px",
    background: "var(--surface-highlight)",
  },
  ".rp-bubble-label": {
    fontSize: "11px",
    padding: "1px 6px",
    borderRadius: "9999px",
    background: "var(--surface-panel)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-default)",
    flexShrink: "0",
  },
  ".rp-bubble-orphan-badge": {
    fontSize: "10px",
    fontWeight: "700",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    color: "var(--color-danger)",
    flexShrink: "0",
  },
  // Bubble expanded thread
  ".rp-bubble-thread": {
    display: "flex",
    flexDirection: "column",
    gap: "8px",
  },
  ".rp-bubble-item": {
    cursor: "pointer",
  },
  ".rp-bubble-reply": {
    paddingLeft: "12px",
    borderLeft: "2px solid var(--border-default)",
  },
  ".rp-bubble-header": {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    marginBottom: "4px",
  },
  ".rp-bubble-reply-indicator": {
    fontSize: "12px",
    color: "var(--text-muted)",
  },
  ".rp-bubble-delete": {
    marginLeft: "auto",
    background: "none",
    border: "none",
    color: "var(--text-secondary)",
    cursor: "pointer",
    fontSize: "16px",
    lineHeight: "1",
    opacity: "0",
    transition: "opacity 150ms, color 150ms",
    padding: "0 2px",
  },
  ".rp-bubble-item:hover .rp-bubble-delete": {
    opacity: "1",
  },
  ".rp-bubble-delete:hover": {
    color: "var(--color-danger)",
  },
  ".rp-bubble-labels": {
    display: "flex",
    flexWrap: "wrap",
    gap: "4px",
    marginTop: "4px",
  },
  // Annotation choices
  ".rp-bubble-choices": {
    display: "flex",
    flexDirection: "column",
    gap: "4px",
    marginTop: "8px",
  },
  ".rp-bubble-choice": {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "5px 10px",
    borderRadius: "6px",
    cursor: "pointer",
    fontSize: "13px",
    color: "var(--text-secondary)",
    background: "var(--surface-panel)",
    border: "1px solid var(--border-default)",
    transition: "background 100ms, border-color 100ms, color 100ms",
  },
  ".rp-bubble-choice:hover": {
    background: "var(--surface-highlight)",
    borderColor: "var(--border-emphasis)",
  },
  ".rp-bubble-choice-selected": {
    background: "var(--accent-subtle)",
    borderColor: "var(--accent)",
    color: "var(--text-primary)",
  },
  ".rp-bubble-choice input": {
    margin: "0",
    accentColor: "var(--accent)",
    cursor: "pointer",
  },
  // Bubble navigation footer
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
  // Diff highlights
  ".rp-diff-added": {
    backgroundColor: "var(--diff-added-bg)",
  },
  ".rp-diff-added .cm-gutterElement": {
    color: "var(--diff-added-fg)",
  },
  ".rp-diff-removed": {
    backgroundColor: "var(--diff-removed-bg)",
  },
  ".rp-diff-removed .cm-gutterElement": {
    color: "var(--diff-removed-fg)",
  },
  ".rp-diff-ghost": {
    backgroundColor: "var(--diff-ghost-bg)",
  },
});
