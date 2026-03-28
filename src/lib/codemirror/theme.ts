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
    backgroundColor: "rgba(217, 107, 95, 0.12)",
    borderBottom: "2px solid rgba(217, 107, 95, 0.5)",
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
    backgroundColor: "rgba(217, 107, 95, 0.5)",
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
  // Search highlights
  ".rp-search-match": {
    backgroundColor: "rgba(251, 191, 36, 0.2)",
    borderBottom: "2px solid rgba(251, 191, 36, 0.45)",
  },
  ".rp-search-current": {
    backgroundColor: "rgba(251, 191, 36, 0.45)",
    borderBottom: "2px solid rgba(251, 191, 36, 0.9)",
  },
  // Annotation bubbles
  ".rp-bubble-widget": {
    padding: "6px 0 6px 56px",
    position: "relative",
  },
  ".rp-bubble-container": {
    maxWidth: "560px",
    background: "var(--surface-raised)",
    border: "1px solid var(--border-default)",
    borderLeft: "3px solid var(--accent-annotation-border)",
    borderRadius: "6px",
    padding: "8px 12px",
    fontSize: "13px",
    fontFamily: "var(--font-sans, system-ui, sans-serif)",
    color: "var(--text-primary)",
    boxShadow: "var(--shadow-card)",
    cursor: "pointer",
    transition: "box-shadow 150ms, transform 150ms",
    position: "relative",
  },
  ".rp-bubble-container:hover": {
    boxShadow: "var(--shadow-card-hover, 0 2px 8px rgba(0,0,0,0.3))",
    transform: "translateY(-0.5px)",
  },
  ".rp-bubble-orphaned": {
    borderLeftColor: "var(--color-danger, #D96B5F)",
    opacity: "0.7",
  },
  ".rp-bubble-kind-explanation": {
    borderLeftColor: "var(--kind-explanation-border)",
  },
  ".rp-bubble-kind-linenote": {
    borderLeftColor: "var(--kind-linenote-border)",
  },
  ".rp-bubble-kind-label": {
    borderLeftColor: "var(--kind-label-border)",
  },
  ".rp-bubble-notch": {
    position: "absolute",
    top: "-6px",
    left: "16px",
    width: "0",
    height: "0",
    borderLeft: "6px solid transparent",
    borderRight: "6px solid transparent",
    borderBottom: "6px solid var(--border-default)",
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
  ".rp-bubble-author": {
    fontSize: "12px",
    fontWeight: "600",
    color: "var(--text-primary)",
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
    color: "var(--color-danger, #D96B5F)",
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
    color: "var(--color-danger, #D96B5F)",
  },
  ".rp-bubble-labels": {
    display: "flex",
    flexWrap: "wrap",
    gap: "4px",
    marginTop: "4px",
  },
  // Diff highlights
  ".rp-diff-added": {
    backgroundColor: "rgba(63, 185, 80, 0.15)",
  },
  ".rp-diff-added .cm-gutterElement": {
    color: "#7ee787",
  },
  ".rp-diff-removed": {
    backgroundColor: "rgba(248, 81, 73, 0.15)",
  },
  ".rp-diff-removed .cm-gutterElement": {
    color: "#ffa198",
  },
  ".rp-diff-ghost": {
    backgroundColor: "rgba(110, 118, 129, 0.05)",
  },
});
